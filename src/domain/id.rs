//! Hierarchical ID system for anchors and tasks
//!
//! ID Format:
//! - Anchor IDs: `a-{7-char-hash}` (e.g., `a-7f2b4c1`)
//! - Task IDs (anchored): `{anchor-id}.{sequence}` (e.g., `a-7f2b4c1.1`)
//! - Task IDs (standalone): `t-{7-char-hash}` (e.g., `t-9d3e5f2`)
//! - Subtask IDs: `{task-id}.{sequence}` (e.g., `a-7f2b4c1.1.1` or `t-9d3e5f2.1`)
//!
//! Hash is derived from title + creation timestamp, ensuring uniqueness.
//! Same title at different times produces different IDs (by design).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum IdError {
    #[error("Invalid anchor ID format: expected 'a-{{7-char-hash}}', got '{0}'")]
    InvalidAnchorId(String),

    #[error("Invalid task ID format: expected '{{anchor-id}}.{{sequence}}' or 't-{{7-char-hash}}', got '{0}'")]
    InvalidTaskId(String),

    #[error("Invalid sequence number: {0}")]
    InvalidSequence(String),
}

/// Generates a 7-character hash from title and timestamp
fn generate_hash(title: &str, timestamp: DateTime<Utc>) -> String {
    let input = format!("{}{}", title, timestamp.timestamp_nanos_opt().unwrap_or(0));
    let hash = blake3::hash(input.as_bytes());
    let hex = hash.to_hex();
    hex[..7].to_string()
}

/// Anchor ID in the format `a-{7-char-hash}`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AnchorId {
    hash: String,
}

impl AnchorId {
    /// Creates a new anchor ID from title and timestamp
    pub fn new(title: &str, timestamp: DateTime<Utc>) -> Self {
        Self {
            hash: generate_hash(title, timestamp),
        }
    }

    /// Returns the hash portion of the ID
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Creates a task ID for this anchor with the given sequence number
    pub fn task_id(&self, sequence: u32) -> TaskId {
        TaskId {
            hash: self.hash.clone(),
            standalone: false,
            segments: vec![sequence],
        }
    }
}

impl fmt::Display for AnchorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a-{}", self.hash)
    }
}

impl FromStr for AnchorId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if !s.starts_with("a-") {
            return Err(IdError::InvalidAnchorId(s.to_string()));
        }

        let hash = &s[2..];
        if hash.len() != 7 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(IdError::InvalidAnchorId(s.to_string()));
        }

        Ok(Self {
            hash: hash.to_string(),
        })
    }
}

impl TryFrom<String> for AnchorId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AnchorId> for String {
    fn from(id: AnchorId) -> Self {
        id.to_string()
    }
}

/// Task ID - can be anchored (`a-{hash}.{seq}`) or standalone (`t-{hash}`)
///
/// Anchored tasks belong to an anchor: `a-7f2b4c1.1`
/// Standalone tasks exist independently: `t-9d3e5f2`
/// Both support subtasks: `a-7f2b4c1.1.1` or `t-9d3e5f2.1`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TaskId {
    /// The hash portion of the ID (from anchor or standalone)
    hash: String,
    /// Whether this is a standalone task (t-) or anchored (a-)
    standalone: bool,
    /// Sequence segments (empty for top-level standalone, non-empty for anchored or subtasks)
    segments: Vec<u32>,
}

impl TaskId {
    /// Creates a new anchored task ID for a given anchor with a sequence number
    pub fn new(anchor_id: &AnchorId, sequence: u32) -> Self {
        Self {
            hash: anchor_id.hash().to_string(),
            standalone: false,
            segments: vec![sequence],
        }
    }

    /// Creates a new standalone task ID from title and timestamp
    pub fn new_standalone(title: &str, timestamp: DateTime<Utc>) -> Self {
        Self {
            hash: generate_hash(title, timestamp),
            standalone: true,
            segments: vec![],
        }
    }

    /// Returns true if this is a standalone task (t- prefix)
    pub fn is_standalone(&self) -> bool {
        self.standalone
    }

    /// Returns the anchor ID this task belongs to, or None if standalone
    pub fn anchor_id(&self) -> Option<AnchorId> {
        if self.standalone {
            None
        } else {
            Some(AnchorId {
                hash: self.hash.clone(),
            })
        }
    }

    /// Returns the hash portion of the ID
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Returns the sequence segments (e.g., `[1]` for anchored task, `[1, 2]` for subtask)
    /// Empty for top-level standalone tasks
    pub fn segments(&self) -> &[u32] {
        &self.segments
    }

    /// Returns the depth of this task
    /// - Anchored top-level: 1
    /// - Standalone top-level: 0
    /// - Subtasks: segments.len()
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Returns true if this is a subtask (has parent task)
    /// For anchored tasks: depth > 1
    /// For standalone tasks: depth > 0 (has any segments)
    pub fn is_subtask(&self) -> bool {
        if self.standalone {
            !self.segments.is_empty()
        } else {
            self.segments.len() > 1
        }
    }

