//! Task domain model
//!
//! Tasks are the executable units of work within a brief.
//! They can have dependencies on other tasks and support subtasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use super::id::{BriefId, TaskId};

/// Type of dependency between tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Task A must complete before Task B can start (affects ready/blocked)
    #[default]
    Blocks,
    /// Task B was created because of Task A (informational)
    Provenance,
    /// Tasks are related but don't block each other (informational)
    Related,
    /// Task B is a duplicate of Task A (informational)
    Duplicates,
}

impl DependencyType {
    /// Returns true if this dependency type affects the ready queue
    pub fn affects_ready(&self) -> bool {
        matches!(self, DependencyType::Blocks)
    }

    /// Returns a display label for the dependency type
    pub fn label(&self) -> &'static str {
        match self {
            DependencyType::Blocks => "blocks",
            DependencyType::Provenance => "from",
            DependencyType::Related => "link",
            DependencyType::Duplicates => "dup",
        }
    }
}

/// A typed dependency on another task
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// The task this depends on
    pub task: TaskId,
    /// The type of dependency
    #[serde(rename = "type", default)]
    pub dep_type: DependencyType,
}

impl Dependency {
    /// Creates a new blocking dependency
    pub fn blocks(task: TaskId) -> Self {
        Self {
            task,
            dep_type: DependencyType::Blocks,
        }
    }

    /// Creates a new provenance dependency
    pub fn provenance(task: TaskId) -> Self {
        Self {
            task,
            dep_type: DependencyType::Provenance,
        }
    }

    /// Creates a new related dependency
    pub fn related(task: TaskId) -> Self {
        Self {
            task,
            dep_type: DependencyType::Related,
        }
    }

    /// Creates a new duplicates dependency
    pub fn duplicates(task: TaskId) -> Self {
        Self {
            task,
            dep_type: DependencyType::Duplicates,
        }
    }
}

/// Collection of dependencies with backward-compatible serialization
///
/// Old format: `["t-1", "t-2"]` (array of strings, implies blocks)
/// New format: `[{"task": "t-1", "type": "blocks"}, {"task": "t-2", "type": "provenance"}]`
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dependencies(Vec<Dependency>);

impl Dependencies {
    /// Creates an empty dependencies collection
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds a dependency
    pub fn add(&mut self, dep: Dependency) -> bool {
        if !self
            .0
            .iter()
            .any(|d| d.task == dep.task && d.dep_type == dep.dep_type)
        {
            self.0.push(dep);
            true
        } else {
            false
        }
    }

    /// Removes a dependency by task ID and optionally by type
    pub fn remove(&mut self, task_id: &TaskId, dep_type: Option<DependencyType>) -> bool {
        let len_before = self.0.len();
        self.0.retain(|d| {
            if &d.task != task_id {
                return true;
            }
            if let Some(dt) = dep_type {
                d.dep_type != dt
            } else {
                false
            }
        });
        self.0.len() != len_before
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of dependencies
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterates over all dependencies
    pub fn iter(&self) -> impl Iterator<Item = &Dependency> {
        self.0.iter()
    }

    /// Returns only blocking dependencies
    pub fn blocking(&self) -> impl Iterator<Item = &Dependency> {
        self.0
            .iter()
            .filter(|d| d.dep_type == DependencyType::Blocks)
    }

    /// Returns dependencies by type
    pub fn by_type(&self, dep_type: DependencyType) -> impl Iterator<Item = &Dependency> {
        self.0.iter().filter(move |d| d.dep_type == dep_type)
    }

    /// Returns blocking task IDs (for ready/blocked calculations)
    pub fn blocking_task_ids(&self) -> impl Iterator<Item = &TaskId> {
        self.blocking().map(|d| &d.task)
    }

    /// Checks if a specific task ID exists as a dependency
    pub fn contains(&self, task_id: &TaskId) -> bool {
        self.0.iter().any(|d| &d.task == task_id)
    }

    /// Checks if a specific task ID exists as a blocking dependency
    pub fn contains_blocking(&self, task_id: &TaskId) -> bool {
        self.blocking().any(|d| &d.task == task_id)
    }
}

impl Serialize for Dependencies {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always serialize as the new format
        self.0.serialize(serializer)
    }
}

impl<'a> IntoIterator for &'a Dependencies {
    type Item = &'a Dependency;
    type IntoIter = std::slice::Iter<'a, Dependency>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'de> Deserialize<'de> for Dependencies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{SeqAccess, Visitor};

        struct DependenciesVisitor;

        impl<'de> Visitor<'de> for DependenciesVisitor {
            type Value = Dependencies;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of dependencies (strings or objects)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut deps = Vec::new();

                // We need to handle both formats:
                // Old: ["t-1", "t-2"]
                // New: [{"task": "t-1", "type": "blocks"}]
                while let Some(value) = seq.next_element::<serde_json::Value>()? {
                    let dep = match value {
                        // Old format: just a string task ID
                        serde_json::Value::String(s) => {
                            let task_id: TaskId = s.parse().map_err(serde::de::Error::custom)?;
                            Dependency::blocks(task_id)
                        }
                        // New format: object with task and type
                        serde_json::Value::Object(obj) => {
                            serde_json::from_value(serde_json::Value::Object(obj))
                                .map_err(serde::de::Error::custom)?
                        }
                        _ => {
                            return Err(serde::de::Error::custom(
                                "expected string or object for dependency",
                            ))
                        }
                    };
                    deps.push(dep);
                }

