use crate::formatter::{sort_json_value, Formatter};
use crate::types::{Change, Changes};
use serde::Serialize;
use serde_json::Value;

/// Represents a JSON Patch operation according to RFC 6902
#[derive(Debug, Clone, Serialize)]
struct JsonPatchOperation {
    /// The operation to perform: "add", "remove", or "replace"
    op: String,

    /// JSON Pointer path to the target location
    path: String,

    /// The value to add or replace (None for remove operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
}

/// Formatter for RFC 6902 JSON Patch output format
pub struct JsonPatchFormatter {
    pretty: bool,
    sort: bool,
}

impl JsonPatchFormatter {
    /// Create a new JsonPatchFormatter with pretty printing enabled
    pub fn new(sort: bool) -> Self {
        Self { pretty: true, sort }
    }
}

impl Default for JsonPatchFormatter {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Formatter for JsonPatchFormatter {
    fn format(&self, changes: &Changes) -> Result<String, Box<dyn std::error::Error>> {
        let mut operations = Vec::new();

        // Process added changes -> "add" operations
        for change in &changes.added {
            if let Change::Added { path, value } = change {
                operations.push(JsonPatchOperation {
                    op: "add".to_string(),
                    path: path.to_json_pointer(),
                    value: Some(value.clone()),
                });
            }
        }

        // Process removed changes -> "remove" operations
        for change in &changes.removed {
            if let Change::Removed { path, .. } = change {
                operations.push(JsonPatchOperation {
                    op: "remove".to_string(),
                    path: path.to_json_pointer(),
                    value: None,
                });
            }
        }

        // Process modified changes -> "replace" operations
        for change in &changes.modified {
            if let Change::Modified {
                path, new_value, ..
            } = change
            {
                operations.push(JsonPatchOperation {
                    op: "replace".to_string(),
                    path: path.to_json_pointer(),
                    value: Some(new_value.clone()),
                });
            }
        }

        // Serialize the array of operations
        let json = if self.pretty {
            serde_json::to_string_pretty(&operations)?
        } else {
            serde_json::to_string(&operations)?
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
    use serde_json::{Map, Value};

    #[test]
    fn test_format_empty_changes() {
        let formatter = JsonPatchFormatter::new(false);
        let changes = Changes::new();

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_format_added_change() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "email".parse().unwrap(),
            value: Value::String("user@example.com".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "add");
        assert_eq!(op["path"], "/email");
        assert_eq!(op["value"], "user@example.com");
    }

    #[test]
    fn test_format_removed_change() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Removed {
            path: "phone".parse().unwrap(),
            value: Value::String("555-1234".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "remove");
        assert_eq!(op["path"], "/phone");
        assert!(op.get("value").is_none());
    }

    #[test]
    fn test_format_modified_change() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Modified {
            path: "name".parse().unwrap(),
            old_value: Value::String("John".to_string()),
            new_value: Value::String("Jane".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "replace");
        assert_eq!(op["path"], "/name");
        assert_eq!(op["value"], "Jane");
    }

    #[test]
    fn test_format_mixed_changes() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "email".parse().unwrap(),
            value: Value::String("user@example.com".to_string()),
        });

        changes.push(Change::Removed {
            path: "phone".parse().unwrap(),
            value: Value::String("555-1234".to_string()),
        });

        changes.push(Change::Modified {
            path: "name".parse().unwrap(),
            old_value: Value::String("John".to_string()),
            new_value: Value::String("Jane".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 3);

        // Operations should be in the order: added, removed, modified
        assert_eq!(ops[0]["op"], "add");
        assert_eq!(ops[0]["path"], "/email");

        assert_eq!(ops[1]["op"], "remove");
        assert_eq!(ops[1]["path"], "/phone");

        assert_eq!(ops[2]["op"], "replace");
        assert_eq!(ops[2]["path"], "/name");
        assert_eq!(ops[2]["value"], "Jane");
    }

    #[test]
    fn test_format_with_nested_paths() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Modified {
            path: "user.address.city".parse().unwrap(),
            old_value: Value::String("NYC".to_string()),
            new_value: Value::String("LA".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "replace");
        assert_eq!(op["path"], "/user/address/city");
        assert_eq!(op["value"], "LA");
    }

    #[test]
    fn test_format_with_array_paths() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "users[0].email".parse().unwrap(),
            value: Value::String("user@example.com".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "add");
        assert_eq!(op["path"], "/users/0/email");
        assert_eq!(op["value"], "user@example.com");
    }

    #[test]
    fn test_format_compact() {
        // Test compact (non-pretty) output by constructing formatter directly
        let formatter = JsonPatchFormatter {
            pretty: false,
            sort: false,
        };
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "name".parse().unwrap(),
            value: Value::String("Alice".to_string()),
        });

        let result = formatter.format(&changes).unwrap();

        // Should not contain newlines
        assert!(!result.contains('\n'));
        // Should be valid JSON
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn test_format_pretty() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "name".parse().unwrap(),
            value: Value::String("Alice".to_string()),
        });

        let result = formatter.format(&changes).unwrap();

        // Should contain newlines for pretty printing
        assert!(result.contains('\n'));
        // Should be valid JSON
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn test_format_complex_value() {
        let formatter = JsonPatchFormatter::new(false);
        let mut changes = Changes::new();

        let mut nested_obj = Map::new();
        nested_obj.insert("city".to_string(), Value::String("NYC".to_string()));
        nested_obj.insert("zip".to_string(), Value::String("10001".to_string()));

        changes.push(Change::Added {
            path: "address".parse().unwrap(),
            value: Value::Object(nested_obj),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed.is_array());
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 1);

        let op = &ops[0];
        assert_eq!(op["op"], "add");
        assert_eq!(op["path"], "/address");
        assert!(op["value"].is_object());
    }

    #[test]
    fn test_format_with_sort() {
        let formatter = JsonPatchFormatter::new(true);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "z_field".parse().unwrap(),
            value: Value::String("z_value".to_string()),
        });

        changes.push(Change::Added {
            path: "a_field".parse().unwrap(),
            value: Value::String("a_value".to_string()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Check that operation keys are sorted alphabetically
        let ops = parsed.as_array().unwrap();
        assert_eq!(ops.len(), 2);

        // Each operation object should have keys sorted: op, path, value
        let op1 = &ops[0];
        let op1_keys: Vec<&str> = op1
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(op1_keys, vec!["op", "path", "value"]);
    }

    #[test]
    fn test_format_with_sort_nested() {
        let formatter = JsonPatchFormatter::new(true);
        let mut changes = Changes::new();

        let mut nested = Map::new();
        nested.insert("z_key".to_string(), Value::String("z_val".to_string()));
        nested.insert("a_key".to_string(), Value::String("a_val".to_string()));

        changes.push(Change::Added {
            path: "obj".parse().unwrap(),
            value: Value::Object(nested),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        let ops = parsed.as_array().unwrap();
        let op = &ops[0];

        // Check that nested object keys within the value are sorted
        let value_obj = op["value"].as_object().unwrap();
        let nested_keys: Vec<&str> = value_obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(nested_keys, vec!["a_key", "z_key"]);
    }
}
