//! Event handling for the TUI

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};

/// Terminal events
#[derive(Debug)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Terminal resize event (width, height - currently unused but kept for future)
    #[allow(dead_code)]
    Resize(u16, u16),
    /// Tick event for periodic updates
    Tick,
}

/// Handles terminal events in a separate thread
pub struct EventHandler {
    /// Event receiver
    rx: mpsc::Receiver<Event>,
    /// Event sender (for sending from the event thread)
    #[allow(dead_code)]
    tx: mpsc::Sender<Event>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate in milliseconds
    pub fn new(tick_rate_ms: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate_ms);
        let (tx, rx) = mpsc::channel();
        let tx_clone = tx.clone();

        thread::spawn(move || {
            loop {
                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                // Only send key press events, not release
                                if key.kind == KeyEventKind::Press
                                    && tx_clone.send(Event::Key(key)).is_err()
                                {
                                    break;
                                }
                            }
                            CrosstermEvent::Resize(w, h) => {
                                if tx_clone.send(Event::Resize(w, h)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Send tick event
                    if tx_clone.send(Event::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// Receive the next event (blocking)
    pub fn next(&self) -> Result<Event> {
        Ok(self.rx.recv()?)
    }
}
