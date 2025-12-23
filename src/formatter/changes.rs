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
}

impl ChangesFormatter {
    /// Create a new ChangesFormatter with pretty printing enabled
    pub fn new() -> Self {
        Self { pretty: true }
    }

    /// Create a ChangesFormatter with custom pretty printing setting
    #[allow(dead_code)]
    pub fn with_pretty(pretty: bool) -> Self {
        Self { pretty }
    }
}

impl Default for ChangesFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for ChangesFormatter {
    fn format(&self, changes: &Changes) -> Result<String, Box<dyn std::error::Error>> {
        if self.pretty {
            Ok(serde_json::to_string_pretty(changes)?)
        } else {
            Ok(serde_json::to_string(changes)?)
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
        let formatter = ChangesFormatter::new();
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
        let formatter = ChangesFormatter::new();
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
}
