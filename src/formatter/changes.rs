use crate::formatter::Formatter;
use crate::types::Changes;

/// Formatter for the "changes" output format
///
/// This formatter outputs a JSON object with three arrays:
/// - added: Items present in the new file but not in the old file
/// - removed: Items present in the old file but not in the new file
/// - modified: Items that changed between the two files
pub struct ChangesFormatter {
    pretty: bool,
    sort: bool,
}

impl ChangesFormatter {
    /// Create a new ChangesFormatter with pretty printing enabled
    pub fn new(sort: bool) -> Self {
        Self { pretty: true, sort }
    }

    /// Create a ChangesFormatter with custom pretty printing setting
    #[allow(dead_code)]
    pub fn with_pretty(pretty: bool) -> Self {
        Self {
            pretty,
            sort: false,
        }
    }
}

impl Default for ChangesFormatter {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Formatter for ChangesFormatter {
    fn format(&self, changes: &Changes) -> Result<String, Box<dyn std::error::Error>> {
        let json = serde_json::to_value(changes)?;

        if self.sort {
            let sorted = sort_json_value(&json);
            if self.pretty {
                Ok(serde_json::to_string_pretty(&sorted)?)
            } else {
                Ok(serde_json::to_string(&sorted)?)
            }
        } else if self.pretty {
            Ok(serde_json::to_string_pretty(changes)?)
        } else {
            Ok(serde_json::to_string(changes)?)
        }
    }
}

/// Recursively sort a JSON value's keys
fn sort_json_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted_map = serde_json::Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_value(map.get(key).unwrap()));
            }
            serde_json::Value::Object(sorted_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(sort_json_value).collect())
        }
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
        let formatter = ChangesFormatter::new(false);
        let changes = Changes::new();

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert!(parsed["added"].is_array());
        assert!(parsed["removed"].is_array());
        assert!(parsed["modified"].is_array());
        assert_eq!(parsed["added"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["removed"].as_array().unwrap().len(), 0);
        assert_eq!(parsed["modified"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_format_with_changes() {
        let formatter = ChangesFormatter::new(false);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "users[0].name".to_string(),
            value: Value::String("Alice".to_string()),
        });

        changes.push(Change::Removed {
            path: "users[0].phone".to_string(),
            value: Value::String("555-1234".to_string()),
        });

        changes.push(Change::Modified {
            path: "users[0].age".to_string(),
            old_value: Value::Number(25.into()),
            new_value: Value::Number(26.into()),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["added"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["removed"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["modified"].as_array().unwrap().len(), 1);

        assert_eq!(parsed["added"][0]["path"], "users[0].name");
        assert_eq!(parsed["added"][0]["value"], "Alice");

        assert_eq!(parsed["removed"][0]["path"], "users[0].phone");
        assert_eq!(parsed["removed"][0]["value"], "555-1234");

        assert_eq!(parsed["modified"][0]["path"], "users[0].age");
        assert_eq!(parsed["modified"][0]["old_value"], 25);
        assert_eq!(parsed["modified"][0]["new_value"], 26);
    }

    #[test]
    fn test_format_with_sort() {
        let formatter = ChangesFormatter::new(true);
        let mut changes = Changes::new();

        changes.push(Change::Added {
            path: "z".to_string(),
            value: Value::String("last".to_string()),
        });

        changes.push(Change::Added {
            path: "a".to_string(),
            value: Value::String("first".to_string()),
        });

        let result = formatter.format(&changes).unwrap();

        // Parse and check that keys are sorted: added, modified, removed (alphabetically)
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let obj = parsed.as_object().unwrap();

        // Get the order of keys
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["added", "modified", "removed"]);
    }

    #[test]
    fn test_format_with_sort_nested() {
        let formatter = ChangesFormatter::new(true);
        let mut changes = Changes::new();

        // Add a change with a nested object value
        let mut nested = serde_json::Map::new();
        nested.insert("z_key".to_string(), Value::String("z_value".to_string()));
        nested.insert("a_key".to_string(), Value::String("a_value".to_string()));

        changes.push(Change::Added {
            path: "obj".to_string(),
            value: Value::Object(nested),
        });

        let result = formatter.format(&changes).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        // Check that nested object keys are also sorted
        let obj = parsed.as_object().unwrap();
        let added = obj["added"].as_array().unwrap();
        let nested_obj = added[0]["value"].as_object().unwrap();
        let nested_keys: Vec<&str> = nested_obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(nested_keys, vec!["a_key", "z_key"]);
    }
}
