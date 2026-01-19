//! # Domain Models
//!
//! Core business logic for Shape CLI, free of I/O concerns.
//!
//! ## Key Types
//!
//! - [`Anchor`] - A document (pitch, RFC, PRD) that spawns tasks
//! - [`Task`] - An executable unit of work belonging to an anchor
//! - [`AnchorId`] / [`TaskId`] - Unique identifiers with format `a-{hash}` and `a-{hash}.{seq}`
//! - [`DependencyGraph`] - DAG of task dependencies with cycle detection
//!
//! ## Status Lifecycles
//!
//! **Anchors**: `Proposed` → `Betting` → `InProgress` → `Shipped` | `Archived`
//!
//! **Tasks**: `Todo` → `InProgress` → `Done`
//!
//! ## Invariants
//!
//! - The [`DependencyGraph`] is always acyclic (DAG)
//! - Task IDs are hierarchical: `{anchor-id}.{sequence}` (e.g., `a-7f2b4c1.1`)
//! - All timestamps are UTC
//!
//! ## Example
//!
//! ```
//! use shape_cli::domain::{Anchor, Task, TaskId, DependencyGraph, TaskStatus};
//! use std::collections::HashMap;
//!
//! // Create an anchor and tasks
//! let anchor = Anchor::new("My Pitch", "minimal");
//! let task1 = Task::new(TaskId::new(&anchor.id, 1), "First task");
//! let task2 = Task::new(TaskId::new(&anchor.id, 2), "Second task");
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

mod id;
mod task;
mod anchor;
mod graph;
mod merge;

pub use id::{AnchorId, TaskId, IdError};
pub use task::{Task, TaskStatus, TaskMeta, FieldVersions, current_timestamp};
pub use anchor::{Anchor, AnchorStatus, AnchorMeta, AnchorFrontmatter};
pub use graph::{DependencyGraph, GraphError};
pub use merge::{merge_tasks, MergeResult};
