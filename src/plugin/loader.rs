//! Plugin discovery and loading
//!
//! Plugins are discovered from:
//! 1. PATH (executables starting with "shape-")
//! 2. `.shape/plugins/` directory

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::protocol::{PluginManifest, PluginRequest, PluginResponse, PluginType};

/// Information about a discovered plugin
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,

    /// Path to the plugin executable
    pub path: PathBuf,

    /// Plugin manifest (loaded on demand)
    pub manifest: Option<PluginManifest>,
}

/// Plugin loader and executor
pub struct PluginLoader {
    /// Discovered plugins
    plugins: HashMap<String, PluginInfo>,

    /// Additional plugin directories
    plugin_dirs: Vec<PathBuf>,
}

impl PluginLoader {
    /// Creates a new plugin loader
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dirs: Vec::new(),
        }
    }

    /// Adds a plugin directory to search
    pub fn add_plugin_dir(&mut self, dir: impl Into<PathBuf>) {
        self.plugin_dirs.push(dir.into());
    }

    /// Discovers all available plugins
    pub fn discover(&mut self) -> Result<()> {
        self.plugins.clear();

        // Search PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                self.scan_directory(&dir)?;
            }
        }

        // Search plugin directories
        for dir in &self.plugin_dirs.clone() {
            self.scan_directory(dir)?;
        }

        Ok(())
    }

    /// Scans a directory for plugins
    fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()), // Ignore unreadable directories
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Check if it's an executable starting with "shape-"
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("shape-") && self.is_executable(&path) {
                    let plugin_name = name.to_string();

                    // Don't override existing plugins (first found wins)
                    if !self.plugins.contains_key(&plugin_name) {
                        self.plugins.insert(
                            plugin_name.clone(),
                            PluginInfo {
                                name: plugin_name,
                                path,
                                manifest: None,
                            },
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Checks if a file is executable
    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = path.metadata() {
                return meta.permissions().mode() & 0o111 != 0;
            }
        }

        #[cfg(windows)]
        {
            if let Some(ext) = path.extension() {
                return ext == "exe" || ext == "bat" || ext == "cmd";
            }
        }

        false
    }

    /// Lists all discovered plugins
    pub fn list(&self) -> Vec<&PluginInfo> {
        self.plugins.values().collect()
    }

    /// Gets a plugin by name
    pub fn get(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// Gets the manifest for a plugin (loads if needed)
    pub fn get_manifest(&mut self, name: &str) -> Result<Option<PluginManifest>> {
        if let Some(info) = self.plugins.get_mut(name) {
            if info.manifest.is_none() {
                info.manifest = Some(Self::load_manifest(&info.path)?);
            }
            Ok(info.manifest.clone())
        } else {
            Ok(None)
        }
    }

    /// Loads the manifest from a plugin
    fn load_manifest(path: &Path) -> Result<PluginManifest> {
        let output = Command::new(path)
            .arg("--manifest")
            .output()
            .with_context(|| format!("Failed to execute plugin: {}", path.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Plugin returned error: {}", stderr);
        }

        let manifest: PluginManifest = serde_json::from_slice(&output.stdout)
            .with_context(|| "Failed to parse plugin manifest")?;

        Ok(manifest)
    }

    /// Executes a plugin request
    pub fn execute(&self, name: &str, request: &PluginRequest) -> Result<PluginResponse> {
        let info = self
            .plugins
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;

        let mut child = Command::new(&info.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn plugin: {}", info.path.display()))?;

        // Send request
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        let request_json = serde_json::to_string(request).context("Failed to serialize request")?;
        writeln!(stdin, "{}", request_json).context("Failed to write to plugin")?;

        // Read response
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let reader = BufReader::new(stdout);

        let response_line = reader
            .lines()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No response from plugin"))?
            .context("Failed to read plugin response")?;

        let response: PluginResponse =
            serde_json::from_str(&response_line).context("Failed to parse plugin response")?;

        // Wait for child to exit
        let _ = child.wait();

        Ok(response)
    }

    /// Tests plugin connectivity
    pub fn test(&self, name: &str) -> Result<bool> {
        let request = PluginRequest::new("test", serde_json::json!({}));
        let response = self.execute(name, &request)?;
        Ok(response.success)
    }

    /// Lists plugins by type
    pub fn list_by_type(&mut self, plugin_type: PluginType) -> Result<Vec<&PluginInfo>> {
        let mut result = Vec::new();

        for (_name, info) in &mut self.plugins {
            if info.manifest.is_none() {
                if let Ok(manifest) = Self::load_manifest(&info.path) {
                    info.manifest = Some(manifest);
                }
            }

            if let Some(ref manifest) = info.manifest {
                if manifest.plugin_type == plugin_type {
                    result.push(info as &PluginInfo);
                }
            }
        }

        Ok(result)
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_loader_is_empty() {
        let loader = PluginLoader::new();
        assert!(loader.list().is_empty());
    }

    #[test]
    fn add_plugin_dir() {
        let mut loader = PluginLoader::new();
        loader.add_plugin_dir("/some/path");

        assert_eq!(loader.plugin_dirs.len(), 1);
    }

    #[test]
    fn discover_empty_dir() {
        let dir = TempDir::new().unwrap();
        let mut loader = PluginLoader::new();
        loader.add_plugin_dir(dir.path());
        loader.discover().unwrap();

        assert!(loader.list().is_empty());
    }

    #[test]
    fn get_nonexistent_plugin() {
        let loader = PluginLoader::new();
        assert!(loader.get("nonexistent").is_none());
    }

    // Note: More comprehensive tests would require creating actual plugin executables
    // which is complex for unit tests. Integration tests should cover this.
}
