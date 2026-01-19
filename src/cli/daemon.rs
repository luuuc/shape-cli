//! Background daemon for automatic git synchronization
//!
//! The daemon watches `.shape/` for changes and automatically commits them.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Subcommand;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

use super::output::Output;
use crate::storage::{DaemonConfig, Project};

/// Maximum log file size before rotation (1MB)
const MAX_LOG_SIZE: u64 = 1024 * 1024;

/// Number of log files to keep
const LOG_ROTATION_COUNT: usize = 7;

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the background daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,

        /// Suppress startup message
        #[arg(long)]
        quiet: bool,
    },

    /// Stop the background daemon
    Stop,

    /// Show daemon status
    Status,

    /// View daemon logs
    Logs {
        /// Number of lines to show (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,

        /// Follow log output (like tail -f)
        #[arg(short = 'F', long)]
        follow: bool,
    },
}

pub fn run(cmd: DaemonCommands, output: &Output) -> Result<()> {
    match cmd {
        DaemonCommands::Start { foreground, quiet } => start_daemon(output, foreground, quiet),
        DaemonCommands::Stop => stop_daemon(output),
        DaemonCommands::Status => show_status(output),
        DaemonCommands::Logs { lines, follow } => show_logs(output, lines, follow),
    }
}

/// Returns the path to the PID file for a project
fn pid_file_path(shape_dir: &Path) -> PathBuf {
    shape_dir.join("daemon.pid")
}

/// Returns the path to the log file for a project
fn log_file_path(shape_dir: &Path) -> PathBuf {
    shape_dir.join("daemon.log")
}

/// Reads the PID from the PID file
fn read_pid(shape_dir: &Path) -> Result<Option<u32>> {
    let pid_path = pid_file_path(shape_dir);
    if !pid_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&pid_path).context("Failed to read PID file")?;
    let pid: u32 = content.trim().parse().context("Invalid PID in file")?;
    Ok(Some(pid))
}

/// Writes the PID to the PID file
fn write_pid(shape_dir: &Path, pid: u32) -> Result<()> {
    let pid_path = pid_file_path(shape_dir);
    fs::write(&pid_path, pid.to_string()).context("Failed to write PID file")?;
    Ok(())
}

/// Removes the PID file
fn remove_pid(shape_dir: &Path) -> Result<()> {
    let pid_path = pid_file_path(shape_dir);
    if pid_path.exists() {
        fs::remove_file(&pid_path).context("Failed to remove PID file")?;
    }
    Ok(())
}

/// Checks if a process with the given PID is running
fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Use kill -0 to check if process exists
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        // On Windows, use tasklist
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

/// Logs a message to the daemon log file
fn log_message(shape_dir: &Path, message: &str) -> Result<()> {
    let log_path = log_file_path(shape_dir);

    // Rotate logs if needed
    rotate_logs_if_needed(&log_path)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context("Failed to open log file")?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}] {}", timestamp, message)?;

    Ok(())
}

/// Rotates log files if the current log exceeds MAX_LOG_SIZE
fn rotate_logs_if_needed(log_path: &Path) -> Result<()> {
    if !log_path.exists() {
        return Ok(());
    }

    let metadata = fs::metadata(log_path)?;
    if metadata.len() < MAX_LOG_SIZE {
        return Ok(());
    }

    // Rotate existing numbered logs
    for i in (1..LOG_ROTATION_COUNT).rev() {
        let old_path = log_path.with_extension(format!("log.{}", i));
        let new_path = log_path.with_extension(format!("log.{}", i + 1));
        if old_path.exists() {
            if i + 1 >= LOG_ROTATION_COUNT {
                fs::remove_file(&old_path)?;
            } else {
                fs::rename(&old_path, &new_path)?;
            }
        }
    }

    // Move current log to .1
    let rotated_path = log_path.with_extension("log.1");
    fs::rename(log_path, rotated_path)?;

    Ok(())
}

