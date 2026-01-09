use crate::json_path::JsonPath;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashSet;

/// Represents a change to a JSON value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added {
        path: JsonPath,
        value: Value,
    },
    Removed {
        path: JsonPath,
        value: Value,
    },
    Modified {
        path: JsonPath,
        old_value: Value,
        new_value: Value,
    },
}

/// Custom serialization for Change that converts JsonPath to String for JSON output
impl Serialize for Change {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Change::Added { path, value } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("path", &path.to_string())?;
                map.serialize_entry("value", value)?;
                map.end()
            }
            Change::Removed { path, value } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("path", &path.to_string())?;
                map.serialize_entry("value", value)?;
                map.end()
            }
            Change::Modified {
                path,
                old_value,
                new_value,
            } => {
                let mut map = serializer.serialize_map(Some(3))?;
                map.serialize_entry("path", &path.to_string())?;
                map.serialize_entry("oldValue", old_value)?;
                map.serialize_entry("newValue", new_value)?;
                map.end()
            }
        }
    }
}

/// Custom deserialization for Change that converts String to JsonPath
impl<'de> Deserialize<'de> for Change {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt::Formatter;

        struct ChangeVisitor;

        impl<'de> Visitor<'de> for ChangeVisitor {
            type Value = Change;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a change object with path, value, and/or oldValue/newValue")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut path = None;
                let mut value = None;
                let mut old_value = None;
                let mut new_value = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "path" => {
                            let path_str: String = map.next_value()?;
                            path = Some(path_str.parse::<JsonPath>().map_err(|_| {
                                serde::de::Error::custom(format!("invalid path: {}", path_str))
                            })?);
                        }
                        "value" => {
                            value = Some(map.next_value()?);
                        }
                        "oldValue" => {
                            old_value = Some(map.next_value()?);
                        }
                        "newValue" => {
                            new_value = Some(map.next_value()?);
                        }
                        _ => {
                            // Ignore unknown fields
                            let _ = map.next_value::<serde::de::IgnoredAny>();
                        }
                    }
                }

                let path = path.ok_or_else(|| serde::de::Error::missing_field("path"))?;

                // Determine the variant based on which fields are present
                match (old_value, new_value) {
                    (None, None) => {
                        let value =
                            value.ok_or_else(|| serde::de::Error::missing_field("value"))?;
                        Ok(Change::Added { path, value })
                    }
                    (Some(old), Some(new)) => Ok(Change::Modified {
                        path,
                        old_value: old,
                        new_value: new,
                    }),
                    (Some(old), None) => Ok(Change::Removed { path, value: old }),
                    (None, Some(_)) => Err(serde::de::Error::custom(
                        "newValue without oldValue is not allowed",
                    )),
                }
            }
        }

        deserializer.deserialize_map(ChangeVisitor)
    }
}

/// Container for all changes found during diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub added: Vec<Change>,
    pub removed: Vec<Change>,
    pub modified: Vec<Change>,
    #[serde(skip)]
    pub after: Option<Value>,
}

impl Changes {
    /// Create a new empty Changes container
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
            after: None,
        }
    }

    /// Add a change to the appropriate category
    pub fn push(&mut self, change: Change) {
        match change {
            Change::Added { .. } => self.added.push(change),
            Change::Removed { .. } => self.removed.push(change),
            Change::Modified { .. } => self.modified.push(change),
        }
    }

    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Filter out changes that match any of the ignore patterns
    pub fn filter_ignore_patterns(&self, patterns: &[String]) -> Self {
        let matcher = PatternMatcher::new(patterns);

        Self {
            added: self
                .added
                .iter()
                .filter(|c| !should_ignore_change(c, &matcher))
                .cloned()
                .collect(),
            removed: self
                .removed
                .iter()
                .filter(|c| !should_ignore_change(c, &matcher))
                .cloned()
                .collect(),
            modified: self
                .modified
                .iter()
                .filter(|c| !should_ignore_change(c, &matcher))
                .cloned()
                .collect(),
            after: self.after.clone(),
        }
    }
}

/// Pattern matcher that pre-computes all possible pattern prefixes for O(1) lookup
struct PatternMatcher {
    /// All possible prefixes for O(1) lookup
    /// Example: Pattern "user.profile" stores {"user", "user.profile"}
    prefixes: HashSet<String>,
}

impl PatternMatcher {
    /// Create a new PatternMatcher by parsing patterns and storing them
    fn new(patterns: &[String]) -> Self {
        let mut prefixes = HashSet::new();

        for pattern_str in patterns {
            // Convert JSON Pointer to dot notation if needed
            let dot_notation = if pattern_str.starts_with('/') {
                json_pointer_to_dot_notation(pattern_str)
            } else {
                pattern_str.clone()
            };

            // Store the full pattern string
            prefixes.insert(dot_notation);
        }

        Self { prefixes }
    }

    /// Check if a path should be ignored (matches any pattern prefix)
    fn should_ignore(&self, path: &JsonPath) -> bool {
        // Check if any prefix of this path matches a pattern in our set
        // This implements the same logic as before: a path is ignored if
        // any pattern matches exactly or is a prefix of the path
        for i in 1..=path.len() {
            if let Some(prefix) = path.prefix(i) {
                let prefix_str = prefix.to_string();
                // Check if this prefix is in our pattern set
                if self.prefixes.contains(&prefix_str) {
                    return true;
                }
            }
        }
        false
    }
}

/// Convert a JSON Pointer path to dot notation
/// Example: "/user/id/0/name" -> "user.id[0].name"
fn json_pointer_to_dot_notation(ptr: &str) -> String {
    let mut result = String::new();
    let parts: Vec<&str> = ptr.split('/').filter(|s| !s.is_empty()).collect();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            result.push('.');
        }
        // Check if part is a numeric array index
        if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            result.push('[');
            result.push_str(part);
            result.push(']');
        } else {
            result.push_str(part);
        }
    }

    result
}

/// Check if a change should be ignored using the pattern matcher
fn should_ignore_change(change: &Change, matcher: &PatternMatcher) -> bool {
    let path = match change {
        Change::Added { path, .. } => path,
        Change::Removed { path, .. } => path,
        Change::Modified { path, .. } => path,
    };

    matcher.should_ignore(path)
}

impl Default for Changes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pattern_matching_with_json_pointer() {
        let patterns = vec!["/user/id".to_string(), "/tags".to_string()];
        let matcher = PatternMatcher::new(&patterns);

        // Test that converted patterns match dot notation paths
        let user_id_path: JsonPath = "user.id".parse().unwrap();
        assert!(matcher.should_ignore(&user_id_path));

        let tags_path: JsonPath = "tags".parse().unwrap();
        assert!(matcher.should_ignore(&tags_path));

        let user_name_path: JsonPath = "user.name".parse().unwrap();
        assert!(!matcher.should_ignore(&user_name_path));
    }

    #[test]
    fn test_filter_ignore_patterns_with_json_path() {
        let mut changes = Changes::new();

        changes.push(Change::Modified {
            path: "user.id".parse().unwrap(),
            old_value: json!(1),
            new_value: json!(2),
        });

        changes.push(Change::Modified {
            path: "user.name".parse().unwrap(),
            old_value: json!("John"),
            new_value: json!("Jane"),
        });

        // Filter out user.id
        let patterns = vec!["/user/id".to_string()];
        let filtered = changes.filter_ignore_patterns(&patterns);

        assert_eq!(filtered.modified.len(), 1);
        if let Change::Modified { path, .. } = &filtered.modified[0] {
            assert_eq!(path.to_string(), "user.name");
        } else {
            panic!("Expected Modified change");
        }
    }
}
