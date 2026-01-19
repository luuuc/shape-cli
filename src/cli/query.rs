//! Query commands (ready, blocked, status)
//!
//! These commands use SQLite cache for fast queries.

use anyhow::Result;

use super::output::Output;
use crate::storage::Project;

/// Show tasks ready to work on
pub fn ready(output: &Output, anchor_filter: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose_ctx(
        "ready",
        &format!("Opened project at: {}", project.root().display()),
    );

    // Get or rebuild cache
    let cache = project.get_or_rebuild_cache()?;
    output.verbose_ctx("ready", "Using SQLite cache for query");

    // Get ready tasks
    let ready_tasks = if let Some(anchor_str) = anchor_filter {
        output.verbose_ctx("ready", &format!("Filtering by anchor: {}", anchor_str));
        cache.ready_tasks_for_anchor(anchor_str)?
    } else {
        cache.ready_tasks_detailed()?
    };

    output.verbose_ctx("ready", &format!("Found {} ready tasks", ready_tasks.len()));

    if output.is_json() {
        let items: Vec<_> = ready_tasks
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "title": t.title,
                    "standalone": t.is_standalone(),
                    "anchor_id": t.anchor_id,
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
    output.verbose_ctx(
        "blocked",
        &format!("Opened project at: {}", project.root().display()),
    );

    // Get or rebuild cache
    let cache = project.get_or_rebuild_cache()?;
    output.verbose_ctx("blocked", "Using SQLite cache for query");

    // Get blocked tasks
    let blocked_tasks = if let Some(anchor_str) = anchor_filter {
        output.verbose_ctx("blocked", &format!("Filtering by anchor: {}", anchor_str));
        cache.blocked_tasks_for_anchor(anchor_str)?
    } else {
        cache.blocked_tasks_detailed()?
    };

    output.verbose_ctx(
        "blocked",
        &format!("Found {} blocked tasks", blocked_tasks.len()),
    );

    if output.is_json() {
        let items: Vec<_> = blocked_tasks
            .iter()
            .map(|(task, blockers)| {
                serde_json::json!({
                    "id": task.id,
                    "title": task.title,
                    "blocked_by": blockers,
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
        for (task, blockers) in blocked_tasks {
            println!("{:<20} {:<30} {}", task.id, task.title, blockers.join(", "));
        }
    }

    Ok(())
}

/// Show project status overview
pub fn status(output: &Output) -> Result<()> {
    let project = Project::open_current()?;

    // Get or rebuild cache
    let cache = project.get_or_rebuild_cache()?;

    // Get task counts from cache
    let (todo_tasks, in_progress_tasks, done_tasks) = cache.task_counts()?;
    let total_tasks = todo_tasks + in_progress_tasks + done_tasks;

    // Get standalone task counts
    let (standalone_todo, standalone_in_progress, standalone_done) =
        cache.standalone_task_counts()?;
    let standalone_tasks = standalone_todo + standalone_in_progress + standalone_done;

    // Get ready/blocked counts
    let ready_count = cache.ready_task_ids()?.len();
    let blocked_count = cache.blocked_task_ids()?.len();

    // Get anchor info from cache
    let anchors = cache.list_anchors()?;
    let active_anchors = anchors.iter().filter(|a| a.is_active()).count();
    let complete_anchors = anchors.iter().filter(|a| a.is_complete()).count();

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
        println!(
            "Anchors: {} total ({} active, {} complete)",
            anchors.len(),
            active_anchors,
            complete_anchors
        );
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
            let active: Vec<_> = anchors.iter().filter(|a| a.is_active()).collect();
            if !active.is_empty() {
                println!();
                println!("Active Anchors:");
                for anchor in active {
                    println!("  {} - {}", anchor.id, anchor.title);
                }
            }
        }
    }

    Ok(())
}
