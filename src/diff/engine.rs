use crate::diff::visitor::{traverse, ValueVisitor, ValueVisitorExt};
use crate::json_path::JsonPath;
use crate::path::{join_array_path, join_path};
use crate::types::{Change, Changes};
use serde_json::Value;

/// Main diff function - compares two JSON values and returns all changes
///
/// This function recursively compares two JSON values and identifies all differences,
/// including added, removed, and modified values. Changes are organized into three
/// categories and returned in a `Changes` struct.
///
/// # Root-Level Changes
///
/// When the entire JSON value is replaced (e.g., two different primitive values),
/// the change will have an empty path (`""`). This represents a modification at
/// the root level.
///
/// # Examples
///
/// ## Object diff
/// ```
/// use rjd::diff;
/// use serde_json::json;
///
/// let old = json!({"name": "John", "age": 30});
/// let new = json!({"name": "Jane", "age": 30});
/// let changes = diff(&old, &new);
///
/// // One modification: "name" changed from "John" to "Jane"
/// assert_eq!(changes.modified.len(), 1);
/// ```
///
/// ## Root-level replacement
/// ```
/// use rjd::diff;
/// use serde_json::json;
///
/// let old = json!("value1");
/// let new = json!("value2");
/// let changes = diff(&old, &new);
///
/// // Root change with empty path
/// assert_eq!(changes.modified.len(), 1);
/// assert_eq!(changes.modified[0].path().to_string(), "");
/// ```
///
/// ## Array diff
/// ```
/// use rjd::diff;
/// use serde_json::json;
///
/// let old = json!([1, 2, 3]);
/// let new = json!([1, 4, 3]);
/// let changes = diff(&old, &new);
///
/// // One modification: index 1 changed from 2 to 4
/// assert_eq!(changes.modified.len(), 1);
/// ```
pub fn diff(old: &Value, new: &Value) -> Changes {
    let mut changes = Changes::new();
    changes.after = Some(new.clone());
    let mut visitor = DiffVisitor {
        changes: &mut changes,
    };

    traverse(Some(old), Some(new), &JsonPath::new(), &mut visitor);

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
        path: &JsonPath,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        // Null values don't need special handling - they are treated like any other value
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }

    fn visit_bool(
        &mut self,
        path: &JsonPath,
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
        path: &JsonPath,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }

    fn visit_string(
        &mut self,
        path: &JsonPath,
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
        path: &JsonPath,
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
        path: &JsonPath,
        old_value: Option<&serde_json::Map<String, Value>>,
        new_value: Option<&serde_json::Map<String, Value>>,
    ) -> Self::Output {
        // Collect all keys from new_value first (preserves "after" file order)
        let mut all_keys: Vec<String> = new_value
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();

        // Add keys only in old_value (removed keys)
        if let Some(old_map) = old_value {
            for key in old_map.keys() {
                if new_value
                    .as_ref()
                    .map(|m| !m.contains_key(key))
                    .unwrap_or(true)
                {
                    all_keys.push(key.clone());
                }
            }
        }

        for key in all_keys {
            let key_path = join_path(path, &key);
            let old_val = old_value.and_then(|m| m.get(&key));
            let new_val = new_value.and_then(|m| m.get(&key));

            traverse(old_val, new_val, &key_path, self);
        }
    }

    fn visit_equal(&mut self, _path: &JsonPath, _value: &Value) -> Self::Output {
        // Values are equal - no change to record
    }
}

