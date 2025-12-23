use crate::formatter::Formatter;
use crate::types::{Change, Changes};
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Formatter for the "after" output format
///
/// This formatter outputs the "after" state (file2) but only includes
/// properties that were added or modified compared to file1.
pub struct AfterFormatter {
    pretty: bool,
}

impl AfterFormatter {
    /// Create a new AfterFormatter with pretty printing enabled
    pub fn new() -> Self {
        Self { pretty: true }
    }

    /// Create an AfterFormatter with custom pretty printing setting
    #[allow(dead_code)]
    pub fn with_pretty(pretty: bool) -> Self {
        Self { pretty }
    }
}

impl Default for AfterFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for AfterFormatter {
    fn format(&self, changes: &Changes) -> Result<String, Box<dyn std::error::Error>> {
        // Get the "after" value
        let after_value = match &changes.after {
            Some(value) => value,
            None => {
                // If no "after" value is available, return empty object
                let empty = Value::Object(Map::new());
                return if self.pretty {
                    Ok(serde_json::to_string_pretty(&empty)?)
                } else {
                    Ok(serde_json::to_string(&empty)?)
                };
            }
        };

        // Collect all changed paths (added and modified only)
        let mut changed_paths = HashSet::new();

        for change in &changes.added {
            if let Change::Added { path, .. } = change {
                changed_paths.insert(path.clone());
            }
        }

        for change in &changes.modified {
            if let Change::Modified { path, .. } = change {
                changed_paths.insert(path.clone());
            }
        }

        // Build the filtered "after" value
        let filtered_after = build_filtered_value(after_value, &changed_paths);

        // Serialize to JSON
        if self.pretty {
            Ok(serde_json::to_string_pretty(&filtered_after)?)
        } else {
            Ok(serde_json::to_string(&filtered_after)?)
        }
    }
}

/// Build a filtered value containing only the paths in the changed_paths set
fn build_filtered_value(value: &Value, changed_paths: &HashSet<String>) -> Value {
    // Special case: if value is a primitive or the changed paths set is empty
    if !value.is_object() && !value.is_array() {
        return value.clone();
    }

    if changed_paths.is_empty() {
        return Value::Null;
    }

    // Group paths by their first segment
    let mut root_map = Map::new();

    for path in changed_paths {
        if path.is_empty() {
            continue;
        }

        insert_value_at_path(&mut root_map, &path, value, &path);
    }

    Value::Object(root_map)
}

/// Insert a value at the given path into the target map
fn insert_value_at_path(
    target: &mut Map<String, Value>,
    path: &str,
    source_value: &Value,
    original_path: &str,
) {
    if path.is_empty() {
        return;
    }

    // Parse the first segment
    let (first_segment, is_array, remaining_path) = parse_first_segment(path);

    // Check if this is the final segment
    let is_final = remaining_path.is_empty();

    // Get or create the value at the first segment in the target
    let target_entry = if is_final {
        // Final segment - ensure it exists and get a mutable reference
        target.entry(first_segment.clone()).or_insert(Value::Null)
    } else {
        // Intermediate segment - create appropriate container
        target.entry(first_segment.clone()).or_insert_with(|| {
            if is_array {
                Value::Array(Vec::new())
            } else {
                Value::Object(Map::new())
            }
        })
    };

    // If we've reached the final segment, update with the actual value from source
    if is_final {
        if let Some(value) = get_value_at_path(source_value, original_path) {
            *target_entry = value;
        }
        return;
    }

    // Recursively insert into the next level
    match target_entry {
        Value::Object(map) => {
            insert_value_at_path(map, remaining_path, source_value, original_path);
        }
        Value::Array(arr) => {
            let (index, rest) = parse_array_index(remaining_path);
            ensure_array_length(arr, index);
            let next_map = if arr[index].is_object() {
                arr[index].as_object_mut().unwrap()
            } else {
                // Replace with a new object
                arr[index] = Value::Object(Map::new());
                arr[index].as_object_mut().unwrap()
            };
            insert_value_at_path(next_map, rest, source_value, original_path);
        }
        _ => {
            // Type mismatch - replace with appropriate container
            let (_, next_is_array, _) = parse_first_segment(remaining_path);
            if next_is_array {
                *target_entry = Value::Array(Vec::new());
            } else {
                *target_entry = Value::Object(Map::new());
            }
            // Retry insertion
            insert_value_at_path(
                target_entry.as_object_mut().unwrap(),
                remaining_path,
                source_value,
                original_path,
            );
        }
    }
}

