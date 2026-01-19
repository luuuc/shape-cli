# Briefs Guide

Briefs are the heart of Shape. They're documents that drive work — the "why" behind your tasks.

## What is a Brief?

A **brief** is a human-readable document stored as markdown with YAML frontmatter. It can represent any document type that spawns work:

- **Epics** — Large features broken into tasks
- **PRDs** — Product requirements with deliverables
- **RFCs** — Technical proposals with action items
- **ADRs** — Architecture decisions with implementation tasks
- **User Stories** — Agile stories with acceptance criteria
- **Pitches** — ShapeUp-style problem/solution documents

```
┌─────────────────────────────────────┐
│  Brief (Markdown)                   │
│  "The Why"                          │
│                                     │
│  - Problem statement                │
│  - Proposed solution                │
│  - Scope and constraints            │
└───────────────┬─────────────────────┘
                │ spawns
                ▼
┌─────────────────────────────────────┐
│  Tasks (JSONL)                      │
│  "The What"                         │
│                                     │
│  - Discrete units of work           │
│  - Dependencies between tasks       │
│  - Status tracking                  │
└─────────────────────────────────────┘
```

## Creating Briefs

### Basic Brief

```bash
shape brief new "Add user authentication"
# → Created brief b-7f2a3b1
```

This creates `.shape/briefs/b-7f2a3b1.md`:

```markdown
---
id: b-7f2a3b1
title: Add user authentication
status: proposed
type: minimal
created: 2025-01-16T10:30:00Z
---

# Add user authentication

Describe your brief here.
```

### ShapeUp Brief

```bash
shape brief new "Search Redesign" --type shapeup
```

Creates a full ShapeUp pitch template:

```markdown
---
id: b-8c3d2e1
title: Search Redesign
status: proposed
type: shapeup
appetite: 6-weeks
created: 2025-01-16T10:30:00Z
---

# Search Redesign

## Problem

What problem are we solving? Who has it? What's the current workaround?

## Appetite

**6 weeks** — This is a big batch.

## Solution

Describe the solution at the right level of abstraction. Include:
- Key user flows
- Important edge cases
- Fat marker sketches (if applicable)

## Rabbit Holes

What could derail this project? What should we explicitly avoid?

-

## No-Gos

What are we explicitly NOT doing?

-
```

## Brief Lifecycle

Briefs move through statuses:

```
proposed → betting → in_progress → shipped
                  ↘              ↗
                    archived
```

| Status | Meaning |
|--------|---------|
| `proposed` | Draft, not yet approved |
| `betting` | Under consideration for a cycle |
| `in_progress` | Actively being worked on |
| `shipped` | Completed and delivered |
| `archived` | Abandoned or indefinitely deferred |

Update status:

```bash
shape brief status b-7f2a3b1 in_progress
shape brief status b-7f2a3b1 shipped
```

## Editing Briefs

Briefs are just markdown files. Edit them directly:

```bash
# Open in your editor
$EDITOR .shape/briefs/b-7f2a3b1.md

# Or use any text editor
code .shape/briefs/b-7f2a3b1.md
vim .shape/briefs/b-7f2a3b1.md
```

The YAML frontmatter is managed by Shape. Edit the markdown body freely.

## Brief Types

### Minimal (Default)

Basic brief with title and status. Good for:
- Quick features
- Bug fixes
- Chores

### ShapeUp

Full ShapeUp pitch template with:
- **Problem** — What we're solving
- **Appetite** — Time budget (1-week, 2-weeks, 6-weeks)
- **Solution** — How we'll solve it
- **Rabbit Holes** — What to avoid
- **No-Gos** — What we're explicitly not doing

Good for:
- Major features
- Product initiatives
- Cross-team projects

### Custom Brief Types (Plugins)

You can create custom brief types via plugins. See [Plugins](PLUGINS.md).

## Briefs and Tasks

Tasks belong to briefs:

```bash
# Create brief
shape brief new "User Authentication"
# → b-7f2a3b1

# Add tasks
shape task add b-7f2a3b1 "Research OAuth providers"
shape task add b-7f2a3b1 "Implement login flow"
shape task add b-7f2a3b1 "Add session management"

# List tasks for brief
shape task list b-7f2a3b1
```

Task IDs include the brief ID:
- `b-7f2a3b1.1` — First task
- `b-7f2a3b1.2` — Second task
- `b-7f2a3b1.3` — Third task

## Standalone Tasks

Not everything needs a brief. Create standalone tasks for:
- Quick fixes
- Chores
- Ad-hoc work

```bash
shape task add "Fix typo in README"
# → standalone.1
```

## Viewing Briefs

```bash
# List all briefs
shape brief list

# Filter by status
shape brief list --status in_progress

# Show details
shape brief show b-7f2a3b1

# Interactive TUI
shape tui --brief b-7f2a3b1
```

## Brief Storage

Briefs are stored in `.shape/briefs/`:

```
.shape/
├── briefs/
│   ├── b-7f2a3b1.md    # Markdown with YAML frontmatter
│   ├── b-8c3d2e1.md
│   └── index.jsonl     # Auto-generated index (git-ignored)
├── tasks.jsonl
└── config.toml
```

- **Markdown files** — Human-editable, version-controlled
- **Index** — Auto-regenerated for fast queries, not committed

## Best Practices

1. **One brief per initiative** — Don't overload briefs with unrelated work

2. **Write for humans first** — Briefs are documentation, not just task containers

3. **Keep tasks atomic** — Each task should be completable in one sitting

4. **Use dependencies** — Let Shape track what's blocked vs. ready

5. **Update status** — Move briefs through the lifecycle as work progresses

6. **Commit regularly** — Briefs are git-backed; commit as you update them
