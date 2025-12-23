//! Formatter module for outputting diff results
//!
//! This module provides different output formatters for diff results.
//! The default is the "changes" format which outputs a structured JSON
//! object with added, removed, and modified arrays.

mod changes;
mod after;

pub use changes::ChangesFormatter;
pub use after::AfterFormatter;

/// Trait for formatting diff results
pub trait Formatter {
    /// Format the changes and return a string representation
    fn format(&self, changes: &crate::types::Changes)
        -> Result<String, Box<dyn std::error::Error>>;
}

/// Factory function to create a formatter based on output format
pub fn create_formatter(format: crate::cli::OutputFormat) -> Box<dyn Formatter> {
    match format {
        crate::cli::OutputFormat::Changes => Box::new(ChangesFormatter::new()),
        crate::cli::OutputFormat::After => Box::new(AfterFormatter::new()),
    }
}
