//! Git merge driver for tasks.jsonl
//!
//! This module implements a custom git merge driver for JSONL files.
//! Git calls this driver with three file paths: base, ours, theirs.
//! We merge task-by-task using field-level timestamps.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::domain::{merge_tasks, Task, TaskId};

/// Git merge driver result codes
pub const MERGE_SUCCESS: i32 = 0;
pub const MERGE_CONFLICT: i32 = 1;

/// Runs the merge driver
///
/// Git calls: shape merge-driver %O %A %B
/// - %O = base (common ancestor)
/// - %A = ours (current branch) - output should be written here
/// - %B = theirs (branch being merged)
pub fn run_merge_driver(base_path: &Path, ours_path: &Path, theirs_path: &Path) -> Result<i32> {
    // Read all three versions
    let base_tasks = read_tasks_file(base_path)?;
    let ours_tasks = read_tasks_file(ours_path)?;
    let theirs_tasks = read_tasks_file(theirs_path)?;

    // Collect all task IDs from all three versions
    let mut all_ids: Vec<TaskId> = base_tasks
        .keys()
        .chain(ours_tasks.keys())
        .chain(theirs_tasks.keys())
        .cloned()
        .collect();
    all_ids.sort_by_key(|a| a.to_string());
    all_ids.dedup();

    let mut merged_tasks: Vec<Task> = Vec::new();
    let mut had_conflicts = false;

    for id in all_ids {
        let base = base_tasks.get(&id);
        let ours = ours_tasks.get(&id);
        let theirs = theirs_tasks.get(&id);

        match (base, ours, theirs) {
            // Task exists in all three - merge
            (Some(b), Some(o), Some(t)) => {
                let result = merge_tasks(b, o, t);
                if result.had_conflicts {
                    had_conflicts = true;
                    eprintln!(
                        "Merge conflict in task {}: ours={:?}, theirs={:?}",
                        id, result.ours_fields, result.theirs_fields
                    );
                }
                merged_tasks.push(result.task);
            }

            // Added in ours only
            (None, Some(o), None) => {
                merged_tasks.push(o.clone());
            }

            // Added in theirs only
            (None, None, Some(t)) => {
                merged_tasks.push(t.clone());
            }

            // Added in both (independently) - this is a conflict
            (None, Some(o), Some(t)) => {
                // Use last-write-wins based on created_at
                if o.created_at >= t.created_at {
                    merged_tasks.push(o.clone());
                } else {
                    merged_tasks.push(t.clone());
                }
                had_conflicts = true;
                eprintln!(
                    "Merge conflict: task {} created independently in both branches",
                    id
                );
            }

            // Deleted in ours, unchanged in theirs - keep deleted
            (Some(b), None, Some(t)) if tasks_equal(b, t) => {
                // Deleted in ours, theirs unchanged - keep deleted
            }

            // Deleted in theirs, unchanged in ours - keep deleted
            (Some(b), Some(o), None) if tasks_equal(b, o) => {
                // Deleted in theirs, ours unchanged - keep deleted
            }

            // Deleted in ours, modified in theirs - conflict, keep theirs
            (Some(_), None, Some(t)) => {
                merged_tasks.push(t.clone());
                had_conflicts = true;
                eprintln!(
                    "Merge conflict: task {} deleted in ours but modified in theirs",
                    id
                );
            }

            // Deleted in theirs, modified in ours - conflict, keep ours
            (Some(_), Some(o), None) => {
                merged_tasks.push(o.clone());
                had_conflicts = true;
                eprintln!(
                    "Merge conflict: task {} deleted in theirs but modified in ours",
                    id
                );
            }

            // Deleted in both - keep deleted
            (Some(_), None, None) => {
                // Both deleted, nothing to do
            }

            // Should not happen
            (None, None, None) => {}
        }
    }

    // Sort tasks by ID for consistent output
    merged_tasks.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

    // Write merged result to ours_path (git expects output there)
    write_tasks_file(ours_path, &merged_tasks)?;

    // Return appropriate exit code
    if had_conflicts {
        Ok(MERGE_CONFLICT)
    } else {
        Ok(MERGE_SUCCESS)
    }
}

/// Reads tasks from a JSONL file
fn read_tasks_file(path: &Path) -> Result<HashMap<TaskId, Task>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    let mut tasks = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let task: Task = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse task at line {}", line_num + 1))?;

        tasks.insert(task.id.clone(), task);
    }

    Ok(tasks)
}

