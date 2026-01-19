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
    verbose: bool,
}

impl Output {
    pub fn new(format: OutputFormat, verbose: bool) -> Self {
        Self { format, verbose }
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

    /// Returns true if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Prints a verbose debug message (only when --verbose is set)
    pub fn verbose(&self, message: &str) {
        if self.verbose {
            eprintln!("[verbose] {}", message);
        }
    }

    /// Prints a verbose debug message with context (only when --verbose is set)
    pub fn verbose_ctx(&self, context: &str, message: &str) {
        if self.verbose {
            eprintln!("[verbose:{}] {}", context, message);
        }
    }
}

/// Helper trait for types that can be displayed as text
#[allow(dead_code)]
pub trait TextDisplay {
    fn display_text(&self, output: &Output);
}
