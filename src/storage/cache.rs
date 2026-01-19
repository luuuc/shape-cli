//! SQLite cache for fast queries
//!
//! The cache sits in `.shape/.cache/shape.db` and mirrors data from
//! the source-of-truth files (tasks.jsonl and anchors/*.md).
//! Cache invalidation is based on file modification times.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::domain::{Anchor, AnchorId, Task, TaskId, TaskStatus};

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache not found at {0}")]
    NotFound(PathBuf),

    #[error("Cache is stale and needs rebuild")]
    Stale,

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// SQLite cache for fast queries
pub struct Cache {
    /// Path to the SQLite database
    db_path: PathBuf,

    /// Path to the tasks.jsonl file (for mtime comparison)
    tasks_path: PathBuf,

    /// Path to the anchors directory (for mtime comparison)
    anchors_dir: PathBuf,

    /// Database connection
    conn: Connection,
}

impl Cache {
    /// Schema version - bump when schema changes to force rebuild
    const SCHEMA_VERSION: i32 = 1;

    /// Creates or opens the cache for a project
    pub fn open(project_root: &Path) -> Result<Self> {
        let shape_dir = project_root.join(".shape");
        let cache_dir = shape_dir.join(".cache");
        let db_path = cache_dir.join("shape.db");
        let tasks_path = shape_dir.join("tasks.jsonl");
        let anchors_dir = shape_dir.join("anchors");

        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open cache database: {}", db_path.display()))?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        let mut cache = Self {
            db_path,
            tasks_path,
            anchors_dir,
            conn,
        };

        cache.ensure_schema()?;

