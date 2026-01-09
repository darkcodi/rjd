use crate::formatter::{path_filter, sort_json_value, Formatter};
use crate::types::{Change, Changes};
use serde_json::{Map, Value};

// Import path filtering utilities
use path_filter::insert_value_at_path;

/// Formatter for the "after" output format
///
/// This formatter outputs the "after" state (file2) but only includes
/// properties that were added or modified compared to file1.
pub struct AfterFormatter {
    pretty: bool,
    sort: bool,
}

impl AfterFormatter {
    /// Create a new AfterFormatter with pretty printing enabled
    pub fn new(sort: bool) -> Self {
        Self { pretty: true, sort }
    }
}

impl Default for AfterFormatter {
    fn default() -> Self {
        Self::new(false)
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

        // Build a set of all changed paths for fast lookup
        let mut changed_paths = std::collections::HashSet::new();
        for change in &changes.added {
            if let Change::Added { path, .. } = change {
                changed_paths.insert(path.to_string());
            }
        }
        for change in &changes.modified {
            if let Change::Modified { path, .. } = change {
                changed_paths.insert(path.to_string());
            }
        }

        // Collect paths in the order they appear in the "after" file
        let ordered_paths = collect_paths_in_order(after_value, "", &changed_paths);

        // Build the filtered "after" value
        let filtered_after = build_filtered_value(after_value, &ordered_paths);

        // Serialize to JSON
        let json = if self.pretty {
            serde_json::to_string_pretty(&filtered_after)?
        } else {
            serde_json::to_string(&filtered_after)?
        };

        // If sort is enabled, parse and re-serialize with sorted keys
        if self.sort {
            let value: Value = serde_json::from_str(&json)?;
            let sorted = sort_json_value(&value);
            Ok(serde_json::to_string_pretty(&sorted)?)
        } else {
            Ok(json)
        }
    }
}

/// Collect paths that have changes, in the order they appear in the value
fn collect_paths_in_order(
    value: &Value,
    prefix: &str,
    changed_paths: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut paths = Vec::new();

    match value {
        Value::Object(map) => {
            for key in map.keys() {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                // Check if this exact path is in the changed set
                if changed_paths.contains(&path) {
                    paths.push(path);
                } else {
                    // Recurse into nested object/array to find changes
                    let nested_paths =
                        collect_paths_in_order(map.get(key).unwrap(), &path, changed_paths);
                    paths.extend(nested_paths);
                }
            }
        }
        Value::Array(arr) => {
            for (i, elem) in arr.iter().enumerate() {
                let path = format!("{}[{}]", prefix, i);
                // Check if this exact path is in the changed set
                if changed_paths.contains(&path) {
                    paths.push(path);
                } else {
                    // Recurse into array element
                    let nested_paths = collect_paths_in_order(elem, &path, changed_paths);
                    paths.extend(nested_paths);
                }
            }
        }
        _ => {
            // Primitive value - check if the path itself is changed
            if changed_paths.contains(prefix) {
                paths.push(prefix.to_string());
            }
        }
    }

    paths
}

