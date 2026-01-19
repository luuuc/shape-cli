//! Graph view: ASCII dependency visualization

use std::collections::{HashMap, HashSet};

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::cli::tui::app::{App, InputMode};
use crate::cli::tui::utils::truncate_str;
use crate::domain::{TaskId, TaskStatus};

/// Draw the graph layout
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: vertical split for main content and status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),    // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    // Draw the dependency graph
    draw_graph_panel(frame, app, main_chunks[0]);

    // Draw status bar
    draw_status_bar(frame, app, main_chunks[1]);
}

fn draw_graph_panel(frame: &mut Frame, app: &App, area: Rect) {
    let graph_content = build_dependency_graph(app);

    let paragraph = Paragraph::new(graph_content)
        .block(
            Block::default()
                .title("Dependency Graph")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn build_dependency_graph(app: &App) -> String {
    let tasks = app.tasks();
    let statuses: HashMap<TaskId, TaskStatus> = tasks
        .iter()
        .map(|(id, t)| (id.clone(), t.status))
        .collect();

    // Find root tasks (tasks with no dependencies or whose dependencies are all from other briefs)
    let mut roots: Vec<&TaskId> = Vec::new();
    let mut has_dependents: HashSet<TaskId> = HashSet::new();

    for task in tasks.values() {
        for dep in task.depends_on.iter() {
            has_dependents.insert(dep.task.clone());
        }
    }

    for task_id in tasks.keys() {
        let task = &tasks[task_id];
        // A task is a root if:
        // 1. It has no blocking dependencies, OR
        // 2. All its blocking dependencies are from other briefs
        let is_root = task.depends_on.blocking().all(|dep| {
            !tasks.contains_key(&dep.task)
        });

        if is_root && !task.status.is_complete() {
            roots.push(task_id);
        }
    }

    // Sort roots by ID (string comparison)
    roots.sort_by_key(|a| a.to_string());

    if roots.is_empty() && !tasks.is_empty() {
        // Show all incomplete tasks if no clear roots
        let mut all_incomplete: Vec<_> = tasks
            .iter()
            .filter(|(_, t)| !t.status.is_complete())
            .map(|(id, _)| id)
            .collect();
        all_incomplete.sort_by_key(|a| a.to_string());
        roots = all_incomplete;
    }

    if roots.is_empty() {
        return "No tasks to display.\n\nAll tasks are complete or no tasks exist.".to_string();
    }

    let mut lines = Vec::new();
    let mut visited: HashSet<TaskId> = HashSet::new();

    lines.push("Dependency Tree:".to_string());
    lines.push(String::new());

    for root in roots {
        render_task_tree(&mut lines, root, tasks, &statuses, &mut visited, "", true);
    }

    // Add legend
    lines.push(String::new());
    lines.push("Legend:".to_string());
    lines.push("  [ ] = Todo (ready)".to_string());
    lines.push("  [~] = In Progress".to_string());
    lines.push("  [x] = Done".to_string());
    lines.push("  [B] = Blocked".to_string());
    lines.push(String::new());
    lines.push("  --> = depends on".to_string());

    lines.join("\n")
}

fn render_task_tree(
    lines: &mut Vec<String>,
    task_id: &TaskId,
    tasks: &HashMap<TaskId, crate::domain::Task>,
    statuses: &HashMap<TaskId, TaskStatus>,
    visited: &mut HashSet<TaskId>,
    prefix: &str,
    is_last: bool,
) {
    if visited.contains(task_id) {
        // Cycle detection - show reference
        lines.push(format!("{}{}-> (see {})", prefix, if is_last { "└" } else { "├" }, task_id));
        return;
    }
    visited.insert(task_id.clone());

    let task = match tasks.get(task_id) {
        Some(t) => t,
        None => return,
    };

    // Determine status indicator
    let indicator = match task.status {
        TaskStatus::Done => "[x]",
        TaskStatus::InProgress => "[~]",
        TaskStatus::Todo => {
            if task.is_blocked(statuses) {
                "[B]"
            } else {
                "[ ]"
            }
        }
    };

    // Color based on status
    // Note: status_color could be used for styling in the future
    let _status_color = match task.status {
        TaskStatus::Done => "dim",
        TaskStatus::InProgress => "yellow",
        TaskStatus::Todo => {
            if task.is_blocked(statuses) {
                "red"
            } else {
                "green"
            }
        }
    };

    let connector = if is_last { "└── " } else { "├── " };
    let line = format!("{}{}{} {} ({})", prefix, connector, indicator, truncate_str(&task.title, 30), task_id);
    lines.push(line);

    // Find tasks that depend on this task (children in the tree)
    let mut children: Vec<&TaskId> = tasks
        .iter()
        .filter(|(_, t)| {
            t.depends_on.blocking().any(|dep| &dep.task == task_id)
        })
        .map(|(id, _)| id)
        .collect();
    children.sort_by_key(|a| a.to_string());

    let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

    for (i, child_id) in children.iter().enumerate() {
        let is_child_last = i == children.len() - 1;
        render_task_tree(lines, child_id, tasks, statuses, visited, &child_prefix, is_child_last);
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (content, style) = match app.input_mode() {
        InputMode::Normal => {
            let msg = app.status_message().unwrap_or(
                "[1-3]views [r]efresh [q]uit"
            );
            (msg.to_string(), Style::default())
        }
        _ => {
            ("Press Esc to cancel".to_string(), Style::default().fg(Color::Yellow))
        }
    };

    let status_text = format!("Shape [3:Graph] {}", content);

    let paragraph = Paragraph::new(status_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
