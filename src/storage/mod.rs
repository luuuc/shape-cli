//! Storage layer for Shape CLI
//!
//! Handles persistence of tasks (JSONL) and anchors (markdown).

mod jsonl;
mod markdown;
mod config;
mod project;

pub use jsonl::TaskStore;
pub use markdown::AnchorStore;
pub use config::{Config, ConfigError};
pub use project::{Project, ProjectError};
