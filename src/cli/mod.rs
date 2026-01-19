//! # Command-Line Interface
//!
//! User-facing CLI commands and output formatting.
//!
//! ## Command Groups
//!
//! | Group | Purpose | Examples |
//! |-------|---------|----------|
//! | Core | Project management | `init`, `status` |
//! | Anchor | Document lifecycle | `anchor new`, `anchor list`, `anchor show` |
//! | Task | Work item management | `task add`, `task start`, `task done` |
//! | Query | Task state queries | `ready`, `blocked` |
//! | Context | AI integration | `context`, `context --compact` |
//! | Advanced | Plugins and sync | `plugin list`, `sync run` |
//!
//! ## Output Formats
//!
//! All commands support `--format` flag:
//! - `text` (default) - Human-readable output
//! - `json` - Machine-parseable JSON
//!
//! ## Verbose Mode
//!
//! Use `--verbose` (or `-v`) for debug output:
//! ```bash
//! shape --verbose ready
//! ```
//!
//! ## Entry Point
//!
//! Call [`run()`] to parse arguments and execute the appropriate command.

mod app;
mod output;
mod anchor;
mod task;
mod query;
mod context;
mod plugin_cmd;
mod sync_cmd;
mod agent_setup;
mod cache_cmd;
mod merge_driver;

pub use app::{Cli, Commands, run};
pub use output::{Output, OutputFormat};
