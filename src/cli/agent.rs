//! Agent coordination commands
//!
//! Commands for multi-agent coordination: claim, unclaim, next, note, link, block, history, summary, handoff
//!
//! ## Concurrency Model
//!
//! Claims use optimistic concurrency - the last write wins. This is acceptable for the
//! typical use case where agents are loosely coordinated and claim conflicts are rare.
//! The claim timeout mechanism (default 4 hours) ensures abandoned claims don't block
//! progress indefinitely.
//!
//! For stricter coordination, agents should:
//! 1. Check `shape next` before claiming to see unclaimed tasks
//! 2. Use brief-specific filtering to reduce contention
//! 3. Monitor `shape claimed` to see active claims

use std::collections::HashMap;

use anyhow::Result;
use chrono::Utc;
use clap::Subcommand;

use super::output::Output;
use crate::domain::{
    BriefId, DependencyGraph, HistoryEventType, LinkType, Task, TaskId, TaskStatus,
};
use crate::storage::Project;

/// Agent subcommands
#[derive(Subcommand)]
pub enum AgentCommands {
    /// Claim a task for the current agent
    Claim {
        /// Task ID to claim
        id: String,

        /// Agent name (defaults to $SHAPE_AGENT or $USER)
        #[arg(long)]
        agent: Option<String>,

        /// Force claim even if already claimed by another agent (requires --reason)
        #[arg(long)]
        force: bool,

        /// Reason for force-claiming (required when --force is used)
        #[arg(long, required_if_eq("force", "true"))]
        reason: Option<String>,
    },

    /// Release a claim on a task
    Unclaim {
        /// Task ID to unclaim
        id: String,
    },

    /// List all claimed tasks
    Claimed,

    /// Suggest the next best task to work on
    Next {
        /// Filter by brief ID
        #[arg(long)]
        brief: Option<String>,

        /// Number of suggestions (default: 1)
        #[arg(short, long, default_value = "1")]
        n: usize,
    },

    /// Add a note to a task
    Note {
        /// Task ID
        id: String,

        /// Note text
        text: String,
    },

    /// Link an artifact to a task
    Link {
        /// Task ID
        id: String,

        /// Commit hash
        #[arg(long)]
        commit: Option<String>,

        /// PR number
        #[arg(long)]
        pr: Option<String>,

        /// File path
        #[arg(long)]
        file: Option<String>,

        /// URL
        #[arg(long)]
        url: Option<String>,
    },

    /// Remove a link from a task
    Unlink {
        /// Task ID
        id: String,

        /// Commit hash
        #[arg(long)]
        commit: Option<String>,

        /// PR number
        #[arg(long)]
        pr: Option<String>,

        /// File path
        #[arg(long)]
        file: Option<String>,

        /// URL
        #[arg(long)]
        url: Option<String>,
    },

    /// Block a task with a reason
    Block {
        /// Task ID
        id: String,

        /// Reason for blocking
        reason: String,

        /// Task ID this is blocked on
        #[arg(long = "on")]
        on_task: Option<String>,
    },

    /// Unblock a task
    Unblock {
        /// Task ID
        id: String,
    },

    /// Show task history/timeline
    History {
        /// Task or brief ID
        id: String,
    },

    /// Show project or brief summary
    Summary {
        /// Brief ID (optional)
        id: Option<String>,
    },

    /// Hand off a task to another agent or human
    Handoff {
        /// Task ID
        id: String,

        /// Reason for handoff
        reason: String,

        /// Agent to hand off to (use "human" for human review)
        #[arg(long)]
        to: Option<String>,
    },

    /// Find tasks by artifact link
    Find {
        /// Find by commit hash
        #[arg(long)]
        commit: Option<String>,

        /// Find by file path
        #[arg(long)]
        file: Option<String>,
    },
}

