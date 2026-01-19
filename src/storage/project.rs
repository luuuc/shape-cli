//! Project management
//!
//! Handles project initialization and provides access to stores.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use thiserror::Error;

use super::{AnchorStore, Cache, Config, TaskStore};

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("Project already exists at {0}")]
    AlreadyExists(PathBuf),

    #[error("Not in a shape project. Run 'shape init' first.")]
    NotInProject,

    #[error("Failed to create project: {0}")]
    CreateFailed(String),
}

/// A Shape project
pub struct Project {
    root: PathBuf,
    config: Config,
}

impl Project {
    /// Opens an existing project at the given path
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let shape_dir = root.join(".shape");

        if !shape_dir.is_dir() {
            return Err(ProjectError::NotInProject.into());
        }

        let config = Config::for_project(&root)?;

        Ok(Self { root, config })
    }

    /// Opens the project at the current directory or a parent
    pub fn open_current() -> Result<Self> {
        let root = Config::find_project_root().ok_or(ProjectError::NotInProject)?;

        Self::open(root)
    }

    /// Initializes a new project at the given path
    pub fn init(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let shape_dir = root.join(".shape");

        // Create directory structure
        fs::create_dir_all(&shape_dir).with_context(|| {
            format!("Failed to create .shape directory: {}", shape_dir.display())
        })?;

        let anchors_dir = shape_dir.join("anchors");
        fs::create_dir_all(&anchors_dir).with_context(|| {
            format!(
                "Failed to create anchors directory: {}",
                anchors_dir.display()
            )
        })?;

        let plugins_dir = shape_dir.join("plugins");
        fs::create_dir_all(&plugins_dir).with_context(|| {
            format!(
                "Failed to create plugins directory: {}",
                plugins_dir.display()
            )
        })?;

        let sync_dir = shape_dir.join("sync");
        fs::create_dir_all(&sync_dir)
            .with_context(|| format!("Failed to create sync directory: {}", sync_dir.display()))?;

        // Create default config
        let config_path = shape_dir.join("config.toml");
        if !config_path.exists() {
            let default_config = r#"# Shape CLI configuration
# See https://shape.dev/docs/config for options

# Default anchor type for 'shape anchor new'
default_anchor_type = "minimal"

# Plugins to load
plugins = []

# Days to include completed tasks in context export
context_days = 7
"#;
            fs::write(&config_path, default_config)
                .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
        }

        // Create .gitignore for .shape
        let gitignore_path = shape_dir.join(".gitignore");
        if !gitignore_path.exists() {
            let gitignore = r#"# Ignore index files (they're regenerated)
anchors/index.jsonl

# Ignore SQLite cache (regenerated from source files)
.cache/

# Ignore sync state (contains remote IDs)
sync/

# Ignore plugin cache
plugins/*.cache
"#;
            fs::write(&gitignore_path, gitignore).with_context(|| {
                format!("Failed to write .gitignore: {}", gitignore_path.display())
            })?;
        }

        Self::open(root)
    }

    /// Returns the project root path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the .shape directory path
    pub fn shape_dir(&self) -> PathBuf {
        self.root.join(".shape")
    }

    /// Returns the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Returns the task store
    pub fn task_store(&self) -> TaskStore {
        TaskStore::for_project(&self.root)
    }

    /// Returns the anchor store
    pub fn anchor_store(&self) -> AnchorStore {
        AnchorStore::for_project(&self.root)
    }

    /// Returns the plugins directory
    pub fn plugins_dir(&self) -> PathBuf {
        self.shape_dir().join("plugins")
    }

    /// Returns the sync directory
    pub fn sync_dir(&self) -> PathBuf {
        self.shape_dir().join("sync")
    }

    /// Returns the cache directory
    pub fn cache_dir(&self) -> PathBuf {
        self.shape_dir().join(".cache")
    }

    /// Opens the SQLite cache for this project
    pub fn cache(&self) -> Result<Cache> {
        Cache::open(&self.root)
    }

    /// Rebuilds the cache from source files
    pub fn rebuild_cache(&self) -> Result<()> {
        let mut cache = self.cache()?;
        let tasks = self.task_store().read_all()?;
        let anchors = self.anchor_store().read_all()?;
        cache.rebuild(&tasks, &anchors)?;
        Ok(())
    }

    /// Gets the cache if it's fresh, or rebuilds it if stale
    pub fn get_or_rebuild_cache(&self) -> Result<Cache> {
        let mut cache = self.cache()?;

        if cache.is_stale()? {
            let tasks = self.task_store().read_all()?;
            let anchors = self.anchor_store().read_all()?;
            cache.rebuild(&tasks, &anchors)?;
        }

        Ok(cache)
    }

    /// Checks if a path is inside this project
    pub fn contains(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    /// Returns a relative path from the project root
    pub fn relative_path(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.root).ok().map(|p| p.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_structure() {
        let dir = TempDir::new().unwrap();
        let project = Project::init(dir.path()).unwrap();

        assert!(project.shape_dir().is_dir());
        assert!(project.shape_dir().join("anchors").is_dir());
        assert!(project.shape_dir().join("plugins").is_dir());
        assert!(project.shape_dir().join("sync").is_dir());
        assert!(project.shape_dir().join("config.toml").is_file());
        assert!(project.shape_dir().join(".gitignore").is_file());
    }

    #[test]
    fn init_is_idempotent() {
        let dir = TempDir::new().unwrap();

        Project::init(dir.path()).unwrap();
        Project::init(dir.path()).unwrap(); // Should not fail

        assert!(dir.path().join(".shape").is_dir());
    }

    #[test]
    fn open_existing_project() {
        let dir = TempDir::new().unwrap();
        Project::init(dir.path()).unwrap();

        let project = Project::open(dir.path()).unwrap();
        assert_eq!(project.root(), dir.path());
    }

    #[test]
    fn open_non_project_fails() {
        let dir = TempDir::new().unwrap();
        let result = Project::open(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn stores_are_accessible() {
        let dir = TempDir::new().unwrap();
        let project = Project::init(dir.path()).unwrap();

        let task_store = project.task_store();
        let anchor_store = project.anchor_store();

        assert!(task_store.path().ends_with("tasks.jsonl"));
        assert!(anchor_store.dir().ends_with("anchors"));
    }

    #[test]
    fn relative_path() {
        let dir = TempDir::new().unwrap();
        let project = Project::init(dir.path()).unwrap();

        let abs_path = dir.path().join("sub").join("file.txt");
        let rel_path = project.relative_path(&abs_path);

        assert_eq!(rel_path, Some(PathBuf::from("sub/file.txt")));
    }
}
