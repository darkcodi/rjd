//! Integration tests for the CLI

use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_compare_two_json_files() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("file2.json");

    fs::write(&file1, r#"{"name": "John"}"#).unwrap();
    fs::write(&file2, r#"{"name": "John", "age": 30}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1).arg(&file2);
    cmd.assert().success();
}

#[test]
fn test_inline_json_comparison() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"a": 1}"#).arg(r#"{"a": 2}"#);
    cmd.assert().success();
}

#[test]
fn test_output_format_changes() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"name": "John"}"#)
        .arg(r#"{"name": "Jane"}"#)
        .arg("--format")
        .arg("changes");
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("modified"));
}

#[test]
fn test_output_format_after() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"name": "John"}"#)
        .arg(r#"{"name": "John", "age": 30}"#)
        .arg("--format")
        .arg("after");
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("age"));
}

#[test]
fn test_output_format_rfc6902() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"name": "John"}"#)
        .arg(r#"{"name": "Jane"}"#)
        .arg("--format")
        .arg("rfc6902");
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("replace"));
}

#[test]
fn test_sort_option() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"b": 1, "a": 2}"#)
        .arg(r#"{"b": 1, "a": 3, "c": 4}"#)
        .arg("--sort");
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_invalid_json_file() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("invalid.json");
    fs::write(&file1, r#"{invalid json"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1).arg("{}");
    cmd.assert().failure();
}

#[test]
fn test_missing_file() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg("/nonexistent/file.json").arg("{}");
    cmd.assert().failure();
}

#[test]
fn test_no_changes_equal_objects() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"name": "John", "age": 30}"#)
        .arg(r#"{"name": "John", "age": 30}"#);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["added"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["removed"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["modified"].as_array().unwrap().len(), 0);
}

#[test]
fn test_nested_json_comparison() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(json!({"user": {"name": "John"}}).to_string())
        .arg(json!({"user": {"name": "Jane"}}).to_string());
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_array_comparison() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"[1, 2, 3]"#).arg(r#"[1, 4, 3]"#);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
}

#[test]
fn test_stdin_flag() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"a": 1}"#)
        .arg("--stdin")
        .write_stdin(r#"{"a": 2}"#);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("modified"));
}
