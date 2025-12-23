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
    sort: bool,
}

impl AfterFormatter {
    /// Create a new AfterFormatter with pretty printing enabled
    pub fn new(sort: bool) -> Self {
        Self { pretty: true, sort }
    }

    /// Create an AfterFormatter with custom pretty printing setting
    #[allow(dead_code)]
    pub fn with_pretty(pretty: bool) -> Self {
        Self {
            pretty,
            sort: false,
        }
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

    // Check if the original_path exists as a single key in the source
    // If so, insert it directly without parsing as nested path
    if let Value::Object(source_map) = source_value {
        if source_map.contains_key(original_path) {
            // Insert the entire path as a single key
            if let Some(value) = get_value_at_path(source_value, original_path) {
                target.insert(original_path.to_string(), value);
            }
            return;
        }
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
        // Special handling for array paths: if this is an array access (ends with [index]),
        // get the entire array from the source, not just the element
        if original_path.ends_with(']') {
            if let Some(bracket_pos) = original_path.find('[') {
                let after_bracket = &original_path[bracket_pos..];
                if after_bracket.contains('.') {
                    // Path like "hobbies[1].name" or "items[0].id" - get the value at that path
                    if let Some(value) = get_value_at_path(source_value, original_path) {
                        *target_entry = value;
                    }
                } else {
                    // Path like "hobbies[1]" - get the array, not the element
                    let array_name = &original_path[..bracket_pos];
                    if let Some(array_value) = get_value_at_path(source_value, array_name) {
                        *target_entry = array_value;
                    }
                }
            }
        } else {
            // Regular path - get the value at the original path
            if let Some(value) = get_value_at_path(source_value, original_path) {
                *target_entry = value;
            }
        }
        return;
    }

    // Recursively insert into the next level
    match target_entry {
        Value::Object(map) => {
            insert_value_at_path(map, remaining_path, source_value, path);
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
            insert_value_at_path(next_map, rest, source_value, path);
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
        // Array index at the start (e.g., "[0].name" or "[1]")
        if let Some(end) = path.find(']') {
            // Extract the index
            let index_str = &path[1..end].trim();
            let after_bracket = &path[end + 1..];

            // Validate that the index is numeric
            if !index_str.is_empty() && index_str.chars().all(|c| c.is_ascii_digit()) {
                let rest = if after_bracket.starts_with('.') {
                    &after_bracket[1..]
                } else {
                    after_bracket
                };
                return (index_str.to_string(), true, rest);
            }
        }
    }

    // Check if the first segment contains array notation (e.g., "items[0].name" or "hobbies[1]")
    if let Some(brackets_pos) = path.find('[') {
        let segment = &path[..brackets_pos];

        // Find the matching closing bracket
        if let Some(end_bracket) = path.find(']') {
            // Extract the part after the brackets
            let after_brackets = &path[end_bracket + 1..];

            // Check if there's more path after the brackets
            let rest = if !after_brackets.is_empty() {
                // Skip the dot separator if present
                if after_brackets.starts_with('.') {
                    &after_brackets[1..]
                } else {
                    after_brackets
                }
            } else {
                // No more path after the array index - this is a final array access
                ""
            };

            // This is an object key followed by array access
            return (segment.to_string(), false, rest);
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

/// Recursively sort a JSON value's keys
fn sort_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_value(map.get(key).unwrap()));
            }
            Value::Object(sorted_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_value).collect()),
        _ => value.clone(),
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
        let formatter = AfterFormatter::new(false);
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
        let formatter = AfterFormatter::new(false);
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

    #[test]
    fn test_format_with_array_addition() {
        let formatter = AfterFormatter::new(false);
        let mut changes = Changes::new();

        // Create the "after" value: hobbies = ["reading", "painting"]
        let mut hobbies = Vec::new();
        hobbies.push(Value::String("reading".to_string()));
        hobbies.push(Value::String("painting".to_string()));

        let mut map = Map::new();
        map.insert("hobbies".to_string(), Value::Array(hobbies));

        let after_value = Value::Object(map);
        changes.after = Some(after_value);

        // Add an "added" change for the new array element
        changes.push(Change::Added {
            path: "hobbies[1]".to_string(),
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
            path: "z_field".to_string(),
            old_value: Value::String("old_z".to_string()),
            new_value: Value::String("z_value".to_string()),
        });

        changes.push(Change::Added {
            path: "a_field".to_string(),
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
            path: "nested".to_string(),
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
