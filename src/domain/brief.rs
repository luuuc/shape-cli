//! Brief domain model
//!
//! Briefs are the high-level documents (pitches, RFCs, PRDs, etc.)
//! that organize related tasks. They are stored as markdown files
//! with YAML frontmatter.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::id::BriefId;

/// Status of a brief
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BriefStatus {
    /// Initial state - not yet started
    #[default]
    Proposed,

    /// In the betting process (for ShapeUp)
    Betting,

    /// Actively being worked on
    InProgress,

    /// Successfully completed
    Shipped,

    /// No longer being pursued
    Archived,
}

impl BriefStatus {
    /// Returns true if this status represents completion
    pub fn is_complete(&self) -> bool {
        matches!(self, BriefStatus::Shipped | BriefStatus::Archived)
    }

    /// Returns true if this brief is actively being worked on
    pub fn is_active(&self) -> bool {
        matches!(self, BriefStatus::InProgress)
    }

    /// Returns all valid status values
    pub fn all() -> &'static [BriefStatus] {
        &[
            BriefStatus::Proposed,
            BriefStatus::Betting,
            BriefStatus::InProgress,
            BriefStatus::Shipped,
            BriefStatus::Archived,
        ]
    }
}

impl std::fmt::Display for BriefStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BriefStatus::Proposed => write!(f, "proposed"),
            BriefStatus::Betting => write!(f, "betting"),
            BriefStatus::InProgress => write!(f, "in_progress"),
            BriefStatus::Shipped => write!(f, "shipped"),
            BriefStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for BriefStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(BriefStatus::Proposed),
            "betting" => Ok(BriefStatus::Betting),
            "in_progress" | "in-progress" | "inprogress" => Ok(BriefStatus::InProgress),
            "shipped" | "done" | "complete" | "completed" => Ok(BriefStatus::Shipped),
            "archived" | "cancelled" | "canceled" => Ok(BriefStatus::Archived),
            _ => Err(format!("Unknown brief status: {}", s)),
        }
    }
}

/// Metadata for a brief - extensible key-value pairs
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BriefMeta(HashMap<String, serde_json::Value>);

impl BriefMeta {
    /// Creates empty metadata
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Gets a value by key
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Sets a value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.0.insert(key.into(), value.into());
    }

    /// Removes a value
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.0.remove(key)
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates over all key-value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.0.iter()
    }

    /// Returns the inner HashMap
    pub fn inner(&self) -> &HashMap<String, serde_json::Value> {
        &self.0
    }
}

/// A brief document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Brief {
    /// Unique identifier
    pub id: BriefId,

    /// Human-readable title
    pub title: String,

    /// Brief type (e.g., "minimal", "shapeup", "rfc")
    #[serde(rename = "type")]
    pub brief_type: String,

    /// Current status
    pub status: BriefStatus,

    /// When the brief was created
    pub created_at: DateTime<Utc>,

    /// When the brief was last updated
    pub updated_at: DateTime<Utc>,

    /// Markdown body content (excluding frontmatter)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,

    /// Extensible metadata from frontmatter
    #[serde(default, skip_serializing_if = "BriefMeta::is_empty")]
    pub meta: BriefMeta,
}

impl Brief {
    /// Creates a new brief with the given title and type
    pub fn new(title: impl Into<String>, brief_type: impl Into<String>) -> Self {
        let title = title.into();
        let now = Utc::now();
        let id = BriefId::new(&title, now);

        Self {
            id,
            title,
            brief_type: brief_type.into(),
            status: BriefStatus::Proposed,
            created_at: now,
            updated_at: now,
            body: String::new(),
            meta: BriefMeta::new(),
        }
    }

    /// Creates a new brief with a specific ID (for deserialization)
    pub fn with_id(id: BriefId, title: impl Into<String>, brief_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            brief_type: brief_type.into(),
            status: BriefStatus::Proposed,
            created_at: now,
            updated_at: now,
            body: String::new(),
            meta: BriefMeta::new(),
        }
    }

    /// Transitions to a new status
    pub fn set_status(&mut self, status: BriefStatus) {
        if self.status != status {
            self.status = status;
            self.updated_at = Utc::now();
        }
    }

    /// Sets the body content
    pub fn set_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
        self.updated_at = Utc::now();
    }

    /// Sets a metadata value
    pub fn set_meta(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.meta.set(key, value);
        self.updated_at = Utc::now();
    }

    /// Gets a metadata value
    pub fn get_meta(&self, key: &str) -> Option<&serde_json::Value> {
        self.meta.get(key)
    }

    /// Removes a metadata value
    pub fn remove_meta(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.meta.remove(key);
        if result.is_some() {
            self.updated_at = Utc::now();
        }
        result
    }

    /// Returns true if this brief is complete (shipped or archived)
    pub fn is_complete(&self) -> bool {
        self.status.is_complete()
    }

    /// Returns true if this brief is actively being worked on
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

