//! Integration tests for the diff algorithm

use rjd::diff;
use serde_json::json;

#[test]
fn test_equal_objects_no_changes() {
    let old = json!({"name": "John", "age": 30});
    let new = json!({"name": "John", "age": 30});
    let changes = diff(&old, &new);
    assert!(changes.is_empty());
}

#[test]
fn test_added_property() {
    let old = json!({"name": "John"});
    let new = json!({"name": "John", "age": 30});
    let changes = diff(&old, &new);
    assert_eq!(changes.added.len(), 1);
    if let rjd::Change::Added { path, value } = &changes.added[0] {
        assert_eq!(path.to_string(), "age");
        assert_eq!(value, &json!(30));
    } else {
        panic!("Expected Added change");
    }
}

#[test]
fn test_removed_property() {
    let old = json!({"name": "John", "age": 30});
    let new = json!({"name": "John"});
    let changes = diff(&old, &new);
    assert_eq!(changes.removed.len(), 1);
    if let rjd::Change::Removed { path, value } = &changes.removed[0] {
        assert_eq!(path.to_string(), "age");
        assert_eq!(value, &json!(30));
    } else {
        panic!("Expected Removed change");
    }
}

#[test]
fn test_modified_value() {
    let old = json!({"name": "John", "age": 30});
    let new = json!({"name": "John", "age": 31});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    if let rjd::Change::Modified {
        path,
        old_value,
        new_value,
    } = &changes.modified[0]
    {
        assert_eq!(path.to_string(), "age");
        assert_eq!(old_value, &json!(30));
        assert_eq!(new_value, &json!(31));
    } else {
        panic!("Expected Modified change");
    }
}

#[test]
fn test_nested_added_property() {
    let old = json!({"user": {"name": "John"}});
    let new = json!({"user": {"name": "John", "email": "john@example.com"}});
    let changes = diff(&old, &new);
    assert_eq!(changes.added.len(), 1);
    if let rjd::Change::Added { path, .. } = &changes.added[0] {
        assert_eq!(path.to_string(), "user.email");
    } else {
        panic!("Expected Added change");
    }
}

#[test]
fn test_nested_removed_property() {
    let old = json!({"user": {"name": "John", "email": "john@example.com"}});
    let new = json!({"user": {"name": "John"}});
    let changes = diff(&old, &new);
    assert_eq!(changes.removed.len(), 1);
    if let rjd::Change::Removed { path, .. } = &changes.removed[0] {
        assert_eq!(path.to_string(), "user.email");
    } else {
        panic!("Expected Removed change");
    }
}

#[test]
fn test_deeply_nested_property() {
    let old = json!({"a": {"b": {"c": {"d": 1}}}});
    let new = json!({"a": {"b": {"c": {"d": 2}}}});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    if let rjd::Change::Modified { path, .. } = &changes.modified[0] {
        assert_eq!(path.to_string(), "a.b.c.d");
    } else {
        panic!("Expected Modified change");
    }
}

#[test]
fn test_type_mismatch_object_to_string() {
    let old = json!({"key": {"nested": "value"}});
    let new = json!({"key": "string_value"});
    let changes = diff(&old, &new);
    // Type mismatch results in changes (either removed+added or modified)
    assert!(!changes.is_empty());
    // Should have either a modified change or remove+add
    let total_changes = changes.removed.len() + changes.added.len() + changes.modified.len();
    assert!(total_changes >= 1);
}

#[test]
fn test_empty_objects() {
    let old = json!({});
    let new = json!({});
    let changes = diff(&old, &new);
    assert!(changes.is_empty());
}

#[test]
fn test_empty_to_object() {
    let old = json!({});
    let new = json!({"key": "value"});
    let changes = diff(&old, &new);
    assert_eq!(changes.added.len(), 1);
    if let rjd::Change::Added { path, .. } = &changes.added[0] {
        assert_eq!(path.to_string(), "key");
    } else {
        panic!("Expected Added change");
    }
}

