//! # Domain Models
//!
//! Core business logic for Shape CLI, free of I/O concerns.
//!
//! ## Key Types
//!
//! - [`Brief`] - A document (pitch, RFC, PRD) that spawns tasks
//! - [`Task`] - An executable unit of work belonging to a brief
//! - [`BriefId`] / [`TaskId`] - Unique identifiers with format `b-{hash}` and `b-{hash}.{seq}`
//! - [`DependencyGraph`] - DAG of task dependencies with cycle detection
//!
//! ## Status Lifecycles
//!
//! **Briefs**: `Proposed` → `Betting` → `InProgress` → `Shipped` | `Archived`
//!
//! **Tasks**: `Todo` → `InProgress` → `Done`
//!
//! ## Invariants
//!
//! - The [`DependencyGraph`] is always acyclic (DAG)
//! - Task IDs are hierarchical: `{brief-id}.{sequence}` (e.g., `b-7f2b4c1.1`)
//! - All timestamps are UTC
//!
//! ## Example
//!
//! ```
//! use shape_cli::domain::{Brief, Task, TaskId, DependencyGraph, TaskStatus};
//! use std::collections::HashMap;
//!
//! // Create a brief and tasks
//! let brief = Brief::new("My Pitch", "minimal");
//! let task1 = Task::new(TaskId::new(&brief.id, 1), "First task");
//! let task2 = Task::new(TaskId::new(&brief.id, 2), "Second task");
//!
//! // Build a dependency graph
//! let mut graph = DependencyGraph::new();
//! graph.add_task(task1.id.clone());
//! graph.add_task(task2.id.clone());
//! graph.add_dependency(&task2.id, &task1.id).unwrap(); // task2 depends on task1
//!
//! // Query ready tasks
//! let mut statuses = HashMap::new();
//! statuses.insert(task1.id.clone(), TaskStatus::Todo);
//! statuses.insert(task2.id.clone(), TaskStatus::Todo);
//!
//! let ready = graph.ready_tasks(&statuses);
//! assert!(ready.contains(&task1.id)); // task1 is ready
//! assert!(!ready.contains(&task2.id)); // task2 is blocked
//! ```

mod brief;
mod graph;
mod id;
mod merge;
mod task;

pub use brief::{Brief, BriefFrontmatter, BriefMeta, BriefStatus};
pub use graph::{DependencyGraph, GraphError};
pub use id::{BriefId, IdError, TaskId};
pub use merge::{merge_tasks, MergeResult};
pub use task::{
    current_timestamp, BlockInfo, Dependencies, Dependency, DependencyType, FieldVersions,
    HistoryEvent, HistoryEventType, Link, LinkType, Note, Task, TaskMeta, TaskStatus,
};
