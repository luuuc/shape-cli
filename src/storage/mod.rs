//! # Storage Layer
//!
//! Persistence layer for Shape CLI with git-friendly file formats.
//!
//! ## Storage Formats
//!
//! | Data | Format | Location |
//! |------|--------|----------|
//! | Briefs | Markdown + YAML frontmatter | `.shape/briefs/{id}.md` |
//! | Tasks | JSONL (one JSON per line) | `.shape/tasks.jsonl` |
//! | Config | TOML | `.shape/config.toml` |
//! | Index | JSONL (auto-regenerated) | `.shape/briefs/index.jsonl` |
//!
//! ## Concurrency Safety
//!
//! - [`TaskStore`] uses file locking (`fs2`) for concurrent access
//! - [`BriefStore`] uses mtime-based index invalidation
//! - All writes are atomic (temp file + rename)
//!
//! ## Project Structure
//!
//! ```text
//! .shape/
//! ├── briefs/
//! │   ├── b-1234567.md      # Brief markdown files
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
//! - [`BriefStore`] - Read/write briefs as markdown files
//! - [`TaskStore`] - Read/write tasks as JSONL
//! - [`Config`] - Project and global configuration

mod cache;
mod config;
mod jsonl;
mod markdown;
mod project;

pub use cache::{Cache, CacheError, CachedBrief, CachedTask, SearchResult, SearchResultType};
pub use config::{CompactionConfig, CompactionStrategy, Config, ConfigError, DaemonConfig};
pub use jsonl::TaskStore;
pub use markdown::BriefStore;
pub use project::{Project, ProjectError};