/// Writes tasks to a JSONL file
fn write_tasks_file(path: &Path, tasks: &[Task]) -> Result<()> {
    let mut content = String::new();

    for task in tasks {
        let line = serde_json::to_string(task).context("Failed to serialize task")?;
        content.push_str(&line);
        content.push('\n');
    }

    fs::write(path, content)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}

/// Checks if two tasks are equal (for delete detection)
fn tasks_equal(a: &Task, b: &Task) -> bool {
    // Compare by serialization for simplicity
    serde_json::to_string(a).ok() == serde_json::to_string(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AnchorId, TaskId};
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_task(anchor: &AnchorId, seq: u32, title: &str) -> Task {
        let task_id = TaskId::new(anchor, seq);
        Task::new(task_id, title)
    }

    fn write_test_file(dir: &Path, name: &str, tasks: &[Task]) -> std::path::PathBuf {
        let path = dir.join(name);
        write_tasks_file(&path, tasks).unwrap();
        path
    }

    #[test]
    fn merge_simple_no_conflict() {
        let dir = TempDir::new().unwrap();
        let anchor = AnchorId::new("Test", Utc::now());

        let task1 = make_task(&anchor, 1, "Task 1");
        let task2 = make_task(&anchor, 2, "Task 2");

        // Base has task1
        let base_path = write_test_file(dir.path(), "base", std::slice::from_ref(&task1));

        // Ours has task1 (unchanged)
        let ours_path = write_test_file(dir.path(), "ours", std::slice::from_ref(&task1));

        // Theirs adds task2
        let theirs_path = write_test_file(dir.path(), "theirs", &[task1.clone(), task2.clone()]);

        let result = run_merge_driver(&base_path, &ours_path, &theirs_path).unwrap();

        assert_eq!(result, MERGE_SUCCESS);

        // Read merged result
        let merged = read_tasks_file(&ours_path).unwrap();
        assert_eq!(merged.len(), 2);
        assert!(merged.contains_key(&task1.id));
        assert!(merged.contains_key(&task2.id));
    }

    #[test]
    fn merge_both_modify_same_task() {
        let dir = TempDir::new().unwrap();
        let anchor = AnchorId::new("Test", Utc::now());

        let task1 = make_task(&anchor, 1, "Original");

        // Base has original task
        let base_path = write_test_file(dir.path(), "base", std::slice::from_ref(&task1));

        // Ours modifies title
        let mut ours_task = task1.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        ours_task.set_title("Ours title");
        let ours_path = write_test_file(dir.path(), "ours", &[ours_task.clone()]);

        // Theirs modifies description
        let mut theirs_task = task1.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        theirs_task.set_description("Theirs description");
        let theirs_path = write_test_file(dir.path(), "theirs", &[theirs_task.clone()]);

        let result = run_merge_driver(&base_path, &ours_path, &theirs_path).unwrap();

        // No conflict since different fields were modified
        assert_eq!(result, MERGE_SUCCESS);

        let merged = read_tasks_file(&ours_path).unwrap();
        let merged_task = merged.get(&task1.id).unwrap();
        assert_eq!(merged_task.title, "Ours title");
        assert_eq!(
            merged_task.description,
            Some("Theirs description".to_string())
        );
    }

    #[test]
    fn merge_delete_vs_modify_conflict() {
        let dir = TempDir::new().unwrap();
        let anchor = AnchorId::new("Test", Utc::now());

        let task1 = make_task(&anchor, 1, "Task 1");

        // Base has task1
        let base_path = write_test_file(dir.path(), "base", std::slice::from_ref(&task1));

        // Ours deletes task1
        let ours_path = write_test_file(dir.path(), "ours", &[]);

        // Theirs modifies task1
        let mut theirs_task = task1.clone();
        theirs_task.set_description("Modified");
        let theirs_path = write_test_file(dir.path(), "theirs", &[theirs_task.clone()]);

        let result = run_merge_driver(&base_path, &ours_path, &theirs_path).unwrap();

        // Should be conflict
        assert_eq!(result, MERGE_CONFLICT);

        // Modified version should be kept
        let merged = read_tasks_file(&ours_path).unwrap();
        assert!(merged.contains_key(&task1.id));
    }
}
