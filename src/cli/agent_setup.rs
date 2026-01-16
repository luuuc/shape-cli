//! Agent setup command for AI integration
//!
//! Detects and configures AI agent config files (CLAUDE.md, .cursorrules, etc.)
//! with Shape CLI instructions.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use super::output::Output;
use crate::storage::Project;

/// Agent configuration files we support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentConfig {
    Claude,
    Cursor,
    Windsurf,
    Agents,
}

impl AgentConfig {
    /// Filename for this config type
    pub fn filename(&self) -> &'static str {
        match self {
            AgentConfig::Claude => "CLAUDE.md",
            AgentConfig::Cursor => ".cursorrules",
            AgentConfig::Windsurf => ".windsurfrules",
            AgentConfig::Agents => "AGENTS.md",
        }
    }

    /// All supported config types
    pub fn all() -> &'static [AgentConfig] {
        &[
            AgentConfig::Claude,
            AgentConfig::Cursor,
            AgentConfig::Windsurf,
            AgentConfig::Agents,
        ]
    }
}

/// The marker that indicates our section has been added
const SECTION_MARKER: &str = "## Shape CLI (Task Management)";

/// Instructions to add to AI config files
const INSTRUCTIONS: &str = r#"## Shape CLI (Task Management)

This project uses Shape CLI for task tracking.

### Before starting work
- `shape ready` - See unblocked tasks
- `shape task start <id>` - Mark task in progress

### After completing work
- `shape task done <id>` - Mark task complete
- `shape status` - Verify project state

### Getting context
- `shape context --compact` - Full project state (token-optimized)
- `shape anchor show <id>` - Single anchor details

Always check `shape ready` before starting work.
"#;

/// Result of configuring a single file
#[derive(Debug, Serialize)]
pub struct ConfigResult {
    pub filename: String,
    pub configured: bool,
    pub already_configured: bool,
    pub created: bool,
}

/// Result of the agent-setup command
#[derive(Debug, Serialize)]
pub struct AgentSetupResult {
    pub files: Vec<ConfigResult>,
    pub instructions: String,
}

/// Check if a file already has the Shape section
fn has_shape_section(content: &str) -> bool {
    content.contains(SECTION_MARKER)
}

/// Detect which config files exist in the project root
fn detect_configs(root: &Path) -> Vec<(AgentConfig, PathBuf)> {
    AgentConfig::all()
        .iter()
        .filter_map(|config| {
            let path = root.join(config.filename());
            if path.exists() {
                Some((*config, path))
            } else {
                None
            }
        })
        .collect()
}

/// Configure a single file
fn configure_file(path: &Path) -> Result<ConfigResult> {
    let filename = path.file_name().unwrap().to_string_lossy().to_string();

    if path.exists() {
        let content = fs::read_to_string(path)?;

        if has_shape_section(&content) {
            return Ok(ConfigResult {
                filename,
                configured: false,
                already_configured: true,
                created: false,
            });
        }

        // Append instructions with a blank line separator
        let new_content = if content.ends_with('\n') {
            format!("{}\n{}", content, INSTRUCTIONS)
        } else if content.is_empty() {
            INSTRUCTIONS.to_string()
        } else {
            format!("{}\n\n{}", content, INSTRUCTIONS)
        };

        fs::write(path, new_content)?;

        Ok(ConfigResult {
            filename,
            configured: true,
            already_configured: false,
            created: false,
        })
    } else {
        // Create new file
        fs::write(path, INSTRUCTIONS)?;

        Ok(ConfigResult {
            filename,
            configured: true,
            already_configured: false,
            created: true,
        })
    }
}

/// Run the agent-setup command
pub fn run(
    output: &Output,
    show_only: bool,
    claude_only: bool,
    cursor_only: bool,
    windsurf_only: bool,
) -> Result<()> {
    let project = Project::open_current()?;
    let root = project.root();

    // If --show, just print the instructions
    if show_only {
        if output.is_json() {
            output.data(&AgentSetupResult {
                files: vec![],
                instructions: INSTRUCTIONS.to_string(),
            });
        } else {
            println!("{}", INSTRUCTIONS);
        }
        return Ok(());
    }

    // Determine which configs to target
    let target_configs: Vec<AgentConfig> = if claude_only || cursor_only || windsurf_only {
        let mut targets = vec![];
        if claude_only {
            targets.push(AgentConfig::Claude);
        }
        if cursor_only {
            targets.push(AgentConfig::Cursor);
        }
        if windsurf_only {
            targets.push(AgentConfig::Windsurf);
        }
        targets
    } else {
        // Auto-detect existing files
        let detected = detect_configs(root);
        if detected.is_empty() {
            // No files found, prompt or create CLAUDE.md by default
            vec![AgentConfig::Claude]
        } else {
            detected.into_iter().map(|(config, _)| config).collect()
        }
    };

    // Configure each target
    let mut results = vec![];
    for config in target_configs {
        let path = root.join(config.filename());
        let result = configure_file(&path)?;
        results.push(result);
    }

    // Output results
    if output.is_json() {
        output.data(&AgentSetupResult {
            files: results,
            instructions: INSTRUCTIONS.to_string(),
        });
    } else {
        let mut configured_any = false;

        for result in &results {
            if result.already_configured {
                println!("{}: already configured", result.filename);
            } else if result.created {
                println!("{}: created with Shape CLI instructions", result.filename);
                configured_any = true;
            } else if result.configured {
                println!("{}: added Shape CLI instructions", result.filename);
                configured_any = true;
            }
        }

        if configured_any {
            output.blank();
            output.success("AI agent configuration complete.");
        } else if results.iter().all(|r| r.already_configured) {
            output.success("All config files already have Shape CLI instructions.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn has_shape_section_detects_marker() {
        assert!(has_shape_section("# My Project\n\n## Shape CLI (Task Management)\n"));
        assert!(!has_shape_section("# My Project\n\n## Other Section\n"));
    }

    #[test]
    fn configure_new_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("CLAUDE.md");

        let result = configure_file(&path).unwrap();

        assert!(result.configured);
        assert!(result.created);
        assert!(!result.already_configured);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(SECTION_MARKER));
    }

    #[test]
    fn configure_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("CLAUDE.md");

        fs::write(&path, "# My Project\n\nSome instructions.\n").unwrap();

        let result = configure_file(&path).unwrap();

        assert!(result.configured);
        assert!(!result.created);
        assert!(!result.already_configured);

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# My Project"));
        assert!(content.contains(SECTION_MARKER));
    }

    #[test]
    fn configure_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("CLAUDE.md");

        fs::write(&path, "# My Project\n").unwrap();

        // First call
        configure_file(&path).unwrap();
        let content_after_first = fs::read_to_string(&path).unwrap();

        // Second call
        let result = configure_file(&path).unwrap();

        assert!(!result.configured);
        assert!(result.already_configured);

        let content_after_second = fs::read_to_string(&path).unwrap();
        assert_eq!(content_after_first, content_after_second);
    }

    #[test]
    fn detect_existing_configs() {
        let dir = TempDir::new().unwrap();

        // Create only CLAUDE.md
        fs::write(dir.path().join("CLAUDE.md"), "# Test").unwrap();

        let detected = detect_configs(dir.path());

        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].0, AgentConfig::Claude);
    }

    #[test]
    fn detect_multiple_configs() {
        let dir = TempDir::new().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "# Test").unwrap();
        fs::write(dir.path().join(".cursorrules"), "rules").unwrap();

        let detected = detect_configs(dir.path());

        assert_eq!(detected.len(), 2);
    }
}
