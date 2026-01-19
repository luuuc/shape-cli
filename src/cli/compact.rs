//! Compact CLI command - Memory decay and compaction for completed tasks

use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, Utc};

use super::output::Output;
use crate::domain::{AnchorId, TaskId};
use crate::storage::{CompactionStrategy, Project};

/// Result of a compaction operation
#[derive(Debug)]
pub struct CompactionResult {
    /// Groups of tasks that were compacted
    pub groups: Vec<CompactedGroup>,
    /// Total tasks compacted
    pub total_compacted: usize,
    /// Whether this was a dry run
    pub dry_run: bool,
}

/// A group of tasks compacted together
#[derive(Debug)]
pub struct CompactedGroup {
    /// The representative task ID (holds the summary)
    pub representative_id: TaskId,
    /// Summary generated for this group
    pub summary: String,
    /// IDs of all tasks in this group
    pub task_ids: Vec<TaskId>,
    /// Anchor ID these tasks belong to (None for standalone)
    pub anchor_id: Option<AnchorId>,
}

/// Candidate task info collected before mutation.
///
/// This struct exists to work around Rust's borrow checker: we need to collect
/// task information while iterating over `tasks`, then later mutate `tasks`.
/// By copying the needed fields into owned values, we release the immutable
/// borrow and can safely mutate afterward.
struct CandidateInfo {
    id: TaskId,
    title: String,
    anchor_id: Option<AnchorId>,
}

