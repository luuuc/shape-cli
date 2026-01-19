//! # Shape CLI
//!
//! A local-first, git-backed task management tool for software teams and AI agents.
//!
//! ## Overview
//!
//! Shape organizes work around **briefs** (documents like pitches, RFCs, PRDs)
//! with dependent **tasks**. The core insight is that documents drive work —
//! the "why" (brief) should live alongside the "what" (tasks).
//!
//! ## Architecture
//!
//! The crate is organized into four main modules:
//!
//! - [`domain`] - Core business logic: Briefs, Tasks, IDs, and the dependency graph
//! - [`storage`] - Persistence layer: Markdown files for briefs, JSONL for tasks
//! - [`plugin`] - Extensibility: Custom brief types and external tool sync
//! - [`cli`] - Command-line interface and output formatting
//!
//! ## Data Flow
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │    Brief    │────▶│    Task     │────▶│  External   │
//! │  (Markdown) │     │   (JSONL)   │     │   (Plugin)  │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!       │                   │
//!       ▼                   ▼
//! ┌─────────────────────────────────────────────────────┐
//! │               Dependency Graph                       │
//! │         (ready tasks, blocked tasks)                │
//! └─────────────────────────────────────────────────────┘
//!                         │
//!                         ▼
//! ┌─────────────────────────────────────────────────────┐
//! │              AI Context Export                       │
//! │           (shape context --compact)                  │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Storage Format
//!
//! - **Briefs**: Markdown files with YAML frontmatter in `.shape/briefs/`
//! - **Tasks**: JSONL (one JSON object per line) in `.shape/tasks.jsonl`
//! - **Config**: TOML in `.shape/config.toml`
//!
//! ## Example Usage
//!
//! ```bash
//! # Initialize a project
//! shape init
//!
//! # Create a brief (pitch, RFC, etc.)
//! shape brief new "User Authentication" --type shapeup
//!
//! # Add tasks to the brief
//! shape task add b-1234567 "Implement OAuth2"
//! shape task add b-1234567 "Write integration tests"
//!
//! # Set up dependencies
//! shape task dep b-1234567.2 b-1234567.1
//!
//! # See what's ready to work on
//! shape ready
//!
//! # Export context for AI agents
//! shape context --compact
//! ```
//!
//! ## Design Principles
//!
//! 1. **Local-first**: All data stored in git-friendly formats
//! 2. **Human-editable**: Briefs are markdown files you can edit directly
//! 3. **AI-optimized**: Context export designed for minimal tokens
//! 4. **Extensible**: Plugin system for custom brief types and sync

pub mod cli;
pub mod domain;
pub mod plugin;
pub mod storage;

pub use domain::{Brief, BriefId, BriefStatus, Task, TaskId, TaskStatus};
