# Multi-Agent Coordination

Shape CLI supports multiple AI agents (or humans) working on the same project simultaneously. This guide covers coordination commands and patterns.

## The Problem

When multiple agents work on a project:
- Two agents might start the same task
- Work gets lost when agents context-switch
- No visibility into who's doing what
- Handoffs are manual and lossy

## The Solution

Shape provides coordination primitives:
- **Claims** — Declare ownership of tasks
- **History** — Track what happened and when
- **Handoffs** — Transfer work with context
- **Notes & Links** — Attach artifacts to tasks

## Claims

### Claiming a Task

```bash
# Basic claim
shape claim b-7f2a3b1.1

# Claim with agent name
shape claim b-7f2a3b1.1 --agent claude

# Force claim (take from another agent)
shape claim b-7f2a3b1.1 --force --reason "Previous agent timed out"
```

### Viewing Claims

```bash
# Show task details including claim
shape task show b-7f2a3b1.1

# Ready tasks exclude claimed tasks by default
shape ready
```

### Releasing Claims

```bash
shape unclaim b-7f2a3b1.1
```

### Claim Behavior

- Only one agent can claim a task at a time
- Claims are advisory — they don't prevent other agents from working
- `--force` overrides existing claims (logged in history)
- Completing a task (`shape task done`) automatically releases the claim

## Task Suggestions

### Get Next Task

```bash
# Best task overall
shape next

# Best task for a specific brief
shape next --brief b-7f2a3b1

# Multiple suggestions
shape next -n 3
```

### Suggestion Algorithm

`shape next` considers:
1. **Unblocked** — No incomplete dependencies
2. **Unclaimed** — Not claimed by another agent
3. **Priority** — Higher priority tasks first
4. **Age** — Older tasks before newer (FIFO)
5. **Dependency depth** — Tasks that unblock others

## Notes

Add context to tasks as you work:

```bash
# Add a note
shape note b-7f2a3b1.1 "Found edge case: empty arrays crash the parser"

# Multiple notes accumulate
shape note b-7f2a3b1.1 "Fixed by adding null check"
shape note b-7f2a3b1.1 "Added regression test"

# View notes
shape task show b-7f2a3b1.1
```

Notes are timestamped and preserved in history.

## Links

Attach artifacts to tasks:

```bash
# Link a commit
shape link b-7f2a3b1.1 --commit abc1234

# Link a PR
shape link b-7f2a3b1.1 --pr 42

# Link a file
shape link b-7f2a3b1.1 --file src/auth.rs

# Link a URL
shape link b-7f2a3b1.1 --url "https://docs.example.com/auth"

# Multiple links
shape link b-7f2a3b1.1 --commit abc1234 --pr 42
```

### Removing Links

```bash
shape unlink b-7f2a3b1.1 --commit abc1234
shape unlink b-7f2a3b1.1 --pr 42
```

## History

View the complete timeline of a task:

```bash
shape history b-7f2a3b1.1
```

Output:

```
b-7f2a3b1.1: Implement OAuth flow

2025-01-16 10:30  Created
2025-01-16 10:35  Claimed by claude
2025-01-16 10:36  Status: Todo → InProgress
2025-01-16 11:00  Note: "Found edge case: empty arrays..."
2025-01-16 11:15  Link: commit abc1234
2025-01-16 11:30  Note: "Fixed by adding null check"
2025-01-16 11:45  Status: InProgress → Done
2025-01-16 11:45  Link: PR #42
2025-01-16 11:45  Unclaimed
```

## Blocking

Explicitly block tasks that can't proceed:

```bash
# Block with reason
shape block b-7f2a3b1.1 "Waiting for API key from vendor"

# Block referencing another task
shape block b-7f2a3b1.1 "Depends on external review" --on b-7f2a3b1.2
```

### Unblocking

```bash
shape unblock b-7f2a3b1.1
```

### Block vs Dependency

- **Dependency** (`shape task dep`) — Structural relationship between tasks
- **Block** (`shape block`) — Temporary impediment with human-readable reason

Dependencies are resolved automatically when the blocker completes. Explicit blocks require manual `unblock`.

## Handoffs

Transfer work to another agent or human:

```bash
# Hand off to human
shape handoff b-7f2a3b1.1 "Need human review of security implications"

# Hand off to specific agent
shape handoff b-7f2a3b1.1 "Needs frontend expertise" --to cursor

# Hand off with context
shape handoff b-7f2a3b1.1 "OAuth flow complete, need UI integration" --to cursor
```

### What Happens on Handoff

1. Current claim is released
2. Handoff reason is recorded in history
3. If `--to` specified, task is claimed by that agent
4. Task status remains unchanged

### Handoff Patterns

**Agent to Human:**
```bash
shape handoff b-7f2a3b1.1 "Need product decision on edge case"
```

**Agent to Agent:**
```bash
shape handoff b-7f2a3b1.1 "Backend complete, need frontend" --to cursor
```

**Stuck Agent:**
```bash
shape handoff b-7f2a3b1.1 "Hit API rate limit, try again later"
```

## Coordination Patterns

### Solo Agent

Simple workflow for one agent:

```bash
shape ready           # What's available?
shape task start X    # Begin work
# ... work ...
shape task done X     # Complete
shape ready           # What's next?
```

### Multiple Agents, Same Project

Each agent claims before working:

```bash
# Agent 1 (Claude)
shape next
shape claim b-7f2a3b1.1 --agent claude
shape task start b-7f2a3b1.1

# Agent 2 (Cursor) - sees b-7f2a3b1.1 is claimed
shape next
shape claim b-7f2a3b1.2 --agent cursor
shape task start b-7f2a3b1.2
```

### Pair Programming (Agent + Human)

Agent does initial work, human reviews:

```bash
# Agent
shape claim b-7f2a3b1.1 --agent claude
shape task start b-7f2a3b1.1
# ... implement ...
shape note b-7f2a3b1.1 "Implementation complete, needs review"
shape handoff b-7f2a3b1.1 "Ready for human review"

# Human reviews, then completes
shape task done b-7f2a3b1.1
```

### Pipeline (Multiple Agents, Sequential)

Work flows through specialized agents:

```bash
# Backend agent
shape claim b-7f2a3b1.1 --agent backend-agent
shape task start b-7f2a3b1.1
# ... implement API ...
shape handoff b-7f2a3b1.1 "API ready, need frontend" --to frontend-agent

# Frontend agent
shape task start b-7f2a3b1.1  # Already claimed by handoff
# ... implement UI ...
shape handoff b-7f2a3b1.1 "UI ready, need QA" --to qa-agent

# QA agent
shape task start b-7f2a3b1.1
# ... test ...
shape task done b-7f2a3b1.1
```

## Summary Commands

Get a quick summary of brief or task state:

```bash
# Brief summary
shape summary b-7f2a3b1

# Task summary
shape summary b-7f2a3b1.1
```

## Best Practices

1. **Always claim before working** — Prevents duplicate effort

2. **Use meaningful agent names** — `--agent claude` not `--agent agent1`

3. **Add notes as you go** — Future agents (and humans) will thank you

4. **Link artifacts** — Commits and PRs provide provenance

5. **Handoff with context** — Explain why and what's needed

6. **Check history when resuming** — `shape history <id>` shows what happened

7. **Force-claim sparingly** — Only when previous agent is truly stuck