pub fn run(cmd: AgentCommands, output: &Output) -> Result<()> {
    match cmd {
        AgentCommands::Claim {
            id,
            agent,
            force,
            reason,
        } => claim_task(output, &id, agent.as_deref(), force, reason.as_deref()),
        AgentCommands::Unclaim { id } => unclaim_task(output, &id),
        AgentCommands::Claimed => list_claimed(output),
        AgentCommands::Next { brief, n } => next_task(output, brief.as_deref(), n),
        AgentCommands::Note { id, text } => add_note(output, &id, &text),
        AgentCommands::Link {
            id,
            commit,
            pr,
            file,
            url,
        } => add_link(output, &id, commit, pr, file, url),
        AgentCommands::Unlink {
            id,
            commit,
            pr,
            file,
            url,
        } => remove_link(output, &id, commit, pr, file, url),
        AgentCommands::Block {
            id,
            reason,
            on_task,
        } => block_task(output, &id, &reason, on_task.as_deref()),
        AgentCommands::Unblock { id } => unblock_task(output, &id),
        AgentCommands::History { id } => show_history(output, &id),
        AgentCommands::Summary { id } => show_summary(output, id.as_deref()),
        AgentCommands::Handoff { id, reason, to } => handoff_task(output, &id, &reason, to),
        AgentCommands::Find { commit, file } => find_by_link(output, commit, file),
    }
}

fn get_agent_name(project: &Project, override_name: Option<&str>) -> String {
    if let Some(name) = override_name {
        return name.to_string();
    }
    project.config().project.agent.effective_name()
}

fn get_claim_timeout(project: &Project) -> u32 {
    project.config().project.agent.claim_timeout_hours
}

/// Claims a task for an agent.
///
/// Note: This uses optimistic concurrency - if two agents claim simultaneously,
/// the last write wins. This is acceptable for loosely coordinated agents where
/// claim conflicts are rare. The claim timeout mechanism ensures abandoned claims
/// don't block progress indefinitely.
fn claim_task(
    output: &Output,
    id_str: &str,
    agent_override: Option<&str>,
    force: bool,
    force_reason: Option<&str>,
) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, agent_override);
    let timeout_hours = get_claim_timeout(&project);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    // Check if already claimed
    if let Some(ref claimed_by) = task.claimed_by {
        if claimed_by == &agent {
            // Re-claiming refreshes the timeout
            task.claimed_at = Some(Utc::now());
            store.update(task)?;

            if output.is_json() {
                output.data(&serde_json::json!({
                    "id": task.id.to_string(),
                    "claimed_by": agent,
                    "refreshed": true,
                }));
            } else {
                output.success(&format!("Refreshed claim on {}", task.id));
            }
            return Ok(());
        }

        // Check if expired
        let expired = task.is_claim_expired(timeout_hours);

        if !force && !expired {
            let remaining = task.claim_remaining_hours(timeout_hours).unwrap_or(0.0);
            anyhow::bail!(
                "Task {} is claimed by \"{}\" (expires in {:.1}h)\nUse --force --reason \"...\" to override",
                id,
                claimed_by,
                remaining
            );
        }

        // If force claiming, add a note explaining why
        if force {
            let reason = force_reason.unwrap_or("No reason provided");
            task.add_note(
                &agent,
                format!("Force claimed from {}: {}", claimed_by, reason),
            );
        }
    }

    task.claim(&agent);
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "claimed_by": agent,
            "status": task.status,
        }));
    } else {
        output.success(&format!("Claimed task: {} (now in progress)", task.id));
    }

    Ok(())
}

fn unclaim_task(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    if task.claimed_by.is_none() {
        anyhow::bail!("Task {} is not claimed", id);
    }

    task.unclaim(Some(&agent));
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "unclaimed": true,
        }));
    } else {
        output.success(&format!("Released claim on {}", task.id));
    }

    Ok(())
}

fn list_claimed(output: &Output) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let tasks = store.read_all()?;
    let timeout_hours = get_claim_timeout(&project);

    let claimed: Vec<_> = tasks.values().filter(|t| t.is_claimed()).collect();

    if output.is_json() {
        let items: Vec<_> = claimed
            .iter()
            .map(|t| {
                let remaining = t.claim_remaining_hours(timeout_hours);
                let expired = t.is_claim_expired(timeout_hours);
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "claimed_by": t.claimed_by,
                    "claimed_at": t.claimed_at,
                    "remaining_hours": remaining,
                    "expired": expired,
                })
            })
            .collect();
        output.data(&items);
    } else if claimed.is_empty() {
        println!("No claimed tasks.");
    } else {
        println!("Claimed tasks ({}):", claimed.len());
        println!("{:<20} {:<15} {:<10} TITLE", "ID", "AGENT", "REMAINING");
        println!("{}", "-".repeat(70));
        for task in claimed {
            let remaining = task.claim_remaining_hours(timeout_hours);
            let remaining_str = match remaining {
                Some(h) if h <= 0.0 => "EXPIRED".to_string(),
                Some(h) => format!("{:.1}h", h),
                None => "-".to_string(),
            };
            println!(
                "{:<20} {:<15} {:<10} {}",
                task.id,
                task.claimed_by.as_deref().unwrap_or("-"),
                remaining_str,
                task.title
            );
        }
    }

    Ok(())
}

