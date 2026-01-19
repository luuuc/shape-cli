//! Sync plugin interface
//!
//! Sync plugins handle bidirectional synchronization with external tools.
//! Operations: push, pull, test

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::loader::PluginLoader;
use super::protocol::PluginRequest;

/// ID mapping between local and remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdMapping {
    /// Local ID (brief or task)
    pub local_id: String,

    /// Remote ID (e.g., GitHub issue number)
    pub remote_id: String,

    /// Type of entity (brief or task)
    pub entity_type: EntityType,

    /// Last sync timestamp
    pub last_sync: DateTime<Utc>,
}

/// Type of entity being synced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    #[serde(alias = "anchor")]
    Brief,
    Task,
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of items pushed
    pub pushed: u32,

    /// Number of items pulled
    pub pulled: u32,

    /// Number of conflicts (resolved by last-write-wins)
    pub conflicts: u32,

    /// Errors encountered
    pub errors: Vec<String>,
}

/// Sync operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOperation {
    Push,
    Pull,
    Both,
}

/// Sync plugin wrapper
pub struct SyncPlugin<'a> {
    loader: &'a PluginLoader,
    plugin_name: String,
    mapping_store: MappingStore,
}

impl<'a> SyncPlugin<'a> {
    /// Creates a new sync plugin wrapper
    pub fn new(loader: &'a PluginLoader, plugin_name: impl Into<String>, sync_dir: &Path) -> Self {
        let plugin_name = plugin_name.into();
        let mapping_store = MappingStore::new(sync_dir.join(format!("{}.jsonl", plugin_name)));

        Self {
            loader,
            plugin_name,
            mapping_store,
        }
    }

    /// Tests the connection to the external service
    pub fn test(&self) -> Result<bool> {
        let request = PluginRequest::new("test", serde_json::json!({}));
        let response = self.loader.execute(&self.plugin_name, &request)?;
        Ok(response.success)
    }

    /// Pushes local changes to remote
    pub fn push(
        &self,
        briefs: &[serde_json::Value],
        tasks: &[serde_json::Value],
    ) -> Result<SyncResult> {
        let mappings = self.mapping_store.read_all()?;

        let request = PluginRequest::new(
            "push",
            serde_json::json!({
                "briefs": briefs,
                "tasks": tasks,
                "mappings": mappings.values().collect::<Vec<_>>(),
            }),
        );

        let response = self.loader.execute(&self.plugin_name, &request)?;

        if !response.success {
            anyhow::bail!(
                "Push failed: {}",
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No push result returned"))?;

        // Update mappings from response
        if let Some(new_mappings) = data.get("mappings") {
            let mappings: Vec<IdMapping> =
                serde_json::from_value(new_mappings.clone()).context("Failed to parse mappings")?;
            self.mapping_store.write_all(&mappings)?;
        }

        let result: SyncResult =
            serde_json::from_value(data).context("Failed to parse sync result")?;

        Ok(result)
    }

    /// Pulls remote changes to local
    pub fn pull(&self) -> Result<(SyncResult, Vec<serde_json::Value>, Vec<serde_json::Value>)> {
        let mappings = self.mapping_store.read_all()?;

        let request = PluginRequest::new(
            "pull",
            serde_json::json!({
                "mappings": mappings.values().collect::<Vec<_>>(),
            }),
        );

        let response = self.loader.execute(&self.plugin_name, &request)?;

        if !response.success {
            anyhow::bail!(
                "Pull failed: {}",
                response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string())
            );
        }

        let data = response
            .data
            .ok_or_else(|| anyhow::anyhow!("No pull result returned"))?;

        // Update mappings from response
        if let Some(new_mappings) = data.get("mappings") {
            let mappings: Vec<IdMapping> =
                serde_json::from_value(new_mappings.clone()).context("Failed to parse mappings")?;
            self.mapping_store.write_all(&mappings)?;
        }

        let briefs: Vec<serde_json::Value> = data
            .get("briefs")
            .cloned()
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();

        let tasks: Vec<serde_json::Value> = data
            .get("tasks")
            .cloned()
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();

        let result: SyncResult =
            serde_json::from_value(data).context("Failed to parse sync result")?;

        Ok((result, briefs, tasks))
    }

    /// Gets the sync status
    pub fn status(&self) -> Result<SyncStatus> {
        let mappings = self.mapping_store.read_all()?;

        Ok(SyncStatus {
            plugin: self.plugin_name.clone(),
            mapped_briefs: mappings
                .values()
                .filter(|m| m.entity_type == EntityType::Brief)
                .count(),
            mapped_tasks: mappings
                .values()
                .filter(|m| m.entity_type == EntityType::Task)
                .count(),
            last_sync: mappings.values().map(|m| m.last_sync).max(),
        })
    }

    /// Links a local ID to a remote ID
    pub fn link(&self, local_id: &str, remote_id: &str, entity_type: EntityType) -> Result<()> {
        let mapping = IdMapping {
            local_id: local_id.to_string(),
            remote_id: remote_id.to_string(),
            entity_type,
            last_sync: Utc::now(),
        };

        let mut mappings = self.mapping_store.read_all()?;
        mappings.insert(local_id.to_string(), mapping);
        self.mapping_store
            .write_all(&mappings.into_values().collect::<Vec<_>>())?;

        Ok(())
    }

    /// Unlinks a local ID
    pub fn unlink(&self, local_id: &str) -> Result<bool> {
        let mut mappings = self.mapping_store.read_all()?;
        let removed = mappings.remove(local_id).is_some();
        if removed {
            self.mapping_store
                .write_all(&mappings.into_values().collect::<Vec<_>>())?;
        }
        Ok(removed)
    }
}

/// Sync status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Plugin name
    pub plugin: String,

