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

impl From<std::io::Error> for RjdError {
    fn from(error: std::io::Error) -> Self {
        RjdError::FileRead {
            path: PathBuf::new(),
            source: error,
        }
    }
}

impl From<serde_json::Error> for RjdError {
    fn from(error: serde_json::Error) -> Self {
        RjdError::JsonParse {
            path: PathBuf::new(),
            source: error,
        }
    }
}
