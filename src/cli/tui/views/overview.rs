//! Overview view: Briefs + Tasks split view

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::cli::tui::app::{App, Focus, InputMode};
use crate::cli::tui::utils::truncate_str;
use crate::domain::{BriefStatus, TaskStatus};

/// Draw the overview layout
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

    // Split main content: briefs panel, tasks panel, details panel
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Briefs
            Constraint::Percentage(40), // Tasks
            Constraint::Percentage(35), // Details
        ])
        .split(main_chunks[0]);

    // Draw each panel
    draw_briefs_panel(frame, app, content_chunks[0]);
    draw_tasks_panel(frame, app, content_chunks[1]);
    draw_details_panel(frame, app, content_chunks[2]);
    draw_status_bar(frame, app, main_chunks[1]);
}

/// Draw the briefs panel
fn draw_briefs_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus() == Focus::Briefs;

    let items: Vec<ListItem> = app
        .brief_list()
        .iter()
        .map(|id| {
            let brief = app.briefs().get(id);
            let (title, status_indicator) = match brief {
                Some(a) => {
                    let indicator = match a.status {
                        BriefStatus::InProgress => "[IP]",
                        BriefStatus::Proposed => "[PROP]",
                        BriefStatus::Betting => "[BET]",
                        BriefStatus::Shipped => "[DONE]",
                        BriefStatus::Archived => "[ARCH]",
                    };
                    (a.title.clone(), indicator)
                }
                None => (id.to_string(), ""),
            };

            let content = format!("{} {}", truncate_str(&title, 20), status_indicator);
            ListItem::new(content)
        })
        .collect();

    let block_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Briefs")
                .borders(Borders::ALL)
                .border_style(block_style),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.brief_index()));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Draw the tasks panel
fn draw_tasks_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus() == Focus::Tasks;
    let statuses = app.tasks().iter().map(|(id, t)| (id.clone(), t.status)).collect();

    // Group tasks by status
    let mut ready_tasks = Vec::new();
    let mut in_progress_tasks = Vec::new();
    let mut blocked_tasks = Vec::new();
    let mut done_tasks = Vec::new();

    for task_id in app.task_list() {
        if let Some(task) = app.tasks().get(task_id) {
            match task.status {
                TaskStatus::Done => done_tasks.push(task_id),
                TaskStatus::InProgress => in_progress_tasks.push(task_id),
                TaskStatus::Todo => {
                    if task.is_blocked(&statuses) {
                        blocked_tasks.push(task_id);
                    } else {
                        ready_tasks.push(task_id);
                    }
                }
            }
        }
    }

    // Build list items with section headers
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_index = 0;
    let mut selected_flat_index = None;

    // Ready tasks
    if !ready_tasks.is_empty() {
        items.push(ListItem::new(format!("Ready ({})", ready_tasks.len()))
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
        for task_id in &ready_tasks {
            if let Some(pos) = app.task_list().iter().position(|id| id == *task_id) {
                if pos == app.task_index() {
                    selected_flat_index = Some(flat_index + 1);
                }
            }
            if let Some(task) = app.tasks().get(task_id) {
                let indicator = "[ ]";
                items.push(ListItem::new(format!("  {} {}", indicator, truncate_str(&task.title, 30))));
            }
        }
        flat_index += ready_tasks.len() + 1;
    }

    // In Progress tasks
    if !in_progress_tasks.is_empty() {
        items.push(ListItem::new(format!("In Progress ({})", in_progress_tasks.len()))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
        for task_id in &in_progress_tasks {
            if let Some(pos) = app.task_list().iter().position(|id| id == *task_id) {
                if pos == app.task_index() {
                    selected_flat_index = Some(flat_index + 1);
                }
            }
            if let Some(task) = app.tasks().get(task_id) {
                let indicator = "[~]";
                items.push(ListItem::new(format!("  {} {}", indicator, truncate_str(&task.title, 30))));
            }
        }
        flat_index += in_progress_tasks.len() + 1;
    }

    // Blocked tasks
    if !blocked_tasks.is_empty() {
        items.push(ListItem::new(format!("Blocked ({})", blocked_tasks.len()))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
        for task_id in &blocked_tasks {
            if let Some(pos) = app.task_list().iter().position(|id| id == *task_id) {
                if pos == app.task_index() {
                    selected_flat_index = Some(flat_index + 1);
                }
            }
            if let Some(task) = app.tasks().get(task_id) {
                let indicator = "[B]";
                items.push(ListItem::new(format!("  {} {}", indicator, truncate_str(&task.title, 30))));
            }
        }
        flat_index += blocked_tasks.len() + 1;
    }

    // Done tasks (if showing completed)
    if app.show_completed() && !done_tasks.is_empty() {
        items.push(ListItem::new(format!("Done ({})", done_tasks.len()))
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)));
        for task_id in &done_tasks {
            if let Some(pos) = app.task_list().iter().position(|id| id == *task_id) {
                if pos == app.task_index() {
                    selected_flat_index = Some(flat_index + 1);
                }
            }
            if let Some(task) = app.tasks().get(task_id) {
                let indicator = "[x]";
                items.push(ListItem::new(format!("  {} {}", indicator, truncate_str(&task.title, 30)))
                    .style(Style::default().fg(Color::DarkGray)));
            }
        }
    }

    let block_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Tasks")
                .borders(Borders::ALL)
                .border_style(block_style),
        )
        .highlight_style(
            Style::default()
                .bg(if focused { Color::DarkGray } else { Color::Black })
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(selected_flat_index);

    frame.render_stateful_widget(list, area, &mut state);
}

