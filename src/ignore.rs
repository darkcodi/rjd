use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::RjdError;

/// Extract paths from a JSON object recursively.
/// For each key with a truthy value, adds the path /prefix/key.
/// Only adds leaf paths (doesn't add intermediate parent paths).
fn extract_paths_from_value(value: &Value, prefix: &str, paths: &mut Vec<String>) {
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            // Check if the value is truthy (true, non-empty object, or number)
            let is_truthy = val == &Value::Bool(true)
                || (val.is_object() && !val.as_object().unwrap().is_empty())
                || val.is_number();

            if is_truthy {
                let path = if prefix.is_empty() {
                    format!("/{}", key)
                } else {
                    format!("{}/{}", prefix, key)
                };

                // If it's a non-empty object with truthy nested values, recurse
                if val.is_object() && !val.as_object().unwrap().is_empty() {
                    extract_paths_from_value(val, &path, paths);
                } else {
                    // Leaf node - add the path
                    paths.push(path);
                }
            }
        }
    }
}

/// Load ignore patterns from a JSON file.
/// The file can contain either:
/// - A JSON array of strings: ["/user/id", "/config/password"]
/// - A JSON object with truthy values: {"user": {"id": true}, "tags": true}
pub fn load_ignore_patterns(path: &Path) -> Result<Vec<String>, RjdError> {
    // Check if file exists
    if !path.exists() {
        return Err(RjdError::FileRead {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ),
        });
    }

    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Err(RjdError::FileRead {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Not a file: {}", path.display()),
            ),
        });
    }

    // Read file contents
    let content = fs::read_to_string(path).map_err(|source| RjdError::FileRead {
        path: path.to_path_buf(),
        source,
    })?;

    // Parse JSON as Value first to detect type
    let value: Value = serde_json::from_str(&content).map_err(|source| RjdError::JsonParse {
        path: path.to_path_buf(),
        source,
    })?;

    // Handle array format
    if let Some(arr) = value.as_array() {
        let patterns: Vec<String> =
            serde_json::from_value(Value::Array(arr.clone())).map_err(|source| {
                RjdError::JsonParse {
                    path: path.to_path_buf(),
                    source,
                }
            })?;

        // Validate that paths start with / (JSON Pointer format)
        for pattern in &patterns {
            if !pattern.starts_with('/') {
                return Err(RjdError::Internal {
                    message: format!(
                        "Ignore pattern '{}' must start with '/' (JSON Pointer format)",
                        pattern
                    ),
                });
            }
        }

        return Ok(patterns);
    }

    // Handle object format
    if value.is_object() {
        let mut patterns = Vec::new();
        extract_paths_from_value(&value, "", &mut patterns);

        // Sort and deduplicate patterns
        patterns.sort();
        patterns.dedup();

        return Ok(patterns);
    }

    // Neither array nor object
    Err(RjdError::Internal {
        message: "Ignore file must be either a JSON array of strings or a JSON object".to_string(),
    })
}

/// Load and combine ignore patterns from multiple JSON files
pub fn load_all_ignore_patterns(paths: &[String]) -> Result<Vec<String>, RjdError> {
    let mut all_patterns = Vec::new();

    for path_str in paths {
        let path = Path::new(path_str);
        let patterns = load_ignore_patterns(path)?;
        all_patterns.extend(patterns);
    }

    Ok(all_patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_patterns() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"["/user/id", "/config/password", "/a/b/c"]"#).unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        let patterns = result.unwrap();
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns[0], "/user/id");
    }

    #[test]
    fn test_load_empty_array() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"[]"#).unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_invalid_json() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"not valid json"#).unwrap();

        let result = load_ignore_patterns(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_object_with_string_values_ignored() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        // String values are not truthy, so they should be ignored
        std::fs::write(&file_path, r#"{"key": "value"}"#).unwrap();

        let result = load_ignore_patterns(&file_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_invalid_pattern_missing_slash() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"["user/id", "/config/password"]"#).unwrap();

        let result = load_ignore_patterns(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_ignore_patterns(Path::new("/nonexistent/paths.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_all_patterns_multiple_files() {
        let temp_file1 = NamedTempFile::new().unwrap();
        let file_path1 = temp_file1.path().to_path_buf();
        drop(temp_file1);
        std::fs::write(&file_path1, r#"["/a/b", "/c/d"]"#).unwrap();

        let temp_file2 = NamedTempFile::new().unwrap();
        let file_path2 = temp_file2.path().to_path_buf();
        drop(temp_file2);
        std::fs::write(&file_path2, r#"["/e/f"]"#).unwrap();

        let paths = vec![
            file_path1.to_string_lossy().to_string(),
            file_path2.to_string_lossy().to_string(),
        ];
        let result = load_all_ignore_patterns(&paths);

        assert!(result.is_ok());
        let patterns = result.unwrap();
        assert_eq!(patterns.len(), 3);
        assert_eq!(patterns, vec!["/a/b", "/c/d", "/e/f"]);
    }

    #[test]
    fn test_load_object_format() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(
            &file_path,
            r#"{"user": {"id": true, "name": true}, "tags": true}"#,
        )
        .unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        let patterns = result.unwrap();
        // Should include only leaf paths: /user/id, /user/name, /tags (not /user)
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"/user/id".to_string()));
        assert!(patterns.contains(&"/user/name".to_string()));
        assert!(patterns.contains(&"/tags".to_string()));
    }

    #[test]
    fn test_load_object_nested() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"{"a": {"b": {"c": true}}}"#).unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        let patterns = result.unwrap();
        // Should include only the leaf path: /a/b/c (not /a or /a/b)
        assert_eq!(patterns.len(), 1);
        assert!(patterns.contains(&"/a/b/c".to_string()));
    }

    #[test]
    fn test_load_object_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"{}"#).unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_load_object_false_values_ignored() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(
            &file_path,
            r#"{"user": {"id": true, "skip": false}, "tags": true}"#,
        )
        .unwrap();

        let result = load_ignore_patterns(&file_path);

        assert!(result.is_ok());
        let patterns = result.unwrap();
        // /user/id, /tags (skip is false so it's ignored, user is not added as it's a parent)
        assert_eq!(patterns.len(), 2);
        assert!(patterns.contains(&"/user/id".to_string()));
        assert!(patterns.contains(&"/tags".to_string()));
        assert!(!patterns.contains(&"/user/skip".to_string()));
    }
}
