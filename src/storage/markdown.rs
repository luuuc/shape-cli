//! Markdown storage for briefs
//!
//! Briefs are stored as markdown files in `.shape/briefs/`.
//! Each file has YAML frontmatter for metadata and markdown body.
//! An index file (`.shape/briefs/index.jsonl`) caches metadata for fast queries.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::domain::{Brief, BriefFrontmatter, BriefId};

/// Index entry for quick brief lookups
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IndexEntry {
    id: BriefId,
    title: String,
    brief_type: String,
    status: crate::domain::BriefStatus,
    updated_at: chrono::DateTime<chrono::Utc>,
    file_name: String,
}

impl From<&Brief> for IndexEntry {
    fn from(brief: &Brief) -> Self {
        Self {
            id: brief.id.clone(),
            title: brief.title.clone(),
            brief_type: brief.brief_type.clone(),
            status: brief.status,
            updated_at: brief.updated_at,
            file_name: format!("{}.md", brief.id),
        }
    }
}

/// Store for brief data as markdown files
pub struct BriefStore {
    /// Directory containing brief files
    dir: PathBuf,

    /// Path to the index file
    index_path: PathBuf,
}

impl BriefStore {
    /// Creates a new brief store at the given directory
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let index_path = dir.join("index.jsonl");
        Self { dir, index_path }
    }

    /// Creates the default store for a project
    pub fn for_project(project_root: &Path) -> Self {
        Self::new(project_root.join(".shape").join("briefs"))
    }

    /// Returns the directory containing brief files
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the path to a brief file
    fn brief_path(&self, id: &BriefId) -> PathBuf {
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
    fn read_index(&self) -> Result<HashMap<BriefId, IndexEntry>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let file = File::open(&self.index_path)
            .with_context(|| format!("Failed to open index: {}", self.index_path.display()))?;

        let reader = BufReader::new(file);
        let mut entries = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line =
                line.with_context(|| format!("Failed to read index line {}", line_num + 1))?;

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
    fn write_index(&self, entries: &HashMap<BriefId, IndexEntry>) -> Result<()> {
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
    fn rebuild_index(&self) -> Result<HashMap<BriefId, IndexEntry>> {
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
                if let Ok(brief) = self.read_from_file(&path) {
                    entries.insert(brief.id.clone(), IndexEntry::from(&brief));
                }
            }
        }

        self.write_index(&entries)?;
        Ok(entries)
    }

    /// Ensures the index is up-to-date
    fn ensure_index(&self) -> Result<HashMap<BriefId, IndexEntry>> {
        if self.index_is_stale() {
            self.rebuild_index()
        } else {
            self.read_index()
        }
    }

    /// Reads a brief from a file
    fn read_from_file(&self, path: &Path) -> Result<Brief> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read brief file: {}", path.display()))?;

        self.parse_markdown(&content)
    }

    /// Parses a markdown string into a Brief
    fn parse_markdown(&self, content: &str) -> Result<Brief> {
        // Manual frontmatter parsing
        let content = content.trim();

        if !content.starts_with("---") {
            anyhow::bail!("Missing frontmatter (must start with ---)");
        }

        // Find the end of frontmatter
        let rest = &content[3..];
        let end_pos = rest
            .find("---")
            .ok_or_else(|| anyhow::anyhow!("Missing frontmatter end delimiter (---)"))?;

        let yaml_content = &rest[..end_pos].trim();
        let body = rest[end_pos + 3..].trim();

        // Parse YAML frontmatter
        let fm: BriefFrontmatter =
            serde_yaml::from_str(yaml_content).context("Failed to parse frontmatter")?;

        Ok(fm.into_brief(body.to_string()))
    }

    /// Writes a brief to its file atomically (temp file + rename)
    fn write_to_file(&self, brief: &Brief) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("Failed to create directory: {}", self.dir.display()))?;

        let path = self.brief_path(&brief.id);
        let temp_path = path.with_extension("md.tmp");
        let content = self.render_markdown(brief)?;

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

    /// Renders a brief to markdown
    fn render_markdown(&self, brief: &Brief) -> Result<String> {
        let frontmatter = BriefFrontmatter::from(brief);
        let yaml =
            serde_yaml::to_string(&frontmatter).context("Failed to serialize frontmatter")?;

        let mut content = String::new();
        content.push_str("---\n");
        content.push_str(&yaml);
        content.push_str("---\n\n");
        content.push_str(&brief.body);

        if !content.ends_with('\n') {
            content.push('\n');
        }

        Ok(content)
    }

    /// Reads all briefs
    pub fn read_all(&self) -> Result<HashMap<BriefId, Brief>> {
        let _ = self.ensure_index()?; // Ensure index is fresh
        let mut briefs = HashMap::new();

        if !self.dir.exists() {
            return Ok(briefs);
        }

        for entry in fs::read_dir(&self.dir)
            .with_context(|| format!("Failed to read directory: {}", self.dir.display()))?
        {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(brief) = self.read_from_file(&path) {
                    briefs.insert(brief.id.clone(), brief);
                }
            }
        }

        Ok(briefs)
    }

    /// Lists briefs with basic info (from index, fast)
    pub fn list(&self) -> Result<Vec<(BriefId, String, crate::domain::BriefStatus)>> {
        let index = self.ensure_index()?;
        Ok(index
            .values()
            .map(|e| (e.id.clone(), e.title.clone(), e.status))
            .collect())
    }

    /// Lists briefs filtered by status
    pub fn list_by_status(
        &self,
        status: crate::domain::BriefStatus,
    ) -> Result<Vec<(BriefId, String)>> {
        let index = self.ensure_index()?;
        Ok(index
            .values()
            .filter(|e| e.status == status)
            .map(|e| (e.id.clone(), e.title.clone()))
            .collect())
    }

    /// Reads a single brief by ID
    pub fn read(&self, id: &BriefId) -> Result<Option<Brief>> {
        let path = self.brief_path(id);
        if !path.exists() {
            return Ok(None);
        }

        Ok(Some(self.read_from_file(&path)?))
    }

    /// Writes a brief
    pub fn write(&self, brief: &Brief) -> Result<()> {
        self.write_to_file(brief)?;

        // Update index
        let mut index = self.read_index().unwrap_or_default();
        index.insert(brief.id.clone(), IndexEntry::from(brief));
        self.write_index(&index)?;

        Ok(())
    }

    /// Removes a brief by ID
    pub fn remove(&self, id: &BriefId) -> Result<bool> {
        let path = self.brief_path(id);
        if !path.exists() {
            return Ok(false);
        }

        fs::remove_file(&path)
            .with_context(|| format!("Failed to remove brief file: {}", path.display()))?;

        // Update index
        let mut index = self.read_index().unwrap_or_default();
        index.remove(id);
        self.write_index(&index)?;

        Ok(true)
    }

    /// Checks if a brief exists
    pub fn exists(&self, id: &BriefId) -> bool {
        self.brief_path(id).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Brief, BriefStatus};
    use tempfile::TempDir;

    #[test]
    fn read_empty_store() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let briefs = store.read_all().unwrap();
        assert!(briefs.is_empty());
    }

    #[test]
    fn write_and_read_brief() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let mut brief = Brief::new("Test Pitch", "minimal");
        brief.set_body("# Problem\n\nThis is a test.");
        brief.set_meta("custom", "value");

        store.write(&brief).unwrap();

        let loaded = store.read(&brief.id).unwrap().unwrap();
        assert_eq!(loaded.title, brief.title);
        assert_eq!(loaded.body, brief.body);
        assert_eq!(loaded.get_meta("custom"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn list_briefs() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let brief1 = Brief::new("Pitch 1", "minimal");
        let brief2 = Brief::new("Pitch 2", "shapeup");

        store.write(&brief1).unwrap();
        store.write(&brief2).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_by_status() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let mut brief1 = Brief::new("Active", "minimal");
        brief1.set_status(BriefStatus::InProgress);

        let brief2 = Brief::new("Proposed", "minimal");

        store.write(&brief1).unwrap();
        store.write(&brief2).unwrap();

        let active = store.list_by_status(BriefStatus::InProgress).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].1, "Active");
    }

    #[test]
    fn remove_brief() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let brief = Brief::new("Test", "minimal");
        store.write(&brief).unwrap();

        assert!(store.exists(&brief.id));

        let removed = store.remove(&brief.id).unwrap();
        assert!(removed);
        assert!(!store.exists(&brief.id));
    }

    #[test]
    fn index_rebuilds_on_manual_edit() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let brief = Brief::new("Test", "minimal");
        store.write(&brief).unwrap();

        // Manually edit the file
        let path = store.brief_path(&brief.id);
        let content = fs::read_to_string(&path).unwrap();
        let new_content = content.replace("Test", "Updated Title");

        // Sleep to ensure mtime changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&path, new_content).unwrap();

        // Index should rebuild and reflect the change
        let loaded = store.read(&brief.id).unwrap().unwrap();
        assert_eq!(loaded.title, "Updated Title");
    }

    #[test]
    fn index_handles_deleted_files() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let brief = Brief::new("Test", "minimal");
        store.write(&brief).unwrap();

        // Manually delete the file
        let path = store.brief_path(&brief.id);
        fs::remove_file(&path).unwrap();

        // List should not include the deleted brief
        let list = store.list().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn markdown_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let mut brief = Brief::new("Complex Pitch", "shapeup");
        brief.set_body("# Problem\n\nMulti-line\ncontent here.\n\n## Solution\n\nMore content.");
        brief.set_meta("appetite", "6 weeks");
        brief.set_meta("tags", serde_json::json!(["tag1", "tag2"]));
        brief.set_status(BriefStatus::InProgress);

        store.write(&brief).unwrap();

        let loaded = store.read(&brief.id).unwrap().unwrap();
        assert_eq!(loaded.title, brief.title);
        assert_eq!(loaded.brief_type, brief.brief_type);
        assert_eq!(loaded.status, brief.status);
        assert_eq!(loaded.body, brief.body);
        assert_eq!(loaded.get_meta("appetite"), brief.get_meta("appetite"));
    }

    #[test]
    fn atomic_write_no_temp_file_left() {
        let dir = TempDir::new().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));

        let brief = Brief::new("Atomic Test", "minimal");
        store.write(&brief).unwrap();

        // Temp file should not exist after write
        let temp_path = store.brief_path(&brief.id).with_extension("md.tmp");
        assert!(
            !temp_path.exists(),
            "Temp file should be removed after atomic write"
        );

        // Actual file should exist
        assert!(store.brief_path(&brief.id).exists());
    }
}