        Ok(cache)
    }

    /// Ensures the schema is up to date
    fn ensure_schema(&mut self) -> Result<()> {
        let current_version = self.get_schema_version()?;

        if current_version != Self::SCHEMA_VERSION {
            self.create_schema()?;
        }

        Ok(())
    }

    /// Gets the current schema version
    fn get_schema_version(&self) -> Result<i32> {
        let result: Option<i32> = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .optional()?
            .flatten();

        Ok(result.unwrap_or(0))
    }

    /// Creates the schema from scratch
    fn create_schema(&mut self) -> Result<()> {
        // Drop existing tables
        self.conn.execute_batch(
            "
            DROP TABLE IF EXISTS dependencies;
            DROP TABLE IF EXISTS tasks;
            DROP TABLE IF EXISTS briefs;
            DROP TABLE IF EXISTS tasks_fts;
            DROP TABLE IF EXISTS briefs_fts;
            DROP TABLE IF EXISTS cache_meta;
            ",
        )?;

        // Create tables
        self.conn.execute_batch(
            "
            CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                anchor_id TEXT,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                description TEXT,
                meta TEXT,
                depends_on TEXT
            );

            CREATE TABLE briefs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                brief_type TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                file_path TEXT NOT NULL,
                body TEXT
            );

            CREATE TABLE dependencies (
                task_id TEXT NOT NULL,
                depends_on_id TEXT NOT NULL,
                PRIMARY KEY (task_id, depends_on_id)
            );

            CREATE TABLE cache_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX idx_tasks_anchor ON tasks(anchor_id);
            CREATE INDEX idx_tasks_status ON tasks(status);
            CREATE INDEX idx_deps_blocked ON dependencies(depends_on_id);
            CREATE INDEX idx_briefs_status ON briefs(status);

            -- Full-text search
            CREATE VIRTUAL TABLE tasks_fts USING fts5(
                id,
                title,
                description,
                content='tasks',
                content_rowid='rowid'
            );

            CREATE VIRTUAL TABLE briefs_fts USING fts5(
                id,
                title,
                body,
                content='briefs',
                content_rowid='rowid'
            );

            -- Triggers to keep FTS in sync
            CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
                INSERT INTO tasks_fts(rowid, id, title, description)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.description);
            END;

            CREATE TRIGGER tasks_ad AFTER DELETE ON tasks BEGIN
                INSERT INTO tasks_fts(tasks_fts, rowid, id, title, description)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.description);
            END;

            CREATE TRIGGER tasks_au AFTER UPDATE ON tasks BEGIN
                INSERT INTO tasks_fts(tasks_fts, rowid, id, title, description)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.description);
                INSERT INTO tasks_fts(rowid, id, title, description)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.description);
            END;

            CREATE TRIGGER briefs_ai AFTER INSERT ON briefs BEGIN
                INSERT INTO briefs_fts(rowid, id, title, body)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.body);
            END;

            CREATE TRIGGER briefs_ad AFTER DELETE ON briefs BEGIN
                INSERT INTO briefs_fts(briefs_fts, rowid, id, title, body)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.body);
            END;

            CREATE TRIGGER briefs_au AFTER UPDATE ON briefs BEGIN
                INSERT INTO briefs_fts(briefs_fts, rowid, id, title, body)
                VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.body);
                INSERT INTO briefs_fts(rowid, id, title, body)
                VALUES (NEW.rowid, NEW.id, NEW.title, NEW.body);
            END;
            ",
        )?;

        // Set schema version
        self.conn.execute(
            &format!("PRAGMA user_version = {}", Self::SCHEMA_VERSION),
            [],
        )?;

        Ok(())
    }

    /// Checks if the cache needs to be rebuilt
    pub fn is_stale(&self) -> Result<bool> {
        let cache_mtime = self.get_cache_mtime()?;

        // Check tasks.jsonl
        if self.tasks_path.exists() {
            let tasks_mtime = fs::metadata(&self.tasks_path)?.modified()?;
            if tasks_mtime > cache_mtime {
                return Ok(true);
            }
        }

        // Check any anchor file
        if self.anchors_dir.exists() {
            for entry in fs::read_dir(&self.anchors_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(meta) = fs::metadata(&path) {
                        if let Ok(mtime) = meta.modified() {
                            if mtime > cache_mtime {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Gets the cache modification time (uses stored timestamp)
    fn get_cache_mtime(&self) -> Result<SystemTime> {
        let mtime_str: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM cache_meta WHERE key = 'last_rebuild'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        match mtime_str {
            Some(s) => {
                let timestamp: i64 = s.parse().unwrap_or(0);
                Ok(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64))
            }
            None => Ok(SystemTime::UNIX_EPOCH),
        }
    }

    /// Updates the cache modification time
    fn update_cache_mtime(&self) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.conn.execute(
            "INSERT OR REPLACE INTO cache_meta (key, value) VALUES ('last_rebuild', ?1)",
            params![now.to_string()],
        )?;

        Ok(())
    }

    /// Rebuilds the cache from source files
    pub fn rebuild(
        &mut self,
        tasks: &HashMap<TaskId, Task>,
        anchors: &HashMap<AnchorId, Anchor>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;

        // Clear existing data
        tx.execute("DELETE FROM dependencies", [])?;
        tx.execute("DELETE FROM tasks", [])?;
        tx.execute("DELETE FROM briefs", [])?;

        // Insert tasks
        {
            let mut stmt = tx.prepare(
                "INSERT INTO tasks (id, anchor_id, title, status, created_at, updated_at, completed_at, description, meta, depends_on)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;

            for task in tasks.values() {
                let anchor_id = task.anchor_id().map(|a| a.to_string());
                let status = match task.status {
                    TaskStatus::Todo => "todo",
                    TaskStatus::InProgress => "in_progress",
                    TaskStatus::Done => "done",
                };
                let completed_at = task.completed_at.map(|t| t.to_rfc3339());
                let meta = if task.meta.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&task.meta)?)
                };
                // Serialize all dependencies (with type info) for the depends_on column
                let depends_on_json = if task.depends_on.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&task.depends_on)?)
                };

                stmt.execute(params![
                    task.id.to_string(),
                    anchor_id,
                    task.title,
                    status,
                    task.created_at.to_rfc3339(),
                    task.updated_at.to_rfc3339(),
                    completed_at,
                    task.description,
                    meta,
                    depends_on_json,
                ])?;
            }
        }

        // Insert blocking dependencies (only blocking dependencies affect ready/blocked queries)
        {
            let mut stmt =
                tx.prepare("INSERT INTO dependencies (task_id, depends_on_id) VALUES (?1, ?2)")?;

            for task in tasks.values() {
                for dep_id in task.depends_on.blocking_task_ids() {
                    stmt.execute(params![task.id.to_string(), dep_id.to_string()])?;
                }
            }
        }

        // Insert briefs (anchors)
        {
            let mut stmt = tx.prepare(
                "INSERT INTO briefs (id, title, brief_type, status, created_at, updated_at, file_path, body)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;

            for anchor in anchors.values() {
                let status = anchor.status.to_string();
                let file_path = format!("{}.md", anchor.id);

                stmt.execute(params![
                    anchor.id.to_string(),
                    anchor.title,
                    anchor.anchor_type,
                    status,
                    anchor.created_at.to_rfc3339(),
                    anchor.updated_at.to_rfc3339(),
                    file_path,
                    anchor.body,
                ])?;
            }
        }

        tx.commit()?;

        self.update_cache_mtime()?;

        Ok(())
    }

    /// Query: Get all tasks with a specific status
    pub fn tasks_by_status(&self, status: TaskStatus) -> Result<Vec<String>> {
        let status_str = match status {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        };

        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks WHERE status = ?1")?;
        let ids: Vec<String> = stmt
            .query_map(params![status_str], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Query: Get all tasks for an anchor
    pub fn tasks_for_anchor(&self, anchor_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks WHERE anchor_id = ?1")?;
        let ids: Vec<String> = stmt
            .query_map(params![anchor_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Query: Get all standalone tasks
    pub fn standalone_tasks(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks WHERE anchor_id IS NULL")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Query: Get task counts by status
    pub fn task_counts(&self) -> Result<(usize, usize, usize)> {
        let todo: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'",
            [],
            |row| row.get(0),
        )?;

        let in_progress: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'",
            [],
            |row| row.get(0),
        )?;

        let done: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'done'",
            [],
            |row| row.get(0),
        )?;

        Ok((todo as usize, in_progress as usize, done as usize))
    }

    /// Query: Get anchor counts by status
    pub fn anchor_counts(&self) -> Result<HashMap<String, usize>> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM briefs GROUP BY status")?;

        let mut counts = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            let (status, count) = row?;
            counts.insert(status, count as usize);
        }

        Ok(counts)
    }

    /// Query: Get ready task IDs (no incomplete dependencies)
    pub fn ready_task_ids(&self) -> Result<Vec<String>> {
        // Tasks that are not done and have no incomplete dependencies
        let mut stmt = self.conn.prepare(
            "SELECT t.id FROM tasks t
             WHERE t.status != 'done'
             AND NOT EXISTS (
                 SELECT 1 FROM dependencies d
                 JOIN tasks dep ON d.depends_on_id = dep.id
                 WHERE d.task_id = t.id
                 AND dep.status != 'done'
             )",
        )?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Query: Get blocked task IDs (has incomplete dependencies)
    pub fn blocked_task_ids(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT t.id FROM tasks t
             JOIN dependencies d ON d.task_id = t.id
             JOIN tasks dep ON d.depends_on_id = dep.id
             WHERE t.status != 'done'
             AND dep.status != 'done'",
        )?;

        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Query: Get ready task IDs with details
    pub fn ready_tasks_detailed(&self) -> Result<Vec<CachedTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.anchor_id, t.title, t.status, t.description
             FROM tasks t
             WHERE t.status != 'done'
             AND NOT EXISTS (
                 SELECT 1 FROM dependencies d
                 JOIN tasks dep ON d.depends_on_id = dep.id
                 WHERE d.task_id = t.id
                 AND dep.status != 'done'
             )",
        )?;

        let tasks = stmt
            .query_map([], |row| {
                Ok(CachedTask {
                    id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Query: Get ready tasks filtered by anchor
    pub fn ready_tasks_for_anchor(&self, anchor_id: &str) -> Result<Vec<CachedTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.anchor_id, t.title, t.status, t.description
             FROM tasks t
             WHERE t.anchor_id = ?1
             AND t.status != 'done'
             AND NOT EXISTS (
                 SELECT 1 FROM dependencies d
                 JOIN tasks dep ON d.depends_on_id = dep.id
                 WHERE d.task_id = t.id
                 AND dep.status != 'done'
             )",
        )?;

        let tasks = stmt
            .query_map(params![anchor_id], |row| {
                Ok(CachedTask {
                    id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    /// Query: Get blocked tasks with what they're blocked by
    pub fn blocked_tasks_detailed(&self) -> Result<Vec<(CachedTask, Vec<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT t.id, t.anchor_id, t.title, t.status, t.description
             FROM tasks t
             JOIN dependencies d ON d.task_id = t.id
             JOIN tasks dep ON d.depends_on_id = dep.id
             WHERE t.status != 'done'
             AND dep.status != 'done'",
        )?;

        let tasks: Vec<CachedTask> = stmt
            .query_map([], |row| {
                Ok(CachedTask {
                    id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // For each blocked task, get what it's blocked by
        let mut blocked_stmt = self.conn.prepare(
            "SELECT dep.id FROM dependencies d
             JOIN tasks dep ON d.depends_on_id = dep.id
             WHERE d.task_id = ?1
             AND dep.status != 'done'",
        )?;

        let mut result = Vec::new();
        for task in tasks {
            let blockers: Vec<String> = blocked_stmt
                .query_map(params![&task.id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            result.push((task, blockers));
        }

        Ok(result)
    }

    /// Query: Get blocked tasks filtered by anchor
    pub fn blocked_tasks_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<Vec<(CachedTask, Vec<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT t.id, t.anchor_id, t.title, t.status, t.description
             FROM tasks t
             JOIN dependencies d ON d.task_id = t.id
             JOIN tasks dep ON d.depends_on_id = dep.id
             WHERE t.anchor_id = ?1
             AND t.status != 'done'
             AND dep.status != 'done'",
        )?;

        let tasks: Vec<CachedTask> = stmt
            .query_map(params![anchor_id], |row| {
                Ok(CachedTask {
                    id: row.get(0)?,
                    anchor_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    description: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut blocked_stmt = self.conn.prepare(
            "SELECT dep.id FROM dependencies d
             JOIN tasks dep ON d.depends_on_id = dep.id
             WHERE d.task_id = ?1
             AND dep.status != 'done'",
        )?;

        let mut result = Vec::new();
        for task in tasks {
            let blockers: Vec<String> = blocked_stmt
                .query_map(params![&task.id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            result.push((task, blockers));
        }

        Ok(result)
    }

    /// Query: Get standalone task counts
    pub fn standalone_task_counts(&self) -> Result<(usize, usize, usize)> {
        let todo: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE anchor_id IS NULL AND status = 'todo'",
            [],
            |row| row.get(0),
        )?;

        let in_progress: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE anchor_id IS NULL AND status = 'in_progress'",
            [],
            |row| row.get(0),
        )?;

        let done: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE anchor_id IS NULL AND status = 'done'",
            [],
            |row| row.get(0),
        )?;

        Ok((todo as usize, in_progress as usize, done as usize))
    }

    /// Query: List all anchors with basic info
    pub fn list_anchors(&self) -> Result<Vec<CachedAnchor>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, brief_type, status FROM briefs ORDER BY id")?;

        let anchors = stmt
            .query_map([], |row| {
                Ok(CachedAnchor {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    brief_type: row.get(2)?,
                    status: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(anchors)
    }

    /// Query: Full-text search across tasks and briefs
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Search tasks
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, title, snippet(tasks_fts, 2, '<mark>', '</mark>', '...', 32)
                 FROM tasks_fts WHERE tasks_fts MATCH ?1
                 ORDER BY rank LIMIT 50",
            )?;

            let rows = stmt.query_map(params![query], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    result_type: SearchResultType::Task,
                })
            })?;

            for row in rows {
                results.push(row?);
            }
        }

        // Search briefs
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, title, snippet(briefs_fts, 2, '<mark>', '</mark>', '...', 32)
                 FROM briefs_fts WHERE briefs_fts MATCH ?1
                 ORDER BY rank LIMIT 50",
            )?;

            let rows = stmt.query_map(params![query], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    result_type: SearchResultType::Brief,
                })
            })?;

            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }

    /// Returns the path to the cache database
    pub fn path(&self) -> &Path {
        &self.db_path
    }
}

/// Result from a search query
#[derive(Debug)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub result_type: SearchResultType,
}

/// Type of search result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchResultType {
    Task,
    Brief,
}

/// Cached task information for quick queries
#[derive(Debug, Clone)]
pub struct CachedTask {
    pub id: String,
    pub anchor_id: Option<String>,
    pub title: String,
    pub status: String,
    pub description: Option<String>,
}

impl CachedTask {
    /// Returns true if this is a standalone task
    pub fn is_standalone(&self) -> bool {
        self.anchor_id.is_none()
    }
}

/// Cached anchor information for quick queries
#[derive(Debug, Clone)]
pub struct CachedAnchor {
    pub id: String,
    pub title: String,
    pub brief_type: String,
    pub status: String,
}

impl CachedAnchor {
    /// Returns true if this anchor is active
    pub fn is_active(&self) -> bool {
        self.status == "in_progress"
    }

    /// Returns true if this anchor is complete
    pub fn is_complete(&self) -> bool {
        self.status == "shipped" || self.status == "archived"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AnchorStatus;
    use chrono::Utc;
    use tempfile::TempDir;

    fn setup_project() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().to_path_buf();

        // Create .shape directory
        fs::create_dir_all(project_root.join(".shape").join("anchors")).unwrap();

        (dir, project_root)
    }

    fn make_task(seq: u32, title: &str) -> Task {
        let anchor = AnchorId::new("Test", Utc::now());
        let task_id = TaskId::new(&anchor, seq);
        Task::new(task_id, title)
    }

    #[test]
    fn test_cache_creation() {
        let (_dir, project_root) = setup_project();
        let cache = Cache::open(&project_root).unwrap();

        assert!(cache.path().exists());
    }

    #[test]
    fn test_cache_rebuild() {
        let (_dir, project_root) = setup_project();
        let mut cache = Cache::open(&project_root).unwrap();

        let mut tasks = HashMap::new();
        let task1 = make_task(1, "First task");
        let task2 = make_task(2, "Second task");
        tasks.insert(task1.id.clone(), task1);
        tasks.insert(task2.id.clone(), task2);

        let anchors = HashMap::new();

        cache.rebuild(&tasks, &anchors).unwrap();

        // Query tasks
        let (todo, in_progress, done) = cache.task_counts().unwrap();
        assert_eq!(todo, 2);
        assert_eq!(in_progress, 0);
        assert_eq!(done, 0);
    }

    #[test]
    fn test_ready_tasks() {
        let (_dir, project_root) = setup_project();
        let mut cache = Cache::open(&project_root).unwrap();

        let task1 = make_task(1, "First task");
        let mut task2 = make_task(2, "Second task");
        task2.add_dependency(task1.id.clone());

        let mut tasks = HashMap::new();
        tasks.insert(task1.id.clone(), task1.clone());
        tasks.insert(task2.id.clone(), task2.clone());

        cache.rebuild(&tasks, &HashMap::new()).unwrap();

        let ready = cache.ready_task_ids().unwrap();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&task1.id.to_string()));

        let blocked = cache.blocked_task_ids().unwrap();
        assert_eq!(blocked.len(), 1);
        assert!(blocked.contains(&task2.id.to_string()));
    }

    #[test]
    fn test_search() {
        let (_dir, project_root) = setup_project();
        let mut cache = Cache::open(&project_root).unwrap();

        let mut task1 = make_task(1, "Fix authentication bug");
        task1.description = Some("User cannot login with SSO".to_string());

        let task2 = make_task(2, "Add dashboard feature");

        let mut tasks = HashMap::new();
        tasks.insert(task1.id.clone(), task1);
        tasks.insert(task2.id.clone(), task2);

        cache.rebuild(&tasks, &HashMap::new()).unwrap();

        // Search for authentication
        let results = cache.search("authentication").unwrap();
        assert!(!results.is_empty());
        assert!(results[0].title.contains("authentication"));

        // Search for SSO (in description)
        let results = cache.search("SSO").unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_anchor_storage() {
        let (_dir, project_root) = setup_project();
        let mut cache = Cache::open(&project_root).unwrap();

        let mut anchor = Anchor::new("Test Pitch", "shapeup");
        anchor.set_body("This is the problem statement.");
        anchor.set_status(AnchorStatus::InProgress);

        let mut anchors = HashMap::new();
        anchors.insert(anchor.id.clone(), anchor);

        cache.rebuild(&HashMap::new(), &anchors).unwrap();

        let counts = cache.anchor_counts().unwrap();
        assert_eq!(counts.get("in_progress"), Some(&1));
    }

    #[test]
    fn test_schema_version() {
        let (_dir, project_root) = setup_project();
        let cache = Cache::open(&project_root).unwrap();

        let version = cache.get_schema_version().unwrap();
        assert_eq!(version, Cache::SCHEMA_VERSION);
    }
}
