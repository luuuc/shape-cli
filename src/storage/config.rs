//! Configuration handling for Shape CLI
//!
//! Configuration is stored in `.shape/config.toml` (project) and
//! `~/.config/shape/config.toml` (global).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Failed to parse configuration: {0}")]
    Parse(String),
}

/// Default brief type for new briefs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DefaultBriefType {
    #[default]
    Minimal,
    Custom(String),
}

impl DefaultBriefType {
    pub fn as_str(&self) -> &str {
        match self {
            DefaultBriefType::Minimal => "minimal",
            DefaultBriefType::Custom(s) => s,
        }
    }
}

/// Compaction strategy for summarizing completed tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Concatenate task titles
    #[default]
    Basic,
    /// Group related tasks by common words
    Smart,
    /// Use LLM to generate summaries
    Llm,
}

impl CompactionStrategy {
    pub fn as_str(&self) -> &str {
        match self {
            CompactionStrategy::Basic => "basic",
            CompactionStrategy::Smart => "smart",
            CompactionStrategy::Llm => "llm",
        }
    }
}

/// Configuration for memory compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CompactionConfig {
    /// Days after which completed tasks can be compacted
    pub auto_compact_days: u32,

    /// Minimum tasks required to trigger compaction
    pub min_tasks: usize,

    /// Compaction strategy
    pub strategy: CompactionStrategy,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto_compact_days: 14,
            min_tasks: 3,
            strategy: CompactionStrategy::Smart,
        }
    }
}

/// Configuration for the background daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    /// Enable daemon functionality
    pub enabled: bool,

    /// Debounce delay in seconds before committing
    pub debounce_seconds: u64,

    /// Auto-commit changes to git
    pub auto_commit: bool,

    /// Commit message format (placeholders: {action}, {id}, {count})
    pub commit_message_format: String,

    /// Auto-push to remote after commit
    pub auto_push: bool,

    /// Remote name for auto-push
    pub push_remote: String,

    /// Branch name for auto-push
    pub push_branch: String,
}

/// Configuration for agent coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Agent name (defaults to $SHAPE_AGENT, then $USER)
    pub name: Option<String>,

    /// Claim timeout in hours (default: 4)
    pub claim_timeout_hours: u32,

    /// Auto-unclaim when task is marked done
    pub auto_unclaim_on_done: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: None,
            claim_timeout_hours: 4,
            auto_unclaim_on_done: true,
        }
    }
}

impl AgentConfig {
    /// Gets the effective agent name from config, environment, or defaults
    pub fn effective_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| std::env::var("SHAPE_AGENT").ok())
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "anonymous".to_string())
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_seconds: 5,
            auto_commit: true,
            commit_message_format: "shape: {action} {id}".to_string(),
            auto_push: false,
            push_remote: "origin".to_string(),
            push_branch: "main".to_string(),
        }
    }
}

/// Project-level configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectConfig {
    /// Default brief type for `shape brief new`
    pub default_brief_type: DefaultBriefType,

    /// Plugins to load
    pub plugins: Vec<String>,

    /// Days to include completed tasks in context (default 7)
    pub context_days: u32,

    /// Compaction settings
    pub compaction: CompactionConfig,

    /// Daemon settings
    pub daemon: DaemonConfig,

    /// Agent coordination settings
    pub agent: AgentConfig,
}

