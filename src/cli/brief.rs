//! Brief CLI commands

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::domain::{Brief, BriefId, BriefStatus};
use crate::plugin::{MinimalBriefType, ShapeUpBriefType};
use crate::storage::Project;

#[derive(Subcommand)]
pub enum BriefCommands {
    /// Create a new brief
    New {
        /// Brief title
        title: String,

        /// Brief type (default: minimal)
        #[arg(long, short = 't', default_value = "minimal")]
        brief_type: String,
    },

    /// List all briefs
    List {
        /// Filter by status
        #[arg(long, short)]
        status: Option<String>,
    },

    /// Show brief details
    Show {
        /// Brief ID
        id: String,
    },

    /// Update brief status
    Status {
        /// Brief ID
        id: String,

        /// New status
        status: String,
    },
}

pub fn run(cmd: BriefCommands, output: &Output) -> Result<()> {
    match cmd {
        BriefCommands::New { title, brief_type } => new_brief(output, &title, &brief_type),
        BriefCommands::List { status } => list_briefs(output, status.as_deref()),
        BriefCommands::Show { id } => show_brief(output, &id),
        BriefCommands::Status { id, status } => set_status(output, &id, &status),
    }
}

fn new_brief(output: &Output, title: &str, brief_type: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.brief_store();

    // Get template based on type
    let template = match brief_type {
        "minimal" => MinimalBriefType::template(title),
        "shapeup" => ShapeUpBriefType::template(title),
        _ => {
            // Fall back to minimal for unknown types
            // External plugin-based types would be loaded here
            MinimalBriefType::template(title)
        }
    };

    // Create brief
    let mut brief = Brief::new(title, brief_type);
    brief.set_body(&template.body);

    // Apply template frontmatter to meta
    if let Some(obj) = template.frontmatter.as_object() {
        for (key, value) in obj {
            if key != "title" && key != "status" {
                let v: serde_json::Value = value.clone();
                brief.set_meta(key, v);
            }
        }
    }

    store.write(&brief)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": brief.id.to_string(),
            "title": brief.title,
            "type": brief.brief_type,
            "status": brief.status,
        }));
    } else {
        output.success(&format!("Created brief: {} ({})", brief.id, brief.title));
    }

    Ok(())
}

fn list_briefs(output: &Output, status_filter: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.brief_store();

    let list = if let Some(status_str) = status_filter {
        let status: BriefStatus = status_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid status: {}", status_str))?;
        store
            .list_by_status(status)?
            .into_iter()
            .map(|(id, title)| (id, title, status))
            .collect()
    } else {
        store.list()?
    };

    if output.is_json() {
        let items: Vec<_> = list
            .iter()
            .map(|(id, title, status)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "title": title,
                    "status": status,
                })
            })
            .collect();
        output.data(&items);
    } else if list.is_empty() {
        println!("No briefs found.");
    } else {
        println!("{:<12} {:<15} TITLE", "ID", "STATUS");
        println!("{}", "-".repeat(60));
        for (id, title, status) in list {
            println!("{:<12} {:<15} {}", id, status, title);
        }
    }

    Ok(())
}

fn show_brief(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.brief_store();
    let task_store = project.task_store();

    let id: BriefId = id_str.parse()?;
    let brief = store
        .read(&id)?
        .ok_or_else(|| anyhow::anyhow!("Brief not found: {}", id))?;

    let tasks = task_store.read_for_brief(&id)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": brief.id.to_string(),
            "title": brief.title,
            "type": brief.brief_type,
            "status": brief.status,
            "created_at": brief.created_at,
            "updated_at": brief.updated_at,
            "body": brief.body,
            "meta": brief.meta,
            "tasks": tasks.values().map(|t| serde_json::json!({
                "id": t.id.to_string(),
                "title": t.title,
                "status": t.status,
            })).collect::<Vec<_>>(),
        }));
    } else {
        println!("Brief: {} ({})", brief.id, brief.brief_type);
        println!("Title: {}", brief.title);
        println!("Status: {}", brief.status);
        println!("Created: {}", brief.created_at.format("%Y-%m-%d %H:%M"));
        println!("Updated: {}", brief.updated_at.format("%Y-%m-%d %H:%M"));

        if !brief.meta.is_empty() {
            println!("\nMetadata:");
            for (key, value) in brief.meta.iter() {
                println!("  {}: {}", key, value);
            }
        }

        if !brief.body.is_empty() {
            println!("\nContent:");
            println!("{}", brief.body);
        }

        if !tasks.is_empty() {
            println!("\nTasks ({}):", tasks.len());
            for task in tasks.values() {
                let status_icon = match task.status {
                    crate::domain::TaskStatus::Todo => "[ ]",
                    crate::domain::TaskStatus::InProgress => "[~]",
                    crate::domain::TaskStatus::Done => "[x]",
                };
                println!("  {} {} {}", status_icon, task.id, task.title);
            }
        }
    }

    Ok(())
}

fn set_status(output: &Output, id_str: &str, status_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.brief_store();

    let id: BriefId = id_str.parse()?;
    let mut brief = store
        .read(&id)?
        .ok_or_else(|| anyhow::anyhow!("Brief not found: {}", id))?;

    let status: BriefStatus = status_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid status: {}", status_str))?;

    brief.set_status(status);
    store.write(&brief)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": brief.id.to_string(),
            "status": brief.status,
        }));
    } else {
        output.success(&format!("Updated {} status to {}", brief.id, brief.status));
    }

    Ok(())
}
