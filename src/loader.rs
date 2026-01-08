use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::error::RjdError;

/// Load and parse a JSON file
pub fn load_json_file(path: &PathBuf) -> Result<Value, RjdError> {
    // Check if file exists
    if !path.exists() {
        return Err(RjdError::FileRead {
            path: path.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ),
        });
    }

    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Err(RjdError::FileRead {
            path: path.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Not a file: {}", path.display()),
            ),
        });
    }

    // Read file contents
    let content = fs::read_to_string(path).map_err(|source| RjdError::FileRead {
        path: path.clone(),
        source,
    })?;

    // Parse JSON
    let value = serde_json::from_str(&content).map_err(|source| RjdError::JsonParse {
        path: path.clone(),
        source,
    })?;

    Ok(value)
}

/// Load JSON from either a file path or an inline JSON string
/// The function will try to parse the input as JSON first (only objects/arrays),
/// and if that fails, it will try to load it as a file path.
pub fn load_json_input(input: &str) -> Result<Value, RjdError> {
    // First, try to parse as inline JSON
    if let Ok(value) = serde_json::from_str::<Value>(input) {
        // Only accept objects or arrays as inline JSON
        // Simple values (number, string, boolean, null) are treated as file paths
        if value.is_object() || value.is_array() {
            return Ok(value);
        }
    }

    // If parsing as inline JSON failed or wasn't an object/array, try as file path
    let path = PathBuf::from(input);
    load_json_file(&path)
}

/// Load JSON from stdin
#[allow(dead_code)]
pub fn load_json_stdin() -> Result<Value, RjdError> {
    let content =
        std::io::read_to_string(std::io::stdin()).map_err(|source| RjdError::Internal {
            message: format!("Failed to read from stdin: {}", source),
        })?;
    let value = serde_json::from_str(&content).map_err(|source| RjdError::Internal {
        message: format!("Failed to parse JSON from stdin: {}", source),
    })?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_valid_json() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"{"name": "test", "value": 42}"#).unwrap();

        let result = load_json_file(&file_path);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_json_file(&PathBuf::from("/nonexistent/file.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"{"invalid": json}"#).unwrap();

        let result = load_json_file(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_json_input_inline_json() {
        let result = load_json_input(r#"{"name": "test", "value": 42}"#);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn test_load_json_input_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, r#"{"name": "test", "value": 42}"#).unwrap();

        let result = load_json_input(&file_path.to_string_lossy());

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[test]
    fn test_load_json_input_simple_values_as_file() {
        // Simple values should be treated as file paths, not inline JSON
        assert!(load_json_input("42").is_err());
        assert!(load_json_input(r#""hello""#).is_err());
        assert!(load_json_input("true").is_err());
        assert!(load_json_input("null").is_err());
    }

    #[test]
    fn test_load_json_input_objects_and_arrays() {
        // Objects and arrays should work as inline JSON
        assert!(load_json_input("{}").unwrap().is_object());
        assert!(load_json_input("[]").unwrap().is_array());
        assert!(load_json_input(r#"{"name": "test"}"#).unwrap().is_object());
        assert!(load_json_input(r#"[1, 2, 3]"#).unwrap().is_array());
    }
}
