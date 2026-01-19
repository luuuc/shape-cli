//! Anchor domain model
//!
//! Anchors are the high-level documents (pitches, RFCs, PRDs, etc.)
//! that organize related tasks. They are stored as markdown files
//! with YAML frontmatter.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::id::AnchorId;

/// Status of an anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnchorStatus {
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

impl AnchorStatus {
    /// Returns true if this status represents completion
    pub fn is_complete(&self) -> bool {
        matches!(self, AnchorStatus::Shipped | AnchorStatus::Archived)
    }

    /// Returns true if this anchor is actively being worked on
    pub fn is_active(&self) -> bool {
        matches!(self, AnchorStatus::InProgress)
    }

    /// Returns all valid status values
    pub fn all() -> &'static [AnchorStatus] {
        &[
            AnchorStatus::Proposed,
            AnchorStatus::Betting,
            AnchorStatus::InProgress,
            AnchorStatus::Shipped,
            AnchorStatus::Archived,
        ]
    }
}

impl std::fmt::Display for AnchorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorStatus::Proposed => write!(f, "proposed"),
            AnchorStatus::Betting => write!(f, "betting"),
            AnchorStatus::InProgress => write!(f, "in_progress"),
            AnchorStatus::Shipped => write!(f, "shipped"),
            AnchorStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for AnchorStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(AnchorStatus::Proposed),
            "betting" => Ok(AnchorStatus::Betting),
            "in_progress" | "in-progress" | "inprogress" => Ok(AnchorStatus::InProgress),
            "shipped" | "done" | "complete" | "completed" => Ok(AnchorStatus::Shipped),
            "archived" | "cancelled" | "canceled" => Ok(AnchorStatus::Archived),
            _ => Err(format!("Unknown anchor status: {}", s)),
        }
    }
}

/// Metadata for an anchor - extensible key-value pairs
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnchorMeta(HashMap<String, serde_json::Value>);

impl AnchorMeta {
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

/// An anchor document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    /// Unique identifier
    pub id: AnchorId,

    /// Human-readable title
    pub title: String,

    /// Anchor type (e.g., "minimal", "shapeup", "rfc")
    #[serde(rename = "type")]
    pub anchor_type: String,

    /// Current status
    pub status: AnchorStatus,

    /// When the anchor was created
    pub created_at: DateTime<Utc>,

    /// When the anchor was last updated
    pub updated_at: DateTime<Utc>,

    /// Markdown body content (excluding frontmatter)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,

    /// Extensible metadata from frontmatter
    #[serde(default, skip_serializing_if = "AnchorMeta::is_empty")]
    pub meta: AnchorMeta,
}

impl Anchor {
    /// Creates a new anchor with the given title and type
    pub fn new(title: impl Into<String>, anchor_type: impl Into<String>) -> Self {
        let title = title.into();
        let now = Utc::now();
        let id = AnchorId::new(&title, now);

        Self {
            id,
            title,
            anchor_type: anchor_type.into(),
            status: AnchorStatus::Proposed,
            created_at: now,
            updated_at: now,
            body: String::new(),
            meta: AnchorMeta::new(),
        }
    }

    /// Creates a new anchor with a specific ID (for deserialization)
    pub fn with_id(id: AnchorId, title: impl Into<String>, anchor_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            anchor_type: anchor_type.into(),
            status: AnchorStatus::Proposed,
            created_at: now,
            updated_at: now,
            body: String::new(),
            meta: AnchorMeta::new(),
        }
    }

    /// Transitions to a new status
    pub fn set_status(&mut self, status: AnchorStatus) {
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

    /// Returns true if this anchor is complete (shipped or archived)
    pub fn is_complete(&self) -> bool {
        self.status.is_complete()
    }

    /// Returns true if this anchor is actively being worked on
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

/// Represents the frontmatter section of an anchor file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorFrontmatter {
    pub id: AnchorId,
    pub title: String,
    #[serde(rename = "type")]
    pub anchor_type: String,
    pub status: AnchorStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub meta: HashMap<String, serde_json::Value>,
}

