//! Type-safe JSON path representation and manipulation
//!
//! This module provides a structured way to work with JSON paths in dot notation,
//! with compile-time type safety and clear error messages.
//!
//! # Format
//!
//! Paths use dot notation with bracket-based array indexing:
//! - Root property: `"name"`
//! - Nested property: `"user.profile.email"`
//! - Array index: `"items[0]"`
//! - Combined: `"users[0].email"`
//!
//! # Example
//!
//! ```rust
//! use rjd::json_path::{JsonPath, PathSegment};
//! use std::str::FromStr;
//!
//! // Parse a path
//! let path = JsonPath::from_str("users[0].email").unwrap();
//! assert_eq!(path.to_string(), "users[0].email");
//!
//! // Convert to JSON Pointer (RFC 6901)
//! assert_eq!(path.to_json_pointer(), "/users/0/email");
//! ```

use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A single segment in a JSON path
///
/// Represents either an object property key or an array index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// Object property key (e.g., "user" in "user.name")
    Key(String),
    /// Array index (e.g., 0 in "items[0]")
    Index(usize),
}

impl Hash for PathSegment {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PathSegment::Key(s) => {
                state.write_u8(0);
                s.hash(state);
            }
            PathSegment::Index(i) => {
                state.write_u8(1);
                i.hash(state);
            }
        }
    }
}

/// A type-safe JSON path
///
/// Represents a path to a location in a JSON value using dot notation.
/// Paths are composed of segments that can be either object keys or array indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPath {
    /// The segments that make up this path
    segments: Vec<PathSegment>,
}

impl JsonPath {
    /// Create a new empty JsonPath
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Create a JsonPath from a vector of segments
    pub fn from_segments(segments: Vec<PathSegment>) -> Self {
        Self { segments }
    }

    /// Get the segments of this path
    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    /// Check if this path is empty
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the number of segments in this path
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Add a segment to this path
    pub fn push(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    /// Get the parent path (all segments except the last)
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            None
        } else {
            Some(Self {
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    /// Check if this path starts with the given prefix
    pub fn matches_prefix(&self, prefix: &JsonPath) -> bool {
        if prefix.segments.len() > self.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(prefix.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Get the first n segments as a new JsonPath
    pub fn prefix(&self, n: usize) -> Option<Self> {
        if n == 0 || n > self.segments.len() {
            return None;
        }
        Some(Self {
            segments: self.segments[..n].to_vec(),
        })
    }

    /// Convert this path to JSON Pointer format (RFC 6901)
    ///
    /// JSON Pointer uses a slash-separated path with special encoding:
    /// - `~0` represents `~`
    /// - `~1` represents `/`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rjd::json_path::JsonPath;
    /// use std::str::FromStr;
    ///
    /// let path = JsonPath::from_str("users[0].email").unwrap();
    /// assert_eq!(path.to_json_pointer(), "/users/0/email");
    /// ```
    pub fn to_json_pointer(&self) -> String {
        if self.segments.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for segment in &self.segments {
            result.push('/');
            match segment {
                PathSegment::Key(key) => {
                    // Encode special characters per RFC 6901
                    let encoded = key.replace('~', "~0").replace('/', "~1");
                    result.push_str(&encoded);
                }
                PathSegment::Index(i) => {
                    result.push_str(&i.to_string());
                }
            }
        }
        result
    }
}

impl Default for JsonPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Display implementation outputs dot notation
///
/// # Examples
///
/// ```rust
/// use rjd::json_path::JsonPath;
/// use std::str::FromStr;
///
/// let path = JsonPath::from_str("users[0].email").unwrap();
/// assert_eq!(path.to_string(), "users[0].email");
/// ```
impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                PathSegment::Key(key) => {
                    if i > 0 {
                        write!(f, ".")?;
                    }
                    write!(f, "{}", key)?;
                }
                PathSegment::Index(idx) => {
                    write!(f, "[{}]", idx)?;
                }
            }
        }
        Ok(())
    }
}

/// Error type for path parsing failures
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    #[error("Empty path")]
    EmptyPath,

    #[error("Invalid array index at position {position}: expected digit, found '{found}'")]
    InvalidArrayIndex { position: usize, found: char },

    #[error("Unclosed bracket at position {position}")]
    UnclosedBracket { position: usize },

    #[error("Unexpected character '{0}' at position {1}")]
    UnexpectedCharacter(char, usize),
}