/// Represents the frontmatter section of a brief file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefFrontmatter {
    pub id: BriefId,
    pub title: String,
    #[serde(rename = "type")]
    pub brief_type: String,
    pub status: BriefStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub meta: HashMap<String, serde_json::Value>,
}

impl From<&Brief> for BriefFrontmatter {
    fn from(brief: &Brief) -> Self {
        Self {
            id: brief.id.clone(),
            title: brief.title.clone(),
            brief_type: brief.brief_type.clone(),
            status: brief.status,
            created_at: brief.created_at,
            updated_at: brief.updated_at,
            meta: brief.meta.inner().clone(),
        }
    }
}

impl BriefFrontmatter {
    /// Converts to a Brief with the given body
    pub fn into_brief(self, body: String) -> Brief {
        Brief {
            id: self.id,
            title: self.title,
            brief_type: self.brief_type,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            body,
            meta: BriefMeta(self.meta),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_brief_has_proposed_status() {
        let brief = Brief::new("Test Pitch", "minimal");
        assert_eq!(brief.status, BriefStatus::Proposed);
    }

    #[test]
    fn brief_id_is_generated_from_title() {
        let brief = Brief::new("My Feature", "minimal");
        assert!(brief.id.to_string().starts_with("b-"));
    }

    #[test]
    fn brief_status_transitions() {
        let mut brief = Brief::new("Test", "minimal");

        brief.set_status(BriefStatus::InProgress);
        assert!(brief.is_active());

        brief.set_status(BriefStatus::Shipped);
        assert!(brief.is_complete());
    }

    #[test]
    fn brief_meta_operations() {
        let mut brief = Brief::new("Test", "minimal");

        brief.set_meta("appetite", "6 weeks");
        brief.set_meta("priority", 1);

        assert_eq!(
            brief.get_meta("appetite"),
            Some(&serde_json::json!("6 weeks"))
        );
        assert_eq!(brief.get_meta("priority"), Some(&serde_json::json!(1)));

        brief.remove_meta("appetite");
        assert!(brief.get_meta("appetite").is_none());
    }

    #[test]
    fn brief_body() {
        let mut brief = Brief::new("Test", "minimal");
        brief.set_body("# Problem\n\nThis is the problem statement.");

        assert_eq!(brief.body, "# Problem\n\nThis is the problem statement.");
    }

    #[test]
    fn brief_status_from_string() {
        assert_eq!(
            "proposed".parse::<BriefStatus>().unwrap(),
            BriefStatus::Proposed
        );
        assert_eq!(
            "in_progress".parse::<BriefStatus>().unwrap(),
            BriefStatus::InProgress
        );
        assert_eq!(
            "in-progress".parse::<BriefStatus>().unwrap(),
            BriefStatus::InProgress
        );
        assert_eq!(
            "shipped".parse::<BriefStatus>().unwrap(),
            BriefStatus::Shipped
        );
        assert_eq!("done".parse::<BriefStatus>().unwrap(), BriefStatus::Shipped);
        assert_eq!(
            "archived".parse::<BriefStatus>().unwrap(),
            BriefStatus::Archived
        );
    }

    #[test]
    fn brief_status_display() {
        assert_eq!(BriefStatus::Proposed.to_string(), "proposed");
        assert_eq!(BriefStatus::InProgress.to_string(), "in_progress");
    }

    #[test]
    fn serde_roundtrip() {
        let mut brief = Brief::new("Test", "shapeup");
        brief.set_body("Some content");
        brief.set_meta("appetite", "2 weeks");

        let json = serde_json::to_string(&brief).unwrap();
        let parsed: Brief = serde_json::from_str(&json).unwrap();

        assert_eq!(brief.id, parsed.id);
        assert_eq!(brief.title, parsed.title);
        assert_eq!(brief.brief_type, parsed.brief_type);
        assert_eq!(brief.body, parsed.body);
    }

    #[test]
    fn frontmatter_conversion() {
        let mut brief = Brief::new("Test", "minimal");
        brief.set_meta("custom", "value");

        let frontmatter = BriefFrontmatter::from(&brief);
        let restored = frontmatter.into_brief(brief.body.clone());

        assert_eq!(brief.id, restored.id);
        assert_eq!(brief.title, restored.title);
        assert_eq!(brief.get_meta("custom"), restored.get_meta("custom"));
    }

    #[test]
    fn updated_at_changes_on_modifications() {
        let mut brief = Brief::new("Test", "minimal");
        let created = brief.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        brief.set_status(BriefStatus::InProgress);

        assert!(brief.updated_at > created);
    }
}
