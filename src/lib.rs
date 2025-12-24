//! RJD - Rust JSON Diff library
//!
//! This library provides JSON comparison and diff functionality.

pub use cli::Args;
pub use cli::OutputFormat;
pub use diff::diff;
pub use error::RjdError;
pub use formatter::create_formatter;
pub use loader::{load_json_file, load_json_input};
pub use types::{Change, Changes};

pub mod cli;
mod diff;
mod error;
pub mod formatter;
mod loader;
mod path;
pub mod types;
