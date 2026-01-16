//! Sync commands

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::plugin::{EntityType, PluginLoader, SyncPlugin};
use crate::storage::Project;

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Sync with an external tool
    #[command(name = "run")]
    Run {
        /// Plugin name (e.g., "github")
        plugin: String,
    },

    /// Show sync status
    Status,

    /// Link a local ID to a remote ID
    Link {
        /// Local ID (anchor or task)
        local: String,

        /// Remote ID
        remote: String,

        /// Plugin name
        #[arg(long)]
        plugin: String,
    },
}

pub fn run(cmd: SyncCommands, output: &Output) -> Result<()> {
    match cmd {
        SyncCommands::Run { plugin } => run_sync(output, &plugin),
        SyncCommands::Status => sync_status(output),
        SyncCommands::Link { local, remote, plugin } => link_ids(output, &local, &remote, &plugin),
    }
}

fn run_sync(output: &Output, plugin_name: &str) -> Result<()> {
    let project = Project::open_current()?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(project.plugins_dir());
    loader.discover()?;

    // Prepend "shape-sync-" if needed
    let full_name = if plugin_name.starts_with("shape-sync-") {
        plugin_name.to_string()
    } else {
        format!("shape-sync-{}", plugin_name)
    };

    if loader.get(&full_name).is_none() {
        anyhow::bail!("Sync plugin not found: {}. Install it or check 'shape plugin list'.", full_name);
    }

    let sync = SyncPlugin::new(&loader, &full_name, &project.sync_dir());

    // Test connection first
    if !sync.test()? {
        anyhow::bail!("Plugin connection test failed. Check your credentials.");
    }

    // Get current anchors and tasks
    let anchor_store = project.anchor_store();
    let task_store = project.task_store();

    let anchors = anchor_store.read_all()?;
    let tasks = task_store.read_all()?;

    // Convert to JSON values for the plugin
    let anchor_values: Vec<_> = anchors
        .values()
        .map(|a| serde_json::to_value(a).unwrap())
        .collect();

    let task_values: Vec<_> = tasks
        .values()
        .map(|t| serde_json::to_value(t).unwrap())
        .collect();

    // Push local changes
    let push_result = sync.push(&anchor_values, &task_values)?;

    // Pull remote changes
    let (pull_result, _pulled_anchors, _pulled_tasks) = sync.pull()?;

    // TODO: Apply pulled changes to local storage
    // This would require merging logic that respects last-write-wins

    if output.is_json() {
        output.data(&serde_json::json!({
            "plugin": full_name,
            "push": {
                "pushed": push_result.pushed,
                "conflicts": push_result.conflicts,
                "errors": push_result.errors,
            },
            "pull": {
                "pulled": pull_result.pulled,
                "conflicts": pull_result.conflicts,
                "errors": pull_result.errors,
            },
        }));
    } else {
        println!("Sync with {} complete", plugin_name);
        println!();
        println!("Push: {} items pushed, {} conflicts", push_result.pushed, push_result.conflicts);
        println!("Pull: {} items pulled, {} conflicts", pull_result.pulled, pull_result.conflicts);

        if !push_result.errors.is_empty() || !pull_result.errors.is_empty() {
            println!();
            println!("Errors:");
            for err in push_result.errors {
                println!("  [push] {}", err);
            }
            for err in pull_result.errors {
                println!("  [pull] {}", err);
            }
        }
    }

    Ok(())
}

fn sync_status(output: &Output) -> Result<()> {
    let project = Project::open_current()?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(project.plugins_dir());
    loader.discover()?;

    // Find all sync plugins
    let sync_plugins: Vec<_> = loader
        .list()
        .iter()
        .filter(|p| p.name.starts_with("shape-sync-"))
        .map(|p| p.name.clone())
        .collect();

    if sync_plugins.is_empty() {
        if output.is_json() {
            output.data(&serde_json::json!({
                "plugins": [],
            }));
        } else {
            println!("No sync plugins installed.");
        }
        return Ok(());
    }

    let mut statuses = Vec::new();

    for plugin_name in &sync_plugins {
        let sync = SyncPlugin::new(&loader, plugin_name, &project.sync_dir());
        if let Ok(status) = sync.status() {
            statuses.push(status);
        }
    }

    if output.is_json() {
        output.data(&serde_json::json!({
            "plugins": statuses,
        }));
    } else {
        println!("Sync Status:");
        println!("{:<25} {:<10} {:<10} {}", "PLUGIN", "ANCHORS", "TASKS", "LAST SYNC");
        println!("{}", "-".repeat(70));

        for status in statuses {
            let last_sync = status
                .last_sync
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "never".to_string());

            println!(
                "{:<25} {:<10} {:<10} {}",
                status.plugin,
                status.mapped_anchors,
                status.mapped_tasks,
                last_sync
            );
        }
    }

    Ok(())
}

fn link_ids(output: &Output, local: &str, remote: &str, plugin_name: &str) -> Result<()> {
    let project = Project::open_current()?;

    let mut loader = PluginLoader::new();
    loader.add_plugin_dir(project.plugins_dir());
    loader.discover()?;

    let full_name = if plugin_name.starts_with("shape-sync-") {
        plugin_name.to_string()
    } else {
        format!("shape-sync-{}", plugin_name)
    };

    let sync = SyncPlugin::new(&loader, &full_name, &project.sync_dir());

    // Determine entity type from ID format
    let entity_type = if local.contains('.') {
        EntityType::Task
    } else {
        EntityType::Anchor
    };

    sync.link(local, remote, entity_type)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "local": local,
            "remote": remote,
            "plugin": full_name,
            "entity_type": entity_type,
        }));
    } else {
        output.success(&format!("Linked {} to {} ({})", local, remote, full_name));
    }

    Ok(())
}
