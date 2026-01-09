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

#[test]
fn test_ignore_json_option() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("ignore.json");

    fs::write(&file1, r#"{"user": {"id": 1, "name": "John"}, "age": 30}"#).unwrap();
    fs::write(&file2, r#"["/user/id"]"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1)
        .arg(r#"{"user": {"id": 2, "name": "Jane"}, "age": 40}"#)
        .arg("--ignore-json")
        .arg(&file2);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // user.id should be filtered out, but user.name and age should remain
    assert!(!stdout.contains("user.id"));
    assert!(stdout.contains("user.name"));
    assert!(stdout.contains("age"));
}

#[test]
fn test_ignore_json_multiple_patterns() {
    let dir = TempDir::new().unwrap();
    let ignore_file = dir.path().join("ignore.json");
    fs::write(&ignore_file, r#"["/user/id", "/tags"]"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"user": {"id": 1, "name": "John"}, "tags": ["a", "b"]}"#)
        .arg(r#"{"user": {"id": 2, "name": "Jane"}, "tags": ["a", "b", "c"]}"#)
        .arg("--ignore-json")
        .arg(&ignore_file);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // user.id and tags should be filtered out
    assert!(!stdout.contains("user.id"));
    assert!(!stdout.contains("tags"));
    // user.name should remain
    assert!(stdout.contains("user.name"));
}

#[test]
fn test_ignore_json_multiple_files() {
    let dir = TempDir::new().unwrap();
    let ignore1 = dir.path().join("ignore1.json");
    let ignore2 = dir.path().join("ignore2.json");

    fs::write(&ignore1, r#"["/user/id"]"#).unwrap();
    fs::write(&ignore2, r#"["/user/name"]"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"user": {"id": 1, "name": "John"}}"#)
        .arg(r#"{"user": {"id": 2, "name": "Jane"}}"#)
        .arg("--ignore-json")
        .arg(&ignore1)
        .arg("--ignore-json")
        .arg(&ignore2);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Both patterns should be applied
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["modified"].as_array().unwrap().len(), 0);
}

#[test]
fn test_ignore_json_missing_file() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"a": 1}"#)
        .arg(r#"{"a": 2}"#)
        .arg("--ignore-json")
        .arg("/nonexistent/path.json");
    cmd.assert().failure();
}

#[test]
fn test_ignore_json_object_format() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("ignore.json");

    fs::write(
        &file1,
        r#"{"user": {"id": 1, "name": "John"}, "tags": ["a", "b"], "age": 30}"#,
    )
    .unwrap();
    fs::write(&file2, r#"{"user": {"id": true}, "tags": true}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1)
        .arg(r#"{"user": {"id": 2, "name": "Jane"}, "tags": ["a", "b", "c"], "age": 40}"#)
        .arg("--ignore-json")
        .arg(&file2);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // user.id and tags should be filtered out (but user and user.name should remain)
    assert!(!stdout.contains("user.id"));
    assert!(!stdout.contains("tags"));
    // user.name should remain since we only ignored user.id (not the parent user)
    assert!(stdout.contains("user.name"));
    assert!(stdout.contains("age"));
}

#[test]
fn test_ignore_json_invalid_path() {
    let dir = TempDir::new().unwrap();
    let ignore_file = dir.path().join("ignore.json");

    // Write an array with paths that don't start with /
    fs::write(&ignore_file, r#"["user/id", "/valid/path"]"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"a": 1}"#)
        .arg(r#"{"a": 2}"#)
        .arg("--ignore-json")
        .arg(&ignore_file);
    cmd.assert().failure();
}

// Integration tests for new security and resource limit features

#[test]
fn test_inline_flag_forces_json_parsing() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    // Use --inline to treat these as JSON even though they look like file paths
    cmd.arg("--inline").arg(r#"{"a": 1}"#).arg(r#"{"b": 2}"#);
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should parse as inline JSON, not try to read from files
    assert!(stdout.contains("modified") || stdout.contains("added") || stdout.contains("removed"));
}

#[test]
fn test_max_file_size_flag() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("file2.json");

    // Create small files
    fs::write(&file1, r#"{"a": 1}"#).unwrap();
    fs::write(&file2, r#"{"a": 2}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1)
        .arg(&file2)
        .arg("--max-file-size")
        .arg("100"); // Very small limit (100 bytes)
    cmd.assert().success(); // Small files should pass
}

#[test]
fn test_max_file_size_rejects_large_file() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("file2.json");

    // Create a larger JSON file (about 200 bytes)
    let large_json = format!(r#"{{"data":"{}"}}"#, "x".repeat(180));
    fs::write(&file1, &large_json).unwrap();
    fs::write(&file2, r#"{"a": 2}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1)
        .arg(&file2)
        .arg("--max-file-size")
        .arg("100"); // Very small limit (100 bytes)
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("File too large") || stderr.contains("exceeds limit"));
}

#[test]
fn test_max_depth_flag() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(r#"{"a": 1}"#)
        .arg(r#"{"a": 2}"#)
        .arg("--max-depth")
        .arg("10"); // Low depth limit
    cmd.assert().success(); // Shallow JSON should pass
}

#[test]
fn test_max_depth_rejects_deep_json() {
    // Create deeply nested JSON (depth 15) by parsing a valid JSON string
    // Build it as: {"a": {"a": {"a": ... {"a": 1} ... }}}
    let mut deep_json = String::from("1");
    for _ in 0..14 {
        // Create {"a": <previous>} 14 times to get depth 15 (leaf value at depth 15)
        deep_json = format!(r#"{{"a":{}}}"#, deep_json);
    }

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&deep_json)
        .arg(r#"{"a": 2}"#)
        .arg("--max-depth")
        .arg("10"); // Limit depth to 10
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The error should mention depth limits or JSON parsing failure
    assert!(
        stderr.contains("limit")
            || stderr.contains("Invalid input")
            || stderr.contains("Failed to parse")
    );
}

#[test]
fn test_follow_symlinks_flag() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let symlink = dir.path().join("symlink.json");

    fs::write(&file1, r#"{"a": 1}"#).unwrap();

    // Create a symbolic link
    #[cfg(unix)]
    std::os::unix::fs::symlink(&file1, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&file1, &symlink).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&symlink)
        .arg(r#"{"a": 2}"#)
        .arg("--follow-symlinks");
    cmd.assert().success(); // Should follow symlink and succeed
}

#[test]
fn test_symlink_rejected_by_default() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let symlink = dir.path().join("symlink.json");

    fs::write(&file1, r#"{"a": 1}"#).unwrap();

    // Create a symbolic link
    #[cfg(unix)]
    std::os::unix::fs::symlink(&file1, &symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&file1, &symlink).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&symlink).arg(r#"{"a": 2}"#);
    let output = cmd.output().unwrap();
    assert!(!output.status.success()); // Should reject symlink by default
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("symlink") || stderr.contains("Symlink"));
}

#[test]
fn test_combined_flags() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");
    let file2 = dir.path().join("file2.json");

    fs::write(&file1, r#"{"a": 1, "b": 2}"#).unwrap();
    fs::write(&file2, r#"{"a": 1, "b": 3}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1)
        .arg(&file2)
        .arg("--sort")
        .arg("--max-file-size")
        .arg("10000")
        .arg("--max-depth")
        .arg("100");
    cmd.assert().success();
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("modified"));
}

#[test]
fn test_cli_validation_missing_file2() {
    let dir = TempDir::new().unwrap();
    let file1 = dir.path().join("file1.json");

    fs::write(&file1, r#"{"a": 1}"#).unwrap();

    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg(&file1); // Missing second file
    cmd.assert().failure();
}

#[test]
fn test_error_message_clarity() {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("rjd").unwrap();
    cmd.arg("/nonexistent/file1.json")
        .arg("/nonexistent/file2.json");
    let output = cmd.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Error message should be clear and helpful
    assert!(
        stderr.contains("file") || stderr.contains("not found") || stderr.contains("No such file")
    );
}
