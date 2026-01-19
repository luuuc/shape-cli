//! # Plugin System
//!
//! Extensibility layer for custom anchor types and external tool sync.
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
//! | Anchor Type | `shape-anchor-{name}` | Custom document templates and validation |
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
//! ## Built-in Anchor Types
//!
//! - `minimal` - Basic title and status (default)
//! - `shapeup` - ShapeUp methodology: appetite, rabbit holes, no-gos
//!
//! ## Key Types
//!
//! - [`PluginLoader`] - Discovers and executes plugins
//! - [`PluginManifest`] - Declares plugin capabilities
//! - [`AnchorTypePlugin`] - Trait for anchor type plugins
//! - [`SyncPlugin`] - Trait for sync plugins

mod anchor_type;
mod loader;
mod protocol;
mod shapeup;
mod sync;

pub use anchor_type::{AnchorTemplate, AnchorTypePlugin, MinimalAnchorType};
pub use loader::{PluginInfo, PluginLoader};
pub use protocol::{PluginManifest, PluginMessage, PluginRequest, PluginResponse};
pub use shapeup::ShapeUpAnchorType;
pub use sync::{EntityType, IdMapping, SyncOperation, SyncPlugin, SyncResult};
