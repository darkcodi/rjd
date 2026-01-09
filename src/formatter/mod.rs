//! Formatter module for outputting diff results
//!
//! This module provides different output formatters for diff results.
//! The default is the "changes" format which outputs a structured JSON
//! object with added, removed, and modified arrays.

mod after;
mod changes;
mod json_patch;
mod util;

pub use after::AfterFormatter;
pub use changes::ChangesFormatter;
pub use json_patch::JsonPatchFormatter;
pub use util::sort_json_value;

/// Trait for formatting diff results
pub trait Formatter {
    /// Format the changes and return a string representation
    fn format(&self, changes: &crate::types::Changes)
        -> Result<String, Box<dyn std::error::Error>>;
}

/// Factory function to create a formatter based on output format string
///
/// # Arguments
/// * `format_str` - One of "changes", "after", or "rfc6902"
/// * `sort` - Whether to sort keys in JSON output
///
/// # Panics
/// Panics if format_str is not one of the valid values
pub fn create_formatter(format_str: &str, sort: bool) -> Box<dyn Formatter> {
    match format_str {
        "changes" => Box::new(ChangesFormatter::new(sort)),
        "after" => Box::new(AfterFormatter::new(sort)),
        "rfc6902" => Box::new(JsonPatchFormatter::new(sort)),
        _ => panic!(
            "Invalid format: {}. Must be one of: changes, after, rfc6902",
            format_str
        ),
    }
}