impl<'a> DiffVisitor<'a> {
    fn handle_change(
        &mut self,
        path: &JsonPath,
        old_value: Option<Value>,
        new_value: Option<Value>,
    ) {
        match (old_value, new_value) {
            (None, Some(value)) => {
                self.changes.push(Change::Added {
                    path: path.clone(),
                    value,
                });
            }
            (Some(value), None) => {
                self.changes.push(Change::Removed {
                    path: path.clone(),
                    value,
                });
            }
            (Some(old_val), Some(new_val)) => {
                self.changes.push(Change::Modified {
                    path: path.clone(),
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
        path: &JsonPath,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output {
        // For type mismatches or primitive modifications, just record the change
        self.handle_change(path, old_value.cloned(), new_value.cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_empty_path_handling() {
        let old = json!(1);
        let new = json!(2);
        let changes = diff(&old, &new);

        assert_eq!(changes.modified.len(), 1);
        let change = &changes.modified[0];
        assert_eq!(change.path().to_string(), "");
        assert_eq!(change.path().to_json_pointer(), "");
    }

    #[test]
    fn test_deeply_nested_paths() {
        let old = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "value": "old"
                        }
                    }
                }
            }
        });
        let new = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "value": "new"
                        }
                    }
                }
            }
        });
        let changes = diff(&old, &new);

        assert_eq!(changes.modified.len(), 1);
        let change = &changes.modified[0];
        assert_eq!(
            change.path().to_string(),
            "level1.level2.level3.level4.value"
        );
        assert_eq!(
            change.path().to_json_pointer(),
            "/level1/level2/level3/level4/value"
        );
    }

    #[test]
    fn test_array_indexing_with_jsonpath() {
        let old = json!({
            "items": [1, 2, 3],
            "nested": {
                "arr": [{"a": 1}, {"a": 2}]
            }
        });
        let new = json!({
            "items": [1, 5, 3],
            "nested": {
                "arr": [{"a": 1}, {"a": 3}]
            }
        });
        let changes = diff(&old, &new);

        assert_eq!(changes.modified.len(), 2);
        // items[1] changed from 2 to 5
        let items_change = changes
            .modified
            .iter()
            .find(|c| c.path().to_string() == "items[1]")
            .unwrap();
        assert_eq!(items_change.path().to_json_pointer(), "/items/1");

        // nested.arr[1].a changed from 2 to 3
        let nested_change = changes
            .modified
            .iter()
            .find(|c| c.path().to_string() == "nested.arr[1].a")
            .unwrap();
        assert_eq!(nested_change.path().to_json_pointer(), "/nested/arr/1/a");
    }

    #[test]
    fn test_path_round_trip() {
        // Test that paths can be correctly represented through the diff system
        // For root-level primitives, the path is empty (which is valid)

        // Test root-level value change
        let old = json!("old");
        let new = json!("new");
        let changes = diff(&old, &new);
        assert_eq!(changes.modified.len(), 1);
        // Root path is empty - this is expected
        assert_eq!(changes.modified[0].path().to_string(), "");

        // Test nested property change
        let old = json!({"name": "old"});
        let new = json!({"name": "new"});
        let changes = diff(&old, &new);
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.modified[0].path().to_string(), "name");

        // Test deeply nested property
        let old = json!({"user": {"profile": {"email": "old@test.com"}}});
        let new = json!({"user": {"profile": {"email": "new@test.com"}}});
        let changes = diff(&old, &new);
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.modified[0].path().to_string(), "user.profile.email");

        // Test array element
        let old = json!({"items": [1, 2, 3]});
        let new = json!({"items": [1, 5, 3]});
        let changes = diff(&old, &new);
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.modified[0].path().to_string(), "items[1]");

        // Test nested array element
        let old = json!({"users": [{"name": "John"}, {"name": "Jane"}]});
        let new = json!({"users": [{"name": "John"}, {"name": "Alice"}]});
        let changes = diff(&old, &new);
        assert_eq!(changes.modified.len(), 1);
        assert_eq!(changes.modified[0].path().to_string(), "users[1].name");
    }

    #[test]
    fn test_visitor_receives_jsonpath() {
        // This test verifies that the visitor pattern works with JsonPath
        let old = json!({"a": 1, "b": 2});
        let new = json!({"a": 1, "b": 3});

        let changes = diff(&old, &new);

        assert_eq!(changes.modified.len(), 1);
        let change = &changes.modified[0];
        // Verify the path is a proper JsonPath object
        assert_eq!(change.path().to_string(), "b");
        assert_eq!(change.path().to_json_pointer(), "/b");
    }

    #[test]
    fn test_traversal_with_jsonpath() {
        // Test complex nested traversal
        let old = json!({
            "users": [
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ],
            "metadata": {
                "count": 2,
                "last_updated": "2024-01-01"
            }
        });
        let new = json!({
            "users": [
                {"name": "Alice", "age": 31},
                {"name": "Bob", "age": 26}
            ],
            "metadata": {
                "count": 2,
                "last_updated": "2024-01-02"
            }
        });

        let changes = diff(&old, &new);

        // Should have 3 modifications: users[0].age, users[1].age, metadata.last_updated
        assert_eq!(changes.modified.len(), 3);

        // Verify each path
        let paths: Vec<String> = changes
            .modified
            .iter()
            .map(|c| c.path().to_string())
            .collect();

        assert!(paths.contains(&"users[0].age".to_string()));
        assert!(paths.contains(&"users[1].age".to_string()));
        assert!(paths.contains(&"metadata.last_updated".to_string()));
    }

    #[test]
    fn test_empty_path_with_complex_values() {
        // Test root-level changes with complex values
        let old = json!({"a": 1});
        let new = json!({"b": 2});
        let changes = diff(&old, &new);

        // Root is replaced, so we should see removals and additions
        assert!(!changes.added.is_empty() || !changes.removed.is_empty());
    }

    #[test]
    fn test_array_operations_with_jsonpath() {
        let old = json!([1, 2, 3]);
        let new = json!([1, 4, 3, 5]);
        let changes = diff(&old, &new);

        // Should have modifications and additions
        assert!(!changes.modified.is_empty() || !changes.added.is_empty());

        // Verify array index paths
        for change in changes.modified.iter().chain(changes.added.iter()) {
            let path_str = change.path().to_string();
            assert!(
                path_str.starts_with('['),
                "Array path should start with '[': {}",
                path_str
            );
            assert!(
                path_str.ends_with(']'),
                "Array path should end with ']': {}",
                path_str
            );
        }
    }
}
