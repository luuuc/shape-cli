//! Field-level three-way merge for tasks
//!
//! Implements "last-write-wins" merge using per-field timestamps from the
//! `_v` (versions) field. For each field, we compare the timestamp from
//! "ours" and "theirs" to determine which value to keep.

use std::collections::HashSet;

use super::task::{Task, TaskMeta};

/// Result of merging two tasks
#[derive(Debug)]
pub struct MergeResult {
    /// The merged task
    pub task: Task,

    /// Whether there were any conflicts (multiple fields modified concurrently)
    pub had_conflicts: bool,

    /// Which fields came from "ours"
    pub ours_fields: Vec<String>,

    /// Which fields came from "theirs"
    pub theirs_fields: Vec<String>,
}

/// Merges two concurrent edits of the same task using last-write-wins
///
/// # Arguments
///
/// * `base` - The common ancestor (before either edit)
/// * `ours` - Our version (local changes)
/// * `theirs` - Their version (remote changes)
///
/// # Returns
///
/// A `MergeResult` containing the merged task and information about which
/// fields came from which version.
pub fn merge_tasks(base: &Task, ours: &Task, theirs: &Task) -> MergeResult {
    // Verify we're merging the same task
    assert_eq!(base.id, ours.id);
    assert_eq!(base.id, theirs.id);

    let mut merged = base.clone();
    let mut had_conflicts = false;
    let mut ours_fields = Vec::new();
    let mut theirs_fields = Vec::new();

    // Helper to pick the newer version of a field
    macro_rules! merge_field {
        ($field:ident, $field_name:expr, $touch:ident) => {
            let ours_v = ours.versions.$field;
            let theirs_v = theirs.versions.$field;
            let base_v = base.versions.$field;

            // Detect if both sides modified this field
            let ours_changed = ours_v > base_v;
            let theirs_changed = theirs_v > base_v;

            if ours_changed && theirs_changed {
                // Both modified - this is a conflict, pick newer
                had_conflicts = true;
                if ours_v >= theirs_v {
                    merged.$field = ours.$field.clone();
                    merged.versions.$field = ours_v;
                    ours_fields.push($field_name.to_string());
                } else {
                    merged.$field = theirs.$field.clone();
                    merged.versions.$field = theirs_v;
                    theirs_fields.push($field_name.to_string());
                }
            } else if ours_changed {
                // Only ours changed
                merged.$field = ours.$field.clone();
                merged.versions.$field = ours_v;
                ours_fields.push($field_name.to_string());
            } else if theirs_changed {
                // Only theirs changed
                merged.$field = theirs.$field.clone();
                merged.versions.$field = theirs_v;
                theirs_fields.push($field_name.to_string());
            }
            // else: neither changed, keep base
        };
    }

    // Merge core fields
    merge_field!(title, "title", touch_title);
    merge_field!(status, "status", touch_status);
    merge_field!(description, "description", touch_description);
    merge_field!(completed_at, "completed_at", touch_completed_at);

    // Merge dependencies (set union - additions win)
    use super::task::{Dependencies, Dependency};

    let ours_deps: HashSet<Dependency> = ours.depends_on.iter().cloned().collect();
    let theirs_deps: HashSet<Dependency> = theirs.depends_on.iter().cloned().collect();
    let base_deps: HashSet<Dependency> = base.depends_on.iter().cloned().collect();

    let ours_added: HashSet<_> = ours_deps.difference(&base_deps).cloned().collect();
    let theirs_added: HashSet<_> = theirs_deps.difference(&base_deps).cloned().collect();
    let ours_removed: HashSet<_> = base_deps.difference(&ours_deps).cloned().collect();
    let theirs_removed: HashSet<_> = base_deps.difference(&theirs_deps).cloned().collect();

    // Start with base, apply additions from both, apply removals only if not re-added
    let mut merged_deps: HashSet<Dependency> = base_deps.clone();
    for dep in ours_added.union(&theirs_added) {
        merged_deps.insert(dep.clone());
    }
    for dep in ours_removed.intersection(&theirs_removed) {
        // Only remove if both sides removed
        merged_deps.remove(dep);
    }

    // Convert HashSet back to Dependencies
    let mut final_deps = Dependencies::new();
    for dep in merged_deps {
        final_deps.add(dep);
    }
    merged.depends_on = final_deps;

    if !ours_added.is_empty() || !ours_removed.is_empty() {
        ours_fields.push("depends_on".to_string());
    }
    if !theirs_added.is_empty() || !theirs_removed.is_empty() {
        theirs_fields.push("depends_on".to_string());
    }

    // Merge metadata per key
    let all_meta_keys: HashSet<_> = base
        .meta
        .keys()
        .chain(ours.meta.keys())
        .chain(theirs.meta.keys())
        .cloned()
        .collect();

    let mut merged_meta = TaskMeta::new();
    let mut merged_versions_meta = std::collections::HashMap::new();

    for key in all_meta_keys {
        let base_v = base.versions.meta_version(&key);
        let ours_v = ours.versions.meta_version(&key);
        let theirs_v = theirs.versions.meta_version(&key);

        let ours_changed = ours_v > base_v;
        let theirs_changed = theirs_v > base_v;

        if ours_changed && theirs_changed {
            // Both modified this key - conflict
            had_conflicts = true;
            if ours_v >= theirs_v {
                if let Some(val) = ours.meta.get(&key) {
                    merged_meta.set(key.clone(), val.clone());
                    merged_versions_meta.insert(key.clone(), ours_v);
                }
                ours_fields.push(format!("meta.{}", key));
            } else {
                if let Some(val) = theirs.meta.get(&key) {
                    merged_meta.set(key.clone(), val.clone());
                    merged_versions_meta.insert(key.clone(), theirs_v);
                }
                theirs_fields.push(format!("meta.{}", key));
            }
        } else if ours_changed {
            if let Some(val) = ours.meta.get(&key) {
                merged_meta.set(key.clone(), val.clone());
                merged_versions_meta.insert(key.clone(), ours_v);
            }
            ours_fields.push(format!("meta.{}", key));
        } else if theirs_changed {
            if let Some(val) = theirs.meta.get(&key) {
                merged_meta.set(key.clone(), val.clone());
                merged_versions_meta.insert(key.clone(), theirs_v);
            }
            theirs_fields.push(format!("meta.{}", key));
        } else {
            // Neither changed - keep base value if it exists
            if let Some(val) = base.meta.get(&key) {
                merged_meta.set(key.clone(), val.clone());
                merged_versions_meta.insert(key.clone(), base_v);
            }
        }
    }

    merged.meta = merged_meta;
    merged.versions.meta = merged_versions_meta;

    // Update the overall updated_at to the max of both
    merged.updated_at = std::cmp::max(ours.updated_at, theirs.updated_at);

    MergeResult {
        task: merged,
        had_conflicts,
        ours_fields,
        theirs_fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BriefId, TaskId, TaskStatus};
    use chrono::Utc;

    fn make_test_task(title: &str) -> Task {
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, 1);
        Task::new(task_id, title)
    }

    #[test]
    fn merge_no_conflicts() {
        let base = make_test_task("Original");

        // Ours changes status
        let mut ours = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        ours.start();

        // Theirs changes description
        let mut theirs = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        theirs.set_description("New description");

        let result = merge_tasks(&base, &ours, &theirs);

        assert!(!result.had_conflicts);
        assert_eq!(result.task.status, TaskStatus::InProgress);
        assert_eq!(result.task.description, Some("New description".to_string()));
    }

    #[test]
    fn merge_conflict_same_field() {
        let base = make_test_task("Original");

        // Both change title
        let mut ours = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        ours.set_title("Ours title");

        let mut theirs = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(10));
        theirs.set_title("Theirs title");

        let result = merge_tasks(&base, &ours, &theirs);

        assert!(result.had_conflicts);
        // Theirs has newer timestamp, should win
        assert_eq!(result.task.title, "Theirs title");
        assert!(result.theirs_fields.contains(&"title".to_string()));
    }

    #[test]
    fn merge_metadata_separate_keys() {
        let base = make_test_task("Task");

        let mut ours = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        ours.set_meta("priority", "high");

        let mut theirs = base.clone();
        std::thread::sleep(std::time::Duration::from_millis(5));
        theirs.set_meta("estimate", 5);

        let result = merge_tasks(&base, &ours, &theirs);

        assert!(!result.had_conflicts);
        assert_eq!(
            result.task.get_meta("priority"),
            Some(&serde_json::json!("high"))
        );
        assert_eq!(
            result.task.get_meta("estimate"),
            Some(&serde_json::json!(5))
        );
    }

    #[test]
    fn merge_dependency_additions() {
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, 1);
        let dep1_id = TaskId::new(&brief, 2);
        let dep2_id = TaskId::new(&brief, 3);

        let base = Task::new(task_id.clone(), "Task");

        let mut ours = base.clone();
        ours.add_dependency(dep1_id.clone());

        let mut theirs = base.clone();
        theirs.add_dependency(dep2_id.clone());

        let result = merge_tasks(&base, &ours, &theirs);

        // Both dependencies should be present (union)
        assert!(result.task.depends_on.contains_blocking(&dep1_id));
        assert!(result.task.depends_on.contains_blocking(&dep2_id));
    }

    #[test]
    fn merge_backward_compat_no_versions() {
        // Simulate old tasks without version tracking
        let brief = BriefId::new("Test", Utc::now());
        let task_id = TaskId::new(&brief, 1);

        let mut base = Task::new(task_id.clone(), "Task");
        base.versions = super::super::task::FieldVersions::from_epoch();

        let mut ours = base.clone();
        ours.title = "Ours".to_string();

        let mut theirs = base.clone();
        theirs.title = "Theirs".to_string();

        // With all zero versions, theirs should win (>= comparison)
        let result = merge_tasks(&base, &ours, &theirs);
        // Neither changed according to versions, so base title is kept
        assert_eq!(result.task.title, "Task");
    }
}