                Ok(Dependencies(deps))
            }
        }

        deserializer.deserialize_seq(DependenciesVisitor)
    }
}

/// Status of a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    /// Returns true if this status represents completion
    pub fn is_complete(&self) -> bool {
        matches!(self, TaskStatus::Done)
    }

    /// Returns true if this task is not yet started
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskStatus::Todo)
    }

    /// Returns true if this task is currently being worked on
    pub fn is_active(&self) -> bool {
        matches!(self, TaskStatus::InProgress)
    }
}

/// Metadata for a task - extensible key-value pairs
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskMeta(HashMap<String, serde_json::Value>);

impl TaskMeta {
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

    /// Returns all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

/// Per-field version timestamps for conflict resolution
///
/// Each field tracks when it was last modified (as milliseconds since epoch).
/// This enables field-level conflict resolution: when merging concurrent edits,
/// the field with the newer timestamp wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FieldVersions {
    /// Version timestamp for title field
    #[serde(default, skip_serializing_if = "is_zero")]
    pub title: i64,

    /// Version timestamp for status field
    #[serde(default, skip_serializing_if = "is_zero")]
    pub status: i64,

    /// Version timestamp for description field
    #[serde(default, skip_serializing_if = "is_zero")]
    pub description: i64,

    /// Version timestamp for completed_at field
    #[serde(default, skip_serializing_if = "is_zero")]
    pub completed_at: i64,

    /// Per-key version timestamps for metadata fields
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, i64>,
}

fn is_zero(val: &i64) -> bool {
    *val == 0
}

impl FieldVersions {
    /// Creates new field versions with current timestamp for all fields
    pub fn new() -> Self {
        let now = current_timestamp();
        Self {
            title: now,
            status: now,
            description: 0,
            completed_at: 0,
            meta: HashMap::new(),
        }
    }

    /// Creates field versions from epoch (for backward compatibility with old tasks)
    pub fn from_epoch() -> Self {
        Self::default()
    }

    /// Updates the title version to current timestamp
    pub fn touch_title(&mut self) {
        self.title = current_timestamp();
    }

    /// Updates the status version to current timestamp
    pub fn touch_status(&mut self) {
        self.status = current_timestamp();
    }

    /// Updates the description version to current timestamp
    pub fn touch_description(&mut self) {
        self.description = current_timestamp();
    }

    /// Updates the completed_at version to current timestamp
    pub fn touch_completed_at(&mut self) {
        self.completed_at = current_timestamp();
    }

    /// Updates a metadata key's version to current timestamp
    pub fn touch_meta(&mut self, key: &str) {
        self.meta.insert(key.to_string(), current_timestamp());
    }

    /// Gets the version timestamp for a metadata key
    pub fn meta_version(&self, key: &str) -> i64 {
        self.meta.get(key).copied().unwrap_or(0)
    }

    /// Returns true if all version timestamps are zero (backward compat mode)
    pub fn is_empty(&self) -> bool {
        self.title == 0
            && self.status == 0
            && self.description == 0
            && self.completed_at == 0
            && self.meta.is_empty()
    }
}

/// Returns current timestamp in milliseconds since epoch
pub fn current_timestamp() -> i64 {
    Utc::now().timestamp_millis()
}

/// A task within a brief
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub id: TaskId,

    /// Human-readable title
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// Dependencies on other tasks (typed: blocks, provenance, related, duplicates)
    #[serde(default, skip_serializing_if = "Dependencies::is_empty")]
    pub depends_on: Dependencies,

    /// When the task was created
    pub created_at: DateTime<Utc>,

    /// When the task was last updated
    pub updated_at: DateTime<Utc>,

    /// When the task was completed (if done)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Extensible metadata
    #[serde(default, skip_serializing_if = "TaskMeta::is_empty")]
    pub meta: TaskMeta,

    /// Per-field version timestamps for conflict resolution
    /// Field is named `_v` in JSON for compactness
    #[serde(
        rename = "_v",
        default,
        skip_serializing_if = "FieldVersions::is_empty"
    )]
    pub versions: FieldVersions,

    /// Summary text for compacted tasks (only set on representative task)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// IDs of tasks that were compacted into this one (only set on representative task)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_tasks: Option<Vec<TaskId>>,

    /// ID of the task this was compacted into (set on non-representative compacted tasks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_into: Option<TaskId>,

    // --- Agent coordination fields ---
    /// Agent that has claimed this task
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,

    /// When the task was claimed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<DateTime<Utc>>,

    /// Notes/context accumulated during work
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,

    /// Links to artifacts (commits, PRs, files, URLs)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,

    /// Explicit block (separate from dependency blocks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<BlockInfo>,

    /// Task history/timeline
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<HistoryEvent>,

    /// Agent this task is assigned to (for handoff)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
}

