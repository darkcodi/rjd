use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a change to a JSON value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Change {
    Added {
        path: String,
        value: Value,
    },
    Removed {
        path: String,
        value: Value,
    },
    Modified {
        path: String,
        old_value: Value,
        new_value: Value,
    },
}

/// Container for all changes found during diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub added: Vec<Change>,
    pub removed: Vec<Change>,
    pub modified: Vec<Change>,
}

impl Changes {
    /// Create a new empty Changes container
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
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
}

impl Default for Changes {
    fn default() -> Self {
        Self::new()
    }
}
