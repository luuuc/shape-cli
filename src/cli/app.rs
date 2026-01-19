//! Main CLI application structure

use clap::{Parser, Subcommand};
use anyhow::Result;

use super::output::{Output, OutputFormat};
use super::{anchor, task, query, context, plugin_cmd, sync_cmd, agent_setup};
use crate::storage::Project;

#[derive(Parser)]
#[command(name = "shape")]
#[command(author, version, about = "Local-first task management for software teams")]
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

    /// Manage anchors (pitches, RFCs, etc.)
    #[command(subcommand)]
    Anchor(anchor::AnchorCommands),

    /// Manage tasks
    #[command(subcommand)]
    Task(task::TaskCommands),

    /// Show tasks ready to work on
    Ready {
        /// Filter by anchor ID
        #[arg(long)]
        anchor: Option<String>,
    },

    /// Show blocked tasks
    Blocked {
        /// Filter by anchor ID
        #[arg(long)]
        anchor: Option<String>,
    },

    /// Show project status overview
    Status,

    /// Export project context for AI
    Context {
        /// Compact mode (minimal tokens)
        #[arg(long, short)]
        compact: bool,

        /// Filter by anchor ID
        #[arg(long)]
        anchor: Option<String>,

        /// Days of completed tasks to include
        #[arg(long, default_value = "7")]
        days: u32,
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

    /// Advanced commands (plugins, sync)
    #[command(subcommand)]
    Advanced(AdvancedCommands),
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
            output.verbose_ctx("init", &format!("Created .shape directory at: {}", project.shape_dir().display()));
            output.success(&format!("Initialized shape project at {}", project.root().display()));
        }

        Commands::Anchor(cmd) => anchor::run(cmd, &output)?,
        Commands::Task(cmd) => task::run(cmd, &output)?,

        Commands::Ready { anchor } => {
            output.verbose_ctx("ready", &format!("Querying ready tasks, anchor filter: {:?}", anchor));
            query::ready(&output, anchor.as_deref())?
        }
        Commands::Blocked { anchor } => {
            output.verbose_ctx("blocked", &format!("Querying blocked tasks, anchor filter: {:?}", anchor));
            query::blocked(&output, anchor.as_deref())?
        }
        Commands::Status => {
            output.verbose("Gathering project status");
            query::status(&output)?
        }

        Commands::Context { compact, anchor, days } => {
            output.verbose_ctx("context", &format!("Exporting context: compact={}, anchor={:?}, days={}", compact, anchor, days));
            context::export(&output, compact, anchor.as_deref(), days)?
        }

        Commands::AgentSetup { show, claude, cursor, windsurf } => {
            agent_setup::run(&output, show, claude, cursor, windsurf)?
        }

        Commands::Advanced(advanced_cmd) => match advanced_cmd {
            AdvancedCommands::Plugin(cmd) => plugin_cmd::run(cmd, &output)?,
            AdvancedCommands::Sync(cmd) => sync_cmd::run(cmd, &output)?,
        },
    }

    output.verbose("Command completed successfully");
    Ok(())
}