/// Draw the details panel
fn draw_details_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus() == Focus::Details;
    let statuses = app.tasks().iter().map(|(id, t)| (id.clone(), t.status)).collect();

    let content = if let Some(task) = app.selected_task() {
        let status_str = match task.status {
            TaskStatus::Todo => {
                if task.is_blocked(&statuses) {
                    "blocked"
                } else {
                    "todo (ready)"
                }
            }
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        };

        let brief_str = task
            .brief_id()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "standalone".to_string());

        let deps_str = if task.depends_on.is_empty() {
            "none".to_string()
        } else {
            task.depends_on
                .iter()
                .map(|d| {
                    let status = app.tasks().get(&d.task).map(|t| {
                        if t.status.is_complete() { " (done)" } else { "" }
                    }).unwrap_or("");
                    format!("{}{}", d.task, status)
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        let created = task.created_at.format("%Y-%m-%d").to_string();

        let mut lines = vec![
            format!("Task: {}", task.id),
            format!("Title: {}", task.title),
            format!("Status: {}", status_str),
            format!("Brief: {}", brief_str),
            format!("Created: {}", created),
            format!("Dependencies: {}", deps_str),
            String::new(),
        ];

        if let Some(ref desc) = task.description {
            lines.push("Description:".to_string());
            lines.push(desc.clone());
        }

        lines.join("\n")
    } else if let Some(brief) = app.selected_brief() {
        let status_str = brief.status.to_string();
        let created = brief.created_at.format("%Y-%m-%d").to_string();

        let mut lines = vec![
            format!("Brief: {}", brief.id),
            format!("Title: {}", brief.title),
            format!("Type: {}", brief.brief_type),
            format!("Status: {}", status_str),
            format!("Created: {}", created),
            String::new(),
        ];

        if !brief.body.is_empty() {
            // Show first few lines of body
            let body_preview: String = brief.body
                .lines()
                .take(10)
                .collect::<Vec<_>>()
                .join("\n");
            lines.push(body_preview);
        }

        lines.join("\n")
    } else {
        "No item selected".to_string()
    };

    let block_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title("Details")
                .borders(Borders::ALL)
                .border_style(block_style),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Draw the status bar
fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (content, style) = match app.input_mode() {
        InputMode::Normal => {
            let msg = app.status_message().unwrap_or(
                "[s]tart [d]one [e]dit [n]ew task [N]ew brief [/]search [c]ompleted [1-3]views [q]uit [?]help"
            );
            (msg.to_string(), Style::default())
        }
        InputMode::Search(query) => {
            (format!("Search: {}_", query), Style::default().fg(Color::Yellow))
        }
        InputMode::Confirm(action) => {
            let msg = match action {
                crate::cli::tui::app::ConfirmAction::CompleteTask(id) => {
                    format!("Complete task {}? [y/n]", id)
                }
            };
            (msg, Style::default().fg(Color::Yellow))
        }
        InputMode::NewTask(title) => {
            (format!("New task: {}_", title), Style::default().fg(Color::Green))
        }
        InputMode::NewBrief(title) => {
            (format!("New brief: {}_", title), Style::default().fg(Color::Green))
        }
    };

    // View mode indicator
    let view_str = match app.view_mode() {
        crate::cli::tui::ViewMode::Overview => "[1:Overview]",
        crate::cli::tui::ViewMode::Kanban => "[2:Kanban]",
        crate::cli::tui::ViewMode::Graph => "[3:Graph]",
    };

    let status_text = format!("{} {} {}", "Shape", view_str, content);

    let paragraph = Paragraph::new(status_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