/// Task scoring for `shape next`
#[derive(Debug)]
struct TaskScore {
    task_id: TaskId,
    title: String,
    brief_id: Option<BriefId>,
    priority_score: f64,
    unblocks_count: usize,
    age_days: i64,
    estimate: Option<i64>,
    total_score: f64,
}

fn next_task(output: &Output, brief_filter: Option<&str>, n: usize) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let tasks = store.read_all()?;
    let timeout_hours = get_claim_timeout(&project);
    let agent = get_agent_name(&project, None);

    // Build status map
    let statuses: HashMap<TaskId, TaskStatus> =
        tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();

    // Filter to brief if specified
    let brief_id: Option<BriefId> = brief_filter.map(|s| s.parse()).transpose()?;

    // Find tasks that other tasks depend on (to calculate unblocks count)
    let mut unblocks_map: HashMap<TaskId, usize> = HashMap::new();
    for task in tasks.values() {
        for dep in task.depends_on.blocking_task_ids() {
            *unblocks_map.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    // Score each ready task
    let mut scored: Vec<TaskScore> = tasks
        .values()
        .filter(|t| {
            // Filter by brief if specified
            if let Some(ref filter_id) = brief_id {
                if t.brief_id().as_ref() != Some(filter_id) {
                    return false;
                }
            }
            // Must be ready for agent (not blocked, not claimed by others)
            t.is_ready_for_agent(&statuses, Some(&agent))
        })
        .filter(|t| {
            // Exclude tasks with expired claims from others
            if let Some(ref claimed_by) = t.claimed_by {
                if claimed_by != &agent && !t.is_claim_expired(timeout_hours) {
                    return false;
                }
            }
            true
        })
        .map(|t| {
            // Priority score (high=3, medium=2, low=1, none=1)
            let priority_score = t
                .get_meta("priority")
                .and_then(|v| v.as_str())
                .map(|p| match p {
                    "high" => 3.0,
                    "medium" => 2.0,
                    "low" => 1.0,
                    _ => 1.0,
                })
                .unwrap_or(1.0);

            // How many tasks does this unblock?
            let unblocks_count = unblocks_map.get(&t.id).copied().unwrap_or(0);

            // Age in days
            let age_days = (Utc::now() - t.created_at).num_days();

            // Estimate (smaller is better for quick wins)
            let estimate = t.get_meta("estimate").and_then(|v| v.as_i64());

            // Calculate total score
            // Formula: priority * 10 + unblocks * 5 + age_factor + quick_win_bonus
            let age_factor = (age_days as f64).min(30.0) / 30.0 * 5.0; // Max 5 points for age
            let quick_win_bonus = estimate
                .map(|e| if e <= 2 { 3.0 } else { 0.0 })
                .unwrap_or(0.0);

            let total_score =
                priority_score * 10.0 + unblocks_count as f64 * 5.0 + age_factor + quick_win_bonus;

            TaskScore {
                task_id: t.id.clone(),
                title: t.title.clone(),
                brief_id: t.brief_id(),
                priority_score,
                unblocks_count,
                age_days,
                estimate,
                total_score,
            }
        })
        .collect();

    // Sort by score (highest first)
    scored.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap());

    // Take top N
    let recommendations: Vec<_> = scored.into_iter().take(n).collect();

    if output.is_json() {
        let items: Vec<_> = recommendations
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.task_id.to_string(),
                    "title": s.title,
                    "brief_id": s.brief_id.as_ref().map(|b| b.to_string()),
                    "priority": match s.priority_score as i32 {
                        3 => "high",
                        2 => "medium",
                        _ => "low",
                    },
                    "unblocks": s.unblocks_count,
                    "age_days": s.age_days,
                    "estimate": s.estimate,
                    "score": s.total_score,
                })
            })
            .collect();

        if n == 1 && !items.is_empty() {
            output.data(&serde_json::json!({
                "recommended": items[0],
                "alternatives": &items[1..],
            }));
        } else {
            output.data(&items);
        }
    } else if recommendations.is_empty() {
        println!("No tasks ready to work on.");
    } else {
        let first = &recommendations[0];

        println!();
        println!("Recommended: {} \"{}\"", first.task_id, first.title);
        if let Some(ref brief) = first.brief_id {
            println!("  Brief: {}", brief);
        }
        println!(
            "  Priority: {}",
            match first.priority_score as i32 {
                3 => "high",
                2 => "medium",
                _ => "low",
            }
        );
        if first.unblocks_count > 0 {
            println!("  Unblocks: {} tasks", first.unblocks_count);
        }
        println!("  Age: {} days", first.age_days);
        println!("  Score: {:.2}", first.total_score);
        println!();
        println!("Run: shape claim {}", first.task_id);

        if recommendations.len() > 1 {
            println!();
            println!("Alternatives:");
            for (i, alt) in recommendations.iter().skip(1).enumerate() {
                println!(
                    "  {}. {} \"{}\" (score: {:.2})",
                    i + 2,
                    alt.task_id,
                    alt.title,
                    alt.total_score
                );
            }
        }
    }

    Ok(())
}

