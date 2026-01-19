//! Main CLI application structure

use anyhow::Result;
use clap::{Parser, Subcommand};

use super::output::{Output, OutputFormat};
use super::{
    agent_setup, brief, cache_cmd, compact, context, daemon, merge_driver, plugin_cmd, query,
    sync_cmd, task, tui,
};
use crate::storage::Project;

#[derive(Parser)]
#[command(name = "shape")]
#[command(
    author,
    version,
    about = "Local-first task management for software teams"
)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Output format
    #[arg(long, short = 'f', global = true, default_value = "text")]
    pub format: OutputFormat,

    /// Enable verbose output for debugging
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new shape project
    Init {
        /// Path to initialize (defaults to current directory)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Manage briefs (pitches, RFCs, etc.)
    #[command(subcommand)]
    Brief(brief::BriefCommands),

    /// Manage tasks
    #[command(subcommand)]
    Task(task::TaskCommands),

    /// Show tasks ready to work on
    Ready {
        /// Filter by brief ID
        #[arg(long)]
        brief: Option<String>,
    },

    /// Show blocked tasks
    Blocked {
        /// Filter by brief ID
        #[arg(long)]
        brief: Option<String>,
    },

    /// Show project status overview
    Status,

    /// Export project context for AI
    Context {
        /// Compact mode (minimal tokens)
        #[arg(long, short)]
        compact: bool,

        /// Filter by brief ID
        #[arg(long)]
        brief: Option<String>,

        /// Days of completed tasks to include
        #[arg(long, default_value = "7")]
        days: u32,
    },

    /// Compact old completed tasks into summaries
    Compact {
        /// Days threshold for compaction (default: 14)
        #[arg(long, default_value = "14")]
        days: u32,

        /// Filter by brief ID
        #[arg(long)]
        brief: Option<String>,

        /// Preview without making changes
        #[arg(long)]
        dry_run: bool,

        /// Compaction strategy (basic, smart, llm)
        #[arg(long)]
        strategy: Option<String>,

        /// Undo compaction for a specific task
        #[arg(long)]
        undo: Option<String>,
    },

    /// Configure AI agent integration
    AgentSetup {
        /// Preview instructions without writing to files
        #[arg(long)]
        show: bool,

        /// Only configure CLAUDE.md
        #[arg(long)]
        claude: bool,

        /// Only configure .cursorrules
        #[arg(long)]
        cursor: bool,

        /// Only configure .windsurfrules
        #[arg(long)]
        windsurf: bool,
    },

    /// Manage the SQLite cache
    #[command(subcommand)]
    Cache(cache_cmd::CacheCommands),

    /// Search tasks and briefs
    Search {
        /// Search query
        query: String,
    },

    /// Git merge driver for tasks.jsonl (internal use)
    #[command(hide = true)]
    MergeDriver {
        /// Path to base version (common ancestor)
        base: std::path::PathBuf,

        /// Path to ours version (current branch) - output written here
        ours: std::path::PathBuf,

        /// Path to theirs version (branch being merged)
        theirs: std::path::PathBuf,
    },

    /// Configure git merge driver for this repository
    MergeSetup,

    /// Background daemon for automatic git sync
    #[command(subcommand)]
    Daemon(daemon::DaemonCommands),

    /// Advanced commands (plugins, sync)
    #[command(subcommand)]
    Advanced(AdvancedCommands),

    /// Launch interactive TUI viewer
    Tui {
        /// Start focused on a specific brief
        #[arg(short, long)]
        brief: Option<String>,

        /// Start with a specific view (overview, kanban, graph)
        #[arg(short, long, default_value = "overview")]
        view: String,
    },
}

/// Advanced commands for plugins and external sync
#[derive(Subcommand)]
pub enum AdvancedCommands {
    /// Manage plugins
    #[command(subcommand)]
    Plugin(plugin_cmd::PluginCommands),

    /// Sync with external tools
    #[command(subcommand)]
    Sync(sync_cmd::SyncCommands),
}