/// Parse dot notation to create a JsonPath
///
/// # Examples
///
/// ```rust
/// use rjd::json_path::JsonPath;
/// use std::str::FromStr;
///
/// let path = JsonPath::from_str("users[0].email").unwrap();
/// assert_eq!(path.to_string(), "users[0].email");
/// ```
impl FromStr for JsonPath {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Ok(Self::new());
        }

        let mut segments = Vec::new();
        let mut chars = s.chars().peekable();
        let mut pos = 0;

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    // Dot separator - skip, next segment starts
                    pos += 1;
                }
                '[' => {
                    // Array index
                    pos += 1;
                    let mut index_str = String::new();

                    // Parse digits
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            index_str.push(c);
                            chars.next();
                            pos += 1;
                        } else {
                            break;
                        }
                    }

                    if index_str.is_empty() {
                        return Err(ParseError::InvalidArrayIndex {
                            position: pos,
                            found: chars.next().unwrap_or(' '),
                        });
                    }

                    // Expect closing bracket
                    match chars.next() {
                        Some(']') => {
                            pos += 1;
                        }
                        Some(c) => {
                            return Err(ParseError::UnexpectedCharacter(c, pos));
                        }
                        None => {
                            return Err(ParseError::UnclosedBracket { position: pos });
                        }
                    }

                    let index: usize =
                        index_str
                            .parse()
                            .map_err(|_| ParseError::InvalidArrayIndex {
                                position: pos - index_str.len() - 1,
                                found: index_str.chars().next().unwrap_or(' '),
                            })?;

                    segments.push(PathSegment::Index(index));
                }
                ']' => {
                    return Err(ParseError::UnexpectedCharacter(ch, pos));
                }
                _ => {
                    // Property key
                    let mut key = String::new();
                    key.push(ch);
                    pos += 1;

                    // Consume until we hit a delimiter
                    while let Some(&c) = chars.peek() {
                        if c == '.' || c == '[' {
                            break;
                        }
                        key.push(c);
                        chars.next();
                        pos += 1;
                    }

                    if !key.is_empty() {
                        segments.push(PathSegment::Key(key));
                    }
                }
            }
        }

        if segments.is_empty() {
            return Err(ParseError::EmptyPath);
        }

        Ok(Self { segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_path() {
        let path = JsonPath::new();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
        assert_eq!(path.to_string(), "");
        assert_eq!(path.to_json_pointer(), "");
    }

    #[test]
    fn test_parse_root_key() {
        let path: JsonPath = "name".parse().unwrap();
        assert_eq!(path.segments(), &[PathSegment::Key("name".to_string())]);
        assert_eq!(path.to_string(), "name");
    }

    #[test]
    fn test_parse_nested_keys() {
        let path: JsonPath = "user.profile.email".parse().unwrap();
        assert_eq!(
            path.segments(),
            &[
                PathSegment::Key("user".to_string()),
                PathSegment::Key("profile".to_string()),
                PathSegment::Key("email".to_string())
            ]
        );
        assert_eq!(path.to_string(), "user.profile.email");
    }

    #[test]
    fn test_parse_array_index() {
        let path: JsonPath = "items[0]".parse().unwrap();
        assert_eq!(
            path.segments(),
            &[PathSegment::Key("items".to_string()), PathSegment::Index(0)]
        );
        assert_eq!(path.to_string(), "items[0]");
    }

    #[test]
    fn test_parse_combined() {
        let path: JsonPath = "users[0].email".parse().unwrap();
        assert_eq!(
            path.segments(),
            &[
                PathSegment::Key("users".to_string()),
                PathSegment::Index(0),
                PathSegment::Key("email".to_string())
            ]
        );
        assert_eq!(path.to_string(), "users[0].email");
    }

    #[test]
    fn test_parse_deep_nesting() {
        let path: JsonPath = "a.b.c.d.e".parse().unwrap();
        assert_eq!(path.len(), 5);
        assert_eq!(path.to_string(), "a.b.c.d.e");
    }

    #[test]
    fn test_to_json_pointer_simple() {
        let path: JsonPath = "name".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/name");
    }

    #[test]
    fn test_to_json_pointer_nested() {
        let path: JsonPath = "user.name".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/user/name");
    }

    #[test]
    fn test_to_json_pointer_array() {
        let path: JsonPath = "users[0]".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/users/0");
    }

    #[test]
    fn test_to_json_pointer_combined() {
        let path: JsonPath = "users[0].email".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/users/0/email");
    }

    #[test]
    fn test_to_json_pointer_special_chars() {
        let path: JsonPath = "user/name".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/user~1name");

        let path: JsonPath = "user~name".parse().unwrap();
        assert_eq!(path.to_json_pointer(), "/user~0name");
    }

    #[test]
    fn test_parent() {
        let path: JsonPath = "user.profile.email".parse().unwrap();
        let parent = path.parent().unwrap();
        assert_eq!(parent.to_string(), "user.profile");

        let root: JsonPath = "name".parse().unwrap();
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_matches_prefix() {
        let path: JsonPath = "user.profile.email".parse().unwrap();
        let prefix: JsonPath = "user.profile".parse().unwrap();

        assert!(path.matches_prefix(&prefix));
        assert!(!prefix.matches_prefix(&path));
    }

    #[test]
    fn test_push_segment() {
        let mut path = JsonPath::new();
        path.push(PathSegment::Key("users".to_string()));
        path.push(PathSegment::Index(0));
        path.push(PathSegment::Key("email".to_string()));

        assert_eq!(path.to_string(), "users[0].email");
    }

    #[test]
    fn test_round_trip() {
        let original = "users[0].profile.email";
        let path: JsonPath = original.parse().unwrap();
        assert_eq!(path.to_string(), original);
    }

    #[test]
    fn test_parse_empty_string() {
        let path: Result<JsonPath, _> = "".parse();
        assert!(path.is_ok());
        assert!(path.unwrap().is_empty());
    }

    #[test]
    fn test_parse_invalid_array_no_digits() {
        let path: Result<JsonPath, _> = "items[]".parse();
        assert!(path.is_err());
    }

    #[test]
    fn test_parse_unclosed_bracket() {
        let path: Result<JsonPath, _> = "items[0".parse();
        assert!(path.is_err());
    }

    #[test]
    fn test_equality() {
        let path1: JsonPath = "users[0].email".parse().unwrap();
        let path2: JsonPath = "users[0].email".parse().unwrap();
        assert_eq!(path1, path2);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let path1: JsonPath = "users[0].email".parse().unwrap();
        let path2: JsonPath = "users[0].email".parse().unwrap();
        let path3: JsonPath = "users[1].email".parse().unwrap();

        let mut set = HashSet::new();
        set.insert(path1.clone());
        set.insert(path2);
        set.insert(path3.clone());

        assert_eq!(set.len(), 2); // path1 and path2 are the same
        assert!(set.contains(&path1));
        assert!(set.contains(&path3));
    }
}