impl ProjectConfig {
    pub fn default() -> Self {
        Self {
            default_brief_type: DefaultBriefType::Minimal,
            plugins: vec![],
            context_days: 7,
            compaction: CompactionConfig::default(),
            daemon: DaemonConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

/// Global user configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GlobalConfig {
    /// Default output format (text or json)
    pub default_format: OutputFormat,

    /// Editor command for editing briefs
    pub editor: Option<String>,
}

/// Output format for commands
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Combined configuration (global + project)
#[derive(Debug, Clone)]
pub struct Config {
    pub project: ProjectConfig,
    pub global: GlobalConfig,
    pub project_root: Option<PathBuf>,
}

impl Config {
    /// Loads configuration from default locations
    pub fn load() -> Result<Self> {
        let global = Self::load_global()?;
        let (project, project_root) = Self::load_project()?;

        Ok(Self {
            project,
            global,
            project_root,
        })
    }

    /// Loads configuration for a specific project
    pub fn for_project(project_root: &Path) -> Result<Self> {
        let global = Self::load_global()?;
        let project = Self::load_project_config(project_root)?;

        Ok(Self {
            project,
            global,
            project_root: Some(project_root.to_path_buf()),
        })
    }

    /// Returns the global config directory
    pub fn global_config_dir() -> Option<PathBuf> {
        ProjectDirs::from("dev", "shape", "shape-cli").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Loads global configuration
    fn load_global() -> Result<GlobalConfig> {
        let config_dir = match Self::global_config_dir() {
            Some(dir) => dir,
            None => return Ok(GlobalConfig::default()),
        };

        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            return Ok(GlobalConfig::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read global config: {}", config_path.display()))?;

        toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
            .context("Failed to parse global config")
    }

    /// Finds and loads project configuration
    fn load_project() -> Result<(ProjectConfig, Option<PathBuf>)> {
        let project_root = Self::find_project_root();

        match project_root {
            Some(root) => {
                let config = Self::load_project_config(&root)?;
                Ok((config, Some(root)))
            }
            None => Ok((ProjectConfig::default(), None)),
        }
    }

    /// Loads project configuration from a specific root
    fn load_project_config(project_root: &Path) -> Result<ProjectConfig> {
        let config_path = project_root.join(".shape").join("config.toml");

        if !config_path.exists() {
            return Ok(ProjectConfig::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read project config: {}", config_path.display()))?;

        toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
            .context("Failed to parse project config")
    }

    /// Finds the project root by looking for `.shape/` directory
    pub fn find_project_root() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;

        loop {
            let shape_dir = current.join(".shape");
            if shape_dir.is_dir() {
                return Some(current);
            }

            if !current.pop() {
                return None;
            }
        }
    }

    /// Returns true if we're in a shape project
    pub fn is_in_project(&self) -> bool {
        self.project_root.is_some()
    }

    /// Returns the project root, or an error if not in a project
    pub fn require_project_root(&self) -> Result<&Path> {
        self.project_root
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Not in a shape project. Run 'shape init' first."))
    }

    /// Saves the project configuration
    pub fn save_project(&self) -> Result<()> {
        let root = self.require_project_root()?;
        let config_path = root.join(".shape").join("config.toml");

        let content =
            toml::to_string_pretty(&self.project).context("Failed to serialize project config")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write project config: {}", config_path.display()))
    }

    /// Saves the global configuration
    pub fn save_global(&self) -> Result<()> {
        let config_dir = Self::global_config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;

        let config_path = config_dir.join("config.toml");
        let content =
            toml::to_string_pretty(&self.global).context("Failed to serialize global config")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write global config: {}", config_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_config() {
        let config = Config {
            project: ProjectConfig::default(),
            global: GlobalConfig::default(),
            project_root: None,
        };

        assert_eq!(config.project.context_days, 7);
        assert_eq!(config.global.default_format, OutputFormat::Text);
    }

    #[test]
    fn parse_project_config() {
        let toml = r#"
default_brief_type = "minimal"
plugins = ["shape-brief-shapeup"]
context_days = 14
"#;

        let config: ProjectConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.context_days, 14);
        assert_eq!(config.plugins, vec!["shape-brief-shapeup"]);
    }

    #[test]
    fn parse_global_config() {
        let toml = r#"
default_format = "json"
editor = "code"
"#;

        let config: GlobalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.default_format, OutputFormat::Json);
        assert_eq!(config.editor, Some("code".to_string()));
    }

    #[test]
    fn find_project_root() {
        let dir = TempDir::new().unwrap();
        let shape_dir = dir.path().join(".shape");
        fs::create_dir_all(&shape_dir).unwrap();

        // Change to a subdirectory
        let sub_dir = dir.path().join("sub").join("dir");
        fs::create_dir_all(&sub_dir).unwrap();
        std::env::set_current_dir(&sub_dir).unwrap();

        let root = Config::find_project_root();
        // Canonicalize both paths to handle macOS /var -> /private/var symlinks
        let expected = dir.path().canonicalize().ok();
        let actual = root.and_then(|p| p.canonicalize().ok());
        assert_eq!(actual, expected);

        // Reset current dir to avoid affecting other tests
        std::env::set_current_dir(dir.path()).unwrap();
    }

    #[test]
    fn config_not_in_project() {
        let config = Config {
            project: ProjectConfig::default(),
            global: GlobalConfig::default(),
            project_root: None,
        };

        assert!(!config.is_in_project());
        assert!(config.require_project_root().is_err());
    }

    #[test]
    fn default_brief_type() {
        let minimal = DefaultBriefType::Minimal;
        assert_eq!(minimal.as_str(), "minimal");

        let custom = DefaultBriefType::Custom("shapeup".to_string());
        assert_eq!(custom.as_str(), "shapeup");
    }
}
