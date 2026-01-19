# AI Integration

Shape CLI is designed for AI agents. This guide covers setup for Claude Code, Cursor, Windsurf, and MCP server integration.

## Quick Setup

The fastest way to configure AI integration:

```bash
shape agent-setup
```

This auto-detects existing AI config files and appends Shape instructions to:
- `CLAUDE.md` (Claude Code)
- `.cursorrules` (Cursor)
- `.windsurfrules` (Windsurf)
- `AGENTS.md` (generic)

### Options

```bash
shape agent-setup --show       # Preview without writing
shape agent-setup --claude     # Only CLAUDE.md
shape agent-setup --cursor     # Only .cursorrules
shape agent-setup --windsurf   # Only .windsurfrules
```

## Claude Code

### Automatic Setup

```bash
shape agent-setup --claude
```

### Manual Setup

Add to `CLAUDE.md`:

```markdown
## Task Management

This project uses Shape CLI for task management.

### Before starting work
1. Run `shape ready` to see available tasks
2. Run `shape claim <task-id>` to claim a task
3. Run `shape task start <task-id>` to mark it in progress

### While working
- Add notes: `shape note <task-id> "Found edge case..."`
- Link commits: `shape link <task-id> --commit <hash>`
- Check context: `shape context --compact`

### When done
1. Run `shape task done <task-id>` to complete
2. Run `shape ready` to see what's next

### If blocked
- Run `shape block <task-id> "reason"` to mark blocked
- Run `shape handoff <task-id> "reason"` to hand off
```

## Cursor

### Automatic Setup

```bash
shape agent-setup --cursor
```

### Manual Setup

Add to `.cursorrules`:

```
# Task Management

Use Shape CLI for task tracking:
- `shape ready` - see available tasks
- `shape context --compact` - get project state
- `shape task start <id>` - begin work
- `shape task done <id>` - complete work
- `shape next` - get suggested task

Always check `shape ready` before starting new work.
Mark tasks complete with `shape task done` when finished.
```

## Windsurf

### Automatic Setup

```bash
shape agent-setup --windsurf
```

### Manual Setup

Add to `.windsurfrules`:

```
# Task Management

This project uses Shape CLI. Key commands:
- shape ready: Show tasks ready to work on
- shape next: Suggest best next task
- shape task start <id>: Mark task in progress
- shape task done <id>: Mark task complete
- shape context --compact: Export state for AI

Workflow:
1. Check shape ready for available work
2. Claim task with shape claim <id>
3. Work on task
4. Complete with shape task done <id>
```

## MCP Server

Shape includes a Model Context Protocol (MCP) server for native AI integration.

### Configuration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "shape": {
      "command": "shape",
      "args": ["mcp-server"]
    }
  }
}
```

### Available Tools

The MCP server exposes these tools:

| Tool | Description |
|------|-------------|
| `shape_ready` | List unblocked tasks |
| `shape_next` | Suggest next task |
| `shape_task_start` | Mark task in progress |
| `shape_task_done` | Mark task complete |
| `shape_context` | Get project context |
| `shape_claim` | Claim a task |
| `shape_note` | Add note to task |
| `shape_link` | Link artifact to task |

### Example Usage

From an MCP-enabled AI:

```
Use shape_ready to see what tasks are available.
```

```
Use shape_task_done with task_id "b-7f2a3b1.1" to complete the task.
```

## Context Export

The `shape context` command exports project state optimized for AI consumption.

### Full Context

```bash
shape context
```

Includes:
- All briefs with full content
- All tasks with dependencies
- Recent history and notes

### Compact Context

```bash
shape context --compact
```

Minimized for token efficiency:
- Brief titles and statuses only
- Task titles, statuses, and blocking relationships
- No historical data

Target: < 2000 tokens for typical projects.

### Filtered Context

```bash
# Single brief
shape context --brief b-7f2a3b1

# Include older completed tasks
shape context --days 14

# Combine options
shape context --compact --brief b-7f2a3b1
```

## JSON Output

All commands support `--format json` for machine parsing:

```bash
shape ready --format json
shape task list --format json
shape brief show b-7f2a3b1 --format json
```

## Agent Workflow

Recommended workflow for AI agents:

```bash
# 1. Check what's ready
shape ready

# 2. Claim and start a task
shape claim b-7f2a3b1.1
shape task start b-7f2a3b1.1

# 3. Work on the task...

# 4. Add notes as you go
shape note b-7f2a3b1.1 "Implemented OAuth flow"

# 5. Link artifacts
shape link b-7f2a3b1.1 --commit abc1234

# 6. Complete the task
shape task done b-7f2a3b1.1

# 7. Check what's next
shape ready
```

## Multi-Agent Coordination

When multiple agents work on the same project:

```bash
# Claim with agent name
shape claim b-7f2a3b1.1 --agent claude

# Hand off to another agent
shape handoff b-7f2a3b1.1 "Need frontend expertise" --to cursor

# Force claim if stuck
shape claim b-7f2a3b1.1 --force --reason "Previous agent timed out"
```

See [Multi-Agent](MULTI_AGENT.md) for details.

## Memory Compaction

For long-running projects, compact old tasks to save context window:

```bash
# Compress tasks older than 7 days
shape compact

# Preview first
shape compact --dry-run

# Custom threshold
shape compact --days 30
```

See `shape compact --help` for options.