/// A note added during task work
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// When the note was added
    pub at: DateTime<Utc>,
    /// Who added the note
    pub by: String,
    /// The note text
    pub text: String,
}

/// A link to an artifact
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Link {
    /// Type of link (commit, pr, file, url)
    #[serde(rename = "type")]
    pub link_type: LinkType,
    /// The reference (commit hash, PR number, file path, URL)
    #[serde(rename = "ref")]
    pub reference: String,
    /// When the link was added
    pub at: DateTime<Utc>,
    /// Who added the link
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// Type of artifact link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Commit,
    Pr,
    File,
    Url,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Commit => "commit",
            LinkType::Pr => "pr",
            LinkType::File => "file",
            LinkType::Url => "url",
        }
    }
}

/// Information about an explicit block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Reason for the block
    pub reason: String,
    /// Who blocked the task
    pub by: String,
    /// When it was blocked
    pub at: DateTime<Utc>,
    /// Optional task ID this is blocked on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_task: Option<TaskId>,
}

/// A history event for the task timeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEvent {
    /// When the event occurred
    pub at: DateTime<Utc>,
    /// Type of event
    pub event: HistoryEventType,
    /// Who caused the event
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Additional event data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Types of history events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryEventType {
    Created,
    Started,
    Completed,
    Reopened,
    Claimed,
    Unclaimed,
    Note,
    Linked,
    Unlinked,
    Blocked,
    Unblocked,
    Assigned,
    Handoff,
}

impl Task {
    /// Creates a new task with the given ID and title
    pub fn new(id: TaskId, title: impl Into<String>) -> Self {
        let now = Utc::now();
        let mut task = Self {
            id,
            title: title.into(),
            status: TaskStatus::Todo,
            depends_on: Dependencies::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            description: None,
            meta: TaskMeta::new(),
            versions: FieldVersions::new(),
            summary: None,
            compacted_tasks: None,
            compacted_into: None,
            claimed_by: None,
            claimed_at: None,
            notes: Vec::new(),
            links: Vec::new(),
            blocked: None,
            history: Vec::new(),
            assigned_to: None,
        };
        task.add_history_event(HistoryEventType::Created, None, None);
        task
    }

    /// Returns the brief ID this task belongs to, or None if standalone
    pub fn brief_id(&self) -> Option<BriefId> {
        self.id.brief_id()
    }

    /// Returns true if this is a standalone task (not belonging to a brief)
    pub fn is_standalone(&self) -> bool {
        self.id.is_standalone()
    }

    /// Returns true if this task has no incomplete blocking dependencies
    pub fn is_ready(&self, task_statuses: &HashMap<TaskId, TaskStatus>) -> bool {
        if self.status.is_complete() {
            return false; // Completed tasks are not "ready"
        }

        // Only blocking dependencies affect readiness
        self.depends_on.blocking_task_ids().all(|dep_id| {
            task_statuses
                .get(dep_id)
                .map(|s| s.is_complete())
                .unwrap_or(false)
        })
    }

    /// Returns true if this task is blocked by incomplete blocking dependencies
    pub fn is_blocked(&self, task_statuses: &HashMap<TaskId, TaskStatus>) -> bool {
        if self.status.is_complete() {
            return false; // Completed tasks are not "blocked"
        }

        // Only blocking dependencies affect blocked status
        self.depends_on.blocking_task_ids().any(|dep_id| {
            task_statuses
                .get(dep_id)
                .map(|s| !s.is_complete())
                .unwrap_or(true) // Unknown dependency = blocked
        })
    }

    /// Transitions to in_progress status
    pub fn start(&mut self) {
        if self.status == TaskStatus::Todo {
            self.status = TaskStatus::InProgress;
            self.updated_at = Utc::now();
            self.versions.touch_status();
        }
    }

    /// Transitions to done status
    pub fn complete(&mut self) {
        if !self.status.is_complete() {
            self.status = TaskStatus::Done;
            let now = Utc::now();
            self.updated_at = now;
            self.completed_at = Some(now);
            self.versions.touch_status();
            self.versions.touch_completed_at();
        }
    }

    /// Transitions back to todo status
    pub fn reopen(&mut self) {
        if self.status.is_complete() {
            self.status = TaskStatus::Todo;
            self.updated_at = Utc::now();
            self.completed_at = None;
            self.versions.touch_status();
            self.versions.touch_completed_at();
        }
    }

    /// Adds a blocking dependency on another task (default behavior)
    pub fn add_dependency(&mut self, task_id: TaskId) {
        self.add_typed_dependency(Dependency::blocks(task_id));
    }

    /// Adds a typed dependency on another task
    pub fn add_typed_dependency(&mut self, dependency: Dependency) {
        if self.depends_on.add(dependency) {
            self.updated_at = Utc::now();
        }
    }

    /// Removes all dependencies on a task ID
    pub fn remove_dependency(&mut self, task_id: &TaskId) {
        if self.depends_on.remove(task_id, None) {
            self.updated_at = Utc::now();
        }
    }

