use crate::formatter::{sort_json_value, Formatter};
use crate::json_path::{JsonPath, PathSegment};
use crate::types::{Change, Changes};
use serde_json::{Map, Value};

// Import PathParser
use super::path_parser::PathParser;
use std::collections::HashSet;

/// Check if a path or any of its descendants are in the changed paths set
///
/// This function determines whether a JSON node should be included in the filtered
/// output by checking if either the exact path matches or if any descendant paths
/// start with this path as a prefix.
fn path_or_descendants_changed(path: &JsonPath, changed_paths: &HashSet<Vec<PathSegment>>) -> bool {
    // Check exact match
    if changed_paths.contains(path.segments()) {
        return true;
    }

    // Check if any changed path is a descendant of this path
    // A path is a descendant if it starts with all segments of this path
    for changed in changed_paths {
        if changed.len() >= path.len() && changed[..path.len()] == path.segments()[..] {
            return true;
        }
    }

    false
}

/// Collect and filter in a single pass through the value tree
///
/// This optimized function traverses the JSON value exactly once, building the
/// filtered output directly during traversal. It includes a node if:
/// - The node's path is in changed_paths, OR
/// - Any descendant of the node is in changed_paths
///
/// When an object/array is directly in changed_paths, ALL its children are included
/// (not just changed ones). This matches the old behavior where the entire structure
/// is preserved when the parent is marked as changed.
///
/// This eliminates the need for separate path collection and value building phases.
fn collect_and_filter_single_pass(
    value: &Value,
    current_path: &JsonPath,
    changed_paths: &HashSet<Vec<PathSegment>>,
) -> Option<Value> {
    match value {
        Value::Object(map) => {
            // Check if this object or any of its descendants are changed
            let object_or_descendants_changed =
                path_or_descendants_changed(current_path, changed_paths);
            let object_directly_changed = changed_paths.contains(current_path.segments());

            if object_or_descendants_changed {
                let mut filtered_map = Map::new();

                for (key, child_value) in map {
                    // Build path for this child
                    let mut child_path = current_path.clone();
                    child_path.push(PathSegment::Key(key.clone()));

                    // Check if this child path or any descendants are changed
                    let child_or_descendants_changed =
                        path_or_descendants_changed(&child_path, changed_paths);

                    if child_or_descendants_changed || object_directly_changed {
                        // If parent object is directly changed, include all children without filtering
                        // Otherwise, recurse normally to filter
                        if object_directly_changed {
                            // Include child as-is (don't filter further)
                            filtered_map.insert(key.clone(), child_value.clone());
                        } else {
                            // Recurse into child with filtering
                            if let Some(filtered_child) = collect_and_filter_single_pass(
                                child_value,
                                &child_path,
                                changed_paths,
                            ) {
                                filtered_map.insert(key.clone(), filtered_child);
                            }
                        }
                    }
                }

                Some(Value::Object(filtered_map))
            } else {
                None
            }
        }
        Value::Array(arr) => {
            // Check if this array or any of its elements/descendants are changed
            let array_or_descendants_changed =
                path_or_descendants_changed(current_path, changed_paths);
            let array_directly_changed = changed_paths.contains(current_path.segments());

            if array_or_descendants_changed {
                // Include ALL elements of the array
                // This matches the old behavior where the entire array is shown
                let mut filtered_arr = Vec::new();

                for (i, child_value) in arr.iter().enumerate() {
                    if array_directly_changed {
                        // Include element as-is (don't filter further)
                        filtered_arr.push(child_value.clone());
                    } else {
                        // Recursively filter child elements
                        let mut child_path = current_path.clone();
                        child_path.push(PathSegment::Index(i));

                        if let Some(filtered_child) =
                            collect_and_filter_single_pass(child_value, &child_path, changed_paths)
                        {
                            filtered_arr.push(filtered_child);
                        } else {
                            // Primitive value not in changed paths, but descendant changed
                            // Include it anyway since we're in an array that has changed descendants
                            filtered_arr.push(child_value.clone());
                        }
                    }
                }

                Some(Value::Array(filtered_arr))
            } else {
                None
            }
        }
        // Primitive values are included if path is in changed_paths
        _ => {
            if changed_paths.contains(current_path.segments()) {
                Some(value.clone())
            } else {
                None
            }
        }
    }
}

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

        // Build a set of all changed paths as strings
        let mut changed_paths_strings = HashSet::new();
        for change in &changes.added {
            if let Change::Added { path, .. } = change {
                changed_paths_strings.insert(path.to_string());
            }
        }
        for change in &changes.modified {
            if let Change::Modified { path, .. } = change {
                changed_paths_strings.insert(path.to_string());
            }
        }

        // Pre-parse changed paths into PathSegment vectors for O(1) comparison
        let changed_paths_segments: HashSet<Vec<PathSegment>> = changed_paths_strings
            .iter()
            .filter_map(|p| PathParser::parse(p).ok())
            .map(|parser| parser.into_segments())
            .collect();

        // Use single-pass traversal with PathParser integration
        let root_path = JsonPath::new();
        let filtered_after =
            collect_and_filter_single_pass(after_value, &root_path, &changed_paths_segments)
                .unwrap_or(Value::Object(Map::new()));

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