/// Build a filtered value containing only the paths in the changed_paths set
fn build_filtered_value(value: &Value, changed_paths: &[String]) -> Value {
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

        insert_value_at_path(&mut root_map, path, value, path);
    }

    Value::Object(root_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Change, Changes};
    use serde_json::Value;

    #[test]
    fn test_format_empty_changes() {
        let formatter = AfterFormatter::new(false);
        let changes = Changes::new();

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Empty changes should produce empty object
        assert!(parsed.is_object());
        assert_eq!(parsed.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_format_only_added() {
        let formatter = AfterFormatter::new(false);
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
            path: "email".parse().unwrap(),
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
        let formatter = AfterFormatter::new(false);
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("age".to_string(), Value::Number(31.into()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Modified {
            path: "age".parse().unwrap(),
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
        let formatter = AfterFormatter::new(false);
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
            path: "name".parse().unwrap(),
            old_value: Value::String("Bob".to_string()),
            new_value: Value::String("Alice".to_string()),
        });

        changes.push(Change::Modified {
            path: "age".parse().unwrap(),
            old_value: Value::Number(30.into()),
            new_value: Value::Number(31.into()),
        });

        changes.push(Change::Added {
            path: "email".parse().unwrap(),
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
        let formatter = AfterFormatter::new(false);
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
            path: "user.name".parse().unwrap(),
            old_value: Value::String("Bob".to_string()),
            new_value: Value::String("Alice".to_string()),
        });

        changes.push(Change::Added {
            path: "user.address.city".parse().unwrap(),
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
        let formatter = AfterFormatter::new(false);
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("name".to_string(), Value::String("Alice".to_string()));
        map.insert("phone".to_string(), Value::String("555-1234".to_string()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        // Add a "removed" change - this should be ignored
        changes.push(Change::Removed {
            path: "phone".parse().unwrap(),
            value: Value::String("555-1234".to_string()),
        });

        // Add a modified change
        changes.push(Change::Modified {
            path: "name".parse().unwrap(),
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

    #[test]
    fn test_format_with_array_addition() {
        let formatter = AfterFormatter::new(false);
        let mut changes = Changes::new();

        // Create the "after" value: hobbies = ["reading", "painting"]
        let hobbies = vec![
            Value::String("reading".to_string()),
            Value::String("painting".to_string()),
        ];

        let mut map = Map::new();
        map.insert("hobbies".to_string(), Value::Array(hobbies));

        let after_value = Value::Object(map);
        changes.after = Some(after_value);

        // Add an "added" change for the new array element
        changes.push(Change::Added {
            path: "hobbies[1]".parse().unwrap(),
            value: Value::String("painting".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_object());
        let obj = parsed.as_object().unwrap();
        // The hobbies array should be present, not "hobbies[1]" as a key
        assert!(obj.contains_key("hobbies"));
        assert!(!obj.contains_key("hobbies[1]"));

        let hobbies_array = obj.get("hobbies").unwrap();
        assert!(hobbies_array.is_array());
        let hobbies_vec = hobbies_array.as_array().unwrap();
        assert_eq!(hobbies_vec.len(), 2);
        assert_eq!(hobbies_vec[0], Value::String("reading".to_string()));
        assert_eq!(hobbies_vec[1], Value::String("painting".to_string()));
    }

    #[test]
    fn test_format_with_sort() {
        let formatter = AfterFormatter::new(true);
        let mut changes = Changes::new();

        let mut map = Map::new();
        map.insert("z_field".to_string(), Value::String("z_value".to_string()));
        map.insert("a_field".to_string(), Value::String("a_value".to_string()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Modified {
            path: "z_field".parse().unwrap(),
            old_value: Value::String("old_z".to_string()),
            new_value: Value::String("z_value".to_string()),
        });

        changes.push(Change::Added {
            path: "a_field".parse().unwrap(),
            value: Value::String("a_value".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Check that keys are sorted alphabetically
        let obj = parsed.as_object().unwrap();
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["a_field", "z_field"]);
    }

    #[test]
    fn test_format_with_sort_nested() {
        let formatter = AfterFormatter::new(true);
        let mut changes = Changes::new();

        let mut nested = Map::new();
        nested.insert("z_key".to_string(), Value::String("z_val".to_string()));
        nested.insert("a_key".to_string(), Value::String("a_val".to_string()));

        let mut map = Map::new();
        map.insert("nested".to_string(), Value::Object(nested.clone()));
        let after_value = Value::Object(map);

        changes.after = Some(after_value);

        changes.push(Change::Added {
            path: "nested".parse().unwrap(),
            value: Value::Object(nested),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Check that nested keys are sorted
        let obj = parsed.as_object().unwrap();
        let nested_obj = obj["nested"].as_object().unwrap();
        let nested_keys: Vec<&str> = nested_obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(nested_keys, vec!["a_key", "z_key"]);
    }
}