    /// Returns the parent task ID, or None if this is a top-level task
    pub fn parent(&self) -> Option<TaskId> {
        if self.standalone {
            // For standalone tasks, parent exists if there are segments
            if self.segments.is_empty() {
                return None;
            }
            if self.segments.len() == 1 {
                // Parent is the top-level standalone task
                return Some(TaskId {
                    hash: self.hash.clone(),
                    standalone: true,
                    segments: vec![],
                });
            }
            // Parent has one fewer segment
            Some(TaskId {
                hash: self.hash.clone(),
                standalone: true,
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        } else {
            // For anchored tasks, parent exists if depth > 1
            if self.segments.len() <= 1 {
                return None;
            }
            Some(TaskId {
                hash: self.hash.clone(),
                standalone: false,
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    /// Creates a subtask ID under this task
    pub fn subtask(&self, sequence: u32) -> TaskId {
        let mut segments = self.segments.clone();
        segments.push(sequence);
        TaskId {
            hash: self.hash.clone(),
            standalone: self.standalone,
            segments,
        }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.standalone { "t" } else { "a" };
        write!(f, "{}-{}", prefix, self.hash)?;
        for seg in &self.segments {
            write!(f, ".{}", seg)?;
        }
        Ok(())
    }
}

impl FromStr for TaskId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Check for standalone task ID (t-{hash} or t-{hash}.{seq}...)
        if let Some(rest) = s.strip_prefix("t-") {
            let parts: Vec<&str> = rest.split('.').collect();

            if parts.is_empty() {
                return Err(IdError::InvalidTaskId(s.to_string()));
            }

            let hash = parts[0];
            if hash.len() != 7 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(IdError::InvalidTaskId(s.to_string()));
            }

            // Standalone tasks may have no segments (top-level) or segments (subtasks)
            let segments: Result<Vec<u32>, _> = parts[1..]
                .iter()
                .map(|p| {
                    p.parse::<u32>()
                        .map_err(|_| IdError::InvalidSequence((*p).to_string()))
                })
                .collect();

            return Ok(Self {
                hash: hash.to_string(),
                standalone: true,
                segments: segments?,
            });
        }

        // Check for anchored task ID (a-{hash}.{seq}...)
        if !s.starts_with("a-") {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        let rest = &s[2..];
        let parts: Vec<&str> = rest.split('.').collect();

        if parts.is_empty() {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        let hash = parts[0];
        if hash.len() != 7 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        // Anchored tasks must have at least one segment
        if parts.len() < 2 {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        let segments: Result<Vec<u32>, _> = parts[1..]
            .iter()
            .map(|p| {
                p.parse::<u32>()
                    .map_err(|_| IdError::InvalidSequence((*p).to_string()))
            })
            .collect();

        let segments = segments?;
        if segments.is_empty() {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        Ok(Self {
            hash: hash.to_string(),
            standalone: false,
            segments,
        })
    }
}

impl TryFrom<String> for TaskId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<TaskId> for String {
    fn from(id: TaskId) -> Self {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_id_generation_is_unique_for_different_timestamps() {
        let title = "Same Title";
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::nanoseconds(1);

        let id1 = AnchorId::new(title, ts1);
        let id2 = AnchorId::new(title, ts2);

        assert_ne!(id1, id2);
    }

    #[test]
    fn anchor_id_format_is_correct() {
        let id = AnchorId::new("Test", Utc::now());
        let s = id.to_string();

        assert!(s.starts_with("a-"));
        assert_eq!(s.len(), 9); // "a-" + 7 chars
    }

    #[test]
    fn anchor_id_parses_correctly() {
        let original = AnchorId::new("Test", Utc::now());
        let s = original.to_string();
        let parsed: AnchorId = s.parse().unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn anchor_id_rejects_invalid_format() {
        assert!("invalid".parse::<AnchorId>().is_err());
        assert!("a-short".parse::<AnchorId>().is_err());
        assert!("a-toolonggg".parse::<AnchorId>().is_err());
        assert!("a-gggggg1".parse::<AnchorId>().is_err()); // 'g' is not hex
    }

    #[test]
    fn task_id_format_is_correct() {
        let anchor = AnchorId::new("Test", Utc::now());
        let task = TaskId::new(&anchor, 1);
        let s = task.to_string();

        assert!(s.starts_with("a-"));
        assert!(s.ends_with(".1"));
    }

    #[test]
    fn task_id_parses_correctly() {
        let anchor = AnchorId::new("Test", Utc::now());
        let original = TaskId::new(&anchor, 42);
        let s = original.to_string();
        let parsed: TaskId = s.parse().unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn subtask_id_works() {
        let anchor = AnchorId::new("Test", Utc::now());
        let task = TaskId::new(&anchor, 1);
        let subtask = task.subtask(2);

        assert_eq!(subtask.depth(), 2);
        assert!(subtask.is_subtask());
        assert_eq!(subtask.parent(), Some(task.clone()));
        assert!(subtask.to_string().ends_with(".1.2"));
    }

    #[test]
    fn task_id_parses_subtasks() {
        let s = "a-1234567.1.2.3";
        let task: TaskId = s.parse().unwrap();

        assert_eq!(task.segments(), &[1, 2, 3]);
        assert_eq!(task.depth(), 3);
    }

    #[test]
    fn task_id_rejects_invalid_format() {
        assert!("invalid".parse::<TaskId>().is_err());
        assert!("a-1234567".parse::<TaskId>().is_err()); // no sequence
        assert!("a-123456.1".parse::<TaskId>().is_err()); // hash too short
        assert!("a-1234567.abc".parse::<TaskId>().is_err()); // non-numeric sequence
    }

    #[test]
    fn anchor_id_creates_task_id() {
        let anchor = AnchorId::new("Test", Utc::now());
        let task = anchor.task_id(5);

        assert_eq!(task.anchor_id(), Some(anchor));
        assert_eq!(task.segments(), &[5]);
    }

    #[test]
    fn serde_roundtrip_anchor_id() {
        let original = AnchorId::new("Test", Utc::now());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: AnchorId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn serde_roundtrip_task_id() {
        let anchor = AnchorId::new("Test", Utc::now());
        let original = TaskId::new(&anchor, 1).subtask(2);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TaskId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, parsed);
    }

    // =========================================================================
    // Standalone Task ID Tests
    // =========================================================================

    #[test]
    fn standalone_task_id_generation() {
        let ts = Utc::now();
        let id = TaskId::new_standalone("Fix typo", ts);

        assert!(id.is_standalone());
        assert!(id.anchor_id().is_none());
        assert!(id.segments().is_empty());
        assert_eq!(id.depth(), 0);
        assert!(!id.is_subtask());
    }

    #[test]
    fn standalone_task_id_format() {
        let ts = Utc::now();
        let id = TaskId::new_standalone("Fix typo", ts);
        let s = id.to_string();

        assert!(s.starts_with("t-"));
        assert_eq!(s.len(), 9); // "t-" + 7 chars
    }

    #[test]
    fn standalone_task_id_parses_correctly() {
        let s = "t-1234567";
        let task: TaskId = s.parse().unwrap();

        assert!(task.is_standalone());
        assert!(task.anchor_id().is_none());
        assert!(task.segments().is_empty());
    }

    #[test]
    fn standalone_task_id_roundtrip() {
        let ts = Utc::now();
        let original = TaskId::new_standalone("Fix bug", ts);
        let s = original.to_string();
        let parsed: TaskId = s.parse().unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn standalone_task_subtask() {
        let ts = Utc::now();
        let task = TaskId::new_standalone("Main task", ts);
        let subtask = task.subtask(1);

        assert!(subtask.is_standalone());
        assert!(subtask.is_subtask());
        assert_eq!(subtask.depth(), 1);
        assert_eq!(subtask.parent(), Some(task.clone()));

        let s = subtask.to_string();
        assert!(s.starts_with("t-"));
        assert!(s.ends_with(".1"));
    }

    #[test]
    fn standalone_subtask_parses() {
        let s = "t-1234567.1.2";
        let task: TaskId = s.parse().unwrap();

        assert!(task.is_standalone());
        assert!(task.is_subtask());
        assert_eq!(task.segments(), &[1, 2]);
        assert_eq!(task.depth(), 2);
    }

    #[test]
    fn standalone_subtask_parent_chain() {
        let s = "t-1234567.1.2";
        let subtask: TaskId = s.parse().unwrap();

        let parent = subtask.parent().unwrap();
        assert_eq!(parent.to_string(), "t-1234567.1");
        assert!(parent.is_subtask());

        let grandparent = parent.parent().unwrap();
        assert_eq!(grandparent.to_string(), "t-1234567");
        assert!(!grandparent.is_subtask());

        assert!(grandparent.parent().is_none());
    }

    #[test]
    fn serde_roundtrip_standalone_task_id() {
        let ts = Utc::now();
        let original = TaskId::new_standalone("Test", ts);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TaskId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, parsed);
        assert!(parsed.is_standalone());
    }

    #[test]
    fn serde_roundtrip_standalone_subtask_id() {
        let ts = Utc::now();
        let original = TaskId::new_standalone("Test", ts).subtask(1).subtask(2);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TaskId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn standalone_id_unique_per_timestamp() {
        let title = "Same Title";
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::nanoseconds(1);

        let id1 = TaskId::new_standalone(title, ts1);
        let id2 = TaskId::new_standalone(title, ts2);

        assert_ne!(id1, id2);
    }

    #[test]
    fn anchored_vs_standalone_are_different() {
        let anchor = AnchorId::new("Test", Utc::now());
        let anchored = TaskId::new(&anchor, 1);

        let ts = Utc::now();
        let standalone = TaskId::new_standalone("Test", ts);

        assert!(!anchored.is_standalone());
        assert!(standalone.is_standalone());
        assert!(anchored.anchor_id().is_some());
        assert!(standalone.anchor_id().is_none());
    }
}