/// Starts the daemon
fn start_daemon(output: &Output, foreground: bool, quiet: bool) -> Result<()> {
    // Ensure we're in a project first
    let project = Project::open_current()?;
    let shape_dir = project.shape_dir();
    let config = project.config().project.daemon.clone();

    // Check if already running
    if let Some(pid) = read_pid(&shape_dir)? {
        if is_process_running(pid) {
            if output.is_json() {
                output.data(&serde_json::json!({
                    "status": "already_running",
                    "pid": pid,
                    "project": project.root().display().to_string(),
                }));
            } else {
                output.error(&format!(
                    "Daemon already running for this project (PID: {})",
                    pid
                ));
            }
            return Ok(());
        }
        // Stale PID file, remove it
        remove_pid(&shape_dir)?;
    }

    if !config.enabled {
        if output.is_json() {
            output.data(&serde_json::json!({
                "status": "disabled",
                "message": "Daemon is disabled in config",
            }));
        } else {
            output.error("Daemon is disabled in config. Set daemon.enabled = true in .shape/config.toml");
        }
        return Ok(());
    }

    if foreground {
        // Run in foreground
        let pid = std::process::id();
        write_pid(&shape_dir, pid)?;
        log_message(&shape_dir, &format!("Daemon starting in foreground (PID: {})", pid))?;

        if !quiet {
            if output.is_json() {
                output.data(&serde_json::json!({
                    "status": "started",
                    "pid": pid,
                    "foreground": true,
                    "project": project.root().display().to_string(),
                }));
            } else {
                output.success(&format!("Daemon started in foreground (PID: {})", pid));
            }
        }

        // Run the daemon loop
        run_daemon_loop(&project, &config)?;

        // Clean up on exit
        remove_pid(&shape_dir)?;
        log_message(&shape_dir, "Daemon stopped")?;
    } else {
        // Daemonize
        let exe = std::env::current_exe()?;
        let project_root = project.root().to_path_buf();

        // Fork using a child process that detaches
        let mut cmd = Command::new(&exe);
        cmd.args(["daemon", "start", "--foreground", "--quiet"])
            .current_dir(&project_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // On Windows, use DETACHED_PROCESS flag to properly detach
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const DETACHED_PROCESS: u32 = 0x00000008;
            cmd.creation_flags(DETACHED_PROCESS);
        }

        let child = cmd.spawn().context("Failed to spawn daemon process")?;
        let pid = child.id();

        if output.is_json() {
            output.data(&serde_json::json!({
                "status": "started",
                "pid": pid,
                "project": project.root().display().to_string(),
            }));
        } else if !quiet {
            output.success(&format!("Daemon started (PID: {})", pid));
        }
    }

    Ok(())
}

/// Stops the daemon
fn stop_daemon(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    let shape_dir = project.shape_dir();

    let pid = match read_pid(&shape_dir)? {
        Some(pid) => pid,
        None => {
            if output.is_json() {
                output.data(&serde_json::json!({
                    "status": "not_running",
                    "project": project.root().display().to_string(),
                }));
            } else {
                println!("Daemon is not running for this project");
            }
            return Ok(());
        }
    };

    if !is_process_running(pid) {
        remove_pid(&shape_dir)?;
        if output.is_json() {
            output.data(&serde_json::json!({
                "status": "not_running",
                "stale_pid": pid,
                "project": project.root().display().to_string(),
            }));
        } else {
            println!("Daemon is not running (cleaned up stale PID file)");
        }
        return Ok(());
    }

    // Send termination signal
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .context("Failed to send TERM signal")?;
    }

    #[cfg(windows)]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string()])
            .status()
            .context("Failed to terminate process")?;
    }

    // Wait for process to exit
    let start = Instant::now();
    while is_process_running(pid) && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(100));
    }

    if is_process_running(pid) {
        // Force kill
        #[cfg(unix)]
        {
            Command::new("kill")
                .args(["-9", &pid.to_string()])
                .status()?;
        }

        #[cfg(windows)]
        {
            Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .status()?;
        }
    }

    remove_pid(&shape_dir)?;
    log_message(&shape_dir, "Daemon stopped by user")?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "status": "stopped",
            "pid": pid,
            "project": project.root().display().to_string(),
        }));
    } else {
        output.success(&format!("Daemon stopped (PID: {})", pid));
    }

    Ok(())
}

