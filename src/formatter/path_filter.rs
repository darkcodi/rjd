//! Path filtering and manipulation utilities
//!
//! This module provides utilities for working with JSON paths,
//! including getting and setting values at nested paths.

use serde_json::{Map, Value};

/// Get a value at a specific path from a JSON value
///
/// This function navigates through a JSON structure using dot notation
/// with bracket-based array indexing to retrieve a value.
///
/// # Arguments
/// * `value` - The root JSON value to search
/// * `path` - The path in dot notation (e.g., "user.name", "items[0].id")
///
/// # Returns
/// * `Some(Value)` - The value at the specified path
/// * `None` - If the path doesn't exist or is invalid
pub fn get_value_at_path(value: &Value, path: &str) -> Option<Value> {
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

/// Insert a value at a given path into a target map
///
/// This function recursively creates nested structures as needed to
/// insert a value at the specified path.
///
/// # Arguments
/// * `target` - The target map to insert into
/// * `path` - The path where to insert (may be modified during recursion)
/// * `source_value` - The source JSON value containing the value to insert
/// * `original_path` - The original path (used for value extraction)
pub fn insert_value_at_path(
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

/// Parse the first segment of a path
///
/// Returns (segment, is_array_index, remaining_path)
///
/// # Arguments
/// * `path` - The path string to parse
///
/// # Returns
/// A tuple of:
/// * The first segment as a String
/// * Whether this is an array index (bool)
/// * The remaining path after this segment
pub fn parse_first_segment(path: &str) -> (String, bool, &str) {
    if path.starts_with('[') {
        // Array index at the start (e.g., "[0].name" or "[1]")
        if let Some(end) = path.find(']') {
            // Extract the index
            let index_str = &path[1..end].trim();
            let after_bracket = &path[end + 1..];

            // Validate that the index is numeric
            if !index_str.is_empty() && index_str.chars().all(|c| c.is_ascii_digit()) {
                let rest = after_bracket.strip_prefix('.').unwrap_or(after_bracket);
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
                after_brackets.strip_prefix('.').unwrap_or(after_brackets)
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
///
/// # Arguments
/// * `path` - The path string starting with an array index like "[0]" or "[1].name"
///
/// # Returns
/// A tuple of:
/// * The parsed index (usize, 0 if invalid)
/// * The remaining path after the array index
pub fn parse_array_index(path: &str) -> (usize, &str) {
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
///
/// Extends the array with null values if necessary
///
/// # Arguments
/// * `arr` - The array to extend
/// * `index` - The minimum required length
pub fn ensure_array_length(arr: &mut Vec<Value>, index: usize) {
    while arr.len() <= index {
        arr.push(Value::Null);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_value_simple_path() {
        let value = json!({"name": "Alice", "age": 30});
        assert_eq!(get_value_at_path(&value, "name"), Some(json!("Alice")));
        assert_eq!(get_value_at_path(&value, "age"), Some(json!(30)));
    }

    #[test]
    fn test_get_value_nested_path() {
        let value = json!({"user": {"name": "Bob", "email": "bob@example.com"}});
        assert_eq!(get_value_at_path(&value, "user.name"), Some(json!("Bob")));
        assert_eq!(
            get_value_at_path(&value, "user.email"),
            Some(json!("bob@example.com"))
        );
    }

    #[test]
    fn test_get_value_array_path() {
        let value = json!({"items": [{"name": "item1"}, {"name": "item2"}]});
        assert_eq!(
            get_value_at_path(&value, "items[0].name"),
            Some(json!("item1"))
        );
        assert_eq!(
            get_value_at_path(&value, "items[1].name"),
            Some(json!("item2"))
        );
    }

    #[test]
    fn test_get_value_empty_path() {
        let value = json!({"name": "Alice"});
        assert_eq!(get_value_at_path(&value, ""), Some(value));
    }

    #[test]
    fn test_get_value_invalid_path() {
        let value = json!({"name": "Alice"});
        assert_eq!(get_value_at_path(&value, "nonexistent"), None);
        assert_eq!(get_value_at_path(&value, "user.nonexistent"), None);
    }

    #[test]
    fn test_parse_first_segment_simple() {
        let (segment, is_array, rest) = parse_first_segment("name");
        assert_eq!(segment, "name");
        assert!(!is_array);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_parse_first_segment_nested() {
        let (segment, is_array, rest) = parse_first_segment("user.name");
        assert_eq!(segment, "user");
        assert!(!is_array);
        assert_eq!(rest, "name");
    }

    #[test]
    fn test_parse_first_segment_array() {
        let (segment, is_array, rest) = parse_first_segment("items[0].name");
        assert_eq!(segment, "items");
        assert!(!is_array);
        assert_eq!(rest, "name");
    }

    #[test]
    fn test_parse_array_index() {
        let (index, rest) = parse_array_index("[0]");
        assert_eq!(index, 0);
        assert_eq!(rest, "");

        let (index, rest) = parse_array_index("[1].name");
        assert_eq!(index, 1);
        assert_eq!(rest, "name");
    }

    #[test]
    fn test_ensure_array_length() {
        let mut arr: Vec<Value> = vec![];
        ensure_array_length(&mut arr, 3);
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0], Value::Null);
        assert_eq!(arr[3], Value::Null);
    }

    #[test]
    fn test_insert_value_simple() {
        let mut target = Map::new();
        let source = json!({"name": "Alice"});
        insert_value_at_path(&mut target, "name", &source, "name");
        assert_eq!(target.get("name"), Some(&json!("Alice")));
    }

    #[test]
    fn test_insert_value_nested() {
        let mut target = Map::new();
        let source = json!({"user": {"name": "Bob"}});
        insert_value_at_path(&mut target, "user.name", &source, "user.name");
        assert_eq!(
            target.get("user").and_then(|v| v.get("name")),
            Some(&json!("Bob"))
        );
    }
}