/// Get the value at a specific path from a source value
fn get_value_at_path(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }

    // First, try to get the value treating the entire path as a single key
    // This handles flat JSON structures where dots are part of key names
    if let Value::Object(map) = value {
        if let Some(value_at_path) = map.get(path) {
            return Some(value_at_path.clone());
        }
    }

    // If that fails, try parsing as a nested path
    // Check if path contains array index notation
    if let Some(dot_pos) = path.find('.') {
        // Check if this segment has array notation
        let (key, rest) = (&path[..dot_pos], &path[dot_pos + 1..]);
        if let Some(bracket_start) = key.find('[') {
            let array_name = &key[..bracket_start];
            let array_index_str = &key[bracket_start..];
            let (index, _) = parse_array_index(array_index_str);

            // Get the array
            let arr = match value {
                Value::Object(map) => map.get(array_name)?.as_array()?,
                _ => return None,
            };

            // Get the element
            if index >= arr.len() {
                return None;
            }
            get_value_at_path(&arr[index], rest)
        } else {
            // Regular object property access
            match value {
                Value::Object(map) => {
                    let next_value = map.get(key)?;
                    get_value_at_path(next_value, rest)
                }
                _ => None,
            }
        }
    } else {
        // No dot - could be simple property or array index
        if let Some(bracket_start) = path.find('[') {
            let array_name = &path[..bracket_start];
            let array_index_str = &path[bracket_start..];
            let (index, _) = parse_array_index(array_index_str);

            let arr = match value {
                Value::Object(map) => map.get(array_name)?.as_array()?,
                _ => return None,
            };

            if index >= arr.len() {
                return None;
            }
            Some(arr[index].clone())
        } else {
            // Simple property
            match value {
                Value::Object(map) => {
                    let next_value = map.get(path)?;
                    Some(next_value.clone())
                }
                _ => None,
            }
        }
    }
}

/// Parse the first segment of a path
/// Returns (segment, is_array_index, remaining_path)
fn parse_first_segment(path: &str) -> (String, bool, &str) {
    if path.starts_with('[') {
        // Array index at the start
        if let Some(end) = path.find(']') {
            let index_str = &path[1..end];
            let rest = if end + 1 < path.len() && path.chars().nth(end + 1) == Some('.') {
                &path[end + 2..]
            } else {
                &path[end + 1..]
            };
            return (index_str.to_string(), true, rest);
        }
    }

    // Check if the first segment is an array index (contains brackets)
    if let Some(brackets_pos) = path.find('[') {
        if let Some(end_bracket) = path.find(']') {
            // Check if this is an array index (ends with ])
            if end_bracket == brackets_pos + 1 || end_bracket < path.len() {
                let segment = &path[..end_bracket + 1];
                let rest = if end_bracket + 1 < path.len()
                    && path.chars().nth(end_bracket + 1) == Some('.')
                {
                    &path[end_bracket + 2..]
                } else {
                    &path[end_bracket + 1..]
                };
                return (segment.to_string(), true, rest);
            }
        }
    }

    // Object key
    if let Some(dot_pos) = path.find('.') {
        (path[..dot_pos].to_string(), false, &path[dot_pos + 1..])
    } else {
        (path.to_string(), false, "")
    }
}

/// Parse an array index from the beginning of a path
fn parse_array_index(path: &str) -> (usize, &str) {
    if !path.starts_with('[') {
        return (0, path);
    }

    if let Some(end) = path.find(']') {
        let index_str = &path[1..end];
        let index = index_str.parse().unwrap_or(0);
        let rest = if end + 1 < path.len() && path.chars().nth(end + 1) == Some('.') {
            &path[end + 2..]
        } else {
            &path[end + 1..]
        };
        (index, rest)
    } else {
        (0, path)
    }
}

