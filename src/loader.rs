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
}