/// Shows daemon status
fn show_status(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    let shape_dir = project.shape_dir();
    let config = project.config().project.daemon.clone();

    let running = match read_pid(&shape_dir)? {
        Some(pid) if is_process_running(pid) => Some(pid),
        Some(_stale_pid) => {
            // Clean up stale PID file
            remove_pid(&shape_dir)?;
            if !output.is_json() {
                output.verbose("Cleaned up stale PID file");
            }
            None
        }
        None => None,
    };

    if output.is_json() {
        let mut status = serde_json::json!({
            "running": running.is_some(),
            "project": project.root().display().to_string(),
        });

        if let Some(pid) = running {
            status["pid"] = serde_json::json!(pid);
        }

        status["config"] = serde_json::json!({
            "enabled": config.enabled,
            "auto_commit": config.auto_commit,
            "auto_push": config.auto_push,
            "debounce_seconds": config.debounce_seconds,
        });

        output.data(&status);
    } else {
        println!("Project: {}", project.root().display());
        match running {
            Some(pid) => {
                println!("Daemon status: RUNNING (PID: {})", pid);
            }
            None => {
                println!("Daemon status: STOPPED");
            }
        }

        println!();
        println!("Configuration:");
        println!("  Enabled: {}", config.enabled);
        println!("  Auto-commit: {}", config.auto_commit);
        println!("  Auto-push: {}", config.auto_push);
        println!("  Debounce: {}s", config.debounce_seconds);
    }

    Ok(())
}