/// Run the compact command
pub fn run(
    output: &Output,
    days: u32,
    anchor_filter: Option<&str>,
    dry_run: bool,
    strategy: Option<&str>,
) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let config = project.config();

    // Determine strategy from CLI or config
    let strategy = match strategy {
        Some("basic") => CompactionStrategy::Basic,
        Some("smart") => CompactionStrategy::Smart,
        Some("llm") => CompactionStrategy::Llm,
        Some(other) => anyhow::bail!("Unknown strategy: {}. Use 'basic', 'smart', or 'llm'", other),
        None => config.project.compaction.strategy.clone(),
    };

    output.verbose_ctx("compact", &format!(
        "Using strategy: {}, days threshold: {}, min_tasks: {}",
        strategy.as_str(),
        days,
        config.project.compaction.min_tasks
    ));

    let mut tasks = store.read_all()?;
    let cutoff = Utc::now() - Duration::days(days as i64);

    // Collect candidate info without holding references
    let mut candidates: Vec<CandidateInfo> = tasks
        .values()
        .filter(|t| {
            t.status.is_complete()
                && !t.is_compacted()
                && !t.is_compaction_representative()
                && t.completed_at.map(|c| c < cutoff).unwrap_or(false)
        })
        .map(|t| CandidateInfo {
            id: t.id.clone(),
            title: t.title.clone(),
            anchor_id: t.anchor_id(),
        })
        .collect();

    // Apply anchor filter if specified
    if let Some(anchor_str) = anchor_filter {
        let anchor_id: AnchorId = anchor_str.parse()?;
        candidates.retain(|c| c.anchor_id.as_ref() == Some(&anchor_id));
        output.verbose_ctx("compact", &format!(
            "Filtered to {} candidates for anchor {}",
            candidates.len(),
            anchor_id
        ));
    }

    if candidates.is_empty() {
        if output.is_json() {
            output.data(&serde_json::json!({
                "compacted": 0,
                "groups": [],
                "dry_run": dry_run,
            }));
        } else {
            println!("No tasks to compact (completed tasks older than {} days)", days);
        }
        return Ok(());
    }

    // Group candidates by anchor (or standalone)
    let mut by_anchor: HashMap<Option<AnchorId>, Vec<CandidateInfo>> = HashMap::new();
    for candidate in candidates {
        by_anchor.entry(candidate.anchor_id.clone()).or_default().push(candidate);
    }

    let min_tasks = config.project.compaction.min_tasks;
    let mut result = CompactionResult {
        groups: Vec::new(),
        total_compacted: 0,
        dry_run,
    };

    for (anchor_id, anchor_candidates) in by_anchor {
        // Skip groups that don't meet minimum threshold
        if anchor_candidates.len() < min_tasks {
            output.verbose_ctx("compact", &format!(
                "Skipping group with {} tasks (min: {})",
                anchor_candidates.len(),
                min_tasks
            ));
            continue;
        }

        // Generate summary based on strategy
        let summary = generate_summary_from_titles(
            &anchor_candidates.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(),
            &strategy,
        );

        // Collect task IDs
        let task_ids: Vec<TaskId> = anchor_candidates.iter().map(|c| c.id.clone()).collect();

        // First task becomes the representative.
        // SAFETY: task_ids is guaranteed non-empty because we skip groups with
        // fewer than `min_tasks` (default 3) at line 134 above.
        let representative_id = task_ids[0].clone();

        result.groups.push(CompactedGroup {
            representative_id: representative_id.clone(),
            summary: summary.clone(),
            task_ids: task_ids.clone(),
            anchor_id: anchor_id.clone(),
        });

        result.total_compacted += task_ids.len();

        // Apply changes if not dry run
        if !dry_run {
            // Mark representative with summary and compacted task list
            if let Some(rep) = tasks.get_mut(&representative_id) {
                rep.set_compaction(summary.clone(), task_ids.clone());
            }

            // Mark other tasks as compacted into representative
            for task_id in task_ids.iter().skip(1) {
                if let Some(task) = tasks.get_mut(task_id) {
                    task.compact_into(representative_id.clone());
                }
            }
        }
    }

    // Save changes if not dry run
    if !dry_run && result.total_compacted > 0 {
        store.write_all(&tasks)?;
    }

    // Output results
    if output.is_json() {
        let groups_json: Vec<_> = result
            .groups
            .iter()
            .map(|g| {
                serde_json::json!({
                    "representative_id": g.representative_id.to_string(),
                    "summary": g.summary,
                    "task_count": g.task_ids.len(),
                    "task_ids": g.task_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                    "anchor_id": g.anchor_id.as_ref().map(|a| a.to_string()),
                })
            })
            .collect();

        output.data(&serde_json::json!({
            "compacted": result.total_compacted,
            "groups": groups_json,
            "dry_run": result.dry_run,
        }));
    } else {
        let action = if dry_run { "Would compact" } else { "Compacted" };

        if result.groups.is_empty() {
            println!("No groups meet the minimum size threshold ({})", min_tasks);
        } else {
            for group in &result.groups {
                let anchor_label = group
                    .anchor_id
                    .as_ref()
                    .map(|a| format!("anchor {}", a))
                    .unwrap_or_else(|| "standalone".to_string());

                println!(
                    "{} {} tasks from {} into {}:",
                    action,
                    group.task_ids.len(),
                    anchor_label,
                    group.representative_id
                );
                println!("  Summary: {}", group.summary);
                println!();
            }

            println!(
                "Total: {} {} tasks in {} group(s)",
                action.to_lowercase(),
                result.total_compacted,
                result.groups.len()
            );

            if dry_run {
                println!("\nRun without --dry-run to apply changes.");
            }
        }
    }

    Ok(())
}

/// Undo compaction for a specific task
pub fn undo(output: &Output, task_id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let task_id: TaskId = task_id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get(&task_id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

    if !task.is_compaction_representative() {
        anyhow::bail!("Task {} is not a compaction representative", task_id);
    }

    let compacted_ids = task
        .compacted_tasks
        .clone()
        .unwrap_or_default();

    if compacted_ids.is_empty() {
        anyhow::bail!("Task {} has no compacted tasks", task_id);
    }

    output.verbose_ctx("compact", &format!(
        "Undoing compaction for {} tasks",
        compacted_ids.len()
    ));

    // Clear compaction data from representative
    if let Some(rep) = tasks.get_mut(&task_id) {
        rep.clear_compaction();
    }

    // Clear compacted_into from other tasks
    for other_id in compacted_ids.iter().skip(1) {
        if let Some(other_task) = tasks.get_mut(other_id) {
            other_task.clear_compaction();
        }
    }

    store.write_all(&tasks)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "undone": task_id.to_string(),
            "restored_tasks": compacted_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        }));
    } else {
        output.success(&format!(
            "Undone compaction: {} tasks restored",
            compacted_ids.len()
        ));
    }

    Ok(())
}

