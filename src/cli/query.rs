//! Query commands (ready, blocked, status)

use std::collections::HashMap;

use anyhow::Result;

use super::output::Output;
use crate::domain::{AnchorId, DependencyGraph, TaskId, TaskStatus};
use crate::storage::Project;

/// Show tasks ready to work on
pub fn ready(output: &Output, anchor_filter: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose_ctx("ready", &format!("Opened project at: {}", project.root().display()));

    let task_store = project.task_store();
    output.verbose_ctx("ready", &format!("Task store path: {}", task_store.path().display()));

    let tasks = if let Some(anchor_str) = anchor_filter {
        let anchor_id: AnchorId = anchor_str.parse()?;
        output.verbose_ctx("ready", &format!("Filtering by anchor: {}", anchor_id));
        task_store.read_for_anchor(&anchor_id)?
    } else {
        task_store.read_all()?
    };

    output.verbose_ctx("ready", &format!("Loaded {} tasks", tasks.len()));

    // Build status map
    let statuses: HashMap<TaskId, TaskStatus> = tasks
        .iter()
        .map(|(id, t)| (id.clone(), t.status))
        .collect();

    // Build dependency graph
    let graph = DependencyGraph::from_tasks(tasks.values())?;
    output.verbose_ctx("ready", &format!("Built dependency graph with {} nodes", graph.len()));

    // Find ready tasks
    let ready_ids = graph.ready_tasks(&statuses);
    output.verbose_ctx("ready", &format!("Found {} ready tasks", ready_ids.len()));
    let ready_tasks: Vec<_> = ready_ids
        .iter()
        .filter_map(|id| tasks.get(id))
        .collect();

    if output.is_json() {
        let items: Vec<_> = ready_tasks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "standalone": t.is_standalone(),
                    "anchor_id": t.anchor_id().map(|a| a.to_string()),
                })
            })
            .collect();
        output.data(&items);
    } else if ready_tasks.is_empty() {
        println!("No tasks ready to work on.");
    } else {
        println!("Ready tasks ({}):", ready_tasks.len());
        println!("{:<20} TITLE", "ID");
        println!("{}", "-".repeat(60));
        for task in ready_tasks {
            println!("{:<20} {}", task.id, task.title);
        }
    }

    Ok(())
}

/// Show blocked tasks
pub fn blocked(output: &Output, anchor_filter: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose_ctx("blocked", &format!("Opened project at: {}", project.root().display()));

    let task_store = project.task_store();

    let tasks = if let Some(anchor_str) = anchor_filter {
        let anchor_id: AnchorId = anchor_str.parse()?;
        output.verbose_ctx("blocked", &format!("Filtering by anchor: {}", anchor_id));
        task_store.read_for_anchor(&anchor_id)?
    } else {
        task_store.read_all()?
    };

    output.verbose_ctx("blocked", &format!("Loaded {} tasks", tasks.len()));

    // Build status map
    let statuses: HashMap<TaskId, TaskStatus> = tasks
        .iter()
        .map(|(id, t)| (id.clone(), t.status))
        .collect();

    // Build dependency graph
    let graph = DependencyGraph::from_tasks(tasks.values())?;
    output.verbose_ctx("blocked", &format!("Built dependency graph with {} nodes", graph.len()));

    // Find blocked tasks
    let blocked_ids = graph.blocked_tasks(&statuses);
    output.verbose_ctx("blocked", &format!("Found {} blocked tasks", blocked_ids.len()));
    let blocked_tasks: Vec<_> = blocked_ids
        .iter()
        .filter_map(|id| tasks.get(id))
        .collect();

    if output.is_json() {
        let items: Vec<_> = blocked_tasks
            .iter()
            .map(|t| {
                let blocking: Vec<_> = t.depends_on
                    .iter()
                    .filter(|dep| !statuses.get(*dep).map(|s| s.is_complete()).unwrap_or(false))
                    .map(|d| d.to_string())
                    .collect();

                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "blocked_by": blocking,
                })
            })
            .collect();
        output.data(&items);
    } else if blocked_tasks.is_empty() {
        println!("No blocked tasks.");
    } else {
        println!("Blocked tasks ({}):", blocked_tasks.len());
        println!("{:<20} {:<30} BLOCKED BY", "ID", "TITLE");
        println!("{}", "-".repeat(80));
        for task in blocked_tasks {
            let blocking: Vec<_> = task.depends_on
                .iter()
                .filter(|dep| !statuses.get(*dep).map(|s| s.is_complete()).unwrap_or(false))
                .map(|d| d.to_string())
                .collect();

            println!("{:<20} {:<30} {}", task.id, task.title, blocking.join(", "));
        }
    }

    Ok(())
}

