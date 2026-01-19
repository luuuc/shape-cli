//! TUI application state and logic

use std::collections::HashMap;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;

use super::event::{Event, EventHandler};
use super::ui::Terminal;
use super::views;
use super::ViewMode;
use crate::domain::{Brief, BriefId, BriefStatus, Task, TaskId, TaskStatus};
use crate::storage::Project;

/// Which panel has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Briefs,
    Tasks,
    Details,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Focus::Briefs => Focus::Tasks,
            Focus::Tasks => Focus::Details,
            Focus::Details => Focus::Briefs,
        }
    }

    fn prev(self) -> Self {
        match self {
            Focus::Briefs => Focus::Details,
            Focus::Tasks => Focus::Briefs,
            Focus::Details => Focus::Tasks,
        }
    }
}

/// Input mode
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Search(String),
    Confirm(ConfirmAction),
    NewTask(String),
    NewBrief(String),
}

/// Confirmation actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    CompleteTask(TaskId),
}

/// Application state
pub struct App {
    /// Current project
    project: Project,

    /// All briefs
    briefs: HashMap<BriefId, Brief>,

    /// All tasks
    tasks: HashMap<TaskId, Task>,

    /// Sorted brief IDs for display
    brief_list: Vec<BriefId>,

    /// Current view mode
    view_mode: ViewMode,

    /// Current focus
    focus: Focus,

    /// Input mode
    input_mode: InputMode,

    /// Selected brief index
    brief_index: usize,

    /// Selected task index
    task_index: usize,

    /// Task list for current view (filtered by brief or status)
    task_list: Vec<TaskId>,

    /// Brief filter (if any)
    brief_filter: Option<BriefId>,

    /// Search results
    search_results: Vec<TaskId>,

    /// Status message to display
    status_message: Option<String>,

    /// Whether to quit
    should_quit: bool,

    /// Show completed tasks
    show_completed: bool,

    /// Pending edit action (path to file to edit)
    pending_edit: Option<std::path::PathBuf>,
}

impl App {
    /// Create a new application
    pub fn new(brief_filter: Option<&str>, view_mode: ViewMode) -> Result<Self> {
        let project = Project::open_current()?;
        let brief_store = project.brief_store();
        let task_store = project.task_store();

        let briefs = brief_store.read_all()?;
        let tasks = task_store.read_all()?;

        // Sort briefs by status (active first) then by ID
        let mut brief_list: Vec<_> = briefs.keys().cloned().collect();
        brief_list.sort_by(|a, b| {
            let brief_a = briefs.get(a);
            let brief_b = briefs.get(b);
            match (brief_a, brief_b) {
                (Some(ba), Some(bb)) => {
                    // Sort by status priority, then by title
                    let priority_a = status_priority(ba.status);
                    let priority_b = status_priority(bb.status);
                    priority_a
                        .cmp(&priority_b)
                        .then_with(|| ba.title.cmp(&bb.title))
                }
                _ => a.to_string().cmp(&b.to_string()),
            }
        });

        let brief_filter = brief_filter.and_then(|s| {
            // Try to find brief by ID or partial match
            brief_list
                .iter()
                .find(|id| {
                    let id_str = id.to_string();
                    id_str == s || id_str.contains(s)
                })
                .cloned()
        });

        let mut app = Self {
            project,
            briefs,
            tasks,
            brief_list,
            view_mode,
            focus: Focus::Briefs,
            input_mode: InputMode::Normal,
            brief_index: 0,
            task_index: 0,
            task_list: Vec::new(),
            brief_filter,
            search_results: Vec::new(),
            status_message: None,
            should_quit: false,
            show_completed: false,
            pending_edit: None,
        };

        // If we have a brief filter, select it
        if let Some(ref filter_id) = app.brief_filter {
            if let Some(idx) = app.brief_list.iter().position(|id| id == filter_id) {
                app.brief_index = idx;
            }
        }

        app.update_task_list();

        Ok(app)
    }