/// Shows daemon logs
fn show_logs(output: &Output, lines: usize, follow: bool) -> Result<()> {
    let project = Project::open_current()?;
    let shape_dir = project.shape_dir();
    let log_path = log_file_path(&shape_dir);

    if !log_path.exists() {
        if output.is_json() {
            output.data(&serde_json::json!({
                "logs": [],
                "message": "No log file found",
                "project": project.root().display().to_string(),
            }));
        } else {
            println!("No daemon logs found for this project");
        }
        return Ok(());
    }

    if follow {
        // Follow mode - continuously read new content
        let file = File::open(&log_path)?;
        let mut reader = BufReader::new(file);

        // Seek to end minus some bytes for initial context
        let metadata = fs::metadata(&log_path)?;
        let start_pos = metadata.len().saturating_sub(4096);
        reader.seek(SeekFrom::Start(start_pos))?;

        // Skip partial first line if we didn't start at beginning
        if start_pos > 0 {
            let mut partial = String::new();
            reader.read_line(&mut partial)?;
        }

        // Print existing content
        for line in reader.by_ref().lines() {
            println!("{}", line?);
        }

        // Follow new content
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new content, wait a bit
                    std::thread::sleep(Duration::from_millis(100));
                }
                Ok(_) => {
                    print!("{}", line);
                }
                Err(e) => {
                    eprintln!("Error reading log: {}", e);
                    break;
                }
            }
        }
    } else {
        // Read last N lines
        let content = fs::read_to_string(&log_path)?;
        let all_lines: Vec<&str> = content.lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        let shown_lines: Vec<&str> = all_lines[start..].to_vec();

        if output.is_json() {
            output.data(&serde_json::json!({
                "logs": shown_lines,
                "total_lines": all_lines.len(),
                "showing": shown_lines.len(),
            }));
        } else {
            for line in shown_lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

/// The main daemon loop that watches for changes and commits them
fn run_daemon_loop(project: &Project, config: &DaemonConfig) -> Result<()> {
    let shape_dir = project.shape_dir();
    let project_root = project.root().to_path_buf();

    log_message(&shape_dir, &format!("Watching directory: {}", shape_dir.display()))?;

    // Set up file watcher with debouncing
    let (tx, rx) = mpsc::channel();
    let debounce_duration = Duration::from_secs(config.debounce_seconds);

    let mut debouncer = new_debouncer(debounce_duration, tx)?;

    debouncer
        .watcher()
        .watch(&shape_dir, RecursiveMode::Recursive)?;

    log_message(
        &shape_dir,
        &format!(
            "Daemon ready (debounce: {}s, auto_commit: {}, auto_push: {})",
            config.debounce_seconds, config.auto_commit, config.auto_push
        ),
    )?;

    // Main event loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Filter out events for ignored paths
                let relevant_events: Vec<_> = events
                    .iter()
                    .filter(|e| !should_ignore_path(&e.path))
                    .collect();

                if relevant_events.is_empty() {
                    continue;
                }

                log_message(
                    &shape_dir,
                    &format!("Detected {} change(s)", relevant_events.len()),
                )?;

                if config.auto_commit {
                    match auto_commit(&project_root, config) {
                        Ok(Some(message)) => {
                            log_message(&shape_dir, &format!("Committed: {}", message))?;

                            if config.auto_push {
                                match auto_push(&project_root, config) {
                                    Ok(true) => {
                                        log_message(&shape_dir, "Pushed to remote")?;
                                    }
                                    Ok(false) => {
                                        log_message(&shape_dir, "Nothing to push")?;
                                    }
                                    Err(e) => {
                                        log_message(
                                            &shape_dir,
                                            &format!("Push failed: {}", e),
                                        )?;
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            log_message(&shape_dir, "No changes to commit")?;
                        }
                        Err(e) => {
                            log_message(&shape_dir, &format!("Commit failed: {}", e))?;
                        }
                    }
                }
            }
            Ok(Err(error)) => {
                log_message(&shape_dir, &format!("Watch error: {:?}", error))?;
            }
            Err(e) => {
                log_message(&shape_dir, &format!("Channel error: {}", e))?;
                break;
            }
        }
    }

    Ok(())
}

/// Checks if a path should be ignored by the watcher
fn should_ignore_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Ignore cache directory
    if path_str.contains(".cache") {
        return true;
    }

    // Ignore sync directory
    if path_str.contains("/sync/") || path_str.ends_with("/sync") {
        return true;
    }

    // Ignore gitignore
    if path_str.ends_with(".gitignore") {
        return true;
    }

    // Ignore index files (auto-regenerated)
    if path_str.ends_with("index.jsonl") {
        return true;
    }

    // Ignore daemon files (our own files)
    if path_str.ends_with("daemon.pid") || path_str.ends_with("daemon.log") {
        return true;
    }

    // Ignore rotated log files
    if path_str.contains("daemon.log.") {
        return true;
    }

    false
}

/// Automatically commits changes to git
fn auto_commit(project_root: &Path, config: &DaemonConfig) -> Result<Option<String>> {
    // Check if there are changes to commit
    let status_output = Command::new("git")
        .args(["status", "--porcelain", ".shape/"])
        .current_dir(project_root)
        .output()
        .context("Failed to run git status")?;

    let status = String::from_utf8_lossy(&status_output.stdout);
    if status.trim().is_empty() {
        return Ok(None);
    }

    // Parse changes to generate commit message
    let message = generate_commit_message(&status, config);

    // Stage .shape/ changes
    Command::new("git")
        .args(["add", ".shape/"])
        .current_dir(project_root)
        .status()
        .context("Failed to stage changes")?;

    // Commit
    let commit_result = Command::new("git")
        .args(["commit", "-m", &message])
        .current_dir(project_root)
        .output()
        .context("Failed to create commit")?;

    if commit_result.status.success() {
        Ok(Some(message))
    } else {
        let stderr = String::from_utf8_lossy(&commit_result.stderr);
        if stderr.contains("nothing to commit") {
            Ok(None)
        } else {
            Err(anyhow::anyhow!("Git commit failed: {}", stderr))
        }
    }
}

/// Generates a commit message based on the changes
fn generate_commit_message(status: &str, _config: &DaemonConfig) -> String {
    let lines: Vec<&str> = status.lines().collect();

    // Count different types of changes
    let mut task_changes = 0;
    let mut anchor_changes = 0;
    let mut config_changes = 0;

    for line in &lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse git status line (e.g., "M  .shape/tasks.jsonl")
        let path = line.get(3..).unwrap_or("").trim();

        if path.contains("tasks.jsonl") {
            task_changes += 1;
        } else if path.contains("anchors/") && path.ends_with(".md") {
            anchor_changes += 1;
        } else if path.contains("config.toml") {
            config_changes += 1;
        }
    }

    // Generate message based on changes
    if task_changes > 0 && anchor_changes == 0 && config_changes == 0 {
        if task_changes == 1 {
            return "shape: update task".to_string();
        } else {
            return format!("shape: update {} tasks", task_changes);
        }
    }

    if anchor_changes > 0 && task_changes == 0 && config_changes == 0 {
        if anchor_changes == 1 {
            return "shape: update anchor".to_string();
        } else {
            return format!("shape: update {} anchors", anchor_changes);
        }
    }

    if config_changes > 0 && task_changes == 0 && anchor_changes == 0 {
        return "shape: update config".to_string();
    }

    // Multiple types of changes
    let mut parts = Vec::new();
    if task_changes > 0 {
        parts.push(format!("{} task{}", task_changes, if task_changes == 1 { "" } else { "s" }));
    }
    if anchor_changes > 0 {
        parts.push(format!("{} anchor{}", anchor_changes, if anchor_changes == 1 { "" } else { "s" }));
    }
    if config_changes > 0 {
        parts.push("config".to_string());
    }

    format!("shape: update {}", parts.join(", "))
}