/// Show project status overview
pub fn status(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    let anchor_store = project.anchor_store();
    let task_store = project.task_store();

    let anchors = anchor_store.list()?;
    let tasks = task_store.read_all()?;

    // Count tasks by status
    let total_tasks = tasks.len();
    let done_tasks = tasks.values().filter(|t| t.status.is_complete()).count();
    let in_progress_tasks = tasks.values().filter(|t| t.status.is_active()).count();
    let todo_tasks = tasks.values().filter(|t| t.status.is_pending()).count();

    // Count standalone tasks
    let standalone_tasks = tasks.values().filter(|t| t.is_standalone()).count();
    let standalone_todo = tasks.values().filter(|t| t.is_standalone() && t.status.is_pending()).count();
    let standalone_in_progress = tasks.values().filter(|t| t.is_standalone() && t.status.is_active()).count();
    let standalone_done = tasks.values().filter(|t| t.is_standalone() && t.status.is_complete()).count();

    // Build status map for ready/blocked
    let statuses: HashMap<TaskId, TaskStatus> = tasks
        .iter()
        .map(|(id, t)| (id.clone(), t.status))
        .collect();

    let graph = DependencyGraph::from_tasks(tasks.values())?;
    let ready_count = graph.ready_tasks(&statuses).len();
    let blocked_count = graph.blocked_tasks(&statuses).len();

    // Count anchors by status
    let active_anchors = anchors.iter().filter(|(_, _, s)| s.is_active()).count();
    let complete_anchors = anchors.iter().filter(|(_, _, s)| s.is_complete()).count();

    if output.is_json() {
        output.data(&serde_json::json!({
            "anchors": {
                "total": anchors.len(),
                "active": active_anchors,
                "complete": complete_anchors,
            },
            "tasks": {
                "total": total_tasks,
                "todo": todo_tasks,
                "in_progress": in_progress_tasks,
                "done": done_tasks,
                "ready": ready_count,
                "blocked": blocked_count,
            },
            "standalone_tasks": {
                "total": standalone_tasks,
                "todo": standalone_todo,
                "in_progress": standalone_in_progress,
                "done": standalone_done,
            },
        }));
    } else {
        println!("Project Status");
        println!("{}", "=".repeat(40));
        println!();
        println!("Anchors: {} total ({} active, {} complete)",
            anchors.len(), active_anchors, complete_anchors);
        println!();
        println!("Tasks: {} total", total_tasks);
        println!("  [ ] Todo:        {}", todo_tasks);
        println!("  [~] In Progress: {}", in_progress_tasks);
        println!("  [x] Done:        {}", done_tasks);
        println!();
        println!("  Ready to work:   {}", ready_count);
        println!("  Blocked:         {}", blocked_count);

        if standalone_tasks > 0 {
            println!();
            println!("Standalone Tasks: {}", standalone_tasks);
            if standalone_todo > 0 {
                println!("  [ ] Todo:        {}", standalone_todo);
            }
            if standalone_in_progress > 0 {
                println!("  [~] In Progress: {}", standalone_in_progress);
            }
            if standalone_done > 0 {
                println!("  [x] Done:        {}", standalone_done);
            }
        }

        if !anchors.is_empty() {
            println!();
            println!("Active Anchors:");
            for (id, title, _status) in anchors.iter().filter(|(_, _, s)| s.is_active()) {
                println!("  {} - {}", id, title);
            }
        }
    }

    Ok(())
}