#[test]
fn test_object_to_empty() {
    let old = json!({"key": "value"});
    let new = json!({});
    let changes = diff(&old, &new);
    assert_eq!(changes.removed.len(), 1);
    if let rjd::Change::Removed { path, .. } = &changes.removed[0] {
        assert_eq!(path.to_string(), "key");
    } else {
        panic!("Expected Removed change");
    }
}

#[test]
fn test_array_element_modification() {
    let old = json!({"items": [1, 2, 3]});
    let new = json!({"items": [1, 10, 3]});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    if let rjd::Change::Modified { path, .. } = &changes.modified[0] {
        // Array paths use index notation
        assert!(path.to_string().starts_with("items[1]"));
    } else {
        panic!("Expected Modified change");
    }
}

#[test]
fn test_multiple_changes() {
    let old = json!({"a": 1, "b": 2, "c": 3});
    let new = json!({"a": 10, "b": 2, "d": 4});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    assert_eq!(changes.added.len(), 1);
    assert_eq!(changes.removed.len(), 1);
}

#[test]
fn test_modified_string_value() {
    let old = json!({"name": "John"});
    let new = json!({"name": "Jane"});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    if let rjd::Change::Modified {
        path,
        old_value,
        new_value,
    } = &changes.modified[0]
    {
        assert_eq!(path.to_string(), "name");
        assert_eq!(old_value, &json!("John"));
        assert_eq!(new_value, &json!("Jane"));
    } else {
        panic!("Expected Modified change");
    }
}

#[test]
fn test_modified_boolean_value() {
    let old = json!({"active": false});
    let new = json!({"active": true});
    let changes = diff(&old, &new);
    assert_eq!(changes.modified.len(), 1);
    if let rjd::Change::Modified { path, .. } = &changes.modified[0] {
        assert_eq!(path.to_string(), "active");
    } else {
        panic!("Expected Modified change");
    }
}

#[test]
fn test_modified_null_value() {
    let old = json!({"value": null});
    let new = json!({"value": "string"});
    let changes = diff(&old, &new);
    // null to string results in changes
    assert!(!changes.is_empty());
    let total_changes = changes.removed.len() + changes.added.len() + changes.modified.len();
    assert!(total_changes >= 1);
}

/// Performance test for pattern matching optimization
/// Tests with 1000+ changes and 50+ patterns to validate O(n log m) performance
#[test]
fn test_pattern_matching_performance() {
    use std::time::Instant;

    // Create a large number of changes
    let mut old_obj = serde_json::Map::new();
    let mut new_obj = serde_json::Map::new();

    for i in 0..1000 {
        old_obj.insert(format!("field_{}", i), json!(i));
        new_obj.insert(format!("field_{}", i), json!(i + 1));
    }

    let old = json!(old_obj);
    let new = json!(new_obj);
    let changes = diff(&old, &new);

    // Should have 1000 modified changes
    assert_eq!(changes.modified.len(), 1000);

    // Create many ignore patterns
    let mut patterns = Vec::new();
    for i in 0..50 {
        patterns.push(format!("/field_{}", i));
    }

    // Time the filtering operation
    let start = Instant::now();
    let filtered = changes.filter_ignore_patterns(&patterns);
    let duration = start.elapsed();

    // Should filter out 50 changes (field_0 through field_49)
    assert_eq!(filtered.modified.len(), 950);

    // Performance assertion: should complete in reasonable time
    // On modern hardware, this should be < 10ms even with debug builds
    // With the optimization, this is typically < 1ms
    assert!(
        duration.as_millis() < 100,
        "Pattern matching took too long: {:?}",
        duration
    );

    // Verify the correct fields were filtered
    for change in &filtered.modified {
        if let rjd::Change::Modified { path, .. } = change {
            // Fields 0-49 should be filtered out
            let field_num = path
                .to_string()
                .trim_start_matches("field_")
                .parse::<usize>()
                .unwrap();
            assert!(
                field_num >= 50,
                "Field {} should have been filtered",
                field_num
            );
        }
    }
}
