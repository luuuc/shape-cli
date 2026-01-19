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

mod agent_setup;
mod anchor;
mod app;
mod cache_cmd;
mod compact;
mod context;
mod daemon;
mod merge_driver;
mod output;
mod plugin_cmd;
mod query;
mod sync_cmd;
mod task;
mod tui;

pub use app::{run, Cli, Commands};
pub use output::{Output, OutputFormat};
