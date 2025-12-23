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

    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

// Note: From implementations for IO/JSON errors are intentionally omitted.
// These errors require a path context which cannot be provided by From.
// Use map_err with explicit path construction instead:
//   fs::read_to_string(path).map_err(|source| RjdError::FileRead { path: path.clone(), source })?;
