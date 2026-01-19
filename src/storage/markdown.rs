//! Markdown storage for anchors
//!
//! Anchors are stored as markdown files in `.shape/anchors/`.
//! Each file has YAML frontmatter for metadata and markdown body.
//! An index file (`.shape/anchors/index.jsonl`) caches metadata for fast queries.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::domain::{Anchor, AnchorFrontmatter, AnchorId};

/// Index entry for quick anchor lookups
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IndexEntry {
    id: AnchorId,
    title: String,
    anchor_type: String,
    status: crate::domain::AnchorStatus,
    updated_at: chrono::DateTime<chrono::Utc>,
    file_name: String,
}

impl From<&Anchor> for IndexEntry {
    fn from(anchor: &Anchor) -> Self {
        Self {
            id: anchor.id.clone(),
            title: anchor.title.clone(),
            anchor_type: anchor.anchor_type.clone(),
            status: anchor.status,
            updated_at: anchor.updated_at,
            file_name: format!("{}.md", anchor.id),
        }
    }
}

/// Store for anchor data as markdown files
pub struct AnchorStore {
    /// Directory containing anchor files
    dir: PathBuf,

    /// Path to the index file
    index_path: PathBuf,
}

impl AnchorStore {
    /// Creates a new anchor store at the given directory
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let index_path = dir.join("index.jsonl");
        Self { dir, index_path }
    }

    /// Creates the default store for a project
    pub fn for_project(project_root: &Path) -> Self {
        Self::new(project_root.join(".shape").join("anchors"))
    }

    /// Returns the directory containing anchor files
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the path to an anchor file
    fn anchor_path(&self, id: &AnchorId) -> PathBuf {
        self.dir.join(format!("{}.md", id))
    }

    /// Checks if the index needs rebuilding
    fn index_is_stale(&self) -> bool {
        if !self.index_path.exists() {
            return true;
        }

        let index_mtime = match fs::metadata(&self.index_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return true,
        };

        // Check if any .md file is newer than the index
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(_) => return true,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(meta) = fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        if mtime > index_mtime {
                            return true;
                        }
                    }
                }
            }
        }

        // Check if any file was deleted (entry in index but no file)
        if let Ok(index) = self.read_index() {
            for entry in index.values() {
                let path = self.dir.join(&entry.file_name);
                if !path.exists() {
                    return true;
                }
            }
        }

        false
    }

    /// Reads the index file
    fn read_index(&self) -> Result<HashMap<AnchorId, IndexEntry>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let file = File::open(&self.index_path)
            .with_context(|| format!("Failed to open index: {}", self.index_path.display()))?;

        let reader = BufReader::new(file);
        let mut entries = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read index line {}", line_num + 1))?;

            if line.trim().is_empty() {
                continue;
            }

            let entry: IndexEntry = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse index entry at line {}", line_num + 1))?;

            entries.insert(entry.id.clone(), entry);
        }

        Ok(entries)
    }

    /// Writes the index file
    fn write_index(&self, entries: &HashMap<AnchorId, IndexEntry>) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("Failed to create directory: {}", self.dir.display()))?;

        let file = File::create(&self.index_path)
            .with_context(|| format!("Failed to create index: {}", self.index_path.display()))?;

        let mut writer = BufWriter::new(file);

        let mut sorted: Vec<_> = entries.values().collect();
        sorted.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

        for entry in sorted {
            let line = serde_json::to_string(entry).context("Failed to serialize index entry")?;
            writeln!(writer, "{}", line).context("Failed to write index entry")?;
        }

        writer.flush().context("Failed to flush index")?;
        Ok(())
    }

    /// Rebuilds the index from files
    fn rebuild_index(&self) -> Result<HashMap<AnchorId, IndexEntry>> {
        let mut entries = HashMap::new();

        if !self.dir.exists() {
            return Ok(entries);
        }

        for entry in fs::read_dir(&self.dir)
            .with_context(|| format!("Failed to read directory: {}", self.dir.display()))?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(anchor) = self.read_from_file(&path) {
                    entries.insert(anchor.id.clone(), IndexEntry::from(&anchor));
                }
            }
        }

        self.write_index(&entries)?;
        Ok(entries)
    }

    /// Ensures the index is up-to-date
    fn ensure_index(&self) -> Result<HashMap<AnchorId, IndexEntry>> {
        if self.index_is_stale() {
            self.rebuild_index()
        } else {
            self.read_index()
        }
    }

    /// Reads an anchor from a file
    fn read_from_file(&self, path: &Path) -> Result<Anchor> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read anchor file: {}", path.display()))?;

        self.parse_markdown(&content)
    }

    /// Parses a markdown string into an Anchor
    fn parse_markdown(&self, content: &str) -> Result<Anchor> {
        // Manual frontmatter parsing
        let content = content.trim();

        if !content.starts_with("---") {
            anyhow::bail!("Missing frontmatter (must start with ---)");
        }

        // Find the end of frontmatter
        let rest = &content[3..];
        let end_pos = rest.find("---")
            .ok_or_else(|| anyhow::anyhow!("Missing frontmatter end delimiter (---)"))?;

        let yaml_content = &rest[..end_pos].trim();
        let body = rest[end_pos + 3..].trim();

        // Parse YAML frontmatter
        let fm: AnchorFrontmatter = serde_yaml::from_str(yaml_content)
            .context("Failed to parse frontmatter")?;

        Ok(fm.into_anchor(body.to_string()))
    }

    /// Writes an anchor to its file atomically (temp file + rename)
    fn write_to_file(&self, anchor: &Anchor) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("Failed to create directory: {}", self.dir.display()))?;

        let path = self.anchor_path(&anchor.id);
        let temp_path = path.with_extension("md.tmp");
        let content = self.render_markdown(anchor)?;

        // Write to temp file first
        fs::write(&temp_path, &content)
            .with_context(|| format!("Failed to write temp file: {}", temp_path.display()))?;

        // Atomic rename
        fs::rename(&temp_path, &path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }

    /// Renders an anchor to markdown
    fn render_markdown(&self, anchor: &Anchor) -> Result<String> {
        let frontmatter = AnchorFrontmatter::from(anchor);
        let yaml = serde_yaml::to_string(&frontmatter).context("Failed to serialize frontmatter")?;

        let mut content = String::new();
        content.push_str("---\n");
        content.push_str(&yaml);
        content.push_str("---\n\n");
        content.push_str(&anchor.body);

        if !content.ends_with('\n') {
            content.push('\n');
        }

        Ok(content)
    }

    /// Reads all anchors
    pub fn read_all(&self) -> Result<HashMap<AnchorId, Anchor>> {
        let _ = self.ensure_index()?; // Ensure index is fresh
        let mut anchors = HashMap::new();

        if !self.dir.exists() {
            return Ok(anchors);
        }

        for entry in fs::read_dir(&self.dir)
            .with_context(|| format!("Failed to read directory: {}", self.dir.display()))?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(anchor) = self.read_from_file(&path) {
                    anchors.insert(anchor.id.clone(), anchor);
                }
            }
        }

        Ok(anchors)
    }

    /// Lists anchors with basic info (from index, fast)
    pub fn list(&self) -> Result<Vec<(AnchorId, String, crate::domain::AnchorStatus)>> {
        let index = self.ensure_index()?;
        Ok(index
            .values()
            .map(|e| (e.id.clone(), e.title.clone(), e.status))
            .collect())
    }

    /// Lists anchors filtered by status
    pub fn list_by_status(
        &self,
        status: crate::domain::AnchorStatus,
    ) -> Result<Vec<(AnchorId, String)>> {
        let index = self.ensure_index()?;
        Ok(index
            .values()
            .filter(|e| e.status == status)
            .map(|e| (e.id.clone(), e.title.clone()))
            .collect())
    }

    /// Reads a single anchor by ID
    pub fn read(&self, id: &AnchorId) -> Result<Option<Anchor>> {
        let path = self.anchor_path(id);
        if !path.exists() {
            return Ok(None);
        }

        Ok(Some(self.read_from_file(&path)?))
    }

    /// Writes an anchor
    pub fn write(&self, anchor: &Anchor) -> Result<()> {
        self.write_to_file(anchor)?;

        // Update index
        let mut index = self.read_index().unwrap_or_default();
        index.insert(anchor.id.clone(), IndexEntry::from(anchor));
        self.write_index(&index)?;

        Ok(())
    }

    /// Removes an anchor by ID
    pub fn remove(&self, id: &AnchorId) -> Result<bool> {
        let path = self.anchor_path(id);
        if !path.exists() {
            return Ok(false);
        }

        fs::remove_file(&path)
            .with_context(|| format!("Failed to remove anchor file: {}", path.display()))?;

        // Update index
        let mut index = self.read_index().unwrap_or_default();
        index.remove(id);
        self.write_index(&index)?;

        Ok(true)
    }

    /// Checks if an anchor exists
    pub fn exists(&self, id: &AnchorId) -> bool {
        self.anchor_path(id).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Anchor, AnchorStatus};
    use tempfile::TempDir;

    #[test]
    fn read_empty_store() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchors = store.read_all().unwrap();
        assert!(anchors.is_empty());
    }

    #[test]
    fn write_and_read_anchor() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let mut anchor = Anchor::new("Test Pitch", "minimal");
        anchor.set_body("# Problem\n\nThis is a test.");
        anchor.set_meta("custom", "value");

        store.write(&anchor).unwrap();

        let loaded = store.read(&anchor.id).unwrap().unwrap();
        assert_eq!(loaded.title, anchor.title);
        assert_eq!(loaded.body, anchor.body);
        assert_eq!(loaded.get_meta("custom"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn list_anchors() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchor1 = Anchor::new("Pitch 1", "minimal");
        let anchor2 = Anchor::new("Pitch 2", "shapeup");

        store.write(&anchor1).unwrap();
        store.write(&anchor2).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_by_status() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let mut anchor1 = Anchor::new("Active", "minimal");
        anchor1.set_status(AnchorStatus::InProgress);

        let anchor2 = Anchor::new("Proposed", "minimal");

        store.write(&anchor1).unwrap();
        store.write(&anchor2).unwrap();

        let active = store.list_by_status(AnchorStatus::InProgress).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].1, "Active");
    }

    #[test]
    fn remove_anchor() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchor = Anchor::new("Test", "minimal");
        store.write(&anchor).unwrap();

        assert!(store.exists(&anchor.id));

        let removed = store.remove(&anchor.id).unwrap();
        assert!(removed);
        assert!(!store.exists(&anchor.id));
    }

    #[test]
    fn index_rebuilds_on_manual_edit() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchor = Anchor::new("Test", "minimal");
        store.write(&anchor).unwrap();

        // Manually edit the file
        let path = store.anchor_path(&anchor.id);
        let content = fs::read_to_string(&path).unwrap();
        let new_content = content.replace("Test", "Updated Title");

        // Sleep to ensure mtime changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&path, new_content).unwrap();

        // Index should rebuild and reflect the change
        let loaded = store.read(&anchor.id).unwrap().unwrap();
        assert_eq!(loaded.title, "Updated Title");
    }

    #[test]
    fn index_handles_deleted_files() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchor = Anchor::new("Test", "minimal");
        store.write(&anchor).unwrap();

        // Manually delete the file
        let path = store.anchor_path(&anchor.id);
        fs::remove_file(&path).unwrap();

        // List should not include the deleted anchor
        let list = store.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn markdown_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let mut anchor = Anchor::new("Complex Pitch", "shapeup");
        anchor.set_body("# Problem\n\nMulti-line\ncontent here.\n\n## Solution\n\nMore content.");
        anchor.set_meta("appetite", "6 weeks");
        anchor.set_meta("tags", serde_json::json!(["tag1", "tag2"]));
        anchor.set_status(AnchorStatus::InProgress);

        store.write(&anchor).unwrap();

        let loaded = store.read(&anchor.id).unwrap().unwrap();
        assert_eq!(loaded.title, anchor.title);
        assert_eq!(loaded.anchor_type, anchor.anchor_type);
        assert_eq!(loaded.status, anchor.status);
        assert_eq!(loaded.body, anchor.body);
        assert_eq!(loaded.get_meta("appetite"), anchor.get_meta("appetite"));
    }

    #[test]
    fn atomic_write_no_temp_file_left() {
        let dir = TempDir::new().unwrap();
        let store = AnchorStore::new(dir.path().join("anchors"));

        let anchor = Anchor::new("Atomic Test", "minimal");
        store.write(&anchor).unwrap();

        // Temp file should not exist after write
        let temp_path = store.anchor_path(&anchor.id).with_extension("md.tmp");
        assert!(!temp_path.exists(), "Temp file should be removed after atomic write");

        // Actual file should exist
        assert!(store.anchor_path(&anchor.id).exists());
    }
}
