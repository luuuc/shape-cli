//! Interactive TUI viewer for Shape
//!
//! Provides a terminal-based interface for browsing and managing
//! briefs and tasks using ratatui.

mod app;
mod event;
mod ui;
mod utils;
mod views;

use std::panic::{self, AssertUnwindSafe};
use std::str::FromStr;

use anyhow::{anyhow, Result};

use super::Output;
use app::App;
use event::EventHandler;

/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Overview,
    Kanban,
    Graph,
}

impl FromStr for ViewMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "overview" | "o" | "1" => Ok(ViewMode::Overview),
            "kanban" | "k" | "2" => Ok(ViewMode::Kanban),
            "graph" | "g" | "3" => Ok(ViewMode::Graph),
            _ => Err(()),
        }
    }
}

/// Launch the TUI
pub fn run(output: &Output, anchor_filter: Option<&str>, view: &str) -> Result<()> {
    output.verbose_ctx("tui", "Initializing TUI application");

    let view_mode = view.parse().unwrap_or_default();

    // Initialize terminal
    let mut terminal = ui::init_terminal()?;

    // Create app state
    let app_result = App::new(anchor_filter, view_mode);

    // Handle app creation failure - restore terminal first
    let mut app = match app_result {
        Ok(app) => app,
        Err(e) => {
            ui::restore_terminal()?;
            return Err(e);
        }
    };

    // Create event handler
    let event_handler = EventHandler::new(250);

    // Run the main loop with panic safety
    // This ensures terminal is restored even if the app panics
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        app.run(&mut terminal, event_handler)
    }));

    // Always restore terminal, even on panic
    let restore_result = ui::restore_terminal();

    // Handle the result
    match result {
        Ok(inner_result) => {
            restore_result?;
            inner_result
        }
        Err(panic_payload) => {
            // Try to restore terminal first
            let _ = restore_result;
            // Re-raise the panic with context
            if let Some(s) = panic_payload.downcast_ref::<&str>() {
                Err(anyhow!("TUI panicked: {}", s))
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                Err(anyhow!("TUI panicked: {}", s))
            } else {
                Err(anyhow!("TUI panicked with unknown error"))
            }
        }
    }
}
