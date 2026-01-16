//! Plugin system for Shape CLI
//!
//! Plugins communicate via JSON over stdin/stdout.
//! Two plugin types in v1:
//! - Anchor Types: Define custom anchor templates and validation
//! - Sync: Bidirectional sync with external tools

mod protocol;
mod loader;
mod anchor_type;
mod sync;
mod shapeup;

pub use protocol::{PluginManifest, PluginMessage, PluginRequest, PluginResponse};
pub use loader::{PluginLoader, PluginInfo};
pub use anchor_type::{AnchorTypePlugin, AnchorTemplate, MinimalAnchorType};
pub use sync::{SyncPlugin, SyncOperation, SyncResult, IdMapping, EntityType};
pub use shapeup::ShapeUpAnchorType;