/// Pushes commits to the remote
fn auto_push(project_root: &Path, config: &DaemonConfig) -> Result<bool> {
    // Check if there are commits to push
    let remote_ref = format!("{}/{}..HEAD", config.push_remote, config.push_branch);
    let log_output = Command::new("git")
        .args(["log", &remote_ref, "--oneline"])
        .current_dir(project_root)
        .output();

    // If we can't determine, try to push anyway
    let has_commits = log_output
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(true);

    if !has_commits {
        return Ok(false);
    }

    let push_result = Command::new("git")
        .args(["push", &config.push_remote, &config.push_branch])
        .current_dir(project_root)
        .output()
        .context("Failed to push to remote")?;

    if push_result.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&push_result.stderr);
        if stderr.contains("Everything up-to-date") {
            Ok(false)
        } else {
            Err(anyhow::anyhow!("Git push failed: {}", stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_path() {
        assert!(should_ignore_path(Path::new(".shape/.cache/db.sqlite")));
        assert!(should_ignore_path(Path::new(".shape/sync/state.json")));
        assert!(should_ignore_path(Path::new(".shape/.gitignore")));
        assert!(should_ignore_path(Path::new(".shape/anchors/index.jsonl")));
        assert!(should_ignore_path(Path::new(".shape/daemon.pid")));
        assert!(should_ignore_path(Path::new(".shape/daemon.log")));
        assert!(should_ignore_path(Path::new(".shape/daemon.log.1")));

        assert!(!should_ignore_path(Path::new(".shape/tasks.jsonl")));
        assert!(!should_ignore_path(Path::new(".shape/anchors/a-1234567.md")));
        assert!(!should_ignore_path(Path::new(".shape/config.toml")));
    }

    #[test]
    fn test_generate_commit_message_single_task() {
        let status = " M .shape/tasks.jsonl\n";
        let config = DaemonConfig::default();
        let message = generate_commit_message(status, &config);
        assert_eq!(message, "shape: update task");
    }

    #[test]
    fn test_generate_commit_message_multiple_tasks() {
        let status = " M .shape/tasks.jsonl\n M .shape/tasks.jsonl\n";
        let config = DaemonConfig::default();
        let message = generate_commit_message(status, &config);
        assert_eq!(message, "shape: update 2 tasks");
    }

    #[test]
    fn test_generate_commit_message_anchor() {
        let status = " M .shape/anchors/a-1234567.md\n";
        let config = DaemonConfig::default();
        let message = generate_commit_message(status, &config);
        assert_eq!(message, "shape: update anchor");
    }

    #[test]
    fn test_generate_commit_message_mixed() {
        let status = " M .shape/tasks.jsonl\n M .shape/anchors/a-1234567.md\n";
        let config = DaemonConfig::default();
        let message = generate_commit_message(status, &config);
        assert_eq!(message, "shape: update 1 task, 1 anchor");
    }
}
