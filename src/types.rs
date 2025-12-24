use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a change to a JSON value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Change {
    Added {
        #[serde(rename = "path")]
        path: String,
        #[serde(rename = "value")]
        value: Value,
    },
    Removed {
        #[serde(rename = "path")]
        path: String,
        #[serde(rename = "value")]
        value: Value,
    },
    Modified {
        #[serde(rename = "path")]
        path: String,
        #[serde(rename = "oldValue")]
        old_value: Value,
        #[serde(rename = "newValue")]
        new_value: Value,
    },
}

/// Container for all changes found during diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub added: Vec<Change>,
    pub removed: Vec<Change>,
    pub modified: Vec<Change>,
    #[serde(skip)]
    pub after: Option<Value>,
}

impl Changes {
    /// Create a new empty Changes container
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
            after: None,
        }
    }

    /// Add a change to the appropriate category
    pub fn push(&mut self, change: Change) {
        match change {
            Change::Added { .. } => self.added.push(change),
            Change::Removed { .. } => self.removed.push(change),
            Change::Modified { .. } => self.modified.push(change),
        }
    }

    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Filter out changes that match any of the ignore patterns
    pub fn filter_ignore_patterns(&self, patterns: &[String]) -> Self {
        Self {
            added: self
                .added
                .iter()
                .filter(|c| !matches_pattern(c, patterns))
                .cloned()
                .collect(),
            removed: self
                .removed
                .iter()
                .filter(|c| !matches_pattern(c, patterns))
                .cloned()
                .collect(),
            modified: self
                .modified
                .iter()
                .filter(|c| !matches_pattern(c, patterns))
                .cloned()
                .collect(),
            after: self.after.clone(),
        }
    }
}

/// Check if a change matches any of the ignore patterns
fn matches_pattern(change: &Change, patterns: &[String]) -> bool {
    let path = match change {
        Change::Added { path, .. } => path,
        Change::Removed { path, .. } => path,
        Change::Modified { path, .. } => path,
    };

    patterns.iter().any(|pattern| {
        let dot_notation = json_pointer_to_dot_notation(pattern);
        path.starts_with(&dot_notation)
    })
}

/// Convert a JSON Pointer path to dot notation
/// Example: "/user/id/0/name" -> "user.id[0].name"
fn json_pointer_to_dot_notation(ptr: &str) -> String {
    let mut result = String::new();
    let parts: Vec<&str> = ptr.split('/').filter(|s| !s.is_empty()).collect();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            result.push('.');
        }
        // Check if part is a numeric array index
        if part.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            result.push('[');
            result.push_str(part);
            result.push(']');
        } else {
            result.push_str(part);
        }
    }

    result
}

impl Default for Changes {
    fn default() -> Self {
        Self::new()
    }
}
