//! # Storage Layer
//!
//! Persistence layer for Shape CLI with git-friendly file formats.
//!
//! ## Storage Formats
//!
//! | Data | Format | Location |
//! |------|--------|----------|
//! | Anchors | Markdown + YAML frontmatter | `.shape/anchors/{id}.md` |
//! | Tasks | JSONL (one JSON per line) | `.shape/tasks.jsonl` |
//! | Config | TOML | `.shape/config.toml` |
//! | Index | JSONL (auto-regenerated) | `.shape/anchors/index.jsonl` |
//!
//! ## Concurrency Safety
//!
//! - [`TaskStore`] uses file locking (`fs2`) for concurrent access
//! - [`AnchorStore`] uses mtime-based index invalidation
//! - All writes are atomic (temp file + rename)
//!
//! ## Project Structure
//!
//! ```text
//! .shape/
//! ├── anchors/
//! │   ├── a-1234567.md      # Anchor markdown files
//! │   └── index.jsonl       # Fast query index (auto-generated)
//! ├── tasks.jsonl           # All tasks in JSONL format
//! ├── config.toml           # Project configuration
//! ├── plugins/              # Local plugins
//! ├── sync/                 # Sync state for external tools
//! └── .gitignore            # Ignores index and sync state
//! ```
//!
//! ## Key Types
//!
//! - [`Project`] - Entry point for accessing a Shape project
//! - [`AnchorStore`] - Read/write anchors as markdown files
//! - [`TaskStore`] - Read/write tasks as JSONL
//! - [`Config`] - Project and global configuration

mod jsonl;
mod markdown;
mod config;
mod project;
mod cache;

pub use jsonl::TaskStore;
pub use markdown::AnchorStore;
pub use config::{Config, ConfigError};
pub use project::{Project, ProjectError};
pub use cache::{Cache, CacheError, CachedAnchor, CachedTask, SearchResult, SearchResultType};
