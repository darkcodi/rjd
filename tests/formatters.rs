//! Integration tests for formatters

use rjd::{cli::OutputFormat, create_formatter, diff};
use serde_json::json;

#[test]
fn test_changes_formatter_output() {
    let old = json!({"name": "John", "age": 30});
    let new = json!({"name": "Jane", "age": 30});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Changes, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("added").is_some());
    assert!(parsed.get("removed").is_some());
    assert!(parsed.get("modified").is_some());
    assert_eq!(parsed["modified"].as_array().unwrap().len(), 1);
}

#[test]
fn test_after_formatter_output() {
    let old = json!({"name": "John"});
    let new = json!({"name": "John", "age": 30, "city": "NYC"});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::After, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    // After format shows the result with added/modified properties
    assert_eq!(parsed["age"], 30);
    assert_eq!(parsed["city"], "NYC");
}

#[test]
fn test_rfc6902_formatter_output() {
    let old = json!({"name": "John"});
    let new = json!({"name": "Jane"});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Rfc6902, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    // RFC 6902 is an array of JSON Patch operations
    assert!(parsed.is_array());
    let ops = parsed.as_array().unwrap();
    assert!(!ops.is_empty());
    // Should have a "replace" operation
    assert!(ops.iter().any(|op| op["op"] == "replace"));
}

#[test]
fn test_changes_formatter_with_sort() {
    let old = json!({"z": 1, "a": 2, "m": 3});
    let new = json!({"z": 10, "a": 2, "m": 3});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Changes, true);
    let output = formatter.format(&changes).unwrap();

    assert!(output.contains("\"z\""));
}

#[test]
fn test_after_formatter_with_sort() {
    let old = json!({});
    let new = json!({"c": 3, "a": 1, "b": 2});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::After, true);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["a"], 1);
    assert_eq!(parsed["b"], 2);
    assert_eq!(parsed["c"], 3);
}

#[test]
fn test_rfc6902_add_operation() {
    let old = json!({});
    let new = json!({"key": "value"});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Rfc6902, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let ops = parsed.as_array().unwrap();
    assert!(ops.iter().any(|op| op["op"] == "add"));
}

#[test]
fn test_rfc6902_remove_operation() {
    let old = json!({"key": "value"});
    let new = json!({});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Rfc6902, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let ops = parsed.as_array().unwrap();
    assert!(ops.iter().any(|op| op["op"] == "remove"));
}

#[test]
fn test_empty_changes_all_formats() {
    let old = json!({"name": "John"});
    let new = json!({"name": "John"});

    for format in [
        OutputFormat::Changes,
        OutputFormat::After,
        OutputFormat::Rfc6902,
    ] {
        let changes = diff(&old, &new);
        let formatter = create_formatter(format, false);
        let output = formatter.format(&changes).unwrap();
        assert!(!output.is_empty());
    }
}

#[test]
fn test_nested_object_formatter() {
    let old = json!({"user": {"name": "John"}});
    let new = json!({"user": {"name": "Jane", "email": "john@example.com"}});
    let changes = diff(&old, &new);

    let formatter = create_formatter(OutputFormat::Changes, false);
    let output = formatter.format(&changes).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed["added"].is_array());
    assert!(parsed["modified"].is_array());
}
