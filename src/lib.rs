//! Shape CLI - A local-first task management tool for software teams
//!
//! Shape organizes work around "anchors" (documents like pitches, RFCs, PRDs)
//! with dependent tasks. It provides AI-optimized context export and
//! plugin-based extensibility.

pub mod domain;
pub mod storage;
pub mod plugin;
pub mod cli;

pub use domain::{Anchor, AnchorId, AnchorStatus, Task, TaskId, TaskStatus};