    /// Removes a specific typed dependency
    pub fn remove_typed_dependency(&mut self, task_id: &TaskId, dep_type: DependencyType) {
        if self.depends_on.remove(task_id, Some(dep_type)) {
            self.updated_at = Utc::now();
        }
    }

    /// Sets a metadata value
    pub fn set_meta(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        let key = key.into();
        self.versions.touch_meta(&key);
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
            self.versions.touch_meta(key);
            self.updated_at = Utc::now();
        }
        result
    }

    /// Sets the description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
        self.updated_at = Utc::now();
        self.versions.touch_description();
    }

    /// Sets the title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.updated_at = Utc::now();
        self.versions.touch_title();
    }

    /// Returns true if this task has been compacted into another task
    pub fn is_compacted(&self) -> bool {
        self.compacted_into.is_some()
    }

    /// Returns true if this task is a compaction representative (holds summary of other tasks)
    pub fn is_compaction_representative(&self) -> bool {
        self.compacted_tasks.is_some()
    }

    /// Returns the number of tasks compacted into this one (including itself)
    pub fn compacted_count(&self) -> usize {
        self.compacted_tasks.as_ref().map(|t| t.len()).unwrap_or(0)
    }

    /// Mark this task as compacted into another task
    pub fn compact_into(&mut self, representative_id: TaskId) {
        self.compacted_into = Some(representative_id);
        self.updated_at = Utc::now();
    }

    /// Mark this task as a compaction representative with the given summary and task IDs
    pub fn set_compaction(&mut self, summary: String, compacted_task_ids: Vec<TaskId>) {
        self.summary = Some(summary);
        self.compacted_tasks = Some(compacted_task_ids);
        self.updated_at = Utc::now();
    }

    /// Clear compaction data (for undo)
    pub fn clear_compaction(&mut self) {
        self.summary = None;
        self.compacted_tasks = None;
        self.compacted_into = None;
        self.updated_at = Utc::now();
    }

    // --- Agent coordination methods ---

    /// Claims this task for an agent
    pub fn claim(&mut self, agent: impl Into<String>) {
        let agent = agent.into();
        let now = Utc::now();
        self.claimed_by = Some(agent.clone());
        self.claimed_at = Some(now);
        self.updated_at = now;
        // Also start the task when claiming
        if self.status == TaskStatus::Todo {
            self.status = TaskStatus::InProgress;
            self.versions.touch_status();
            self.add_history_event(HistoryEventType::Started, Some(&agent), None);
        }
        self.add_history_event(HistoryEventType::Claimed, Some(&agent), None);
    }

    /// Unclaims this task
    pub fn unclaim(&mut self, agent: Option<&str>) {
        if self.claimed_by.is_some() {
            // Get the by string before modifying self
            let by_str = agent
                .map(|s| s.to_string())
                .or_else(|| self.claimed_by.clone());
            self.add_history_event(HistoryEventType::Unclaimed, by_str.as_deref(), None);
            self.claimed_by = None;
            self.claimed_at = None;
            self.updated_at = Utc::now();
        }
    }

    /// Returns true if the task is currently claimed
    pub fn is_claimed(&self) -> bool {
        self.claimed_by.is_some()
    }

    /// Returns true if the claim has expired
    pub fn is_claim_expired(&self, timeout_hours: u32) -> bool {
        if let Some(claimed_at) = self.claimed_at {
            let duration = chrono::Duration::hours(timeout_hours as i64);
            Utc::now() > claimed_at + duration
        } else {
            false
        }
    }

    /// Returns the remaining time on the claim in hours (or None if not claimed)
    pub fn claim_remaining_hours(&self, timeout_hours: u32) -> Option<f64> {
        if let Some(claimed_at) = self.claimed_at {
            let duration = chrono::Duration::hours(timeout_hours as i64);
            let expires_at = claimed_at + duration;
            let remaining = expires_at - Utc::now();
            if remaining.num_seconds() > 0 {
                Some(remaining.num_minutes() as f64 / 60.0)
            } else {
                Some(0.0)
            }
        } else {
            None
        }
    }

    /// Adds a note to the task
    pub fn add_note(&mut self, agent: impl Into<String>, text: impl Into<String>) {
        let agent = agent.into();
        let text = text.into();
        let now = Utc::now();
        self.notes.push(Note {
            at: now,
            by: agent.clone(),
            text: text.clone(),
        });
        self.updated_at = now;
        self.add_history_event(
            HistoryEventType::Note,
            Some(&agent),
            Some(serde_json::json!({ "text": text })),
        );
    }

    /// Adds a link to an artifact
    pub fn add_link(
        &mut self,
        link_type: LinkType,
        reference: impl Into<String>,
        agent: Option<&str>,
    ) {
        let reference = reference.into();
        let now = Utc::now();
        self.links.push(Link {
            link_type,
            reference: reference.clone(),
            at: now,
            by: agent.map(|s| s.to_string()),
        });
        self.updated_at = now;
        self.add_history_event(
            HistoryEventType::Linked,
            agent,
            Some(serde_json::json!({ "type": link_type, "ref": reference })),
        );
    }

    /// Removes a link
    pub fn remove_link(
        &mut self,
        link_type: LinkType,
        reference: &str,
        agent: Option<&str>,
    ) -> bool {
        let len_before = self.links.len();
        self.links
            .retain(|l| !(l.link_type == link_type && l.reference == reference));
        if self.links.len() != len_before {
            self.updated_at = Utc::now();
            self.add_history_event(
                HistoryEventType::Unlinked,
                agent,
                Some(serde_json::json!({ "type": link_type, "ref": reference })),
            );
            true
        } else {
            false
        }
    }

    /// Blocks the task with a reason
    pub fn block(
        &mut self,
        reason: impl Into<String>,
        agent: impl Into<String>,
        on_task: Option<TaskId>,
    ) {
        let reason = reason.into();
        let agent = agent.into();
        let now = Utc::now();
        self.blocked = Some(BlockInfo {
            reason: reason.clone(),
            by: agent.clone(),
            at: now,
            on_task: on_task.clone(),
        });
        self.updated_at = now;
        let mut data = serde_json::json!({ "reason": reason });
        if let Some(ref task_id) = on_task {
            data["on_task"] = serde_json::json!(task_id.to_string());
        }
        self.add_history_event(HistoryEventType::Blocked, Some(&agent), Some(data));
    }

    /// Unblocks the task
    pub fn unblock(&mut self, agent: Option<&str>) {
        if self.blocked.is_some() {
            self.blocked = None;
            self.updated_at = Utc::now();
            self.add_history_event(HistoryEventType::Unblocked, agent, None);
        }
    }

    /// Returns true if the task is explicitly blocked (not just dependency blocked)
    pub fn is_explicitly_blocked(&self) -> bool {
        self.blocked.is_some()
    }

    /// Returns true if the task is ready, considering both dependencies and explicit blocks
    pub fn is_ready_for_agent(
        &self,
        task_statuses: &HashMap<TaskId, TaskStatus>,
        exclude_claimed_by: Option<&str>,
    ) -> bool {
        // Not ready if complete
        if self.status.is_complete() {
            return false;
        }
        // Not ready if explicitly blocked
        if self.is_explicitly_blocked() {
            return false;
        }
        // Not ready if claimed by someone else
        if let Some(claimed_by) = &self.claimed_by {
            if let Some(exclude) = exclude_claimed_by {
                if claimed_by != exclude {
                    return false;
                }
            } else {
                return false;
            }
        }
        // Check dependencies
        self.is_ready(task_statuses)
    }

    /// Assigns the task to an agent (for handoff)
    pub fn assign(&mut self, agent: impl Into<String>, by: Option<&str>) {
        let agent = agent.into();
        self.assigned_to = Some(agent.clone());
        self.updated_at = Utc::now();
        self.add_history_event(
            HistoryEventType::Assigned,
            by,
            Some(serde_json::json!({ "to": agent })),
        );
    }

    /// Hands off the task (unclaims and optionally assigns)
    pub fn handoff(&mut self, reason: impl Into<String>, agent: &str, to: Option<String>) {
        let reason = reason.into();
        // Add note with handoff reason
        self.add_note(agent, format!("Handoff: {}", reason));
        // Record handoff event
        let mut data = serde_json::json!({ "reason": reason });
        if let Some(ref to_agent) = to {
            data["to"] = serde_json::json!(to_agent);
        }
        self.add_history_event(HistoryEventType::Handoff, Some(agent), Some(data));
        // Unclaim
        self.unclaim(Some(agent));
        // Optionally assign to new agent
        if let Some(to_agent) = to {
            self.assigned_to = Some(to_agent);
        }
    }

    /// Adds a history event
    fn add_history_event(
        &mut self,
        event: HistoryEventType,
        by: Option<&str>,
        data: Option<serde_json::Value>,
    ) {
        self.history.push(HistoryEvent {
            at: Utc::now(),
            event,
            by: by.map(|s| s.to_string()),
            data,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(seq: u32) -> Task {
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, seq);
        Task::new(task_id, format!("Task {}", seq))
    }

    #[test]
    fn new_task_has_todo_status() {
        let task = make_task(1);
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.status.is_pending());
    }

    #[test]
    fn task_status_transitions() {
        let mut task = make_task(1);

        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.status.is_active());

        task.complete();
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.status.is_complete());
        assert!(task.completed_at.is_some());

        task.reopen();
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn task_dependencies() {
        let mut task1 = make_task(1);
        let task2 = make_task(2);
        let mut task3 = make_task(3);

        // Task 3 depends on task 1 and task 2
        task3.add_dependency(task1.id.clone());
        task3.add_dependency(task2.id.clone());

        // Build status map
        let mut statuses = HashMap::new();
        statuses.insert(task1.id.clone(), task1.status);
        statuses.insert(task2.id.clone(), task2.status);

        // Task 3 is blocked (both deps incomplete)
        assert!(task3.is_blocked(&statuses));
        assert!(!task3.is_ready(&statuses));

        // Complete task 1
        task1.complete();
        statuses.insert(task1.id.clone(), task1.status);

        // Still blocked (task 2 incomplete)
        assert!(task3.is_blocked(&statuses));

        // Complete task 2
        statuses.insert(task2.id.clone(), TaskStatus::Done);

        // Now ready
        assert!(task3.is_ready(&statuses));
        assert!(!task3.is_blocked(&statuses));
    }

    #[test]
    fn task_without_deps_is_ready() {
        let task = make_task(1);
        let statuses = HashMap::new();

        assert!(task.is_ready(&statuses));
        assert!(!task.is_blocked(&statuses));
    }

    #[test]
    fn completed_task_is_neither_ready_nor_blocked() {
        let mut task = make_task(1);
        task.complete();

        let statuses = HashMap::new();
        assert!(!task.is_ready(&statuses));
        assert!(!task.is_blocked(&statuses));
    }

    #[test]
    fn task_meta_operations() {
        let mut task = make_task(1);

        task.set_meta("priority", "high");
        task.set_meta("estimate", 5);

        assert_eq!(task.get_meta("priority"), Some(&serde_json::json!("high")));
        assert_eq!(task.get_meta("estimate"), Some(&serde_json::json!(5)));

        task.remove_meta("priority");
        assert!(task.get_meta("priority").is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let mut task = make_task(1);
        task.set_meta("key", "value");
        task.set_description("A test task");

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(task.id, parsed.id);
        assert_eq!(task.title, parsed.title);
        assert_eq!(task.description, parsed.description);
    }

    #[test]
    fn task_belongs_to_brief() {
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, 1);
        let task = Task::new(task_id, "Task 1");

        assert_eq!(task.brief_id(), Some(brief));
        assert!(!task.is_standalone());
    }

    #[test]
    fn standalone_task_has_no_brief() {
        let task_id = TaskId::new_standalone("Standalone task", Utc::now());
        let task = Task::new(task_id, "Standalone task");

        assert!(task.brief_id().is_none());
        assert!(task.is_standalone());
    }

    #[test]
    fn remove_dependency() {
        let task1 = make_task(1);
        let mut task2 = make_task(2);

        task2.add_dependency(task1.id.clone());
        assert!(task2.depends_on.contains(&task1.id));

        task2.remove_dependency(&task1.id);
        assert!(!task2.depends_on.contains(&task1.id));
    }

    #[test]
    fn typed_dependencies() {
        let task1 = make_task(1);
        let task2 = make_task(2);
        let task3 = make_task(3);
        let mut task4 = make_task(4);

        // Add different types of dependencies
        task4.add_typed_dependency(Dependency::blocks(task1.id.clone()));
        task4.add_typed_dependency(Dependency::provenance(task2.id.clone()));
        task4.add_typed_dependency(Dependency::related(task3.id.clone()));

        // Check counts
        assert_eq!(task4.depends_on.len(), 3);
        assert_eq!(task4.depends_on.blocking().count(), 1);
        assert_eq!(
            task4.depends_on.by_type(DependencyType::Provenance).count(),
            1
        );
        assert_eq!(task4.depends_on.by_type(DependencyType::Related).count(), 1);

        // Only blocking deps affect ready/blocked
        let mut statuses = HashMap::new();
        statuses.insert(task1.id.clone(), TaskStatus::Todo);
        statuses.insert(task2.id.clone(), TaskStatus::Todo);
        statuses.insert(task3.id.clone(), TaskStatus::Todo);

        assert!(task4.is_blocked(&statuses));

        // Complete only the blocking dependency
        statuses.insert(task1.id.clone(), TaskStatus::Done);

        // Now ready even though provenance/related deps are incomplete
        assert!(task4.is_ready(&statuses));
        assert!(!task4.is_blocked(&statuses));
    }

    #[test]
    fn backward_compatible_deserialization() {
        // Create test task IDs using the proper format
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, 1);
        let dep1_id = TaskId::new(&brief, 2);
        let dep2_id = TaskId::new(&brief, 3);

        // Old format: array of strings (just task IDs)
        let old_format = format!(
            r#"{{"id":"{}","title":"Test","status":"todo","depends_on":["{}","{}"],"created_at":"2025-01-01T00:00:00Z","updated_at":"2025-01-01T00:00:00Z"}}"#,
            task_id, dep1_id, dep2_id
        );

        let task: Task = serde_json::from_str(&old_format).unwrap();
        assert_eq!(task.depends_on.len(), 2);

        // Both should be interpreted as blocking dependencies
        assert_eq!(task.depends_on.blocking().count(), 2);
    }

    #[test]
    fn new_format_serialization() {
        let task1 = make_task(1);
        let task2 = make_task(2);
        let mut task3 = make_task(3);

        task3.add_typed_dependency(Dependency::blocks(task1.id.clone()));
        task3.add_typed_dependency(Dependency::provenance(task2.id.clone()));

        let json = serde_json::to_string(&task3).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.depends_on.len(), 2);
        assert_eq!(parsed.depends_on.blocking().count(), 1);
        assert_eq!(
            parsed
                .depends_on
                .by_type(DependencyType::Provenance)
                .count(),
            1
        );
    }

    #[test]
    fn dependency_type_label() {
        assert_eq!(DependencyType::Blocks.label(), "blocks");
        assert_eq!(DependencyType::Provenance.label(), "from");
        assert_eq!(DependencyType::Related.label(), "link");
        assert_eq!(DependencyType::Duplicates.label(), "dup");
    }

    #[test]
    fn updated_at_changes_on_modifications() {
        let mut task = make_task(1);
        let created = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        task.start();

        assert!(task.updated_at > created);
    }

    #[test]
    fn new_task_is_not_compacted() {
        let task = make_task(1);
        assert!(!task.is_compacted());
        assert!(!task.is_compaction_representative());
        assert_eq!(task.compacted_count(), 0);
    }

    #[test]
    fn task_compaction() {
        let mut task1 = make_task(1);
        let mut task2 = make_task(2);
        let task3 = make_task(3);

        // Mark task1 as the representative
        task1.set_compaction(
            "Auth foundation: schema, model, tests".to_string(),
            vec![task1.id.clone(), task2.id.clone(), task3.id.clone()],
        );

        assert!(task1.is_compaction_representative());
        assert_eq!(task1.compacted_count(), 3);
        assert_eq!(
            task1.summary,
            Some("Auth foundation: schema, model, tests".to_string())
        );

        // Mark task2 as compacted into task1
        task2.compact_into(task1.id.clone());
        assert!(task2.is_compacted());
        assert_eq!(task2.compacted_into, Some(task1.id.clone()));
    }

    #[test]
    fn task_compaction_clear() {
        let mut task = make_task(1);

        task.set_compaction("Summary".to_string(), vec![task.id.clone()]);
        assert!(task.is_compaction_representative());

        task.clear_compaction();
        assert!(!task.is_compaction_representative());
        assert!(task.summary.is_none());
        assert!(task.compacted_tasks.is_none());
    }

    #[test]
    fn compaction_serde_roundtrip() {
        let mut task = make_task(1);
        let other_task = make_task(2);

        task.set_compaction(
            "Test summary".to_string(),
            vec![task.id.clone(), other_task.id.clone()],
        );

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.summary, task.summary);
        assert_eq!(parsed.compacted_tasks, task.compacted_tasks);
    }

    #[test]
    fn compacted_into_serde_roundtrip() {
        let task1 = make_task(1);
        let mut task2 = make_task(2);

        task2.compact_into(task1.id.clone());

        let json = serde_json::to_string(&task2).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.compacted_into, Some(task1.id));
    }

    // --- Agent coordination tests ---

    #[test]
    fn claim_unclaim_lifecycle() {
        let mut task = make_task(1);

        // Initially not claimed
        assert!(!task.is_claimed());
        assert!(task.claimed_by.is_none());
        assert!(task.claimed_at.is_none());

        // Claim the task
        task.claim("agent-1");
        assert!(task.is_claimed());
        assert_eq!(task.claimed_by, Some("agent-1".to_string()));
        assert!(task.claimed_at.is_some());
        // Claiming also starts the task
        assert_eq!(task.status, TaskStatus::InProgress);

        // Check history has claim and start events
        let claim_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Claimed)
            .collect();
        assert_eq!(claim_events.len(), 1);
        assert_eq!(claim_events[0].by, Some("agent-1".to_string()));

        // Unclaim the task
        task.unclaim(Some("agent-1"));
        assert!(!task.is_claimed());
        assert!(task.claimed_by.is_none());
        assert!(task.claimed_at.is_none());

        // Check history has unclaim event
        let unclaim_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Unclaimed)
            .collect();
        assert_eq!(unclaim_events.len(), 1);
    }

    #[test]
    fn claim_expiration() {
        let mut task = make_task(1);
        task.claim("agent-1");

        // With a 4-hour timeout, claim should not be expired
        assert!(!task.is_claim_expired(4));

        // Remaining hours should be close to 4
        let remaining = task.claim_remaining_hours(4).unwrap();
        assert!(remaining > 3.9 && remaining <= 4.0);

        // With a 0-hour timeout, claim should be expired
        assert!(task.is_claim_expired(0));
        let remaining_zero = task.claim_remaining_hours(0).unwrap();
        assert_eq!(remaining_zero, 0.0);
    }

    #[test]
    fn add_and_remove_note() {
        let mut task = make_task(1);

        assert!(task.notes.is_empty());

        task.add_note("agent-1", "First note");
        assert_eq!(task.notes.len(), 1);
        assert_eq!(task.notes[0].by, "agent-1");
        assert_eq!(task.notes[0].text, "First note");

        task.add_note("agent-2", "Second note");
        assert_eq!(task.notes.len(), 2);

        // Check history
        let note_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Note)
            .collect();
        assert_eq!(note_events.len(), 2);
    }

    #[test]
    fn add_and_remove_link() {
        let mut task = make_task(1);

        assert!(task.links.is_empty());

        // Add links
        task.add_link(LinkType::Commit, "abc123", Some("agent-1"));
        task.add_link(LinkType::Pr, "42", Some("agent-1"));
        task.add_link(LinkType::File, "src/main.rs", None);
        assert_eq!(task.links.len(), 3);

        // Check link details
        assert_eq!(task.links[0].link_type, LinkType::Commit);
        assert_eq!(task.links[0].reference, "abc123");
        assert_eq!(task.links[0].by, Some("agent-1".to_string()));

        // Remove a link
        let removed = task.remove_link(LinkType::Commit, "abc123", Some("agent-1"));
        assert!(removed);
        assert_eq!(task.links.len(), 2);

        // Try to remove non-existent link
        let not_removed = task.remove_link(LinkType::Commit, "xyz789", None);
        assert!(!not_removed);
        assert_eq!(task.links.len(), 2);

        // Check history
        let linked_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Linked)
            .collect();
        assert_eq!(linked_events.len(), 3);

        let unlinked_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Unlinked)
            .collect();
        assert_eq!(unlinked_events.len(), 1);
    }

    #[test]
    fn block_and_unblock() {
        let mut task = make_task(1);

        assert!(!task.is_explicitly_blocked());
        assert!(task.blocked.is_none());

        // Block the task
        task.block("Waiting for API spec", "agent-1", None);
        assert!(task.is_explicitly_blocked());
        assert!(task.blocked.is_some());

        let block_info = task.blocked.as_ref().unwrap();
        assert_eq!(block_info.reason, "Waiting for API spec");
        assert_eq!(block_info.by, "agent-1");
        assert!(block_info.on_task.is_none());

        // Unblock the task
        task.unblock(Some("agent-2"));
        assert!(!task.is_explicitly_blocked());
        assert!(task.blocked.is_none());

        // Check history
        let blocked_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Blocked)
            .collect();
        assert_eq!(blocked_events.len(), 1);

        let unblocked_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Unblocked)
            .collect();
        assert_eq!(unblocked_events.len(), 1);
    }

    #[test]
    fn block_on_another_task() {
        let task1 = make_task(1);
        let mut task2 = make_task(2);

        task2.block(
            "Depends on task 1 completion",
            "agent-1",
            Some(task1.id.clone()),
        );

        let block_info = task2.blocked.as_ref().unwrap();
        assert_eq!(block_info.on_task, Some(task1.id));
    }

    #[test]
    fn is_ready_for_agent() {
        let mut task = make_task(1);
        let statuses = HashMap::new();

        // Task is ready for any agent initially
        assert!(task.is_ready_for_agent(&statuses, None));
        assert!(task.is_ready_for_agent(&statuses, Some("agent-1")));

        // Claim the task
        task.claim("agent-1");

        // Not ready for other agents
        assert!(!task.is_ready_for_agent(&statuses, None));
        assert!(!task.is_ready_for_agent(&statuses, Some("agent-2")));

        // Still ready for the claiming agent
        assert!(task.is_ready_for_agent(&statuses, Some("agent-1")));

        // Block the task
        task.block("Blocked", "agent-1", None);

        // Not ready for anyone when explicitly blocked
        assert!(!task.is_ready_for_agent(&statuses, Some("agent-1")));
    }

    #[test]
    fn handoff_creates_note_and_unclaims() {
        let mut task = make_task(1);
        task.claim("agent-1");

        assert!(task.is_claimed());
        assert!(task.notes.is_empty());

        // Handoff to another agent
        task.handoff("Need human review", "agent-1", Some("human".to_string()));

        // Task should be unclaimed
        assert!(!task.is_claimed());

        // Should have assigned_to set
        assert_eq!(task.assigned_to, Some("human".to_string()));

        // Should have a handoff note
        assert_eq!(task.notes.len(), 1);
        assert!(task.notes[0].text.contains("Handoff: Need human review"));

        // Check history has handoff event
        let handoff_events: Vec<_> = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Handoff)
            .collect();
        assert_eq!(handoff_events.len(), 1);
    }

    #[test]
    fn new_task_has_created_history_event() {
        let task = make_task(1);

        assert!(!task.history.is_empty());
        assert_eq!(task.history[0].event, HistoryEventType::Created);
    }

    #[test]
    fn reclaim_does_not_duplicate_start_event() {
        let mut task = make_task(1);

        // First claim starts the task
        task.claim("agent-1");
        let start_count_1 = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Started)
            .count();
        assert_eq!(start_count_1, 1);

        // Unclaim
        task.unclaim(Some("agent-1"));

        // Re-claim should not add another start event (task is already in progress)
        task.claim("agent-1");
        let start_count_2 = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Started)
            .count();
        assert_eq!(start_count_2, 1); // Still just 1

        // But should have 2 claim events
        let claim_count = task
            .history
            .iter()
            .filter(|e| e.event == HistoryEventType::Claimed)
            .count();
        assert_eq!(claim_count, 2);
    }

    #[test]
    fn agent_fields_serde_roundtrip() {
        let mut task = make_task(1);
        task.claim("agent-1");
        task.add_note("agent-1", "Test note");
        task.add_link(LinkType::Commit, "abc123", Some("agent-1"));
        task.block("Test block", "agent-1", None);

        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.claimed_by, task.claimed_by);
        assert_eq!(parsed.notes.len(), task.notes.len());
        assert_eq!(parsed.links.len(), task.links.len());
        assert!(parsed.blocked.is_some());
        assert!(!parsed.history.is_empty());
    }
}
