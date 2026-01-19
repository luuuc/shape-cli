//! Kanban view: Tasks grouped by status in columns

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::cli::tui::app::{App, InputMode};
use crate::cli::tui::utils::truncate_str;
use crate::domain::TaskStatus;

/// Draw the kanban layout
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

    // Split into three columns for kanban
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(main_chunks[0]);

    // Collect tasks by status
    let statuses = app.tasks().iter().map(|(id, t)| (id.clone(), t.status)).collect();
    let mut todo_tasks = Vec::new();
    let mut in_progress_tasks = Vec::new();
    let mut done_tasks = Vec::new();

    for (task_id, task) in app.tasks() {
        match task.status {
            TaskStatus::Todo => {
                let blocked = task.is_blocked(&statuses);
                todo_tasks.push((task_id, task, blocked));
            }
            TaskStatus::InProgress => {
                in_progress_tasks.push((task_id, task));
            }
            TaskStatus::Done => {
                if app.show_completed() {
                    done_tasks.push((task_id, task));
                }
            }
        }
    }

    // Sort tasks by title
    todo_tasks.sort_by(|a, b| a.1.title.cmp(&b.1.title));
    in_progress_tasks.sort_by(|a, b| a.1.title.cmp(&b.1.title));
    done_tasks.sort_by(|a, b| a.1.title.cmp(&b.1.title));

    // Draw columns
    draw_todo_column(frame, &todo_tasks, columns[0]);
    draw_in_progress_column(frame, &in_progress_tasks, columns[1]);
    draw_done_column(frame, &done_tasks, columns[2]);

    // Draw status bar
    draw_status_bar(frame, app, main_chunks[1]);
}

fn draw_todo_column(
    frame: &mut Frame,
    tasks: &[(&crate::domain::TaskId, &crate::domain::Task, bool)],
    area: Rect,
) {
    let items: Vec<ListItem> = tasks
        .iter()
        .map(|(_, task, blocked)| {
            let indicator = if *blocked { "[B]" } else { "[ ]" };
            let style = if *blocked {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            ListItem::new(format!("{} {}", indicator, truncate_str(&task.title, 25)))
                .style(style)
        })
        .collect();

    let title = format!("Todo ({})", tasks.len());
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(list, area);
}

fn draw_in_progress_column(
    frame: &mut Frame,
    tasks: &[(&crate::domain::TaskId, &crate::domain::Task)],
    area: Rect,
) {
    let items: Vec<ListItem> = tasks
        .iter()
        .map(|(_, task)| {
            ListItem::new(format!("[~] {}", truncate_str(&task.title, 25)))
                .style(Style::default().fg(Color::Yellow))
        })
        .collect();

    let title = format!("In Progress ({})", tasks.len());
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    frame.render_widget(list, area);
}

fn draw_done_column(
    frame: &mut Frame,
    tasks: &[(&crate::domain::TaskId, &crate::domain::Task)],
    area: Rect,
) {
    let items: Vec<ListItem> = tasks
        .iter()
        .map(|(_, task)| {
            ListItem::new(format!("[x] {}", truncate_str(&task.title, 25)))
                .style(Style::default().fg(Color::DarkGray))
        })
        .collect();

    let title = format!("Done ({})", tasks.len());
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(list, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (content, style) = match app.input_mode() {
        InputMode::Normal => {
            let msg = app.status_message().unwrap_or(
                "[1-3]views [c]ompleted [r]efresh [q]uit"
            );
            (msg.to_string(), Style::default())
        }
        _ => {
            ("Press Esc to cancel".to_string(), Style::default().fg(Color::Yellow))
        }
    };

    let status_text = format!("Shape [2:Kanban] {}", content);

    let paragraph = Paragraph::new(status_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}
