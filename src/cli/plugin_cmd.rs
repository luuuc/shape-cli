//! Plugin management commands

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::plugin::PluginLoader;
use crate::storage::Project;

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List available plugins
    List,

    /// Test plugin connectivity
    Test {
        /// Plugin name
        name: String,
    },
}

pub fn run(cmd: PluginCommands, output: &Output) -> Result<()> {
    match cmd {
        PluginCommands::List => list_plugins(output),
        PluginCommands::Test { name } => test_plugin(output, &name),
    }
}

fn list_plugins(output: &Output) -> Result<()> {
    let mut loader = PluginLoader::new();

    // Add project plugins directory if in a project
    if let Ok(project) = Project::open_current() {
        loader.add_plugin_dir(project.plugins_dir());
    }

    loader.discover()?;

    let plugins = loader.list();

    if output.is_json() {
        let items: Vec<_> = plugins
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "path": p.path.display().to_string(),
                })
            })
            .collect();
        output.data(&items);
    } else {
        if plugins.is_empty() {
            println!("No plugins found.");
            println!();
            println!("Plugins are discovered from:");
            println!("  - PATH (executables starting with 'shape-')");
            println!("  - .shape/plugins/ directory");
        } else {
            println!("Available plugins:");
            println!("{:<30} {}", "NAME", "PATH");
            println!("{}", "-".repeat(70));
            for plugin in plugins {
                println!("{:<30} {}", plugin.name, plugin.path.display());
            }
        }
    }

    Ok(())
}

fn test_plugin(output: &Output, name: &str) -> Result<()> {
    let mut loader = PluginLoader::new();

    if let Ok(project) = Project::open_current() {
        loader.add_plugin_dir(project.plugins_dir());
    }

    loader.discover()?;

    // First check if plugin exists
    if loader.get(name).is_none() {
        anyhow::bail!("Plugin not found: {}", name);
    }

    // Try to get manifest
    let manifest = loader.get_manifest(name)?;

    // Test connectivity
    let test_result = loader.test(name);

    if output.is_json() {
        output.data(&serde_json::json!({
            "name": name,
            "manifest": manifest,
            "test_success": test_result.as_ref().ok().copied().unwrap_or(false),
            "test_error": test_result.as_ref().err().map(|e| e.to_string()),
        }));
    } else {
        if let Some(manifest) = manifest {
            println!("Plugin: {}", manifest.name);
            println!("Version: {}", manifest.version);
            println!("Description: {}", manifest.description);
            println!("Type: {:?}", manifest.plugin_type);
            println!("Operations: {}", manifest.operations.join(", "));
            println!();
        }

        match test_result {
            Ok(true) => output.success(&format!("Plugin '{}' is working correctly", name)),
            Ok(false) => output.error(&format!("Plugin '{}' test returned false", name)),
            Err(e) => output.error(&format!("Plugin '{}' test failed: {}", name, e)),
        }
    }

    Ok(())
}
