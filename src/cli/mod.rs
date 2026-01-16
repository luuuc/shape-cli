//! CLI module for Shape
//!
//! Provides all command-line interface functionality.

mod app;
mod output;
mod anchor;
mod task;
mod query;
mod context;
mod plugin_cmd;
mod sync_cmd;

pub use app::{Cli, Commands, run};
pub use output::{Output, OutputFormat};
