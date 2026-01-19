//! Hierarchical ID system for briefs and tasks
//!
//! ID Format:
//! - Brief IDs: `b-{7-char-hash}` (e.g., `b-7f2b4c1`)
//! - Task IDs (under brief): `{brief-id}.{sequence}` (e.g., `b-7f2b4c1.1`)
//! - Task IDs (standalone): `t-{7-char-hash}` (e.g., `t-9d3e5f2`)
//! - Subtask IDs: `{task-id}.{sequence}` (e.g., `b-7f2b4c1.1.1` or `t-9d3e5f2.1`)
//!
//! Hash is derived from title + creation timestamp, ensuring uniqueness.
//! Same title at different times produces different IDs (by design).
//!
//! Note: Old `a-` prefixed IDs are still accepted for backward compatibility
//! and are automatically treated as brief IDs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum IdError {
    #[error("Invalid brief ID format: expected 'b-{{7-char-hash}}', got '{0}'")]
    InvalidBriefId(String),

    #[error("Invalid task ID format: expected '{{brief-id}}.{{sequence}}' or 't-{{7-char-hash}}', got '{0}'")]
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

/// Brief ID in the format `b-{7-char-hash}`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BriefId {
    hash: String,
}

impl BriefId {
    /// Creates a new brief ID from title and timestamp
    pub fn new(title: &str, timestamp: DateTime<Utc>) -> Self {
        Self {
            hash: generate_hash(title, timestamp),
        }
    }

    /// Returns the hash portion of the ID
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Creates a task ID for this brief with the given sequence number
    pub fn task_id(&self, sequence: u32) -> TaskId {
        TaskId {
            hash: self.hash.clone(),
            standalone: false,
            segments: vec![sequence],
        }
    }
}

impl fmt::Display for BriefId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b-{}", self.hash)
    }
}

impl FromStr for BriefId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Accept both b- (new) and a- (legacy) prefixes
        let hash = if let Some(rest) = s.strip_prefix("b-") {
            rest
        } else if let Some(rest) = s.strip_prefix("a-") {
            // Legacy format - still accepted
            rest
        } else {
            return Err(IdError::InvalidBriefId(s.to_string()));
        };

        if hash.len() != 7 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(IdError::InvalidBriefId(s.to_string()));
        }

        Ok(Self {
            hash: hash.to_string(),
        })
    }
}

impl TryFrom<String> for BriefId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<BriefId> for String {
    fn from(id: BriefId) -> Self {
        id.to_string()
    }
}

/// Task ID - can be under a brief (`b-{hash}.{seq}`) or standalone (`t-{hash}`)
///
/// Tasks under briefs: `b-7f2b4c1.1`
/// Standalone tasks exist independently: `t-9d3e5f2`
/// Both support subtasks: `b-7f2b4c1.1.1` or `t-9d3e5f2.1`
///
/// Note: Legacy `a-` prefixed IDs are still accepted for backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TaskId {
    /// The hash portion of the ID (from brief or standalone)
    hash: String,
    /// Whether this is a standalone task (t-) or under a brief (b-/a-)
    standalone: bool,
    /// Sequence segments (empty for top-level standalone, non-empty for brief tasks or subtasks)
    segments: Vec<u32>,
}