/// Ensure an array has at least the specified length
fn ensure_array_length(arr: &mut Vec<Value>, index: usize) {
    while arr.len() <= index {
        arr.push(Value::Null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Change, Changes};
    use serde_json::Value;

    #[test]
    fn test_format_empty_changes() {
        let formatter = AfterFormatter::new();
        let changes = Changes::new();

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Empty changes should produce empty object
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_format_only_added() {
        let formatter = AfterFormatter::new();
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("age".to_string(), Value::Number(30.into()));
        map.insert(
            "email".to_string(),
            Value::String("alice@example.com".to_string()),
        );
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Added {
            path: "email".to_string(),
            value: Value::String("alice@example.com".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(
            obj.get("email"),
            Some(&Value::String("alice@example.com".to_string()))
        );
    }

    #[test]
    fn test_format_only_modified() {
        let formatter = AfterFormatter::new();
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("age".to_string(), Value::Number(31.into()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Modified {
            path: "age".to_string(),
            old_value: Value::Number(30.into()),
            new_value: Value::Number(31.into()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj.get("age"), Some(&Value::Number(31.into())));
    }

    #[test]
    fn test_format_mixed_added_and_modified() {
        let formatter = AfterFormatter::new();
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("age".to_string(), Value::Number(31.into()));
        map.insert(
            "email".to_string(),
            Value::String("alice@example.com".to_string()),
        );
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Modified {
            path: "name".to_string(),
            old_value: Value::String("Bob".to_string()),
            new_value: Value::String("Alice".to_string()),
        });

        changes.push(Change::Modified {
            path: "age".to_string(),
            old_value: Value::Number(30.into()),
            new_value: Value::Number(31.into()),
        });

        changes.push(Change::Added {
            path: "email".to_string(),
            value: Value::String("alice@example.com".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        assert_eq!(obj.len(), 3);
        assert_eq!(obj.get("name"), Some(&Value::String("Alice".to_string())));
        assert_eq!(obj.get("age"), Some(&Value::Number(31.into())));
        assert_eq!(
            obj.get("email"),
            Some(&Value::String("alice@example.com".to_string()))
        );
    }

    #[test]
    fn test_format_nested_objects() {
        let formatter = AfterFormatter::new();
        let mut changes = Changes::new();

        let mut address_map = Map::new();
        address_map.insert("city".to_string(), Value::String("NYC".to_string()));
        address_map.insert("zip".to_string(), Value::String("10001".to_string()));

        let mut user_map = Map::new();
        user_map.insert("name".to_string(), Value::String("Alice".to_string()));
        user_map.insert("age".to_string(), Value::Number(30.into()));
        user_map.insert("address".to_string(), Value::Object(address_map));

        let mut root_map = Map::new();
        root_map.insert("user".to_string(), Value::Object(user_map));

        let after_value = Value::Object(root_map);

        changes.after = Some(after_value);

        changes.push(Change::Modified {
            path: "user.name".to_string(),
            old_value: Value::String("Bob".to_string()),
            new_value: Value::String("Alice".to_string()),
        });

        changes.push(Change::Added {
            path: "user.address.city".to_string(),
            value: Value::String("NYC".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        assert!(obj.contains_key("user"));

        let user = obj.get("user").unwrap().as_object().unwrap();
        assert!(user.contains_key("name"));
        assert!(user.contains_key("address"));

        let address = user.get("address").unwrap().as_object().unwrap();
        assert!(address.contains_key("city"));
    }

    #[test]
    fn test_format_with_removed_ignored() {
        let formatter = AfterFormatter::new();
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("phone".to_string(), Value::String("555-1234".to_string()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        // Add a "removed" change - this should be ignored
        changes.push(Change::Removed {
            path: "phone".to_string(),
            value: Value::String("555-1234".to_string()),
        });

        // Add a modified change
        changes.push(Change::Modified {
            path: "name".to_string(),
            old_value: Value::String("Bob".to_string()),
            new_value: Value::String("Alice".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        // Only the modified change should appear, not the removed one
        assert_eq!(obj.len(), 1);
        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("phone"));
    }
}