    /// Number of briefs with mappings
    pub mapped_briefs: usize,

    /// Number of tasks with mappings
    pub mapped_tasks: usize,

    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
}

/// Storage for ID mappings
struct MappingStore {
    path: PathBuf,
}

impl MappingStore {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn read_all(&self) -> Result<HashMap<String, IdMapping>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open mapping file: {}", self.path.display()))?;

        let reader = BufReader::new(file);
        let mut mappings = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {}", line_num + 1))?;

            if line.trim().is_empty() {
                continue;
            }

            let mapping: IdMapping = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse mapping at line {}", line_num + 1))?;

            mappings.insert(mapping.local_id.clone(), mapping);
        }

        Ok(mappings)
    }

    fn write_all(&self, mappings: &[IdMapping]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let file = File::create(&self.path)
            .with_context(|| format!("Failed to create mapping file: {}", self.path.display()))?;

        let mut writer = BufWriter::new(file);

        for mapping in mappings {
            let line = serde_json::to_string(mapping).context("Failed to serialize mapping")?;
            writeln!(writer, "{}", line).context("Failed to write mapping")?;
        }

        writer.flush().context("Failed to flush mapping file")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn mapping_store_empty() {
        let dir = TempDir::new().unwrap();
        let store = MappingStore::new(dir.path().join("mappings.jsonl"));

        let mappings = store.read_all().unwrap();
        assert!(mappings.is_empty());
    }

    #[test]
    fn mapping_store_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = MappingStore::new(dir.path().join("mappings.jsonl"));

        let mappings = vec![
            IdMapping {
                local_id: "a-1234567".to_string(),
                remote_id: "123".to_string(),
                entity_type: EntityType::Brief,
                last_sync: Utc::now(),
            },
            IdMapping {
                local_id: "a-1234567.1".to_string(),
                remote_id: "456".to_string(),
                entity_type: EntityType::Task,
                last_sync: Utc::now(),
            },
        ];

        store.write_all(&mappings).unwrap();

        let loaded = store.read_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains_key("a-1234567"));
        assert!(loaded.contains_key("a-1234567.1"));
    }

    #[test]
    fn id_mapping_serialization() {
        let mapping = IdMapping {
            local_id: "a-1234567.1".to_string(),
            remote_id: "42".to_string(),
            entity_type: EntityType::Task,
            last_sync: Utc::now(),
        };

        let json = serde_json::to_string(&mapping).unwrap();
        let parsed: IdMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.local_id, mapping.local_id);
        assert_eq!(parsed.remote_id, mapping.remote_id);
        assert_eq!(parsed.entity_type, EntityType::Task);
    }
}