impl TaskId {
    /// Creates a new task ID for a given brief with a sequence number
    pub fn new(brief_id: &BriefId, sequence: u32) -> Self {
        Self {
            hash: brief_id.hash().to_string(),
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

    /// Returns the brief ID this task belongs to, or None if standalone
    pub fn brief_id(&self) -> Option<BriefId> {
        if self.standalone {
            None
        } else {
            Some(BriefId {
                hash: self.hash.clone(),
            })
        }
    }

    /// Returns the hash portion of the ID
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Returns the sequence segments (e.g., `[1]` for brief task, `[1, 2]` for subtask)
    /// Empty for top-level standalone tasks
    pub fn segments(&self) -> &[u32] {
        &self.segments
    }

    /// Returns the depth of this task
    /// - Brief top-level: 1
    /// - Standalone top-level: 0
    /// - Subtasks: segments.len()
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Returns true if this is a subtask (has parent task)
    /// For brief tasks: depth > 1
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
            // For brief tasks, parent exists if depth > 1
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
        let prefix = if self.standalone { "t" } else { "b" };
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

        // Check for brief task ID (b-{hash}.{seq}... or legacy a-{hash}.{seq}...)
        let rest = if let Some(r) = s.strip_prefix("b-") {
            r
        } else if let Some(r) = s.strip_prefix("a-") {
            // Legacy format - still accepted
            r
        } else {
            return Err(IdError::InvalidTaskId(s.to_string()));
        };

        let parts: Vec<&str> = rest.split('.').collect();

        if parts.is_empty() {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        let hash = parts[0];
        if hash.len() != 7 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(IdError::InvalidTaskId(s.to_string()));
        }

        // Brief tasks must have at least one segment
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
    fn brief_id_generation_is_unique_for_different_timestamps() {
        let title = "Same Title";
        let ts1 = Utc::now();
        let ts2 = ts1 + chrono::Duration::nanoseconds(1);

        let id1 = BriefId::new(title, ts1);
        let id2 = BriefId::new(title, ts2);

        assert_ne!(id1, id2);
    }

    #[test]
    fn brief_id_format_is_correct() {
        let id = BriefId::new("Test", Utc::now());
        let s = id.to_string();

        assert!(s.starts_with("b-"));
        assert_eq!(s.len(), 9); // "b-" + 7 chars
    }

    #[test]
    fn brief_id_parses_correctly() {
        let original = BriefId::new("Test", Utc::now());
        let s = original.to_string();
        let parsed: BriefId = s.parse().unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn brief_id_parses_legacy_format() {
        // Legacy a- prefix should still work
        let parsed: BriefId = "a-1234567".parse().unwrap();
        assert_eq!(parsed.hash(), "1234567");
        // But when displayed, uses new b- format
        assert!(parsed.to_string().starts_with("b-"));
    }

    #[test]
    fn brief_id_rejects_invalid_format() {
        assert!("invalid".parse::<BriefId>().is_err());
        assert!("b-short".parse::<BriefId>().is_err());
        assert!("b-toolonggg".parse::<BriefId>().is_err());
        assert!("b-gggggg1".parse::<BriefId>().is_err()); // 'g' is not hex
    }

    #[test]
    fn task_id_format_is_correct() {
        let brief = BriefId::new("Test", Utc::now());
        let task = TaskId::new(&brief, 1);
        let s = task.to_string();

        assert!(s.starts_with("b-"));
        assert!(s.ends_with(".1"));
    }

    #[test]
    fn task_id_parses_correctly() {
        let brief = BriefId::new("Test", Utc::now());
        let original = TaskId::new(&brief, 42);
        let s = original.to_string();
        let parsed: TaskId = s.parse().unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn task_id_parses_legacy_format() {
        // Legacy a- prefix should still work
        let parsed: TaskId = "a-1234567.1".parse().unwrap();
        assert_eq!(parsed.hash(), "1234567");
        assert!(!parsed.is_standalone());
        // But when displayed, uses new b- format
        assert!(parsed.to_string().starts_with("b-"));
    }

    #[test]
    fn subtask_id_works() {
        let brief = BriefId::new("Test", Utc::now());
        let task = TaskId::new(&brief, 1);
        let subtask = task.subtask(2);

        assert_eq!(subtask.depth(), 2);
        assert!(subtask.is_subtask());
        assert_eq!(subtask.parent(), Some(task.clone()));
        assert!(subtask.to_string().ends_with(".1.2"));
    }

    #[test]
    fn task_id_parses_subtasks() {
        let s = "b-1234567.1.2.3";
        let task: TaskId = s.parse().unwrap();

        assert_eq!(task.segments(), &[1, 2, 3]);
        assert_eq!(task.depth(), 3);
    }

    #[test]
    fn task_id_rejects_invalid_format() {
        assert!("invalid".parse::<TaskId>().is_err());
        assert!("b-1234567".parse::<TaskId>().is_err()); // no sequence
        assert!("b-123456.1".parse::<TaskId>().is_err()); // hash too short
        assert!("b-1234567.abc".parse::<TaskId>().is_err()); // non-numeric sequence
    }

    #[test]
    fn brief_id_creates_task_id() {
        let brief = BriefId::new("Test", Utc::now());
        let task = brief.task_id(5);

        assert_eq!(task.brief_id(), Some(brief));
        assert_eq!(task.segments(), &[5]);
    }

    #[test]
    fn serde_roundtrip_brief_id() {
        let original = BriefId::new("Test", Utc::now());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: BriefId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn serde_roundtrip_task_id() {
        let brief = BriefId::new("Test", Utc::now());
        let original = TaskId::new(&brief, 1).subtask(2);
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
        assert!(id.brief_id().is_none());
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
        assert!(task.brief_id().is_none());
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
    fn brief_vs_standalone_are_different() {
        let brief = BriefId::new("Test", Utc::now());
        let brief_task = TaskId::new(&brief, 1);

        let ts = Utc::now();
        let standalone = TaskId::new_standalone("Test", ts);

        assert!(!brief_task.is_standalone());
        assert!(standalone.is_standalone());
        assert!(brief_task.brief_id().is_some());
        assert!(standalone.brief_id().is_none());
    }
}
