//! Domain models for Shape CLI
//!
//! Contains the core business logic without any I/O concerns.

mod id;
mod task;
mod anchor;
mod graph;

pub use id::{AnchorId, TaskId, IdError};
pub use task::{Task, TaskStatus, TaskMeta};
pub use anchor::{Anchor, AnchorStatus, AnchorMeta, AnchorFrontmatter};
pub use graph::{DependencyGraph, GraphError};
