//! Task domain model
//!
//! Tasks are the executable units of work within an anchor.
//! They can have dependencies on other tasks and support subtasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::id::{AnchorId, TaskId};

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
}

/// A task within an anchor
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier
    pub id: TaskId,

    /// Human-readable title
    pub title: String,

    /// Current status
    pub status: TaskStatus,

    /// IDs of tasks this task depends on (blocked by)
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub depends_on: HashSet<TaskId>,

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
}

impl Task {
    /// Creates a new task with the given ID and title
    pub fn new(id: TaskId, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: title.into(),
            status: TaskStatus::Todo,
            depends_on: HashSet::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            description: None,
            meta: TaskMeta::new(),
        }
    }

    /// Returns the anchor ID this task belongs to, or None if standalone
    pub fn anchor_id(&self) -> Option<AnchorId> {
        self.id.anchor_id()
    }

    /// Returns true if this is a standalone task (not belonging to an anchor)
    pub fn is_standalone(&self) -> bool {
        self.id.is_standalone()
    }

    /// Returns true if this task has no incomplete dependencies
    pub fn is_ready(&self, task_statuses: &HashMap<TaskId, TaskStatus>) -> bool {
        if self.status.is_complete() {
            return false; // Completed tasks are not "ready"
        }

        self.depends_on.iter().all(|dep_id| {
            task_statuses
                .get(dep_id)
                .map(|s| s.is_complete())
                .unwrap_or(false)
        })
    }

    /// Returns true if this task is blocked by incomplete dependencies
    pub fn is_blocked(&self, task_statuses: &HashMap<TaskId, TaskStatus>) -> bool {
        if self.status.is_complete() {
            return false; // Completed tasks are not "blocked"
        }

        self.depends_on.iter().any(|dep_id| {
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
        }
    }

    /// Transitions to done status
    pub fn complete(&mut self) {
        if !self.status.is_complete() {
            self.status = TaskStatus::Done;
            let now = Utc::now();
            self.updated_at = now;
            self.completed_at = Some(now);
        }
    }

    /// Transitions back to todo status
    pub fn reopen(&mut self) {
        if self.status.is_complete() {
            self.status = TaskStatus::Todo;
            self.updated_at = Utc::now();
            self.completed_at = None;
        }
    }

    /// Adds a dependency on another task
    pub fn add_dependency(&mut self, task_id: TaskId) {
        if self.depends_on.insert(task_id) {
            self.updated_at = Utc::now();
        }
    }

    /// Removes a dependency
    pub fn remove_dependency(&mut self, task_id: &TaskId) {
        if self.depends_on.remove(task_id) {
            self.updated_at = Utc::now();
        }
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

    /// Sets the description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = Some(description.into());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(seq: u32) -> Task {
        let anchor = AnchorId::new("Test", Utc::now());
        let task_id = TaskId::new(&anchor, seq);
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
    fn task_belongs_to_anchor() {
        let anchor = AnchorId::new("Test", Utc::now());
        let task_id = TaskId::new(&anchor, 1);
        let task = Task::new(task_id, "Task 1");

        assert_eq!(task.anchor_id(), Some(anchor));
        assert!(!task.is_standalone());
    }

    #[test]
    fn standalone_task_has_no_anchor() {
        let task_id = TaskId::new_standalone("Standalone task", Utc::now());
        let task = Task::new(task_id, "Standalone task");

        assert!(task.anchor_id().is_none());
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
    fn updated_at_changes_on_modifications() {
        let mut task = make_task(1);
        let created = task.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        task.start();

        assert!(task.updated_at > created);
    }
}
