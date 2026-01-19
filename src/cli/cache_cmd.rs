//! Cache CLI commands

use anyhow::Result;
use clap::Subcommand;

use super::output::Output;
use crate::storage::Project;

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Rebuild the cache from source files
    Rebuild,

    /// Show cache status
    Status,
}

pub fn run(cmd: CacheCommands, output: &Output) -> Result<()> {
    match cmd {
        CacheCommands::Rebuild => rebuild(output),
        CacheCommands::Status => status(output),
    }
}

fn rebuild(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    output.verbose("Rebuilding cache from source files");

    let start = std::time::Instant::now();
    project.rebuild_cache()?;
    let duration = start.elapsed();

    let cache = project.cache()?;
    let (todo, in_progress, done) = cache.task_counts()?;
    let anchor_counts = cache.anchor_counts()?;
    let total_anchors: usize = anchor_counts.values().sum();

    if output.is_json() {
        output.data(&serde_json::json!({
            "rebuilt": true,
            "duration_ms": duration.as_millis(),
            "tasks": todo + in_progress + done,
            "anchors": total_anchors,
        }));
    } else {
        output.success(&format!(
            "Cache rebuilt in {:?} ({} tasks, {} anchors)",
            duration,
            todo + in_progress + done,
            total_anchors
        ));
    }

    Ok(())
}

fn status(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    let cache = project.cache()?;

    let is_stale = cache.is_stale()?;
    let cache_path = cache.path().to_path_buf();

    let (todo, in_progress, done) = cache.task_counts()?;
    let anchor_counts = cache.anchor_counts()?;
    let total_anchors: usize = anchor_counts.values().sum();

    if output.is_json() {
        output.data(&serde_json::json!({
            "path": cache_path.display().to_string(),
            "stale": is_stale,
            "tasks": {
                "total": todo + in_progress + done,
                "todo": todo,
                "in_progress": in_progress,
                "done": done,
            },
            "anchors": total_anchors,
        }));
    } else {
        println!("Cache Status");
        println!("{}", "=".repeat(40));
        println!("Path: {}", cache_path.display());
        println!(
            "Status: {}",
            if is_stale {
                "STALE (needs rebuild)"
            } else {
                "fresh"
            }
        );
        println!();
        println!("Cached Data:");
        println!(
            "  Tasks: {} ({} todo, {} in progress, {} done)",
            todo + in_progress + done,
            todo,
            in_progress,
            done
        );
        println!("  Anchors: {}", total_anchors);

        if is_stale {
            println!();
            println!("Run 'shape cache rebuild' to update the cache.");
        }
    }

    Ok(())
}
