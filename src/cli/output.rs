//! Output formatting for CLI commands

use serde::Serialize;

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Output helper for consistent formatting
pub struct Output {
    format: OutputFormat,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Prints a success message
    pub fn success(&self, message: &str) {
        match self.format {
            OutputFormat::Text => println!("{}", message),
            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "success": true,
                        "message": message
                    })
                );
            }
        }
    }

    /// Prints an error message
    pub fn error(&self, message: &str) {
        match self.format {
            OutputFormat::Text => eprintln!("Error: {}", message),
            OutputFormat::Json => {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "success": false,
                        "error": message
                    })
                );
            }
        }
    }

    /// Prints structured data
    pub fn data<T: Serialize>(&self, data: &T) {
        match self.format {
            OutputFormat::Text => {
                // For text format, we expect the caller to handle it
                // This is a fallback that pretty-prints JSON
                if let Ok(json) = serde_json::to_string_pretty(data) {
                    println!("{}", json);
                }
            }
            OutputFormat::Json => {
                if let Ok(json) = serde_json::to_string(data) {
                    println!("{}", json);
                }
            }
        }
    }

    /// Prints a table row (text only, ignored in JSON mode)
    pub fn row(&self, columns: &[&str]) {
        if self.format == OutputFormat::Text {
            println!("{}", columns.join("\t"));
        }
    }

    /// Prints a blank line (text only)
    pub fn blank(&self) {
        if self.format == OutputFormat::Text {
            println!();
        }
    }

    /// Returns true if using JSON format
    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }

    /// Returns true if using text format
    pub fn is_text(&self) -> bool {
        self.format == OutputFormat::Text
    }
}

/// Helper trait for types that can be displayed as text
#[allow(dead_code)]
pub trait TextDisplay {
    fn display_text(&self, output: &Output);
}
