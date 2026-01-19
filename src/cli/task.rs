//! Task CLI commands

use std::collections::HashMap;

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::domain::{AnchorId, DependencyGraph, Task, TaskId, TaskStatus};
use crate::storage::Project;

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Add a task (standalone, to an anchor, or as a subtask)
    ///
    /// Examples:
    ///   shape task add "Fix typo"              # Standalone task
    ///   shape task add a-1234567 "Build API"   # Task under anchor
    ///   shape task add a-1234567.1 "Subtask"   # Subtask under task
    Add {
        /// For standalone: just the title
        /// For anchored: parent ID (anchor or task)
        first: String,

        /// Task title (when first arg is parent ID)
        second: Option<String>,
    },

    /// List tasks (all, for an anchor, or standalone only)
    List {
        /// Anchor ID (omit for all tasks, use --standalone for standalone only)
        anchor: Option<String>,

        /// Show only standalone tasks
        #[arg(long)]
        standalone: bool,
    },

    /// Show task details
    Show {
        /// Task ID
        id: String,
    },

    /// Mark task as in progress
    Start {
        /// Task ID
        id: String,
    },

    /// Mark task as done
    Done {
        /// Task ID
        id: String,
    },

    /// Add a dependency between tasks
    Dep {
        /// Task that will be blocked
        task: String,

        /// Task that must be completed first
        depends_on: String,
    },

    /// Remove a dependency
    Undep {
        /// Task to unblock
        task: String,

        /// Dependency to remove
        depends_on: String,
    },

    /// Set task metadata
    Meta {
        /// Task ID
        id: String,

        /// Metadata key
        key: String,

        /// Metadata value (JSON)
        value: String,
    },
}

pub fn run(cmd: TaskCommands, output: &Output) -> Result<()> {
    match cmd {
        TaskCommands::Add { first, second } => {
            // Determine if this is standalone or anchored based on arguments:
            // - One arg: standalone task with title = first
            // - Two args: anchored task with parent = first, title = second
            let (parent, title) = match second {
                Some(title) => (Some(first.as_str()), title),
                None => (None, first),
            };
            add_task(output, parent, &title)
        }
        TaskCommands::List { anchor, standalone } => {
            list_tasks(output, anchor.as_deref(), standalone)
        }
        TaskCommands::Show { id } => show_task(output, &id),
        TaskCommands::Start { id } => start_task(output, &id),
        TaskCommands::Done { id } => complete_task(output, &id),
        TaskCommands::Dep { task, depends_on } => add_dependency(output, &task, &depends_on),
        TaskCommands::Undep { task, depends_on } => remove_dependency(output, &task, &depends_on),
        TaskCommands::Meta { id, key, value } => set_meta(output, &id, &key, &value),
    }
}

fn add_task(output: &Output, parent_str: Option<&str>, title: &str) -> Result<()> {
    use chrono::Utc;

    let project = Project::open_current()?;
    let store = project.task_store();

    let task_id = match parent_str {
        None => {
            // No parent - create standalone task
            TaskId::new_standalone(title, Utc::now())
        }
        Some(parent) => {
            // Check if parent is a task ID (contains '.' or starts with 't-')
            if parent.contains('.') || parent.starts_with("t-") {
                // Parent is a task - create subtask
                let parent_id: TaskId = parent.parse()?;
                let tasks = store.read_all()?;

                // Find max subtask sequence for this parent
                let max_seq = tasks
                    .values()
                    .filter(|t| t.id.parent().as_ref() == Some(&parent_id))
                    .map(|t| *t.id.segments().last().unwrap_or(&0))
                    .max()
                    .unwrap_or(0);

                parent_id.subtask(max_seq + 1)
            } else {
                // Parent is an anchor - create top-level task under anchor
                let anchor_id: AnchorId = parent.parse()?;

                // Verify anchor exists
                let anchor_store = project.anchor_store();
                if !anchor_store.exists(&anchor_id) {
                    anyhow::bail!("Anchor not found: {}", anchor_id);
                }

                let tasks = store.read_for_anchor(&anchor_id)?;

                // Find max task sequence for this anchor
                let max_seq = tasks
                    .values()
                    .filter(|t| t.id.depth() == 1)
                    .map(|t| *t.id.segments().first().unwrap_or(&0))
                    .max()
                    .unwrap_or(0);

                TaskId::new(&anchor_id, max_seq + 1)
            }
        }
    };

    let task = Task::new(task_id.clone(), title);
    store.append(&task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "title": task.title,
            "status": task.status,
            "standalone": task.is_standalone(),
        }));
    } else {
        output.success(&format!("Created task: {} - {}", task.id, task.title));
    }

    Ok(())
}