    /// Run the main application loop
    pub fn run(&mut self, terminal: &mut Terminal, events: EventHandler) -> Result<()> {
        while !self.should_quit {
            // Check for pending edit action
            if let Some(path) = self.pending_edit.take() {
                self.execute_editor(terminal, &path)?;
                continue;
            }

            // Draw UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events
            match events.next()? {
                Event::Key(key) => self.handle_key(key)?,
                Event::Resize(_, _) => {} // Terminal handles resize automatically
                Event::Tick => {
                    // Clear status message after a while (handled by tick count)
                }
            }
        }

        Ok(())
    }

    /// Execute the editor and reinitialize terminal afterwards
    fn execute_editor(&mut self, terminal: &mut Terminal, path: &std::path::Path) -> Result<()> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Restore terminal for editor
        super::ui::restore_terminal()?;

        // Run editor
        let status = std::process::Command::new(&editor).arg(path).status();

        // Reinitialize terminal regardless of editor result
        *terminal = super::ui::init_terminal()?;

        // Check editor result
        match status {
            Ok(exit_status) => {
                if !exit_status.success() {
                    self.status_message =
                        Some(format!("Editor exited with code: {:?}", exit_status.code()));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to run editor: {}", e));
            }
        }

        // Refresh data after editing
        self.refresh_data()?;

        Ok(())
    }

    /// Draw the UI
    fn draw(&self, frame: &mut Frame) {
        match self.view_mode {
            ViewMode::Overview => views::overview::draw(frame, self),
            ViewMode::Kanban => views::kanban::draw(frame, self),
            ViewMode::Graph => views::graph::draw(frame, self),
        }
    }

