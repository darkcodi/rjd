use crate::diff::visitor::{traverse, ValueVisitor, ValueVisitorExt};
use crate::json_path::JsonPath;
use crate::path::{join_array_path, join_path};
use crate::types::{Change, Changes};
use serde_json::Value;

/// Main diff function - compares two JSON values and returns all changes
pub fn diff(old: &Value, new: &Value) -> Changes {
    let mut changes = Changes::new();
    changes.after = Some(new.clone());
    let mut visitor = DiffVisitor {
        changes: &mut changes,
    };

    traverse(Some(old), Some(new), "", &mut visitor);

    changes
}

/// Visitor implementation that collects changes during traversal
struct DiffVisitor<'a> {
    changes: &'a mut Changes,
}

impl<'a> ValueVisitor for DiffVisitor<'a> {
    type Output = ();

    fn visit_null(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        // Null values don't need special handling - they are treated like any other value
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }

    fn visit_bool(
        &mut self,
        path: &str,
        old_value: Option<&bool>,
        new_value: Option<&bool>,
    ) -> Self::Output {
        self.handle_change(
            path,
            old_value.cloned().map(Value::Bool),
            new_value.cloned().map(Value::Bool),
        )
    }

    fn visit_number(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }

    fn visit_string(
        &mut self,
        path: &str,
        old_value: Option<&String>,
        new_value: Option<&String>,
    ) -> Self::Output {
        self.handle_change(
            path,
            old_value.cloned().map(Value::String),
            new_value.cloned().map(Value::String),
        )
    }

    fn visit_array(
        &mut self,
        path: &str,
        old_value: Option<&Vec<Value>>,
        new_value: Option<&Vec<Value>>,
    ) -> Self::Output {
        let old_len = old_value.map(|v| v.len()).unwrap_or(0);
        let new_len = new_value.map(|v| v.len()).unwrap_or(0);
        let max_len = old_len.max(new_len);

        for i in 0..max_len {
            let element_path = join_array_path(path, i);
            let old_element = old_value.and_then(|v| v.get(i));
            let new_element = new_value.and_then(|v| v.get(i));

            traverse(old_element, new_element, &element_path, self);
        }
    }

    fn visit_object(
        &mut self,
        path: &str,
        old_value: Option<&serde_json::Map<String, Value>>,
        new_value: Option<&serde_json::Map<String, Value>>,
    ) -> Self::Output {
        // Collect all keys in order from the "after" file (new_value), preserving insertion order
        // This ensures the output order matches the "after" file's key order
        let mut all_keys: Vec<String> = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        // Process new keys first to preserve "after" file order
        for key in new_value
            .as_ref()
            .map(|m| m.keys())
            .into_iter()
            .flatten()
            .cloned()
        {
            if seen_keys.insert(key.clone()) {
                all_keys.push(key);
            }
        }

        // Then add any keys that only exist in old_value
        for key in old_value
            .as_ref()
            .map(|m| m.keys())
            .into_iter()
            .flatten()
            .cloned()
        {
            if seen_keys.insert(key.clone()) {
                all_keys.push(key);
            }
        }

        for key in all_keys {
            let key_path = join_path(path, &key);
            let old_val = old_value.and_then(|m| m.get(&key));
            let new_val = new_value.and_then(|m| m.get(&key));

            traverse(old_val, new_val, &key_path, self);
        }
    }

    fn visit_equal(&mut self, _path: &str, _value: &Value) -> Self::Output {
        // Values are equal - no change to record
    }
}

impl<'a> DiffVisitor<'a> {
    fn handle_change(&mut self, path: &str, old_value: Option<Value>, new_value: Option<Value>) {
        // Parse the path string into JsonPath
        // Empty path is valid (root level)
        let json_path = if path.is_empty() {
            JsonPath::new()
        } else {
            path.parse::<JsonPath>().unwrap_or_default()
        };

        match (old_value, new_value) {
            (None, Some(value)) => {
                self.changes.push(Change::Added {
                    path: json_path,
                    value,
                });
            }
            (Some(value), None) => {
                self.changes.push(Change::Removed {
                    path: json_path,
                    value,
                });
            }
            (Some(old_val), Some(new_val)) => {
                self.changes.push(Change::Modified {
                    path: json_path,
                    old_value: old_val,
                    new_value: new_val,
                });
            }
            (None, None) => {
                // Both are None - nothing to do
            }
        }
    }
}

impl<'a> ValueVisitorExt for DiffVisitor<'a> {
    fn visit_modified(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        // For type mismatches or primitive modifications, just record the change
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }
}
