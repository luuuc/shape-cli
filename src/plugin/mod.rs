//! # Plugin System
//!
//! Extensibility layer for custom brief types and external tool sync.
//!
//! ## Overview
//!
//! Plugins are separate binaries that communicate with Shape via JSON over stdin/stdout.
//! This makes plugins language-agnostic — any language can implement a plugin.
//!
//! ## Plugin Types
//!
//! | Type | Binary Pattern | Purpose |
//! |------|----------------|---------|
//! | Brief Type | `shape-brief-{name}` | Custom document templates and validation |
//! | Sync | `shape-sync-{name}` | Bidirectional sync with external tools |
//!
//! ## Plugin Discovery
//!
//! Plugins are discovered in two locations:
//! 1. `.shape/plugins/` - Project-local plugins
//! 2. `$PATH` - System-wide plugins
//!
//! ## Protocol
//!
//! ```text
//! CLI                          Plugin Binary
//!  │                               │
//!  ├── Spawn: shape-sync-github    │
//!  │                               │
//!  ├── Stdin: {"operation": "sync", "params": {...}}
//!  │                               │
//!  └── Stdout: {"success": true, "data": {...}}
//! ```
//!
//! Every plugin must support `--manifest` to declare its capabilities.
//!
//! ## Built-in Brief Types
//!
//! - `minimal` - Basic title and status (default)
//! - `shapeup` - ShapeUp methodology: appetite, rabbit holes, no-gos
//!
//! ## Key Types
//!
//! - [`PluginLoader`] - Discovers and executes plugins
//! - [`PluginManifest`] - Declares plugin capabilities
//! - [`BriefTypePlugin`] - Trait for brief type plugins
//! - [`SyncPlugin`] - Trait for sync plugins

mod brief_type;
mod loader;
mod protocol;
mod shapeup;
mod sync;

pub use brief_type::{BriefTemplate, BriefTypePlugin, MinimalBriefType};
pub use loader::{PluginInfo, PluginLoader};
pub use protocol::{PluginManifest, PluginMessage, PluginRequest, PluginResponse};
pub use shapeup::ShapeUpBriefType;
pub use sync::{EntityType, IdMapping, SyncOperation, SyncPlugin, SyncResult};
