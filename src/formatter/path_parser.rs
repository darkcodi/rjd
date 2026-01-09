//! Path parser for JSON path expressions
//!
//! This module provides a structured parser for JSON path expressions,
//! converting strings like "user.items[0].name" into a sequence of
//! path segments with proper error handling.

#![allow(dead_code)]

use crate::json_path::PathSegment;
use std::fmt;

/// Errors that can occur during path parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Invalid array index (non-numeric characters)
    InvalidArrayIndex { position: usize, found: String },

    /// Unclosed bracket in array notation
    UnclosedBracket { position: usize },

    /// Empty path segment
    EmptySegment { position: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidArrayIndex { position, found } => write!(
                f,
                "Invalid array index at position {}: '{}', expected numeric value",
                position, found
            ),
            ParseError::UnclosedBracket { position } => {
                write!(f, "Unclosed bracket at position {}, expected ']'", position)
            }
            ParseError::EmptySegment { position } => {
                write!(f, "Empty path segment at position {}", position)
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parser for JSON path expressions
///
/// This struct provides a clean, state-machine-based parser for JSON paths,
/// avoiding the complex string manipulation of the original implementation.
pub struct PathParser {
    segments: Vec<PathSegment>,
    current: String,
    pos: usize,
}

impl PathParser {
    /// Create a new parser
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            current: String::new(),
            pos: 0,
        }
    }

    /// Parse a path string into a vector of segments
    ///
    /// # Examples
    ///
    /// ```
    /// use rjd::formatter::path_parser::{PathParser, ParseError};
    ///
    /// let parser = PathParser::parse("user.name").unwrap();
    /// let segments = parser.into_segments();
    /// assert_eq!(segments.len(), 2);
    ///
    /// let parser = PathParser::parse("items[0]").unwrap();
    /// let segments = parser.into_segments();
    /// assert_eq!(segments.len(), 2);
    /// ```
    pub fn parse(path: &str) -> Result<Self, ParseError> {
        let mut parser = Self::new();
        parser.parse_path(path)?;
        Ok(parser)
    }

    /// Get the parsed segments
    pub fn into_segments(self) -> Vec<PathSegment> {
        self.segments
    }

    /// Main parsing logic with clear state machine
    fn parse_path(&mut self, path: &str) -> Result<(), ParseError> {
        let chars: Vec<char> = path.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            match chars[i] {
                '.' => {
                    self.flush_key()?;
                    i += 1;
                }
                '[' => {
                    self.flush_key()?;
                    i = self.parse_array_index(&chars, i)?;
                }
                ']' => {
                    return Err(ParseError::UnclosedBracket { position: i });
                }
                c => {
                    self.current.push(c);
                    i += 1;
                }
            }
        }

        // Flush any remaining key
        self.flush_key()?;
        Ok(())
    }

    /// Parse an array index like [0] or [123]
    fn parse_array_index(&mut self, chars: &[char], start: usize) -> Result<usize, ParseError> {
        let start_pos = start;
        let mut i = start + 1; // Skip '['
        let mut index_str = String::new();

        // Extract index between brackets
        while i < chars.len() && chars[i] != ']' {
            index_str.push(chars[i]);
            i += 1;
        }

        if i >= chars.len() {
            return Err(ParseError::UnclosedBracket {
                position: start_pos,
            });
        }

        // Validate and parse index
        let index: usize = index_str
            .parse()
            .map_err(|_| ParseError::InvalidArrayIndex {
                position: start + 1,
                found: index_str,
            })?;

        self.segments.push(PathSegment::Index(index));
        Ok(i + 1) // Skip ']'
    }

    /// Flush the current key as a Key segment
    fn flush_key(&mut self) -> Result<(), ParseError> {
        if !self.current.is_empty() {
            if self.current.contains('.') || self.current.contains('[') {
                // This shouldn't happen with proper parsing, but handle it
                return Err(ParseError::InvalidArrayIndex {
                    position: self.pos,
                    found: self.current.clone(),
                });
            }
            self.segments.push(PathSegment::Key(self.current.clone()));
            self.current.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_property() {
        let parser = PathParser::parse("user.name").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 2);
        assert!(matches!(segments[0], PathSegment::Key(ref k) if k == "user"));
        assert!(matches!(segments[1], PathSegment::Key(ref k) if k == "name"));
    }

    #[test]
    fn test_parse_array_index() {
        let parser = PathParser::parse("items[0]").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 2);
        assert!(matches!(segments[0], PathSegment::Key(ref k) if k == "items"));
        assert!(matches!(segments[1], PathSegment::Index(0)));
    }

    #[test]
    fn test_parse_nested_with_array() {
        let parser = PathParser::parse("items[0].name").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], PathSegment::Key(ref k) if k == "items"));
        assert!(matches!(segments[1], PathSegment::Index(0)));
        assert!(matches!(segments[2], PathSegment::Key(ref k) if k == "name"));
    }

    #[test]
    fn test_parse_invalid_array_index() {
        let result = PathParser::parse("items[abc]");
        assert!(result.is_err());
        match result {
            Err(ParseError::InvalidArrayIndex { found, .. }) => {
                assert_eq!(found, "abc");
            }
            _ => panic!("Expected InvalidArrayIndex error"),
        }
    }

    #[test]
    fn test_parse_unclosed_bracket() {
        let result = PathParser::parse("items[0");
        assert!(result.is_err());
        match result {
            Err(ParseError::UnclosedBracket { .. }) => {}
            _ => panic!("Expected UnclosedBracket error"),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let parser = PathParser::parse("").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 0);
    }

    #[test]
    fn test_parse_unicode_keys() {
        let parser = PathParser::parse("user.名前.email").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[1], PathSegment::Key(ref k) if k == "名前"));
    }

    #[test]
    fn test_parse_special_chars() {
        let parser = PathParser::parse("user-info.field_name").unwrap();
        let segments = parser.into_segments();
        assert_eq!(segments.len(), 2);
        assert!(matches!(segments[0], PathSegment::Key(ref k) if k == "user-info"));
        assert!(matches!(segments[1], PathSegment::Key(ref k) if k == "field_name"));
    }
}