fn add_note(output: &Output, id_str: &str, text: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    task.add_note(&agent, text);
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "note_added": true,
            "note_count": task.notes.len(),
        }));
    } else {
        output.success(&format!("Added note to {}", task.id));
    }

    Ok(())
}

fn add_link(
    output: &Output,
    id_str: &str,
    commit: Option<String>,
    pr: Option<String>,
    file: Option<String>,
    url: Option<String>,
) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    let mut links_added = Vec::new();

    if let Some(ref hash) = commit {
        task.add_link(LinkType::Commit, hash, Some(&agent));
        links_added.push(format!("commit:{}", hash));
    }
    if let Some(ref num) = pr {
        task.add_link(LinkType::Pr, num, Some(&agent));
        links_added.push(format!("pr:#{}", num));
    }
    if let Some(ref path) = file {
        task.add_link(LinkType::File, path, Some(&agent));
        links_added.push(format!("file:{}", path));
    }
    if let Some(ref u) = url {
        task.add_link(LinkType::Url, u, Some(&agent));
        links_added.push(format!("url:{}", u));
    }

    if links_added.is_empty() {
        anyhow::bail!("No link specified. Use --commit, --pr, --file, or --url");
    }

    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "links_added": links_added,
        }));
    } else {
        output.success(&format!("Added {} to {}", links_added.join(", "), task.id));
    }

    Ok(())
}

fn remove_link(
    output: &Output,
    id_str: &str,
    commit: Option<String>,
    pr: Option<String>,
    file: Option<String>,
    url: Option<String>,
) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    let mut links_removed = Vec::new();

    if let Some(ref hash) = commit {
        if task.remove_link(LinkType::Commit, hash, Some(&agent)) {
            links_removed.push(format!("commit:{}", hash));
        }
    }
    if let Some(ref num) = pr {
        if task.remove_link(LinkType::Pr, num, Some(&agent)) {
            links_removed.push(format!("pr:#{}", num));
        }
    }
    if let Some(ref path) = file {
        if task.remove_link(LinkType::File, path, Some(&agent)) {
            links_removed.push(format!("file:{}", path));
        }
    }
    if let Some(ref u) = url {
        if task.remove_link(LinkType::Url, u, Some(&agent)) {
            links_removed.push(format!("url:{}", u));
        }
    }

    if links_removed.is_empty() {
        anyhow::bail!("No matching link found to remove");
    }

    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "links_removed": links_removed,
        }));
    } else {
        output.success(&format!(
            "Removed {} from {}",
            links_removed.join(", "),
            task.id
        ));
    }

    Ok(())
}

fn block_task(
    output: &Output,
    id_str: &str,
    reason: &str,
    on_task_str: Option<&str>,
) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let on_task: Option<TaskId> = on_task_str.map(|s| s.parse()).transpose()?;

    let mut tasks = store.read_all()?;

    // Verify on_task exists if specified
    if let Some(ref on_id) = on_task {
        if !tasks.contains_key(on_id) {
            anyhow::bail!("Task not found: {}", on_id);
        }
    }

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    task.block(reason, &agent, on_task.clone());
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "blocked": true,
            "reason": reason,
            "on_task": on_task.map(|t| t.to_string()),
        }));
    } else {
        let on_str = on_task.map(|t| format!(" (on {})", t)).unwrap_or_default();
        output.success(&format!("Blocked {}{}: {}", task.id, on_str, reason));
    }

    Ok(())
}

