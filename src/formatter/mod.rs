//! Formatter module for outputting diff results
//!
//! This module provides different output formatters for diff results.
//! The default is the "changes" format which outputs a structured JSON
//! object with added, removed, and modified arrays.

mod after;
mod changes;
mod json_patch;
mod path_filter;
pub mod path_parser;
mod util;

pub use after::AfterFormatter;
pub use changes::ChangesFormatter;
pub use json_patch::JsonPatchFormatter;
pub use util::sort_json_value;

use crate::error::FormatterError;

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
/// # Returns
/// * `Ok(Box<dyn Formatter>)` - If the format string is valid
/// * `Err(FormatterError)` - If the format string is invalid
///
/// # Errors
/// Returns an error if format_str is not one of: "changes", "after", or "rfc6902"
pub fn create_formatter(
    format_str: &str,
    sort: bool,
) -> Result<Box<dyn Formatter>, FormatterError> {
    match format_str {
        "changes" => Ok(Box::new(ChangesFormatter::new(sort))),
        "after" => Ok(Box::new(AfterFormatter::new(sort))),
        "rfc6902" => Ok(Box::new(JsonPatchFormatter::new(sort))),
        _ => Err(FormatterError::UnknownFormat {
            format: format_str.to_string(),
            valid: "changes, after, rfc6902".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_formatter_valid_formats() {
        // Test all valid format strings
        let valid_formats = ["changes", "after", "rfc6902"];

        for format in valid_formats {
            let result = create_formatter(format, false);
            assert!(result.is_ok(), "Format '{}' should be valid", format);
        }
    }

    #[test]
    fn test_create_formatter_invalid_format() {
        let result = create_formatter("invalid", false);
        assert!(result.is_err());

        if let Err(FormatterError::UnknownFormat { format, valid }) = result {
            assert_eq!(format, "invalid");
            assert!(valid.contains("changes"));
            assert!(valid.contains("after"));
            assert!(valid.contains("rfc6902"));
        } else {
            panic!("Expected UnknownFormat error");
        }
    }

    #[test]
    fn test_create_formatter_empty_format() {
        let result = create_formatter("", false);
        assert!(result.is_err());

        if let Err(FormatterError::UnknownFormat { format, .. }) = result {
            assert_eq!(format, "");
        } else {
            panic!("Expected UnknownFormat error");
        }
    }

    #[test]
    fn test_create_formatter_json_format() {
        // Test with "json" which is a common mistake
        let result = create_formatter("json", false);
        assert!(result.is_err());

        match result {
            Err(err) => {
                let err_msg = format!("{}", err);
                assert!(err_msg.contains("json"));
                assert!(err_msg.contains("changes"));
            }
            Ok(_) => panic!("Expected error for invalid format 'json'"),
        }
    }
}
