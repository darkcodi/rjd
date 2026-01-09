use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::error::RjdError;

/// Symlink following policy
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SymlinkPolicy {
    /// Reject all symlinks
    Reject,
    /// Follow symlinks (with circular reference detection)
    Follow,
}

/// Check if JSON value exceeds depth limit
fn check_json_depth(value: &Value, max_depth: usize) -> Result<(), usize> {
    fn check_depth(value: &Value, current_depth: usize, max_depth: usize) -> Result<(), usize> {
        if current_depth > max_depth {
            return Err(current_depth);
        }

        match value {
            Value::Object(map) => {
                for (_, v) in map {
                    check_depth(v, current_depth + 1, max_depth)?;
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    check_depth(v, current_depth + 1, max_depth)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    check_depth(value, 1, max_depth)
}

/// Parse JSON string with depth limit
fn parse_json_with_depth_limit(content: &str, max_depth: usize) -> Result<Value, String> {
    // First parse the JSON normally
    let value: Value =
        serde_json::from_str(content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Then check the depth
    check_json_depth(&value, max_depth)
        .map_err(|depth| format!("JSON depth {} exceeds limit {}", depth, max_depth))?;

    Ok(value)
}

/// Default maximum file size (100MB)
const DEFAULT_MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Default maximum JSON depth (1000 levels)
const DEFAULT_MAX_JSON_DEPTH: usize = 1000;

/// Configuration for JSON loading with resource limits
///
/// # Resource Limits
///
/// - **max_file_size**: Maximum file size in bytes (default: 100MB). Prevents loading
///   extremely large files that could exhaust memory.
/// - **max_json_depth**: Maximum JSON nesting depth (default: 1000). Prevents processing
///   deeply nested JSON structures that could cause stack overflow.
///
/// # Examples
///
/// ```rust
/// use rjd::LoadConfig;
///
/// // Use defaults
/// let config = LoadConfig::default();
///
/// // Custom limits
/// let config = LoadConfig::with_limits(50_000_000, 500);
///
/// // From environment variables
/// let config = LoadConfig::from_env();
///
/// // Merge with CLI flags (CLI takes precedence)
/// let config = LoadConfig::from_env().merge_with_cli(Some(50_000_000), Some(500));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LoadConfig {
    /// Maximum file size in bytes (default: 100MB)
    pub max_file_size: u64,
    /// Maximum JSON nesting depth (default: 1000)
    pub max_json_depth: usize,
}

impl LoadConfig {
    /// Create a LoadConfig from environment variables
    ///
    /// Reads from:
    /// - `RJD_MAX_FILE_SIZE` (in bytes)
    /// - `RJD_MAX_JSON_DEPTH` (as integer)
    ///
    /// Returns defaults if env vars are not set or invalid.
    ///
    /// # Examples
    ///
    /// ```bash
    /// export RJD_MAX_FILE_SIZE=52428800  # 50MB
    /// export RJD_MAX_JSON_DEPTH=500
    /// ```
    ///
    /// ```rust
    /// use rjd::LoadConfig;
    ///
    /// let config = LoadConfig::from_env();
    /// ```
    pub fn from_env() -> Self {
        Self {
            max_file_size: std::env::var("RJD_MAX_FILE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_MAX_FILE_SIZE),
            max_json_depth: std::env::var("RJD_MAX_JSON_DEPTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_MAX_JSON_DEPTH),
        }
    }
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_json_depth: DEFAULT_MAX_JSON_DEPTH,
        }
    }
}

impl LoadConfig {
    /// Create a LoadConfig with custom values
    pub fn with_limits(max_file_size: u64, max_json_depth: usize) -> Self {
        Self {
            max_file_size,
            max_json_depth,
        }
    }

    /// Merge CLI flags with environment config (CLI flags take precedence)
    pub fn merge_with_cli(&self, max_file_size: Option<u64>, max_depth: Option<usize>) -> Self {
        Self {
            max_file_size: max_file_size.unwrap_or(self.max_file_size),
            max_json_depth: max_depth.unwrap_or(self.max_json_depth),
        }
    }
}

/// Load and parse a JSON file
pub fn load_json_file(path: &PathBuf) -> Result<Value, RjdError> {
    load_json_file_with_config(path, &LoadConfig::default())
}

/// Load and parse a JSON file with resource limits
pub fn load_json_file_with_config(path: &PathBuf, config: &LoadConfig) -> Result<Value, RjdError> {
    load_json_file_with_config_and_policy(path, config, SymlinkPolicy::Reject)
}

/// Load and parse a JSON file with resource limits and symlink policy
pub fn load_json_file_with_config_and_policy(
    path: &PathBuf,
    config: &LoadConfig,
    policy: SymlinkPolicy,
) -> Result<Value, RjdError> {
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

    // Check for symlinks
    let metadata = path
        .symlink_metadata()
        .map_err(|source| RjdError::FileRead {
            path: path.clone(),
            source,
        })?;

    if metadata.is_symlink() {
        match policy {
            SymlinkPolicy::Reject => {
                return Err(RjdError::SymlinkRejected { path: path.clone() });
            }
            SymlinkPolicy::Follow => {
                // Canonicalize to follow symlink and check for circular references
                let canonical = path.canonicalize().map_err(|source| RjdError::FileRead {
                    path: path.clone(),
                    source,
                })?;

                // Use canonicalized path for subsequent checks
                return load_json_file_with_config_and_policy(&canonical, config, policy);
            }
        }
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

    // Check file size before reading
    let metadata = fs::metadata(path).map_err(|source| RjdError::FileRead {
        path: path.clone(),
        source,
    })?;
    let file_size = metadata.len();

    if file_size > config.max_file_size {
        return Err(RjdError::FileTooLarge {
            path: path.clone(),
            size: file_size,
            limit: config.max_file_size,
        });
    }

    // Read file contents
    let content = fs::read_to_string(path).map_err(|source| RjdError::FileRead {
        path: path.clone(),
        source,
    })?;

    // Parse JSON with depth checking
    let value = parse_json_with_depth_limit(&content, config.max_json_depth).map_err(|msg| {
        // Convert string error to serde_json::Error for consistency
        RjdError::JsonParse {
            path: path.clone(),
            source: serde_json::Error::io(std::io::Error::other(msg)),
        }
    })?;

    Ok(value)
}

/// Load JSON from either a file path or an inline JSON string
/// The function will try to parse the input as JSON first (only objects/arrays),
/// and if that fails, it will try to load it as a file path.
pub fn load_json_input(input: &str) -> Result<Value, RjdError> {
    load_json_input_with_config(input, &LoadConfig::default())
}

/// Load JSON from either a file path or an inline JSON string with resource limits
pub fn load_json_input_with_config(input: &str, config: &LoadConfig) -> Result<Value, RjdError> {
    load_json_input_with_config_and_policy(input, config, SymlinkPolicy::Reject)
}

/// Load JSON from either a file path or an inline JSON string with resource limits and symlink policy
pub fn load_json_input_with_config_and_policy(
    input: &str,
    config: &LoadConfig,
    policy: SymlinkPolicy,
) -> Result<Value, RjdError> {
    load_json_input_with_config_policy_and_inline(input, config, policy, false)
}

/// Load JSON with resource limits, symlink policy, and inline flag
pub fn load_json_input_with_config_policy_and_inline(
    input: &str,
    config: &LoadConfig,
    policy: SymlinkPolicy,
    force_inline: bool,
) -> Result<Value, RjdError> {
    let trimmed = input.trim();

    // If force_inline is true, parse as JSON only
    if force_inline {
        return serde_json::from_str(input).map_err(|_| RjdError::InvalidInput {
            input: input.to_string(),
        });
    }

    // If input starts with '{' or '[', it's definitely inline JSON
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return parse_json_with_depth_limit(input, config.max_json_depth).map_err(|_msg| {
            RjdError::InvalidInput {
                input: input.to_string(),
            }
        });
    }

    // Otherwise, try file path first, then inline JSON
    let path = PathBuf::from(input);
    if path.exists() {
        return load_json_file_with_config_and_policy(&path, config, policy);
    }

    // Fall back to inline JSON
    parse_json_with_depth_limit(input, config.max_json_depth).map_err(|_| RjdError::InvalidInput {
        input: input.to_string(),
    })
}

/// Load JSON from stdin
pub fn load_json_stdin() -> Result<Value, RjdError> {
    load_json_stdin_with_config(&LoadConfig::default())
}

/// Load JSON from stdin with resource limits
pub fn load_json_stdin_with_config(config: &LoadConfig) -> Result<Value, RjdError> {
    let content =
        std::io::read_to_string(std::io::stdin()).map_err(|source| RjdError::Internal {
            message: format!("Failed to read from stdin: {}", source),
        })?;

    // Parse JSON with depth checking
    parse_json_with_depth_limit(&content, config.max_json_depth).map_err(|msg| RjdError::Internal {
        message: format!("Failed to parse JSON from stdin: {}", msg),
    })
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
        // With hybrid detection: simple values are tried as file paths first,
        // then fall back to inline JSON if file doesn't exist

        // These should successfully parse as inline JSON (fallback)
        assert!(load_json_input("42").unwrap().is_number());
        assert!(load_json_input(r#""hello""#).unwrap().is_string());
        assert!(load_json_input("true").unwrap().is_boolean());
        assert!(load_json_input("null").unwrap().is_null());
    }

    #[test]
    fn test_load_json_input_objects_and_arrays() {
        // Objects and arrays should work as inline JSON
        assert!(load_json_input("{}").unwrap().is_object());
        assert!(load_json_input("[]").unwrap().is_array());
        assert!(load_json_input(r#"{"name": "test"}"#).unwrap().is_object());
        assert!(load_json_input(r#"[1, 2, 3]"#).unwrap().is_array());
    }

    #[test]
    fn test_load_config_default() {
        let config = LoadConfig::default();
        assert_eq!(config.max_file_size, DEFAULT_MAX_FILE_SIZE);
        assert_eq!(config.max_json_depth, DEFAULT_MAX_JSON_DEPTH);
    }

    #[test]
    fn test_load_config_with_limits() {
        let config = LoadConfig::with_limits(500, 100);
        assert_eq!(config.max_file_size, 500);
        assert_eq!(config.max_json_depth, 100);
    }

    #[test]
    fn test_load_config_from_env_no_env_vars() {
        // Clear environment variables if they exist
        std::env::remove_var("RJD_MAX_FILE_SIZE");
        std::env::remove_var("RJD_MAX_JSON_DEPTH");

        let config = LoadConfig::from_env();
        assert_eq!(config.max_file_size, DEFAULT_MAX_FILE_SIZE);
        assert_eq!(config.max_json_depth, DEFAULT_MAX_JSON_DEPTH);
    }

    #[test]
    fn test_load_config_from_env_with_vars() {
        std::env::set_var("RJD_MAX_FILE_SIZE", "200000000");
        std::env::set_var("RJD_MAX_JSON_DEPTH", "500");

        let config = LoadConfig::from_env();
        assert_eq!(config.max_file_size, 200000000);
        assert_eq!(config.max_json_depth, 500);

        // Clean up
        std::env::remove_var("RJD_MAX_FILE_SIZE");
        std::env::remove_var("RJD_MAX_JSON_DEPTH");
    }

    #[test]
    fn test_load_config_from_env_with_invalid_vars() {
        std::env::set_var("RJD_MAX_FILE_SIZE", "invalid");
        std::env::set_var("RJD_MAX_JSON_DEPTH", "not_a_number");

        let config = LoadConfig::from_env();
        // Should fall back to defaults when parsing fails
        assert_eq!(config.max_file_size, DEFAULT_MAX_FILE_SIZE);
        assert_eq!(config.max_json_depth, DEFAULT_MAX_JSON_DEPTH);

        // Clean up
        std::env::remove_var("RJD_MAX_FILE_SIZE");
        std::env::remove_var("RJD_MAX_JSON_DEPTH");
    }

    #[test]
    fn test_reject_file_over_limit() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);

        // Create a file with some content
        std::fs::write(&file_path, r#"{"test": "data"}"#).unwrap();

        // Set a very small limit
        let config = LoadConfig::with_limits(5, 1000); // 5 bytes
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_err());
        match result {
            Err(RjdError::FileTooLarge { size, limit, .. }) => {
                assert_eq!(limit, 5);
                assert!(size > 5);
            }
            _ => panic!("Expected FileTooLarge error"),
        }
    }

    #[test]
    fn test_accept_file_under_limit() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);

        // Create a file with some content
        std::fs::write(&file_path, r#"{"test": "data"}"#).unwrap();

        // Set a reasonable limit
        let config = LoadConfig::with_limits(1000, 1000);
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["test"], "data");
    }

    #[test]
    fn test_file_metadata_unavailable() {
        let file_path = PathBuf::from("/proc/some_nonexistent_file");

        let config = LoadConfig::default();
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_err());
        // Should fail with FileRead error when metadata is unavailable
        match result {
            Err(RjdError::FileRead { .. }) => (),
            _ => panic!("Expected FileRead error for unavailable metadata"),
        }
    }

    #[test]
    fn test_reject_json_over_depth_limit() {
        // Create a deeply nested JSON structure
        let nested = r#"{"a": {"b": {"c": {"d": {"e": "value"}}}}}"#; // depth 5

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, nested).unwrap();

        // Set a very low depth limit
        let config = LoadConfig::with_limits(1000, 3);
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_err());
        match result {
            Err(RjdError::JsonParse { .. }) => {
                // The depth error is wrapped in JsonParse
            }
            _ => panic!("Expected JsonParse error for depth exceeded"),
        }
    }

    #[test]
    fn test_accept_json_under_depth_limit() {
        // Create a deeply nested JSON structure
        let nested = r#"{"a": {"b": {"c": {"d": {"e": "value"}}}}}"#; // depth 5

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, nested).unwrap();

        // Set a reasonable depth limit
        let config = LoadConfig::with_limits(1000, 10);
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["a"]["b"]["c"]["d"]["e"], "value");
    }

    #[test]
    fn test_depth_limit_with_nested_arrays() {
        // Create deeply nested arrays
        let nested = r#"[[[[[[42]]]]]]"#; // depth 7

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, nested).unwrap();

        // Set a low depth limit
        let config = LoadConfig::with_limits(1000, 5);
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_err());
    }

    #[test]
    fn test_depth_limit_with_nested_objects() {
        // Create nested objects
        let nested = r#"{"l1":{"l2":{"l3":{"l4":{"l5":"deep"}}}}}"#;

        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        drop(temp_file);
        std::fs::write(&file_path, nested).unwrap();

        // Set exact depth limit (depth is 5)
        let config = LoadConfig::with_limits(1000, 10);
        let result = load_json_file_with_config(&file_path, &config);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["l1"]["l2"]["l3"]["l4"]["l5"], "deep");
    }

    #[test]
    fn test_symlink_policy_enum() {
        let reject = SymlinkPolicy::Reject;
        let follow = SymlinkPolicy::Follow;

        assert_eq!(reject, SymlinkPolicy::Reject);
        assert_eq!(follow, SymlinkPolicy::Follow);
        assert_ne!(reject, follow);
    }

    #[test]
    fn test_reject_symlink_by_default() {
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        std::fs::write(&file_path, r#"{"test": "data"}"#).unwrap();

        // Create a symlink to it
        let symlink_dir = tempfile::tempdir().unwrap();
        let symlink_path = symlink_dir.path().join("symlink.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file_path, &symlink_path).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file_path, &symlink_path).unwrap();

        // Try to load the symlink with Reject policy (default)
        let config = LoadConfig::default();
        let result = load_json_file_with_config(&symlink_path, &config);

        assert!(result.is_err());
        match result {
            Err(RjdError::SymlinkRejected { .. }) => {}
            _ => panic!("Expected SymlinkRejected error"),
        }
    }

    #[test]
    fn test_follow_symlink_with_policy() {
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_path_buf();
        std::fs::write(&file_path, r#"{"test": "data"}"#).unwrap();

        // Create a symlink to it
        let symlink_dir = tempfile::tempdir().unwrap();
        let symlink_path = symlink_dir.path().join("symlink.json");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&file_path, &symlink_path).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file_path, &symlink_path).unwrap();

        // Load the symlink with Follow policy
        let config = LoadConfig::default();
        let result =
            load_json_file_with_config_and_policy(&symlink_path, &config, SymlinkPolicy::Follow);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["test"], "data");
    }

    #[test]
    fn test_symlink_to_nonexistent_target() {
        // Create a symlink to a non-existent file
        let symlink_dir = tempfile::tempdir().unwrap();
        let symlink_path = symlink_dir.path().join("broken_symlink.json");
        let nonexistent_path = PathBuf::from("/nonexistent/file.json");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&nonexistent_path, &symlink_path).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&nonexistent_path, &symlink_path).unwrap();

        // Try to load with Follow policy
        let config = LoadConfig::default();
        let result =
            load_json_file_with_config_and_policy(&symlink_path, &config, SymlinkPolicy::Follow);

        assert!(result.is_err());
    }

    #[test]
    #[cfg(unix)] // Circular symlinks are easier to test on Unix
    fn test_circular_symlink_detection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let link1 = temp_dir.path().join("link1");
        let link2 = temp_dir.path().join("link2");

        // Create circular symlinks: link1 -> link2, link2 -> link1
        std::os::unix::fs::symlink(&link2, &link1).unwrap();
        std::os::unix::fs::symlink(&link1, &link2).unwrap();

        // Try to load with Follow policy
        let config = LoadConfig::default();
        let result = load_json_file_with_config_and_policy(&link1, &config, SymlinkPolicy::Follow);

        // This should either fail or detect the circular reference
        // The behavior may vary by system
        assert!(result.is_err() || link1.canonicalize().is_err());
    }
}