fn unblock_task(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    if task.blocked.is_none() {
        anyhow::bail!("Task {} is not blocked", id);
    }

    task.unblock(Some(&agent));
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "unblocked": true,
        }));
    } else {
        output.success(&format!("Unblocked {}", task.id));
    }

    Ok(())
}

fn show_history(output: &Output, id_str: &str) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();

    let id: TaskId = id_str.parse()?;
    let tasks = store.read_all()?;

    let task = tasks
        .get(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "title": task.title,
            "history": task.history,
        }));
    } else {
        println!();
        println!("Task: {} \"{}\"", task.id, task.title);
        println!();
        println!("Timeline:");

        if task.history.is_empty() {
            println!("  (no history recorded)");
        } else {
            for event in &task.history {
                let time = event.at.format("%Y-%m-%d %H:%M");
                let by = event
                    .by
                    .as_ref()
                    .map(|s| format!(" by {}", s))
                    .unwrap_or_default();

                let event_str = match &event.event {
                    HistoryEventType::Created => "created".to_string(),
                    HistoryEventType::Started => "started".to_string(),
                    HistoryEventType::Completed => "completed".to_string(),
                    HistoryEventType::Reopened => "reopened".to_string(),
                    HistoryEventType::Claimed => "claimed".to_string(),
                    HistoryEventType::Unclaimed => "unclaimed".to_string(),
                    HistoryEventType::Note => {
                        let text = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("text"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        format!("note: \"{}\"", text)
                    }
                    HistoryEventType::Linked => {
                        let link_type = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let reference = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("ref"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("linked {}:{}", link_type, reference)
                    }
                    HistoryEventType::Unlinked => {
                        let link_type = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("type"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let reference = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("ref"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("unlinked {}:{}", link_type, reference)
                    }
                    HistoryEventType::Blocked => {
                        let reason = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("reason"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("blocked: \"{}\"", reason)
                    }
                    HistoryEventType::Unblocked => "unblocked".to_string(),
                    HistoryEventType::Assigned => {
                        let to = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("to"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("assigned to {}", to)
                    }
                    HistoryEventType::Handoff => {
                        let reason = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("reason"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let to = event
                            .data
                            .as_ref()
                            .and_then(|d| d.get("to"))
                            .and_then(|v| v.as_str());
                        if let Some(to_agent) = to {
                            format!("handoff to {}: \"{}\"", to_agent, reason)
                        } else {
                            format!("handoff: \"{}\"", reason)
                        }
                    }
                };

                println!("  {}  {}{}", time, event_str, by);
            }
        }

        // Also show notes if any
        if !task.notes.is_empty() {
            println!();
            println!("Notes:");
            for note in &task.notes {
                let time = note.at.format("%H:%M");
                println!("  [{}] {}: {}", time, note.by, note.text);
            }
        }

        // Show links if any
        if !task.links.is_empty() {
            println!();
            println!("Links:");
            for link in &task.links {
                println!("  {}: {}", link.link_type.as_str(), link.reference);
            }
        }
    }

    Ok(())
}

fn show_summary(output: &Output, id: Option<&str>) -> Result<()> {
    let project = Project::open_current()?;
    let brief_store = project.brief_store();
    let task_store = project.task_store();

    let briefs = brief_store.read_all()?;
    let tasks = task_store.read_all()?;
    let timeout_hours = get_claim_timeout(&project);

    // Build status map
    let statuses: HashMap<TaskId, TaskStatus> =
        tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();

    // Build dependency graph for ready/blocked
    let graph = DependencyGraph::from_tasks(tasks.values())?;
    let ready_ids = graph.ready_tasks(&statuses);
    let blocked_ids = graph.blocked_tasks(&statuses);

    if let Some(brief_str) = id {
        // Brief-specific summary
        let brief_id: BriefId = brief_str.parse()?;
        let brief = briefs
            .get(&brief_id)
            .ok_or_else(|| anyhow::anyhow!("Brief not found: {}", brief_id))?;

        let brief_tasks: Vec<_> = tasks
            .values()
            .filter(|t| t.brief_id().as_ref() == Some(&brief_id))
            .collect();

        let total = brief_tasks.len();
        let done = brief_tasks
            .iter()
            .filter(|t| t.status.is_complete())
            .count();
        let in_progress: Vec<_> = brief_tasks
            .iter()
            .filter(|t| t.status.is_active())
            .collect();
        let ready: Vec<_> = brief_tasks
            .iter()
            .filter(|t| ready_ids.contains(&t.id))
            .collect();
        let explicitly_blocked: Vec<_> = brief_tasks
            .iter()
            .filter(|t| t.is_explicitly_blocked())
            .collect();
        let dep_blocked: Vec<_> = brief_tasks
            .iter()
            .filter(|t| blocked_ids.contains(&t.id) && !t.is_explicitly_blocked())
            .collect();

        if output.is_json() {
            output.data(&serde_json::json!({
                "brief": {
                    "id": brief_id.to_string(),
                    "title": brief.title,
                    "status": brief.status,
                },
                "progress": {
                    "total": total,
                    "done": done,
                    "percent": if total > 0 { done * 100 / total } else { 0 },
                },
                "in_progress": in_progress.iter().map(|t| serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "claimed_by": t.claimed_by,
                })).collect::<Vec<_>>(),
                "ready": ready.iter().map(|t| serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                })).collect::<Vec<_>>(),
                "blocked": {
                    "by_dependencies": dep_blocked.iter().map(|t| t.id.to_string()).collect::<Vec<_>>(),
                    "explicitly": explicitly_blocked.iter().map(|t| serde_json::json!({
                        "id": t.id.to_string(),
                        "reason": t.blocked.as_ref().map(|b| &b.reason),
                    })).collect::<Vec<_>>(),
                },
            }));
        } else {
            println!();
            println!("{} ({})", brief.title, brief_id);
            println!(
                "  Progress: {}/{} tasks ({}%)",
                done,
                total,
                if total > 0 { done * 100 / total } else { 0 }
            );
            println!(
                "  Status: {} ready, {} in progress, {} blocked",
                ready.len(),
                in_progress.len(),
                explicitly_blocked.len() + dep_blocked.len()
            );

            if !in_progress.is_empty() {
                println!();
                println!("  In Progress:");
                for t in &in_progress {
                    let claim_str = t
                        .claimed_by
                        .as_ref()
                        .map(|c| {
                            let remaining = t.claim_remaining_hours(timeout_hours).unwrap_or(0.0);
                            format!(" ({}, {:.1}h)", c, remaining)
                        })
                        .unwrap_or_default();
                    println!("    {} \"{}\"{}", t.id, t.title, claim_str);
                }
            }

            if !explicitly_blocked.is_empty() {
                println!();
                println!("  Blocked:");
                for t in &explicitly_blocked {
                    let reason = t.blocked.as_ref().map(|b| b.reason.as_str()).unwrap_or("?");
                    println!("    {} \"{}\"", t.id, reason);
                }
            }

            if !ready.is_empty() {
                println!();
                println!("  Ready:");
                for t in &ready {
                    println!("    {} \"{}\"", t.id, t.title);
                }
            }
        }
    } else {
        // Project-wide summary
        let active_briefs: Vec<_> = briefs.values().filter(|b| b.is_active()).collect();
        let complete_briefs = briefs.values().filter(|b| b.is_complete()).count();

        let total_tasks = tasks.len();
        let done_tasks = tasks.values().filter(|t| t.status.is_complete()).count();
        let in_progress_tasks: Vec<_> = tasks.values().filter(|t| t.status.is_active()).collect();
        let explicitly_blocked: Vec<_> = tasks
            .values()
            .filter(|t| t.is_explicitly_blocked())
            .collect();

        // Find "hot" brief (most activity)
        let hot_brief = active_briefs.iter().max_by_key(|b| {
            let brief_tasks: Vec<_> = tasks
                .values()
                .filter(|t| t.brief_id().as_ref() == Some(&b.id))
                .collect();
            let ready = brief_tasks
                .iter()
                .filter(|t| ready_ids.contains(&t.id))
                .count();
            ready
        });

        // Get next recommendation
        let agent = get_agent_name(&project, None);
        let next_task = tasks
            .values()
            .filter(|t| t.is_ready_for_agent(&statuses, Some(&agent)))
            .max_by_key(|t| {
                let priority = t
                    .get_meta("priority")
                    .and_then(|v| v.as_str())
                    .map(|p| match p {
                        "high" => 3,
                        "medium" => 2,
                        _ => 1,
                    })
                    .unwrap_or(1);
                priority
            });

        if output.is_json() {
            output.data(&serde_json::json!({
                "briefs": {
                    "total": briefs.len(),
                    "active": active_briefs.len(),
                    "complete": complete_briefs,
                },
                "tasks": {
                    "total": total_tasks,
                    "done": done_tasks,
                    "in_progress": in_progress_tasks.len(),
                    "ready": ready_ids.len(),
                    "blocked": blocked_ids.len(),
                    "explicitly_blocked": explicitly_blocked.len(),
                },
                "hot_brief": hot_brief.map(|b| serde_json::json!({
                    "id": b.id.to_string(),
                    "title": b.title,
                })),
                "next": next_task.map(|t| serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                })),
            }));
        } else {
            println!();
            println!("Shape Project Status");
            println!(
                "  Briefs: {} active, {} complete",
                active_briefs.len(),
                complete_briefs
            );
            println!(
                "  Tasks: {} total ({} done, {} in progress, {} blocked, {} ready)",
                total_tasks,
                done_tasks,
                in_progress_tasks.len(),
                explicitly_blocked.len() + blocked_ids.len(),
                ready_ids.len()
            );

            if let Some(brief) = hot_brief {
                let brief_ready = tasks
                    .values()
                    .filter(|t| {
                        t.brief_id().as_ref() == Some(&brief.id) && ready_ids.contains(&t.id)
                    })
                    .count();
                let brief_blocked = tasks
                    .values()
                    .filter(|t| {
                        t.brief_id().as_ref() == Some(&brief.id) && t.is_explicitly_blocked()
                    })
                    .count();
                println!();
                println!(
                    "  Hot: {} \"{}\" - {} tasks ready, {} blocked",
                    brief.id, brief.title, brief_ready, brief_blocked
                );
            }

            if let Some(task) = next_task {
                let priority = task
                    .get_meta("priority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("normal");
                println!();
                println!(
                    "  Next: {} \"{}\" ({} priority)",
                    task.id, task.title, priority
                );
            }
        }
    }

    Ok(())
}

fn handoff_task(output: &Output, id_str: &str, reason: &str, to: Option<String>) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let agent = get_agent_name(&project, None);

    let id: TaskId = id_str.parse()?;
    let mut tasks = store.read_all()?;

    let task = tasks
        .get_mut(&id)
        .ok_or_else(|| anyhow::anyhow!("Task not found: {}", id))?;

    task.handoff(reason, &agent, to.clone());
    store.update(task)?;

    if output.is_json() {
        output.data(&serde_json::json!({
            "id": task.id.to_string(),
            "handed_off": true,
            "to": to,
            "reason": reason,
        }));
    } else {
        let to_str = to.map(|t| format!(" to {}", t)).unwrap_or_default();
        output.success(&format!("Handed off {}{}: {}", task.id, to_str, reason));
    }

    Ok(())
}

fn find_by_link(output: &Output, commit: Option<String>, file: Option<String>) -> Result<()> {
    let project = Project::open_current()?;
    let store = project.task_store();
    let tasks = store.read_all()?;

    let mut matches: Vec<&Task> = Vec::new();

    for task in tasks.values() {
        let found = task.links.iter().any(|link| {
            if let Some(ref hash) = commit {
                if link.link_type == LinkType::Commit && link.reference.contains(hash) {
                    return true;
                }
            }
            if let Some(ref path) = file {
                if link.link_type == LinkType::File && link.reference.contains(path) {
                    return true;
                }
            }
            false
        });
        if found {
            matches.push(task);
        }
    }

    if output.is_json() {
        let items: Vec<_> = matches
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "links": t.links.iter().map(|l| serde_json::json!({
                        "type": l.link_type.as_str(),
                        "ref": l.reference,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        output.data(&items);
    } else if matches.is_empty() {
        println!("No tasks found matching the link criteria.");
    } else {
        println!("Tasks matching link criteria:");
        println!("{:<20} TITLE", "ID");
        println!("{}", "-".repeat(50));
        for task in matches {
            println!("{:<20} {}", task.id, task.title);
        }
    }

    Ok(())
}
