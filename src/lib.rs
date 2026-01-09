//! RJD - Rust JSON Diff library
//!
//! This library provides JSON comparison and diff functionality.

pub use diff::diff;
pub use error::RjdError;
pub use formatter::create_formatter;
pub use ignore::{load_all_ignore_patterns, load_ignore_patterns};
pub use json_path::{JsonPath, ParseError, PathSegment};
pub use loader::{
    load_json_file, load_json_file_with_config, load_json_file_with_config_and_policy,
    load_json_input, load_json_input_with_config, load_json_input_with_config_and_policy,
    load_json_input_with_config_policy_and_inline, load_json_stdin, load_json_stdin_with_config,
    LoadConfig, SymlinkPolicy,
};
pub use types::{Change, Changes};

mod diff;
mod error;
pub mod formatter;
pub mod ignore;
pub mod json_path;
mod loader;
mod path;
pub mod types;
