//! # Shape CLI
//!
//! A local-first, git-backed task management tool for software teams and AI agents.
//!
//! ## Overview
//!
//! Shape organizes work around **anchors** (documents like pitches, RFCs, PRDs)
//! with dependent **tasks**. The core insight is that documents drive work —
//! the "why" (anchor) should live alongside the "what" (tasks).
//!
//! ## Architecture
//!
//! The crate is organized into four main modules:
//!
//! - [`domain`] - Core business logic: Anchors, Tasks, IDs, and the dependency graph
//! - [`storage`] - Persistence layer: Markdown files for anchors, JSONL for tasks
//! - [`plugin`] - Extensibility: Custom anchor types and external tool sync
//! - [`cli`] - Command-line interface and output formatting
//!
//! ## Data Flow
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Anchor    │────▶│    Task     │────▶│  External   │
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
//! - **Anchors**: Markdown files with YAML frontmatter in `.shape/anchors/`
//! - **Tasks**: JSONL (one JSON object per line) in `.shape/tasks.jsonl`
//! - **Config**: TOML in `.shape/config.toml`
//!
//! ## Example Usage
//!
//! ```bash
//! # Initialize a project
//! shape init
//!
//! # Create an anchor (pitch, RFC, etc.)
//! shape anchor new "User Authentication" --type shapeup
//!
//! # Add tasks to the anchor
//! shape task add a-1234567 "Implement OAuth2"
//! shape task add a-1234567 "Write integration tests"
//!
//! # Set up dependencies
//! shape task dep a-1234567.2 a-1234567.1
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
//! 2. **Human-editable**: Anchors are markdown files you can edit directly
//! 3. **AI-optimized**: Context export designed for minimal tokens
//! 4. **Extensible**: Plugin system for custom anchor types and sync

pub mod domain;
pub mod storage;
pub mod plugin;
pub mod cli;

pub use domain::{Anchor, AnchorId, AnchorStatus, Task, TaskId, TaskStatus};
