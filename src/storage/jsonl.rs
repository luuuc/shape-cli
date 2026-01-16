//! JSONL storage for tasks
//!
//! Tasks are stored in `.shape/tasks.jsonl` with one JSON object per line.
//! Uses file locking for concurrent access safety.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs2::FileExt;

use crate::domain::{Task, TaskId};

/// Store for task data in JSONL format
pub struct TaskStore {
    path: PathBuf,
}

impl TaskStore {
    /// Creates a new task store at the given path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Creates the default store for a project
    pub fn for_project(project_root: &Path) -> Self {
        Self::new(project_root.join(".shape").join("tasks.jsonl"))
    }

    /// Returns the path to the store file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads all tasks from the store
    pub fn read_all(&self) -> Result<HashMap<TaskId, Task>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }

        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open task store: {}", self.path.display()))?;

        // Acquire shared lock for reading
        file.lock_shared()
            .context("Failed to acquire read lock on task store")?;

        let reader = BufReader::new(&file);
        let mut tasks = HashMap::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {}", line_num + 1))?;

            if line.trim().is_empty() {
                continue;
            }

            let task: Task = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse task at line {}", line_num + 1))?;

            tasks.insert(task.id.clone(), task);
        }

        // Lock is released when file is dropped
        Ok(tasks)
    }

    /// Reads tasks for a specific anchor
    pub fn read_for_anchor(
        &self,
        anchor_id: &crate::domain::AnchorId,
    ) -> Result<HashMap<TaskId, Task>> {
        let all = self.read_all()?;
        Ok(all
            .into_iter()
            .filter(|(_, task)| &task.anchor_id() == anchor_id)
            .collect())
    }

    /// Writes all tasks to the store (full rewrite)
    pub fn write_all(&self, tasks: &HashMap<TaskId, Task>) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write to temp file first
        let temp_path = self.path.with_extension("jsonl.tmp");

        {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)
                .with_context(|| format!("Failed to create temp file: {}", temp_path.display()))?;

            // Acquire exclusive lock
            file.lock_exclusive()
                .context("Failed to acquire write lock on task store")?;

            let mut writer = BufWriter::new(&file);

            // Sort by ID for consistent output
            let mut sorted: Vec<_> = tasks.values().collect();
            sorted.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

            for task in sorted {
                let line = serde_json::to_string(task).context("Failed to serialize task")?;
                writeln!(writer, "{}", line).context("Failed to write task")?;
            }

            writer.flush().context("Failed to flush task store")?;
        }

        // Atomic rename
        fs::rename(&temp_path, &self.path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                self.path.display()
            )
        })?;

        Ok(())
    }

    /// Appends a single task (used for quick adds without full rewrite)
    pub fn append(&self, task: &Task) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open task store: {}", self.path.display()))?;

        // Acquire exclusive lock
        file.lock_exclusive()
            .context("Failed to acquire write lock on task store")?;

        let mut writer = BufWriter::new(&file);
        let line = serde_json::to_string(task).context("Failed to serialize task")?;
        writeln!(writer, "{}", line).context("Failed to write task")?;

        writer.flush().context("Failed to flush task store")?;

        Ok(())
    }

    /// Updates a single task (reads all, updates, writes all)
    pub fn update(&self, task: &Task) -> Result<()> {
        let mut tasks = self.read_all()?;
        tasks.insert(task.id.clone(), task.clone());
        self.write_all(&tasks)
    }

    /// Removes a task by ID
    pub fn remove(&self, task_id: &TaskId) -> Result<bool> {
        let mut tasks = self.read_all()?;
        let removed = tasks.remove(task_id).is_some();
        if removed {
            self.write_all(&tasks)?;
        }
        Ok(removed)
    }

    /// Compacts the store (removes duplicates, rewrites clean)
    pub fn compact(&self) -> Result<usize> {
        let tasks = self.read_all()?;
        let count = tasks.len();
        self.write_all(&tasks)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_task(seq: u32) -> Task {
        let anchor = crate::domain::AnchorId::new("Test", Utc::now());
        let task_id = crate::domain::TaskId::new(&anchor, seq);
        Task::new(task_id, format!("Task {}", seq))
    }

    #[test]
    fn read_empty_store() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let tasks = store.read_all().unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn write_and_read_tasks() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let task1 = make_task(1);
        let task2 = make_task(2);

        let mut tasks = HashMap::new();
        tasks.insert(task1.id.clone(), task1.clone());
        tasks.insert(task2.id.clone(), task2.clone());

        store.write_all(&tasks).unwrap();

        let loaded = store.read_all().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get(&task1.id).unwrap().title, task1.title);
        assert_eq!(loaded.get(&task2.id).unwrap().title, task2.title);
    }

    #[test]
    fn append_task() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let task1 = make_task(1);
        let task2 = make_task(2);

        store.append(&task1).unwrap();
        store.append(&task2).unwrap();

        let loaded = store.read_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn update_task() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let mut task = make_task(1);
        store.append(&task).unwrap();

        task.start();
        store.update(&task).unwrap();

        let loaded = store.read_all().unwrap();
        let loaded_task = loaded.get(&task.id).unwrap();
        assert_eq!(loaded_task.status, crate::domain::TaskStatus::InProgress);
    }

    #[test]
    fn remove_task() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let task1 = make_task(1);
        let task2 = make_task(2);

        let mut tasks = HashMap::new();
        tasks.insert(task1.id.clone(), task1.clone());
        tasks.insert(task2.id.clone(), task2.clone());
        store.write_all(&tasks).unwrap();

        let removed = store.remove(&task1.id).unwrap();
        assert!(removed);

        let loaded = store.read_all().unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(!loaded.contains_key(&task1.id));
    }

    #[test]
    fn compact_removes_duplicates() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let task = make_task(1);

        // Append same task multiple times (simulating updates)
        store.append(&task).unwrap();
        store.append(&task).unwrap();
        store.append(&task).unwrap();

        // Compact should result in one task
        let count = store.compact().unwrap();
        assert_eq!(count, 1);

        let loaded = store.read_all().unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("nested").join("dir").join("tasks.jsonl"));

        let task = make_task(1);
        store.append(&task).unwrap();

        assert!(store.path().exists());
    }

    #[test]
    fn atomic_write() {
        let dir = TempDir::new().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.jsonl"));

        let task = make_task(1);
        let mut tasks = HashMap::new();
        tasks.insert(task.id.clone(), task.clone());
        store.write_all(&tasks).unwrap();

        // Temp file should not exist after write
        let temp_path = store.path().with_extension("jsonl.tmp");
        assert!(!temp_path.exists());
    }
}