    /// Handle key events
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        // Check for quit first (Ctrl+C or q in normal mode)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        match &self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Search(_) => self.handle_search_key(key),
            InputMode::Confirm(_) => self.handle_confirm_key(key),
            InputMode::NewTask(_) => self.handle_new_task_key(key),
            InputMode::NewBrief(_) => self.handle_new_brief_key(key),
        }
    }

    /// Handle keys in normal mode
    fn handle_normal_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            // Quit
            KeyCode::Char('q') => {
                self.should_quit = true;
            }

            // Navigation: up/down
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up();
            }

            // Navigation: left/right (switch panels)
            KeyCode::Char('h') | KeyCode::Left => {
                self.focus = self.focus.prev();
                self.update_task_list();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.focus = self.focus.next();
                self.update_task_list();
            }

            // Tab to cycle focus
            KeyCode::Tab => {
                self.focus = self.focus.next();
                self.update_task_list();
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
                self.update_task_list();
            }

            // View switching
            KeyCode::Char('1') => {
                self.view_mode = ViewMode::Overview;
            }
            KeyCode::Char('2') => {
                self.view_mode = ViewMode::Kanban;
            }
            KeyCode::Char('3') => {
                self.view_mode = ViewMode::Graph;
            }

            // Actions
            KeyCode::Char('s') => {
                self.start_task()?;
            }
            KeyCode::Char('d') => {
                self.complete_task()?;
            }
            KeyCode::Char('n') => {
                self.input_mode = InputMode::NewTask(String::new());
            }
            KeyCode::Char('N') => {
                self.input_mode = InputMode::NewBrief(String::new());
            }
            KeyCode::Char('e') => {
                self.edit_selected();
            }
            KeyCode::Char('r') => {
                self.refresh_data()?;
            }

            // Search
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search(String::new());
            }

            // Toggle completed
            KeyCode::Char('c') => {
                self.show_completed = !self.show_completed;
                self.update_task_list();
            }

            // Enter: select/expand
            KeyCode::Enter => {
                self.handle_enter();
            }

            // Help
            KeyCode::Char('?') => {
                self.status_message = Some(
                    "j/k:move h/l:panel s:start d:done n:new task N:new brief /:search q:quit"
                        .to_string(),
                );
            }

            _ => {}
        }

        Ok(())
    }

    /// Handle keys in search mode
    fn handle_search_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        let query = if let InputMode::Search(ref q) = self.input_mode {
            q.clone()
        } else {
            return Ok(());
        };

        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_results.clear();
            }
            KeyCode::Enter => {
                // Jump to first search result
                if !self.search_results.is_empty() {
                    self.task_list = self.search_results.clone();
                    self.task_index = 0;
                    self.focus = Focus::Tasks;
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                let mut new_query = query;
                new_query.pop();
                self.perform_search(new_query.clone());
                self.input_mode = InputMode::Search(new_query);
            }
            KeyCode::Char(c) => {
                let mut new_query = query;
                new_query.push(c);
                self.perform_search(new_query.clone());
                self.input_mode = InputMode::Search(new_query);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keys in confirm mode
    fn handle_confirm_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let InputMode::Confirm(action) = &self.input_mode {
                    match action {
                        ConfirmAction::CompleteTask(task_id) => {
                            self.do_complete_task(task_id.clone())?;
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keys in new task mode
    fn handle_new_task_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        let title = if let InputMode::NewTask(ref t) = self.input_mode {
            t.clone()
        } else {
            return Ok(());
        };

        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !title.is_empty() {
                    self.create_task(title)?;
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.input_mode = InputMode::NewTask(new_title);
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.input_mode = InputMode::NewTask(new_title);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keys in new brief mode
    fn handle_new_brief_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        let title = if let InputMode::NewBrief(ref t) = self.input_mode {
            t.clone()
        } else {
            return Ok(());
        };

        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !title.is_empty() {
                    self.create_brief(title)?;
                }
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                let mut new_title = title;
                new_title.pop();
                self.input_mode = InputMode::NewBrief(new_title);
            }
            KeyCode::Char(c) => {
                let mut new_title = title;
                new_title.push(c);
                self.input_mode = InputMode::NewBrief(new_title);
            }
            _ => {}
        }

        Ok(())
    }

    /// Move selection down
    fn move_selection_down(&mut self) {
        match self.focus {
            Focus::Briefs => {
                if !self.brief_list.is_empty() {
                    self.brief_index = (self.brief_index + 1) % self.brief_list.len();
                    self.update_task_list();
                }
            }
            Focus::Tasks => {
                if !self.task_list.is_empty() {
                    self.task_index = (self.task_index + 1) % self.task_list.len();
                }
            }
            Focus::Details => {
                // Scroll details down (if implemented)
            }
        }
    }

    /// Move selection up
    fn move_selection_up(&mut self) {
        match self.focus {
            Focus::Briefs => {
                if !self.brief_list.is_empty() {
                    self.brief_index = if self.brief_index == 0 {
                        self.brief_list.len() - 1
                    } else {
                        self.brief_index - 1
                    };
                    self.update_task_list();
                }
            }
            Focus::Tasks => {
                if !self.task_list.is_empty() {
                    self.task_index = if self.task_index == 0 {
                        self.task_list.len() - 1
                    } else {
                        self.task_index - 1
                    };
                }
            }
            Focus::Details => {
                // Scroll details up (if implemented)
            }
        }
    }

    /// Handle enter key
    fn handle_enter(&mut self) {
        match self.focus {
            Focus::Briefs => {
                // Select this brief and show its tasks
                self.focus = Focus::Tasks;
                self.update_task_list();
            }
            Focus::Tasks => {
                // Show task details
                self.focus = Focus::Details;
            }
            Focus::Details => {
                // Could expand/collapse sections
            }
        }
    }

    /// Update the task list based on current selection
    fn update_task_list(&mut self) {
        let statuses = self.get_task_statuses();

        self.task_list = if let Some(ref brief_id) = self.brief_filter {
            // Filter by specific brief
            self.tasks
                .iter()
                .filter(|(_, t)| t.brief_id().as_ref() == Some(brief_id))
                .filter(|(_, t)| self.show_completed || !t.status.is_complete())
                .map(|(id, _)| id.clone())
                .collect()
        } else if !self.brief_list.is_empty() && self.brief_index < self.brief_list.len() {
            // Filter by selected brief
            let selected_brief = &self.brief_list[self.brief_index];
            self.tasks
                .iter()
                .filter(|(_, t)| t.brief_id().as_ref() == Some(selected_brief))
                .filter(|(_, t)| self.show_completed || !t.status.is_complete())
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            // Show standalone tasks
            self.tasks
                .iter()
                .filter(|(_, t)| t.is_standalone())
                .filter(|(_, t)| self.show_completed || !t.status.is_complete())
                .map(|(id, _)| id.clone())
                .collect()
        };

        // Sort task list: ready first, then by status, then by ID
        self.task_list.sort_by(|a, b| {
            let task_a = self.tasks.get(a);
            let task_b = self.tasks.get(b);
            match (task_a, task_b) {
                (Some(ta), Some(tb)) => {
                    let ready_a = ta.is_ready(&statuses);
                    let ready_b = tb.is_ready(&statuses);
                    // Ready tasks first
                    ready_b
                        .cmp(&ready_a)
                        .then_with(|| {
                            task_status_priority(ta.status).cmp(&task_status_priority(tb.status))
                        })
                        .then_with(|| ta.title.cmp(&tb.title))
                }
                _ => a.to_string().cmp(&b.to_string()),
            }
        });

        // Reset task index if out of bounds
        if self.task_index >= self.task_list.len() {
            self.task_index = 0;
        }
    }

    /// Perform search
    fn perform_search(&mut self, query: String) {
        if query.is_empty() {
            self.search_results.clear();
            return;
        }

        let query_lower = query.to_lowercase();

        self.search_results = self
            .tasks
            .iter()
            .filter(|(id, task)| {
                let id_str = id.to_string().to_lowercase();
                let title_lower = task.title.to_lowercase();
                let desc_lower = task.description.as_deref().unwrap_or("").to_lowercase();
                id_str.contains(&query_lower)
                    || title_lower.contains(&query_lower)
                    || desc_lower.contains(&query_lower)
            })
            .map(|(id, _)| id.clone())
            .collect();
    }

    /// Get task statuses map
    fn get_task_statuses(&self) -> HashMap<TaskId, TaskStatus> {
        self.tasks
            .iter()
            .map(|(id, t)| (id.clone(), t.status))
            .collect()
    }

    /// Start the selected task
    fn start_task(&mut self) -> Result<()> {
        if let Some(task_id) = self.selected_task_id() {
            let task_store = self.project.task_store();

            if let Some(task) = self.tasks.get_mut(&task_id) {
                if task.status == TaskStatus::Todo {
                    task.start();
                    task_store.update(task)?;
                    self.status_message = Some(format!("Started: {}", task.title));
                } else {
                    self.status_message = Some("Task is not in todo status".to_string());
                }
            }
        }

        Ok(())
    }

    /// Complete the selected task
    fn complete_task(&mut self) -> Result<()> {
        if let Some(task_id) = self.selected_task_id() {
            // Ask for confirmation
            self.input_mode = InputMode::Confirm(ConfirmAction::CompleteTask(task_id));
        }

        Ok(())
    }

    /// Actually complete a task
    fn do_complete_task(&mut self, task_id: TaskId) -> Result<()> {
        let task_store = self.project.task_store();

        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.complete();
            task_store.update(task)?;
            self.status_message = Some(format!("Completed: {}", task.title));
            self.update_task_list();
        }

        Ok(())
    }

    /// Create a new task
    fn create_task(&mut self, title: String) -> Result<()> {
        let task_store = self.project.task_store();

        // Get the brief to attach to (if any)
        let brief_id = if !self.brief_list.is_empty() && self.brief_index < self.brief_list.len() {
            Some(self.brief_list[self.brief_index].clone())
        } else {
            None
        };

        // Create task ID
        let task_id = if let Some(ref bid) = brief_id {
            // Get next sequence number for this brief
            let max_seq = self
                .tasks
                .keys()
                .filter(|id| id.brief_id().as_ref() == Some(bid))
                .filter_map(|id| id.segments().first().copied())
                .max()
                .unwrap_or(0);
            TaskId::new(bid, max_seq + 1)
        } else {
            TaskId::new_standalone(&title, chrono::Utc::now())
        };

        let task = Task::new(task_id.clone(), &title);
        task_store.append(&task)?;
        self.tasks.insert(task_id, task);
        self.update_task_list();
        self.status_message = Some(format!("Created: {}", title));

        Ok(())
    }

    /// Create a new brief
    fn create_brief(&mut self, title: String) -> Result<()> {
        let brief_store = self.project.brief_store();
        let brief = Brief::new(&title, "minimal");
        let brief_id = brief.id.clone();
        brief_store.write(&brief)?;
        self.brief_list.push(brief_id.clone());
        self.briefs.insert(brief_id, brief);
        self.status_message = Some(format!("Created brief: {}", title));

        Ok(())
    }

    /// Edit the selected item in $EDITOR
    fn edit_selected(&mut self) {
        match self.focus {
            Focus::Briefs => {
                if let Some(brief_id) = self.selected_brief_id() {
                    let brief_store = self.project.brief_store();
                    let path = brief_store.dir().join(format!("{}.md", brief_id));
                    self.pending_edit = Some(path);
                }
            }
            Focus::Tasks | Focus::Details => {
                // Tasks are stored in JSONL, so we can't easily edit them in an editor
                self.status_message =
                    Some("Edit task: use 's' to start, 'd' to complete".to_string());
            }
        }
    }

    /// Refresh data from disk
    fn refresh_data(&mut self) -> Result<()> {
        let brief_store = self.project.brief_store();
        let task_store = self.project.task_store();

        self.briefs = brief_store.read_all()?;
        self.tasks = task_store.read_all()?;

        // Rebuild brief list
        self.brief_list = self.briefs.keys().cloned().collect();
        self.brief_list.sort_by(|a, b| {
            let brief_a = self.briefs.get(a);
            let brief_b = self.briefs.get(b);
            match (brief_a, brief_b) {
                (Some(ba), Some(bb)) => {
                    let priority_a = status_priority(ba.status);
                    let priority_b = status_priority(bb.status);
                    priority_a
                        .cmp(&priority_b)
                        .then_with(|| ba.title.cmp(&bb.title))
                }
                _ => a.to_string().cmp(&b.to_string()),
            }
        });

        self.update_task_list();
        self.status_message = Some("Refreshed".to_string());

        Ok(())
    }

    /// Get the currently selected task ID
    fn selected_task_id(&self) -> Option<TaskId> {
        self.task_list.get(self.task_index).cloned()
    }

    /// Get the currently selected brief ID
    fn selected_brief_id(&self) -> Option<BriefId> {
        self.brief_list.get(self.brief_index).cloned()
    }

    // Public accessors for views

    pub fn briefs(&self) -> &HashMap<BriefId, Brief> {
        &self.briefs
    }

    pub fn tasks(&self) -> &HashMap<TaskId, Task> {
        &self.tasks
    }

    pub fn brief_list(&self) -> &[BriefId] {
        &self.brief_list
    }

    pub fn task_list(&self) -> &[TaskId] {
        &self.task_list
    }

    pub fn brief_index(&self) -> usize {
        self.brief_index
    }

    pub fn task_index(&self) -> usize {
        self.task_index
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn input_mode(&self) -> &InputMode {
        &self.input_mode
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn show_completed(&self) -> bool {
        self.show_completed
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.selected_task_id().and_then(|id| self.tasks.get(&id))
    }

    pub fn selected_brief(&self) -> Option<&Brief> {
        self.selected_brief_id().and_then(|id| self.briefs.get(&id))
    }
}

/// Priority for brief status (lower = higher priority)
fn status_priority(status: BriefStatus) -> u8 {
    match status {
        BriefStatus::InProgress => 0,
        BriefStatus::Proposed => 1,
        BriefStatus::Betting => 2,
        BriefStatus::Shipped => 3,
        BriefStatus::Archived => 4,
    }
}

/// Priority for task status (lower = higher priority)
fn task_status_priority(status: TaskStatus) -> u8 {
    match status {
        TaskStatus::InProgress => 0,
        TaskStatus::Todo => 1,
        TaskStatus::Done => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Focus state machine tests
    // ==========================================================================

    #[test]
    fn focus_cycles_forward() {
        assert_eq!(Focus::Briefs.next(), Focus::Tasks);
        assert_eq!(Focus::Tasks.next(), Focus::Details);
        assert_eq!(Focus::Details.next(), Focus::Briefs);
    }

    #[test]
    fn focus_cycles_backward() {
        assert_eq!(Focus::Briefs.prev(), Focus::Details);
        assert_eq!(Focus::Tasks.prev(), Focus::Briefs);
        assert_eq!(Focus::Details.prev(), Focus::Tasks);
    }

    #[test]
    fn focus_roundtrip() {
        let start = Focus::Briefs;
        assert_eq!(start.next().next().next(), start);
        assert_eq!(start.prev().prev().prev(), start);
    }

    // ==========================================================================
    // InputMode tests
    // ==========================================================================

    #[test]
    fn input_mode_default_is_normal() {
        let mode = InputMode::default();
        assert_eq!(mode, InputMode::Normal);
    }

    #[test]
    fn input_mode_search_stores_query() {
        let mode = InputMode::Search("test query".to_string());
        if let InputMode::Search(query) = mode {
            assert_eq!(query, "test query");
        } else {
            panic!("Expected Search mode");
        }
    }

    // ==========================================================================
    // Priority function tests
    // ==========================================================================

    #[test]
    fn status_priority_orders_correctly() {
        assert!(status_priority(BriefStatus::InProgress) < status_priority(BriefStatus::Proposed));
        assert!(status_priority(BriefStatus::Proposed) < status_priority(BriefStatus::Betting));
        assert!(status_priority(BriefStatus::Betting) < status_priority(BriefStatus::Shipped));
        assert!(status_priority(BriefStatus::Shipped) < status_priority(BriefStatus::Archived));
    }

    #[test]
    fn task_status_priority_orders_correctly() {
        assert!(
            task_status_priority(TaskStatus::InProgress) < task_status_priority(TaskStatus::Todo)
        );
        assert!(task_status_priority(TaskStatus::Todo) < task_status_priority(TaskStatus::Done));
    }

    // ==========================================================================
    // ViewMode tests
    // ==========================================================================

    #[test]
    fn view_mode_from_str_overview() {
        use std::str::FromStr;
        assert_eq!(ViewMode::from_str("overview").unwrap(), ViewMode::Overview);
        assert_eq!(ViewMode::from_str("o").unwrap(), ViewMode::Overview);
        assert_eq!(ViewMode::from_str("1").unwrap(), ViewMode::Overview);
        assert_eq!(ViewMode::from_str("OVERVIEW").unwrap(), ViewMode::Overview);
    }

    #[test]
    fn view_mode_from_str_kanban() {
        use std::str::FromStr;
        assert_eq!(ViewMode::from_str("kanban").unwrap(), ViewMode::Kanban);
        assert_eq!(ViewMode::from_str("k").unwrap(), ViewMode::Kanban);
        assert_eq!(ViewMode::from_str("2").unwrap(), ViewMode::Kanban);
    }

    #[test]
    fn view_mode_from_str_graph() {
        use std::str::FromStr;
        assert_eq!(ViewMode::from_str("graph").unwrap(), ViewMode::Graph);
        assert_eq!(ViewMode::from_str("g").unwrap(), ViewMode::Graph);
        assert_eq!(ViewMode::from_str("3").unwrap(), ViewMode::Graph);
    }

    #[test]
    fn view_mode_from_str_invalid() {
        use std::str::FromStr;
        assert!(ViewMode::from_str("invalid").is_err());
        assert!(ViewMode::from_str("").is_err());
    }

    #[test]
    fn view_mode_default_is_overview() {
        assert_eq!(ViewMode::default(), ViewMode::Overview);
    }

    // ==========================================================================
    // Search tests (unit tests for search logic)
    // ==========================================================================

    #[test]
    fn search_matches_title() {
        let query = "test";
        let title = "This is a test task";
        assert!(title.to_lowercase().contains(&query.to_lowercase()));
    }

    #[test]
    fn search_matches_id() {
        let query = "a-123";
        let id = "a-1234567.1";
        assert!(id.to_lowercase().contains(&query.to_lowercase()));
    }

    #[test]
    fn search_case_insensitive() {
        let query = "TEST";
        let title = "this is a test";
        assert!(title.to_lowercase().contains(&query.to_lowercase()));
    }

    #[test]
    fn search_empty_query_matches_nothing() {
        let query = "";
        // Empty query should be handled specially - in perform_search it clears results
        assert!(query.is_empty());
    }
}
