//! Context export for AI agents

use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, Utc};

use super::output::Output;
use crate::domain::{AnchorId, DependencyGraph, TaskId, TaskStatus};
use crate::storage::Project;

/// Export project context for AI consumption
pub fn export(
    output: &Output,
    compact: bool,
    anchor_filter: Option<&str>,
    days: u32,
) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose_ctx(
        "context",
        &format!("Opened project at: {}", project.root().display()),
    );

    let anchor_store = project.anchor_store();
    let task_store = project.task_store();

    let anchors = anchor_store.read_all()?;
    let tasks = task_store.read_all()?;

    output.verbose_ctx(
        "context",
        &format!("Loaded {} anchors, {} tasks", anchors.len(), tasks.len()),
    );

    // Filter by anchor if specified
    let (anchors, tasks) = if let Some(anchor_str) = anchor_filter {
        let anchor_id: AnchorId = anchor_str.parse()?;
        output.verbose_ctx("context", &format!("Filtering by anchor: {}", anchor_id));

        let anchor = anchors
            .get(&anchor_id)
            .ok_or_else(|| anyhow::anyhow!("Anchor not found: {}", anchor_id))?;

        let filtered_tasks: HashMap<_, _> = tasks
            .into_iter()
            .filter(|(_, t)| t.anchor_id().as_ref() == Some(&anchor_id))
            .collect();

        output.verbose_ctx(
            "context",
            &format!("Filtered to {} tasks for anchor", filtered_tasks.len()),
        );

        let mut filtered_anchors = HashMap::new();
        filtered_anchors.insert(anchor_id.clone(), anchor.clone());

        (filtered_anchors, filtered_tasks)
    } else {
        (anchors, tasks)
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
            &anchors,
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
            &anchors,
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
    anchors: &HashMap<AnchorId, crate::domain::Anchor>,
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
            let deps: Vec<_> = t.depends_on.iter().map(|d| d.to_string()).collect();
            format!("{}: {} (blocked by {})", t.id, t.title, deps.join(", "))
        })
        .collect();

    // Compact format: optimized for token efficiency
    let context = serde_json::json!({
        "anchors": anchors.values().map(|a| {
            serde_json::json!({
                "id": a.id.to_string(),
                "title": a.title,
                "status": a.status,
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
                let deps: Vec<_> = t.depends_on.iter().map(|d| d.to_string()).collect();
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
    anchors: &HashMap<AnchorId, crate::domain::Anchor>,
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
                .iter()
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
        "anchors": anchors.values().map(|a| {
            serde_json::json!({
                "id": a.id.to_string(),
                "title": a.title,
                "type": a.anchor_type,
                "status": a.status,
                "body": if a.body.len() > 500 {
                    format!("{}...", &a.body[..500])
                } else {
                    a.body.clone()
                },
                "meta": a.meta,
            })
        }).collect::<Vec<_>>(),

        "tasks": {
            "ready": ready_ids.iter().filter_map(|id| {
                tasks.get(id).map(|t| {
                    serde_json::json!({
                        "id": t.id.to_string(),
                        "title": t.title,
                        "standalone": t.is_standalone(),
                        "anchor": t.anchor_id().map(|a| a.to_string()),
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
                    "anchor": t.anchor_id().map(|a| a.to_string()),
                    "started_at": t.updated_at,
                    "description": t.description,
                    "meta": t.meta,
                })
            }).collect::<Vec<_>>(),

            "blocked": blocked_ids.iter().filter_map(|id| {
                tasks.get(id).map(|t| {
                    let blocking: Vec<_> = t.depends_on.iter().filter_map(|dep_id| {
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
                        "anchor": t.anchor_id().map(|a| a.to_string()),
                        "blocked_by": blocking,
                    })
                })
            }).collect::<Vec<_>>(),

            "recently_completed": recent_completed.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "standalone": t.is_standalone(),
                    "anchor": t.anchor_id().map(|a| a.to_string()),
                    "completed_at": t.completed_at,
                })
            }).collect::<Vec<_>>(),

            "compacted": compacted.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "summary": t.summary.clone().unwrap_or_default(),
                    "task_count": t.compacted_count(),
                    "task_ids": t.compacted_tasks.clone().unwrap_or_default().iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                    "anchor": t.anchor_id().map(|a| a.to_string()),
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
            "total_anchors": anchors.len(),
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
