# Commands Reference

Complete reference for all Shape CLI commands.

## Global Flags

These flags work with any command:

| Flag | Description |
|------|-------------|
| `-f, --format <FORMAT>` | Output format: `text` (default) or `json` |
| `-v, --verbose` | Enable debug output |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

## Project Commands

### `shape init [PATH]`

Initialize a new Shape project.

```bash
shape init              # Current directory
shape init ./myproject  # Specific path
```

Creates `.shape/` directory with default configuration.

### `shape status`

Show project overview: brief counts, task counts, what's ready.

```bash
shape status
```

### `shape search <QUERY>`

Full-text search across briefs and tasks.

```bash
shape search "authentication"
shape search "OAuth" --format json
```

## Brief Commands

### `shape brief new <TITLE> [--type TYPE]`

Create a new brief.

```bash
shape brief new "User Authentication"
shape brief new "API Redesign" --type shapeup
shape brief new "Database Migration" --type minimal
```

**Brief types:**
- `minimal` — Basic title and status (default)
- `shapeup` — Full ShapeUp pitch template

### `shape brief list [--status STATUS]`

List all briefs.

```bash
shape brief list
shape brief list --status in_progress
shape brief list --format json
```

**Statuses:** `proposed`, `betting`, `in_progress`, `shipped`, `archived`

### `shape brief show <BRIEF_ID>`

Show brief details.

```bash
shape brief show b-7f2a3b1
```

### `shape brief status <BRIEF_ID> <STATUS>`

Update brief status.

```bash
shape brief status b-7f2a3b1 in_progress
shape brief status b-7f2a3b1 shipped
```

## Task Commands

### `shape task add [BRIEF_ID] <TITLE>`

Add a task. If no brief ID, creates a standalone task.

```bash
shape task add b-7f2a3b1 "Implement OAuth"
shape task add "Fix typo in README"  # Standalone
```

### `shape task list [BRIEF_ID] [--standalone]`

List tasks.

```bash
shape task list                    # All tasks
shape task list b-7f2a3b1          # Tasks for brief
shape task list --standalone       # Standalone tasks only
shape task list --format json
```

### `shape task show <TASK_ID>`

Show task details including dependencies, notes, and history.

```bash
shape task show b-7f2a3b1.1
```

### `shape task start <TASK_ID>`

Mark task as in progress.

```bash
shape task start b-7f2a3b1.1
```

### `shape task done <TASK_ID>`

Mark task as complete.

```bash
shape task done b-7f2a3b1.1
```

### `shape task dep <TASK_ID> <DEPENDS_ON> [--TYPE]`

Add a dependency between tasks.

```bash
shape task dep b-7f2a3b1.2 b-7f2a3b1.1              # Default: blocks
shape task dep b-7f2a3b1.2 b-7f2a3b1.1 --blocks     # Explicit blocks
shape task dep b-7f2a3b1.2 b-7f2a3b1.1 --from       # Provenance
shape task dep b-7f2a3b1.2 b-7f2a3b1.1 --related    # Related
shape task dep b-7f2a3b1.2 b-7f2a3b1.1 --duplicates # Duplicate
```

**Dependency types:**
- `--blocks` — Task cannot start until dependency is done (affects ready queue)
- `--from` — Provenance tracking (for debugging/forensics)
- `--related` — Informational link
- `--duplicates` — Marks as duplicate

### `shape task undep <TASK_ID> <DEPENDS_ON> [--TYPE]`

Remove a dependency.

```bash
shape task undep b-7f2a3b1.2 b-7f2a3b1.1
shape task undep b-7f2a3b1.2 b-7f2a3b1.1 --related
```

## Query Commands

### `shape ready [--brief BRIEF_ID]`

Show tasks that are unblocked and ready to work on.

```bash
shape ready
shape ready --brief b-7f2a3b1
shape ready --format json
```

### `shape blocked [--brief BRIEF_ID]`

Show tasks that are blocked by dependencies.

```bash
shape blocked
shape blocked --brief b-7f2a3b1
```

## Agent Coordination Commands

### `shape next [--brief BRIEF_ID] [-n NUM]`

Suggest the best next task to work on.

```bash
shape next                    # Best task overall
shape next --brief b-7f2a3b1  # Best task for brief
shape next -n 3               # Top 3 suggestions
```

### `shape claim <TASK_ID> [--agent NAME] [--force --reason TEXT]`