impl From<&Anchor> for AnchorFrontmatter {
    fn from(anchor: &Anchor) -> Self {
        Self {
            id: anchor.id.clone(),
            title: anchor.title.clone(),
            anchor_type: anchor.anchor_type.clone(),
            status: anchor.status,
            created_at: anchor.created_at,
            updated_at: anchor.updated_at,
            meta: anchor.meta.inner().clone(),
        }
    }
}

impl AnchorFrontmatter {
    /// Converts to an Anchor with the given body
    pub fn into_anchor(self, body: String) -> Anchor {
        Anchor {
            id: self.id,
            title: self.title,
            anchor_type: self.anchor_type,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            body,
            meta: AnchorMeta(self.meta),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_anchor_has_proposed_status() {
        let anchor = Anchor::new("Test Pitch", "minimal");
        assert_eq!(anchor.status, AnchorStatus::Proposed);
    }

    #[test]
    fn anchor_id_is_generated_from_title() {
        let anchor = Anchor::new("My Feature", "minimal");
        assert!(anchor.id.to_string().starts_with("a-"));
    }

    #[test]
    fn anchor_status_transitions() {
        let mut anchor = Anchor::new("Test", "minimal");

        anchor.set_status(AnchorStatus::InProgress);
        assert!(anchor.is_active());

        anchor.set_status(AnchorStatus::Shipped);
        assert!(anchor.is_complete());
    }

    #[test]
    fn anchor_meta_operations() {
        let mut anchor = Anchor::new("Test", "minimal");

        anchor.set_meta("appetite", "6 weeks");
        anchor.set_meta("priority", 1);

        assert_eq!(
            anchor.get_meta("appetite"),
            Some(&serde_json::json!("6 weeks"))
        );
        assert_eq!(anchor.get_meta("priority"), Some(&serde_json::json!(1)));

        anchor.remove_meta("appetite");
        assert!(anchor.get_meta("appetite").is_none());
    }

    #[test]
    fn anchor_body() {
        let mut anchor = Anchor::new("Test", "minimal");
        anchor.set_body("# Problem\n\nThis is the problem statement.");

        assert_eq!(anchor.body, "# Problem\n\nThis is the problem statement.");
    }

    #[test]
    fn anchor_status_from_string() {
        assert_eq!(
            "proposed".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::Proposed
        );
        assert_eq!(
            "in_progress".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::InProgress
        );
        assert_eq!(
            "in-progress".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::InProgress
        );
        assert_eq!(
            "shipped".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::Shipped
        );
        assert_eq!(
            "done".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::Shipped
        );
        assert_eq!(
            "archived".parse::<AnchorStatus>().unwrap(),
            AnchorStatus::Archived
        );
    }

    #[test]
    fn anchor_status_display() {
        assert_eq!(AnchorStatus::Proposed.to_string(), "proposed");
        assert_eq!(AnchorStatus::InProgress.to_string(), "in_progress");
    }

    #[test]
    fn serde_roundtrip() {
        let mut anchor = Anchor::new("Test", "shapeup");
        anchor.set_body("Some content");
        anchor.set_meta("appetite", "2 weeks");

        let json = serde_json::to_string(&anchor).unwrap();
        let parsed: Anchor = serde_json::from_str(&json).unwrap();

        assert_eq!(anchor.id, parsed.id);
        assert_eq!(anchor.title, parsed.title);
        assert_eq!(anchor.anchor_type, parsed.anchor_type);
        assert_eq!(anchor.body, parsed.body);
    }

    #[test]
    fn frontmatter_conversion() {
        let mut anchor = Anchor::new("Test", "minimal");
        anchor.set_meta("custom", "value");

        let frontmatter = AnchorFrontmatter::from(&anchor);
        let restored = frontmatter.into_anchor(anchor.body.clone());

        assert_eq!(anchor.id, restored.id);
        assert_eq!(anchor.title, restored.title);
        assert_eq!(anchor.get_meta("custom"), restored.get_meta("custom"));
    }

    #[test]
    fn updated_at_changes_on_modifications() {
        let mut anchor = Anchor::new("Test", "minimal");
        let created = anchor.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        anchor.set_status(AnchorStatus::InProgress);

        assert!(anchor.updated_at > created);
    }
}