fn list_tasks(output: &Output, anchor_str: Option<&str>, standalone_only: bool) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let tasks = if standalone_only {
        store.read_standalone()?
    } else if let Some(anchor_str) = anchor_str {
        let anchor_id: AnchorId = anchor_str.parse()?;
        store.read_for_anchor(&anchor_id)?
    } else {
        store.read_all()?
    };

    if output.is_json() {
        let items: Vec<_> = tasks
            .values()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "status": t.status,
                    "standalone": t.is_standalone(),
                    "anchor_id": t.anchor_id().map(|a| a.to_string()),
                    "depends_on": t.depends_on.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
                })
            })
            .collect();
        output.data(&items);
    } else if tasks.is_empty() {
        if standalone_only {
            println!("No standalone tasks");
        } else if let Some(anchor_str) = anchor_str {
            println!("No tasks for anchor {}", anchor_str);
        } else {
            println!("No tasks");
        }
    } else {
        println!("{:<20} {:<12} TITLE", "ID", "STATUS");
        println!("{}", "-".repeat(60));

        // Sort by ID
        let mut sorted: Vec<_> = tasks.values().collect();
        sorted.sort_by(|a, b| a.id.to_string().cmp(&b.id.to_string()));

        for task in sorted {
            let status = match task.status {
                TaskStatus::Todo => "todo",
                TaskStatus::InProgress => "in_progress",
                TaskStatus::Done => "done",
            };
            println!("{:<20} {:<12} {}", task.id, status, task.title);
        }
    }

    Ok(())
}

fn show_task(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let id: TaskId = id_str.parse()?;
    let tasks = store.read_all()?;

    let task = tasks
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    // Build status map for ready/blocked calculation
    let statuses: HashMap<TaskId, TaskStatus> =
        tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();

    let is_ready = task.is_ready(&statuses);
    let is_blocked = task.is_blocked(&statuses);

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "title": task.title,
            "status": task.status,
            "standalone": task.is_standalone(),
            "anchor_id": task.anchor_id().map(|a| a.to_string()),
            "depends_on": task.depends_on.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
            "created_at": task.created_at,
            "updated_at": task.updated_at,
            "completed_at": task.completed_at,
            "description": task.description,
            "meta": task.meta,
            "is_ready": is_ready,
            "is_blocked": is_blocked,
        }));
    } else {
        println!("Task: {}", task.id);
        println!("Title: {}", task.title);
        println!("Status: {:?}", task.status);
        if let Some(anchor) = task.anchor_id() {
            println!("Anchor: {}", anchor);
        } else {
            println!("Type: Standalone");
        }
        println!("Created: {}", task.created_at.format("%Y-%m-%d %H:%M"));
        println!("Updated: {}", task.updated_at.format("%Y-%m-%d %H:%M"));

        if let Some(completed) = task.completed_at {
            println!("Completed: {}", completed.format("%Y-%m-%d %H:%M"));
        }

        if !task.depends_on.is_empty() {
            println!("\nDepends on:");
            for dep in &task.depends_on {
                let dep_status = statuses
                    .get(dep)
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "?".to_string());
                println!("  {} ({})", dep, dep_status);
            }
        }

        if let Some(desc) = &task.description {
            println!("\nDescription:");
            println!("{}", desc);
        }

        if !task.meta.is_empty() {
            println!("\nMetadata:");
            for (key, value) in task.meta.iter() {
                println!("  {}: {}", key, value);
            }
        }

        println!();
        if is_ready {
            println!("Status: READY (all dependencies complete)");
        } else if is_blocked {
            println!("Status: BLOCKED (waiting on dependencies)");
        }
    }

    Ok(())
}

fn start_task(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    task.start();
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "status": task.status,
        }));
    } else {
        output.success(&format!("Started task: {}", task.id));
    }

    Ok(())
}

fn complete_task(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    task.complete();
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "status": task.status,
            "completed_at": task.completed_at,
        }));
    } else {
        output.success(&format!("Completed task: {}", task.id));
    }

    Ok(())
}

fn add_dependency(output: &Output, task_str: &str, depends_on_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let task_id: TaskId = task_str.parse()?;
    let depends_on_id: TaskId = depends_on_str.parse()?;

    let mut tasks = store.read_all()?;

    // Verify both tasks exist
    if !tasks.contains_key(&task_id) {
        anyhow::bail!("Task not found: {}", task_id);
    }
    if !tasks.contains_key(&depends_on_id) {
        anyhow::bail!("Dependency task not found: {}", depends_on_id);
    }

    // Check for cycles using the graph
    let mut graph = DependencyGraph::from_tasks(tasks.values())?;
    graph.add_dependency(&task_id, &depends_on_id)?;

    // Update the task
    let task = tasks.get_mut(&task_id).unwrap();
    task.add_dependency(depends_on_id.clone());
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "task": task_id.to_string(),
            "depends_on": depends_on_id.to_string(),
        }));
    } else {
        output.success(&format!("{} now depends on {}", task_id, depends_on_id));
    }

    Ok(())
}

fn remove_dependency(output: &Output, task_str: &str, depends_on_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let task_id: TaskId = task_str.parse()?;
    let depends_on_id: TaskId = depends_on_str.parse()?;

    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&task_id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

    task.remove_dependency(&depends_on_id);
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "task": task_id.to_string(),
            "removed_dependency": depends_on_id.to_string(),
        }));
    } else {
        output.success(&format!(
            "Removed dependency: {} no longer depends on {}",
            task_id, depends_on_id
        ));
    }

    Ok(())
}

fn set_meta(output: &Output, id_str: &str, key: &str, value_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    // Try to parse value as JSON, fall back to string
    let value: serde_json::Value = serde_json::from_str(value_str)
        .unwrap_or_else(|_| serde_json::Value::String(value_str.to_string()));

    task.set_meta(key, value.clone());
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "key": key,
            "value": value,
        }));
    } else {
        output.success(&format!("Set {} = {} on {}", key, value, task.id));
    }

    Ok(())
}