Claim a task for an agent.

```bash
shape claim b-7f2a3b1.1
shape claim b-7f2a3b1.1 --agent claude
shape claim b-7f2a3b1.1 --force --reason "Taking over from stuck agent"
```

### `shape unclaim <TASK_ID>`

Release a claim on a task.

```bash
shape unclaim b-7f2a3b1.1
```

### `shape note <TASK_ID> <TEXT>`

Add a note to a task.

```bash
shape note b-7f2a3b1.1 "Found edge case in OAuth flow"
```

### `shape link <TASK_ID> [OPTIONS]`

Link artifacts to a task.

```bash
shape link b-7f2a3b1.1 --commit abc1234
shape link b-7f2a3b1.1 --pr 42
shape link b-7f2a3b1.1 --file src/auth.rs
shape link b-7f2a3b1.1 --url "https://docs.example.com"
```

### `shape unlink <TASK_ID> [OPTIONS]`

Remove links from a task.

```bash
shape unlink b-7f2a3b1.1 --commit abc1234
shape unlink b-7f2a3b1.1 --pr 42
```

### `shape block <TASK_ID> <REASON> [--on TASK_ID]`

Explicitly block a task with a reason.

```bash
shape block b-7f2a3b1.1 "Waiting for API key"
shape block b-7f2a3b1.1 "Blocked by external team" --on b-7f2a3b1.2
```

### `shape unblock <TASK_ID>`

Remove explicit block from a task.

```bash
shape unblock b-7f2a3b1.1
```

### `shape history <TASK_ID>`

Show task timeline: status changes, notes, links.

```bash
shape history b-7f2a3b1.1
```

### `shape summary [ID]`

Show summary of a brief or task.

```bash
shape summary b-7f2a3b1      # Brief summary
shape summary b-7f2a3b1.1    # Task summary
```

### `shape handoff <TASK_ID> <REASON> [--to AGENT]`

Hand off task to another agent or human.

```bash
shape handoff b-7f2a3b1.1 "Need human review"
shape handoff b-7f2a3b1.1 "Passing to specialist" --to cursor
```

## Context Commands

### `shape context [OPTIONS]`

Export project state for AI agents.

```bash
shape context                      # Full context
shape context --compact            # Minimal tokens
shape context --brief b-7f2a3b1    # Single brief
shape context --days 14            # Include older tasks
```

### `shape compact [OPTIONS]`

Compress old completed tasks to save context window.

```bash
shape compact                      # Default: 7 days
shape compact --days 30            # Keep 30 days uncompressed
shape compact --brief b-7f2a3b1    # Single brief
shape compact --dry-run            # Preview changes
shape compact --undo               # Restore from backup
```

## Infrastructure Commands

### `shape tui [--brief ID] [--view VIEW]`

Launch interactive terminal UI.

```bash
shape tui
shape tui --brief b-7f2a3b1
shape tui --view kanban
shape tui --view graph
shape tui --view overview
```

### `shape daemon start|stop|status|logs`

Manage background sync daemon.

```bash
shape daemon start    # Start daemon
shape daemon stop     # Stop daemon
shape daemon status   # Check if running
shape daemon logs     # View daemon logs
```

### `shape cache build|clear|analyze`

Manage SQLite cache.

```bash
shape cache build     # Rebuild cache
shape cache clear     # Clear cache
shape cache analyze   # Show cache stats
```

### `shape merge-setup`

Configure git merge driver for JSONL conflict resolution.

```bash
shape merge-setup
```

### `shape agent-setup [OPTIONS]`

Configure AI agent integration files.

```bash
shape agent-setup              # Auto-detect and configure
shape agent-setup --show       # Preview without writing
shape agent-setup --claude     # Only CLAUDE.md
shape agent-setup --cursor     # Only .cursorrules
shape agent-setup --windsurf   # Only .windsurfrules
```

## Plugin Commands

### `shape advanced plugin list`

List available plugins.

```bash
shape advanced plugin list
```

### `shape advanced plugin show <NAME>`

Show plugin details.

```bash
shape advanced plugin show shape-sync-github
```

### `shape advanced sync run <PLUGIN>`

Run sync with external tool.

```bash
shape advanced sync run github
shape advanced sync run linear
```

### `shape advanced sync status`

Show sync status for all configured plugins.

```bash
shape advanced sync status
```