/// Generate summary for a group of tasks based on strategy
fn generate_summary_from_titles(titles: &[&str], strategy: &CompactionStrategy) -> String {
    match strategy {
        CompactionStrategy::Basic => generate_basic_summary(titles),
        CompactionStrategy::Smart => generate_smart_summary(titles),
        CompactionStrategy::Llm => {
            // LLM summarization requires additional setup; fall back to smart
            generate_smart_summary(titles)
        }
    }
}

/// Basic strategy: concatenate task titles
fn generate_basic_summary(titles: &[&str]) -> String {
    if titles.len() <= 3 {
        format!("Completed {} tasks: {}", titles.len(), titles.join(", "))
    } else {
        format!(
            "Completed {} tasks: {}, and {} more",
            titles.len(),
            titles[..3].join(", "),
            titles.len() - 3
        )
    }
}

/// Smart strategy: group by common words and generate concise summary
fn generate_smart_summary(titles: &[&str]) -> String {
    // Extract common words from titles
    let mut word_counts: HashMap<String, usize> = HashMap::new();

    for title in titles {
        let words: Vec<String> = title
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)  // Skip short words
            .filter(|w| !is_stop_word(w))
            .map(String::from)
            .collect();

        for word in words {
            *word_counts.entry(word).or_insert(0) += 1;
        }
    }

    // Find words that appear in multiple tasks (at least 40% of tasks)
    let threshold = (titles.len() as f64 * 0.4).ceil() as usize;
    let mut common_words: Vec<_> = word_counts
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .collect();

    common_words.sort_by(|a, b| b.1.cmp(&a.1));

    if common_words.is_empty() {
        // Fall back to basic if no common theme found
        return generate_basic_summary(titles);
    }

    // Build summary from common words
    let theme_words: Vec<String> = common_words
        .into_iter()
        .take(3)
        .map(|(word, _)| capitalize(&word))
        .collect();

    let theme = theme_words.join(" ");

    format!("{}: {} tasks completed", theme, titles.len())
}

/// Check if a word is a common stop word
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
        "be", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "must", "shall", "can", "need",
        "this", "that", "these", "those", "it", "its", "add", "update",
        "fix", "implement", "create", "remove", "delete", "change",
    ];
    STOP_WORDS.contains(&word)
}

/// Capitalize first letter of a word
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_summary_short() {
        let titles = vec!["Set up database", "Create user model"];

        let summary = generate_basic_summary(&titles);
        assert!(summary.contains("2 tasks"));
        assert!(summary.contains("Set up database"));
        assert!(summary.contains("Create user model"));
    }

    #[test]
    fn basic_summary_long() {
        let titles = vec![
            "Task one",
            "Task two",
            "Task three",
            "Task four",
            "Task five",
        ];

        let summary = generate_basic_summary(&titles);
        assert!(summary.contains("5 tasks"));
        assert!(summary.contains("and 2 more"));
    }

    #[test]
    fn smart_summary_common_theme() {
        let titles = vec![
            "Authentication login page",
            "Authentication session handling",
            "Authentication token refresh",
            "Authentication logout flow",
        ];

        let summary = generate_smart_summary(&titles);
        assert!(summary.contains("Authentication"));
        assert!(summary.contains("4 tasks"));
    }

    #[test]
    fn smart_summary_fallback() {
        let titles = vec!["Fix bug", "Add feature", "Update docs"];

        // These have no common theme, should fall back to basic
        let summary = generate_smart_summary(&titles);
        assert!(summary.contains("3 tasks"));
    }

    #[test]
    fn is_stop_word_works() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("add"));
        assert!(!is_stop_word("authentication"));
        assert!(!is_stop_word("database"));
    }

    #[test]
    fn capitalize_works() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("WORLD"), "WORLD");
        assert_eq!(capitalize(""), "");
    }
}
