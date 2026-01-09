use std::path::PathBuf;
use thiserror::Error;

/// Custom error type for rjd operations
#[derive(Debug, Error)]
pub enum RjdError {
    #[error("Failed to read file {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse JSON from {path}: {source}")]
    JsonParse {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("File too large: {path} (size: {size} bytes, limit: {limit} bytes)")]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("JSON depth exceeded: depth {depth} exceeds limit {limit}")]
    JsonDepthExceeded { depth: usize, limit: usize },

    #[error("Symlink rejected: {path}")]
    SymlinkRejected { path: PathBuf },

    #[error("Circular symlink detected: {path}")]
    CircularSymlink { path: PathBuf },

    #[error("Missing second file argument")]
    MissingFile2,

    #[error("Invalid input: {input}")]
    InvalidInput { input: String },

    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Formatter error: {message}")]
    Formatter { message: String },
}

/// Formatter-specific errors
#[derive(Debug, Error)]
pub enum FormatterError {
    #[error("Unknown format '{format}'. Valid formats are: {valid}")]
    UnknownFormat { format: String, valid: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatter_error_unknown_format_display() {
        let error = FormatterError::UnknownFormat {
            format: "json".to_string(),
            valid: "changes, after, rfc6902".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("json"));
        assert!(msg.contains("changes, after, rfc6902"));
        assert!(msg.contains("Unknown format"));
    }

    #[test]
    fn test_formatter_error_empty_format() {
        let error = FormatterError::UnknownFormat {
            format: "".to_string(),
            valid: "changes, after, rfc6902".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("changes"));
        assert!(msg.contains("after"));
        assert!(msg.contains("rfc6902"));
    }
}

// Note: From implementations for IO/JSON errors are intentionally omitted.
// These errors require a path context which cannot be provided by From.
// Use map_err with explicit path construction instead:
//   fs::read_to_string(path).map_err(|source| RjdError::FileRead { path: path.clone(), source })?;