/// Main entry point for the CLI
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let output = Output::new(cli.format, cli.verbose);

    output.verbose("Shape CLI starting");

    match cli.command {
        Commands::Init { path } => {
            output.verbose_ctx("init", &format!("Initializing project at: {}", path));
            let project = Project::init(&path)?;
            output.verbose_ctx(
                "init",
                &format!(
                    "Created .shape directory at: {}",
                    project.shape_dir().display()
                ),
            );
            output.success(&format!(
                "Initialized shape project at {}",
                project.root().display()
            ));
        }

        Commands::Brief(cmd) => brief::run(cmd, &output)?,
        Commands::Task(cmd) => task::run(cmd, &output)?,

        Commands::Ready { brief } => {
            output.verbose_ctx(
                "ready",
                &format!("Querying ready tasks, brief filter: {:?}", brief),
            );
            query::ready(&output, brief.as_deref())?
        }
        Commands::Blocked { brief } => {
            output.verbose_ctx(
                "blocked",
                &format!("Querying blocked tasks, brief filter: {:?}", brief),
            );
            query::blocked(&output, brief.as_deref())?
        }
        Commands::Status => {
            output.verbose("Gathering project status");
            query::status(&output)?
        }

        Commands::Context {
            compact: compact_mode,
            brief,
            days,
        } => {
            output.verbose_ctx(
                "context",
                &format!(
                    "Exporting context: compact={}, brief={:?}, days={}",
                    compact_mode, brief, days
                ),
            );
            context::export(&output, compact_mode, brief.as_deref(), days)?
        }

        Commands::Compact {
            days,
            brief,
            dry_run,
            strategy,
            undo,
        } => {
            if let Some(task_id) = undo {
                compact::undo(&output, &task_id)?
            } else {
                compact::run(
                    &output,
                    days,
                    brief.as_deref(),
                    dry_run,
                    strategy.as_deref(),
                )?
            }
        }

        Commands::AgentSetup {
            show,
            claude,
            cursor,
            windsurf,
        } => agent_setup::run(&output, show, claude, cursor, windsurf)?,

        Commands::Cache(cmd) => cache_cmd::run(cmd, &output)?,

        Commands::Search { query } => search(&output, &query)?,

        Commands::MergeDriver { base, ours, theirs } => {
            // This is called by git, return the exit code directly
            let exit_code = merge_driver::run_merge_driver(&base, &ours, &theirs)?;
            std::process::exit(exit_code);
        }

        Commands::MergeSetup => setup_merge_driver(&output)?,

        Commands::Daemon(cmd) => daemon::run(cmd, &output)?,

        Commands::Advanced(advanced_cmd) => match advanced_cmd {
            AdvancedCommands::Plugin(cmd) => plugin_cmd::run(cmd, &output)?,
            AdvancedCommands::Sync(cmd) => sync_cmd::run(cmd, &output)?,
        },

        Commands::Tui { brief, view } => {
            output.verbose_ctx("tui", &format!("Launching TUI, brief={:?}, view={}", brief, view));
            tui::run(&output, brief.as_deref(), &view)?
        }
    }

    output.verbose("Command completed successfully");
    Ok(())
}

/// Search tasks and briefs using the SQLite cache
fn search(output: &Output, query: &str) -> Result<()> {
    use crate::storage::SearchResultType;

    let project = Project::open_current()?;
    output.verbose_ctx("search", &format!("Searching for: {}", query));

    // Ensure cache is up to date
    let cache = project.get_or_rebuild_cache()?;

    let results = cache.search(query)?;
    output.verbose_ctx("search", &format!("Found {} results", results.len()));

    if output.is_json() {
        let items: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "title": r.title,
                    "snippet": r.snippet,
                    "type": match r.result_type {
                        SearchResultType::Task => "task",
                        SearchResultType::Brief => "brief",
                    },
                })
            })
            .collect();
        output.data(&items);
    } else if results.is_empty() {
        println!("No results found for '{}'", query);
    } else {
        println!("Search results for '{}':", query);
        println!("{:<12} {:<20} TITLE", "TYPE", "ID");
        println!("{}", "-".repeat(70));

        for result in &results {
            let type_str = match result.result_type {
                SearchResultType::Task => "task",
                SearchResultType::Brief => "brief",
            };
            println!("{:<12} {:<20} {}", type_str, result.id, result.title);

            // Show snippet if not empty
            if !result.snippet.is_empty() && result.snippet != result.title {
                // Clean up HTML tags in snippet for terminal display
                let clean_snippet = result
                    .snippet
                    .replace("<mark>", "\x1b[1m")
                    .replace("</mark>", "\x1b[0m");
                println!("             {}", clean_snippet);
            }
        }

        println!();
        println!("Found {} result(s)", results.len());
    }

    Ok(())
}

/// Sets up git merge driver for tasks.jsonl
fn setup_merge_driver(output: &Output) -> Result<()> {
    use std::fs;
    use std::process::Command;

    let project = Project::open_current()?;

    // Check if we're in a git repository
    let git_status = Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .current_dir(project.root())
        .output();

    if git_status.is_err() || !git_status.unwrap().status.success() {
        anyhow::bail!("Not in a git repository");
    }

    // 1. Create/update .gitattributes
    let gitattributes_path = project.root().join(".gitattributes");
    let gitattributes_entry = ".shape/tasks.jsonl merge=shape-tasks\n";

    let existing_content = fs::read_to_string(&gitattributes_path).unwrap_or_default();

    if !existing_content.contains("merge=shape-tasks") {
        let new_content = if existing_content.is_empty() {
            gitattributes_entry.to_string()
        } else if existing_content.ends_with('\n') {
            format!("{}{}", existing_content, gitattributes_entry)
        } else {
            format!("{}\n{}", existing_content, gitattributes_entry)
        };

        fs::write(&gitattributes_path, new_content)?;
        output.success("Updated .gitattributes with merge driver");
    } else {
        output.verbose(".gitattributes already configured");
    }

    // 2. Configure git merge driver (local to repo)
    let config_commands = [
        ("merge.shape-tasks.name", "Shape tasks merge driver"),
        ("merge.shape-tasks.driver", "shape merge-driver %O %A %B"),
    ];

    for (key, value) in config_commands {
        let status = Command::new("git")
            .args(["config", "--local", key, value])
            .current_dir(project.root())
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to configure git: {} = {}", key, value);
        }
    }

    output.success("Configured git merge driver 'shape-tasks'");

    if output.is_json() {
        output.data(&serde_json::json!({
            "configured": true,
            "gitattributes": gitattributes_path.display().to_string(),
            "driver": "shape merge-driver %O %A %B",
        }));
    } else {
        println!();
        println!("Git merge driver setup complete.");
        println!();
        println!("When you merge branches with conflicting task edits, git will");
        println!("automatically use the shape merge driver to resolve conflicts");
        println!("at the field level (last-write-wins based on timestamps).");
    }

    Ok(())
}
