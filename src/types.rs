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
}

impl Default for Changes {
    fn default() -> Self {
        Self::new()
    }
}
