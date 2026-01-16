//! Hierarchical ID system for anchors and tasks
//!
//! ID Format:
//! - Anchor IDs: `a-{7-char-hash}` (e.g., `a-7f2b4c1`)
//! - Task IDs: `{anchor-id}.{sequence}` (e.g., `a-7f2b4c1.1`)
//! - Subtask IDs: `{task-id}.{sequence}` (e.g., `a-7f2b4c1.1.1`)
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

    #[error("Invalid task ID format: expected '{{anchor-id}}.{{sequence}}', got '{0}'")]
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
            anchor_hash: self.hash.clone(),
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

/// Task ID in the format `{anchor-id}.{sequence}` or `{anchor-id}.{seq}.{seq}...`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TaskId {
    anchor_hash: String,
    segments: Vec<u32>,
}

impl TaskId {
    /// Creates a new task ID for a given anchor with a sequence number
    pub fn new(anchor_id: &AnchorId, sequence: u32) -> Self {
        Self {
            anchor_hash: anchor_id.hash().to_string(),
            segments: vec![sequence],
        }
    }

    /// Returns the anchor ID this task belongs to
    pub fn anchor_id(&self) -> AnchorId {
        AnchorId {
            hash: self.anchor_hash.clone(),
        }
    }

    /// Returns the sequence segments (e.g., [1] for task, [1, 2] for subtask)
    pub fn segments(&self) -> &[u32] {
        &self.segments
    }

    /// Returns the depth of this task (1 for top-level, 2 for subtask, etc.)
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Returns true if this is a subtask (depth > 1)
    pub fn is_subtask(&self) -> bool {
        self.segments.len() > 1
    }

    /// Returns the parent task ID, or None if this is a top-level task
    pub fn parent(&self) -> Option<TaskId> {
        if self.segments.len() <= 1 {
            return None;
        }

        Some(TaskId {
            anchor_hash: self.anchor_hash.clone(),
            segments: self.segments[..self.segments.len() - 1].to_vec(),
        })
    }

    /// Creates a subtask ID under this task
    pub fn subtask(&self, sequence: u32) -> TaskId {
        let mut segments = self.segments.clone();
        segments.push(sequence);
        TaskId {
            anchor_hash: self.anchor_hash.clone(),
            segments,
        }
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a-{}", self.anchor_hash)?;
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
            anchor_hash: hash.to_string(),
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

        assert_eq!(task.anchor_id(), anchor);
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
}
