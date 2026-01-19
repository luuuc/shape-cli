//! Context export for AI agents

use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, Utc};

use super::output::Output;
use crate::domain::{BriefId, DependencyGraph, TaskId, TaskStatus};
use crate::storage::Project;

/// Export project context for AI consumption
pub fn export(
    output: &Output,
    compact: bool,
    brief_filter: Option<&str>,
    days: u32,
) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose_ctx(
        "context",
        &format!("Opened project at: {}", project.root().display()),
    );

    let brief_store = project.brief_store();
    let task_store = project.task_store();

    let briefs = brief_store.read_all()?;
    let tasks = task_store.read_all()?;

    output.verbose_ctx(
        "context",
        &format!("Loaded {} briefs, {} tasks", briefs.len(), tasks.len()),
    );

    // Filter by brief if specified
    let (briefs, tasks) = if let Some(brief_str) = brief_filter {
        let brief_id: BriefId = brief_str.parse()?;
        output.verbose_ctx("context", &format!("Filtering by brief: {}", brief_id));

        let brief = briefs
            .get(&brief_id)
            .ok_or_else(|| anyhow::anyhow!("Brief not found: {}", brief_id))?;

        let filtered_tasks: HashMap<_, _> = tasks
            .into_iter()
            .filter(|(_, t)| t.brief_id().as_ref() == Some(&brief_id))
            .collect();

        output.verbose_ctx(
            "context",
            &format!("Filtered to {} tasks for brief", filtered_tasks.len()),
        );

        let mut filtered_briefs = HashMap::new();
        filtered_briefs.insert(brief_id.clone(), brief.clone());

        (filtered_briefs, filtered_tasks)
    } else {
        (briefs, tasks)
    };

    // Build status map
    let statuses: HashMap<TaskId, TaskStatus> =
        tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();

    // Build dependency graph
    let graph = DependencyGraph::from_tasks(tasks.values())?;

    // Get ready and blocked tasks
    let ready_ids = graph.ready_tasks(&statuses);
    let blocked_ids = graph.blocked_tasks(&statuses);

    // Filter completed tasks by date (excluding compacted tasks)
    let cutoff = Utc::now() - Duration::days(days as i64);
    let recent_completed: Vec<_> = tasks
        .values()
        .filter(|t| {
            t.status.is_complete()
                && !t.is_compacted()
                && !t.is_compaction_representative()
                && t.completed_at.map(|c| c > cutoff).unwrap_or(false)
        })
        .collect();

    // Collect compacted task representatives (summaries)
    let compacted: Vec<_> = tasks
        .values()
        .filter(|t| t.is_compaction_representative())
        .collect();

    // In-progress tasks
    let in_progress: Vec<_> = tasks.values().filter(|t| t.status.is_active()).collect();

    output.verbose_ctx("context", &format!(
        "Context summary: {} ready, {} blocked, {} in_progress, {} recently_completed, {} compacted groups",
        ready_ids.len(), blocked_ids.len(), in_progress.len(), recent_completed.len(), compacted.len()
    ));

    // Collect standalone tasks
    let standalone_tasks: Vec<_> = tasks.values().filter(|t| t.is_standalone()).collect();

    if compact {
        // Compact format - minimal tokens
        export_compact(
            output,
            &briefs,
            &tasks,
            &ready_ids,
            &blocked_ids,
            &in_progress,
            &recent_completed,
            &compacted,
            &standalone_tasks,
        )
    } else {
        // Full format
        export_full(
            output,
            &briefs,
            &tasks,
            &ready_ids,
            &blocked_ids,
            &in_progress,
            &recent_completed,
            &compacted,
            &statuses,
            &standalone_tasks,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn export_compact(
    output: &Output,
    briefs: &HashMap<BriefId, crate::domain::Brief>,
    tasks: &HashMap<TaskId, crate::domain::Task>,
    ready_ids: &[TaskId],
    blocked_ids: &[TaskId],
    in_progress: &[&crate::domain::Task],
    recent_completed: &[&crate::domain::Task],
    compacted: &[&crate::domain::Task],
    standalone_tasks: &[&crate::domain::Task],
) -> Result<()> {
    // Separate standalone tasks by status for the standalone_tasks section
    let standalone_ready: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| ready_ids.contains(&t.id))
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect();

    let standalone_in_progress: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| t.status.is_active())
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect();

    let standalone_blocked: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| blocked_ids.contains(&t.id))
        .map(|t| {
            let deps: Vec<_> = t.depends_on.blocking_task_ids().map(|d| d.to_string()).collect();
            format!("{}: {} (blocked by {})", t.id, t.title, deps.join(", "))
        })
        .collect();

    // Compact format: optimized for token efficiency
    let context = serde_json::json!({
        "briefs": briefs.values().map(|b| {
            serde_json::json!({
                "id": b.id.to_string(),
                "title": b.title,
                "status": b.status,
            })
        }).collect::<Vec<_>>(),

        "ready": ready_ids.iter().filter_map(|id| {
            tasks.get(id).map(|t| format!("{}: {}", t.id, t.title))
        }).collect::<Vec<_>>(),

        "in_progress": in_progress.iter().map(|t| {
            format!("{}: {}", t.id, t.title)
        }).collect::<Vec<_>>(),

        "blocked": blocked_ids.iter().filter_map(|id| {
            tasks.get(id).map(|t| {
                let deps: Vec<_> = t.depends_on.blocking_task_ids().map(|d| d.to_string()).collect();
                format!("{}: {} (blocked by {})", t.id, t.title, deps.join(", "))
            })
        }).collect::<Vec<_>>(),

        "recently_done": recent_completed.iter().map(|t| {
            format!("{}: {}", t.id, t.title)
        }).collect::<Vec<_>>(),

        "compacted": compacted.iter().map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "summary": t.summary.clone().unwrap_or_default(),
                "task_count": t.compacted_count(),
                "completed_at": t.completed_at,
            })
        }).collect::<Vec<_>>(),

        "standalone_tasks": {
            "ready": standalone_ready,
            "in_progress": standalone_in_progress,
            "blocked": standalone_blocked,
        },
    });

    output.data(&context);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn export_full(
    output: &Output,
    briefs: &HashMap<BriefId, crate::domain::Brief>,
    tasks: &HashMap<TaskId, crate::domain::Task>,
    ready_ids: &[TaskId],
    blocked_ids: &[TaskId],
    in_progress: &[&crate::domain::Task],
    recent_completed: &[&crate::domain::Task],
    compacted: &[&crate::domain::Task],
    statuses: &HashMap<TaskId, TaskStatus>,
    standalone_tasks: &[&crate::domain::Task],
) -> Result<()> {
    // Separate standalone tasks by status
    let standalone_ready: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| ready_ids.contains(&t.id))
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "title": t.title,
                "description": t.description,
                "meta": t.meta,
            })
        })
        .collect();

    let standalone_in_progress: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| t.status.is_active())
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "title": t.title,
                "started_at": t.updated_at,
                "description": t.description,
                "meta": t.meta,
            })
        })
        .collect();

    let standalone_blocked: Vec<_> = standalone_tasks
        .iter()
        .filter(|t| blocked_ids.contains(&t.id))
        .map(|t| {
            let blocking: Vec<_> = t
                .depends_on
                .blocking_task_ids()
                .filter_map(|dep_id| {
                    if !statuses
                        .get(dep_id)
                        .map(|s| s.is_complete())
                        .unwrap_or(false)
                    {
                        tasks.get(dep_id).map(|dep| {
                            serde_json::json!({
                                "id": dep.id.to_string(),
                                "title": dep.title,
                                "status": dep.status,
                            })
                        })
                    } else {
                        None
                    }
                })
                .collect();

            serde_json::json!({
                "id": t.id.to_string(),
                "title": t.title,
                "blocked_by": blocking,
            })
        })
        .collect();

    // Full format: more detail for comprehensive understanding
    let context = serde_json::json!({
        "briefs": briefs.values().map(|b| {
            serde_json::json!({
                "id": b.id.to_string(),
                "title": b.title,
                "type": b.brief_type,
                "status": b.status,
                "body": if b.body.len() > 500 {
                    format!("{}...", &b.body[..500])
                } else {
                    b.body.clone()
                },
                "meta": b.meta,
            })
        }).collect::<Vec<_>>(),

        "tasks": {
            "ready": ready_ids.iter().filter_map(|id| {
                tasks.get(id).map(|t| {
                    serde_json::json!({
                        "id": t.id.to_string(),
                        "title": t.title,
                        "standalone": t.is_standalone(),
                        "brief": t.brief_id().map(|a| a.to_string()),
                        "description": t.description,
                        "meta": t.meta,
                    })
                })
            }).collect::<Vec<_>>(),

            "in_progress": in_progress.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "standalone": t.is_standalone(),
                    "brief": t.brief_id().map(|a| a.to_string()),
                    "started_at": t.updated_at,
                    "description": t.description,
                    "meta": t.meta,
                })
            }).collect::<Vec<_>>(),

            "blocked": blocked_ids.iter().filter_map(|id| {
                tasks.get(id).map(|t| {
                    let blocking: Vec<_> = t.depends_on.blocking_task_ids().filter_map(|dep_id| {
                        if !statuses.get(dep_id).map(|s| s.is_complete()).unwrap_or(false) {
                            tasks.get(dep_id).map(|dep| {
                                serde_json::json!({
                                    "id": dep.id.to_string(),
                                    "title": dep.title,
                                    "status": dep.status,
                                })
                            })
                        } else {
                            None
                        }
                    }).collect();

                    serde_json::json!({
                        "id": t.id.to_string(),
                        "title": t.title,
                        "standalone": t.is_standalone(),
                        "brief": t.brief_id().map(|a| a.to_string()),
                        "blocked_by": blocking,
                    })
                })
            }).collect::<Vec<_>>(),

            "recently_completed": recent_completed.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "standalone": t.is_standalone(),
                    "brief": t.brief_id().map(|a| a.to_string()),
                    "completed_at": t.completed_at,
                })
            }).collect::<Vec<_>>(),

            "compacted": compacted.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "summary": t.summary.clone().unwrap_or_default(),
                    "task_count": t.compacted_count(),
                    "task_ids": t.compacted_tasks.clone().unwrap_or_default().iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                    "brief": t.brief_id().map(|a| a.to_string()),
                    "completed_at": t.completed_at,
                })
            }).collect::<Vec<_>>(),
        },

        "standalone_tasks": {
            "ready": standalone_ready,
            "in_progress": standalone_in_progress,
            "blocked": standalone_blocked,
        },

        "summary": {
            "total_briefs": briefs.len(),
            "total_tasks": tasks.len(),
            "standalone_tasks": standalone_tasks.len(),
            "ready_count": ready_ids.len(),
            "blocked_count": blocked_ids.len(),
            "in_progress_count": in_progress.len(),
            "compacted_groups": compacted.len(),
        },
    });

    output.data(&context);
    Ok(())
}
