//! Anchor CLI commands

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::domain::{Anchor, AnchorId, AnchorStatus};
use crate::plugin::{MinimalAnchorType, ShapeUpAnchorType};
use crate::storage::Project;

#[derive(Subcommand)]
pub enum AnchorCommands {
    /// Create a new anchor
    New {
        /// Anchor title
        title: String,

        /// Anchor type (default: minimal)
        #[arg(long, short = 't', default_value = "minimal")]
        anchor_type: String,
    },

    /// List all anchors
    List {
        /// Filter by status
        #[arg(long, short)]
        status: Option<String>,
    },

    /// Show anchor details
    Show {
        /// Anchor ID
        id: String,
    },

    /// Update anchor status
    Status {
        /// Anchor ID
        id: String,

        /// New status
        status: String,
    },
}

pub fn run(cmd: AnchorCommands, output: &Output) -> Result<()> {
    match cmd {
        AnchorCommands::New { title, anchor_type } => new_anchor(output, &title, &anchor_type),
        AnchorCommands::List { status } => list_anchors(output, status.as_deref()),
        AnchorCommands::Show { id } => show_anchor(output, &id),
        AnchorCommands::Status { id, status } => set_status(output, &id, &status),
    }
}

fn new_anchor(output: &Output, title: &str, anchor_type: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.anchor_store();

    // Get template based on type
    let template = match anchor_type {
        "minimal" => MinimalAnchorType::template(title),
        "shapeup" => ShapeUpAnchorType::template(title),
        _ => {
            // Fall back to minimal for unknown types
            // External plugin-based types would be loaded here
            MinimalAnchorType::template(title)
        }
    };

    // Create anchor
    let mut anchor = Anchor::new(title, anchor_type);
    anchor.set_body(&template.body);

    // Apply template frontmatter to meta
    if let Some(obj) = template.frontmatter.as_object() {
        for (key, value) in obj {
            if key != "title" && key != "status" {
                let v: serde_json::Value = value.clone();
                anchor.set_meta(key, v);
            }
        }
    }

    store.write(&anchor)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": anchor.id.to_string(),
            "title": anchor.title,
            "type": anchor.anchor_type,
            "status": anchor.status,
        }));
    } else {
        output.success(&format!("Created anchor: {} ({})", anchor.id, anchor.title));
    }

    Ok(())
}

fn list_anchors(output: &Output, status_filter: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.anchor_store();

    let list = if let Some(status_str) = status_filter {
        let status: AnchorStatus = status_str
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
        println!("No anchors found.");
    } else {
        println!("{:<12} {:<15} TITLE", "ID", "STATUS");
        println!("{}", "-".repeat(60));
        for (id, title, status) in list {
            println!("{:<12} {:<15} {}", id, status, title);
        }
    }

    Ok(())
}

fn show_anchor(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.anchor_store();
    let task_store = project.task_store();

    let id: AnchorId = id_str.parse()?;
    let anchor = store
        .read(&id)?
        .ok_or_else(|| anyhow::anyhow!("Anchor not found: {}", id))?;

    let tasks = task_store.read_for_anchor(&id)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": anchor.id.to_string(),
            "title": anchor.title,
            "type": anchor.anchor_type,
            "status": anchor.status,
            "created_at": anchor.created_at,
            "updated_at": anchor.updated_at,
            "body": anchor.body,
            "meta": anchor.meta,
            "tasks": tasks.values().map(|t| serde_json::json!({
                "id": t.id.to_string(),
                "title": t.title,
                "status": t.status,
            })).collect::<Vec<_>>(),
        }));
    } else {
        println!("Anchor: {} ({})", anchor.id, anchor.anchor_type);
        println!("Title: {}", anchor.title);
        println!("Status: {}", anchor.status);
        println!("Created: {}", anchor.created_at.format("%Y-%m-%d %H:%M"));
        println!("Updated: {}", anchor.updated_at.format("%Y-%m-%d %H:%M"));

        if !anchor.meta.is_empty() {
            println!("\nMetadata:");
            for (key, value) in anchor.meta.iter() {
                println!("  {}: {}", key, value);
            }
        }

        if !anchor.body.is_empty() {
            println!("\nContent:");
            println!("{}", anchor.body);
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
    let store = project.anchor_store();

    let id: AnchorId = id_str.parse()?;
    let mut anchor = store
        .read(&id)?
        .ok_or_else(|| anyhow::anyhow!("Anchor not found: {}", id))?;

    let status: AnchorStatus = status_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid status: {}", status_str))?;

    anchor.set_status(status);
    store.write(&anchor)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": anchor.id.to_string(),
            "status": anchor.status,
        }));
    } else {
        output.success(&format!(
            "Updated {} status to {}",
            anchor.id, anchor.status
        ));
    }

    Ok(())
}
