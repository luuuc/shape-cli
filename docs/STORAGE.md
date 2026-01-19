# Storage

Shape CLI stores all data in git-friendly formats. This guide explains the storage structure, file formats, and how to work with them directly.

## Directory Structure

```
.shape/
├── briefs/
│   ├── b-7f2a3b1.md       # Brief markdown files
│   ├── b-8c3d2e1.md
│   └── index.jsonl        # Auto-generated index (git-ignored)
├── tasks.jsonl            # All tasks
├── config.toml            # Project configuration
├── plugins/               # Local plugins
├── sync/                  # Sync state (git-ignored)
│   ├── github.jsonl
│   └── linear.jsonl
└── .cache/                # SQLite cache (git-ignored)
    └── shape.db
```

## Briefs (Markdown)

Briefs are stored as markdown files with YAML frontmatter.

### Format

```markdown
---
id: b-7f2a3b1
title: User Authentication
status: in_progress
type: shapeup
appetite: 2-weeks
created: 2025-01-16T10:30:00Z
updated: 2025-01-16T14:00:00Z
---

# User Authentication

## Problem

Users currently have no way to...

## Solution

Implement OAuth-based authentication...
```

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Brief identifier (e.g., `b-7f2a3b1`) |
| `title` | Yes | Brief title |
| `status` | Yes | `proposed`, `betting`, `in_progress`, `shipped`, `archived` |
| `type` | Yes | Brief type (e.g., `minimal`, `shapeup`) |
| `created` | Yes | ISO 8601 timestamp |
| `updated` | No | ISO 8601 timestamp |
| `appetite` | No | Time budget (ShapeUp: `1-week`, `2-weeks`, `6-weeks`) |

### ID Generation

Brief IDs are derived from a BLAKE3 hash of the title and creation timestamp:

```
b-{blake3(title + created)[0:7]}
```

This ensures:
- Deterministic IDs
- Low collision probability
- Short, readable identifiers

## Tasks (JSONL)

Tasks are stored in `tasks.jsonl` — one JSON object per line.

### Format

```jsonl
{"id":"b-7f2a3b1.1","brief_id":"b-7f2a3b1","title":"Research OAuth providers","status":"done","created":"2025-01-16T10:35:00Z","dependencies":[],"notes":[],"links":[]}
{"id":"b-7f2a3b1.2","brief_id":"b-7f2a3b1","title":"Implement OAuth flow","status":"in_progress","created":"2025-01-16T10:36:00Z","dependencies":[{"id":"b-7f2a3b1.1","type":"blocks"}],"notes":["Found edge case"],"links":[{"type":"commit","value":"abc1234"}]}
```

### Task Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Task identifier (e.g., `b-7f2a3b1.1`) |
| `brief_id` | string | Parent brief ID (null for standalone) |
| `title` | string | Task title |
| `status` | string | `todo`, `in_progress`, `done` |
| `created` | string | ISO 8601 timestamp |
| `updated` | string | ISO 8601 timestamp |
| `dependencies` | array | List of dependency objects |
| `claimed_by` | string | Agent name (if claimed) |
| `blocked_reason` | string | Explicit block reason |
| `notes` | array | List of note strings |
| `links` | array | List of link objects |
| `history` | array | List of history events |

### Dependency Object

```json
{
  "id": "b-7f2a3b1.1",
  "type": "blocks"
}
```

Types: `blocks`, `from`, `related`, `duplicates`

### Link Object

```json
{
  "type": "commit",
  "value": "abc1234"
}
```

Types: `commit`, `pr`, `file`, `url`

### Why JSONL?

- **Git-friendly** — Line-based diffs
- **Append-only friendly** — New tasks add lines, don't modify existing
- **Streaming** — Can process without loading entire file
- **Conflict-resolvable** — Each line is independent

## Configuration (TOML)

Project configuration in `config.toml`:

```toml
[project]
name = "my-project"
default_brief_type = "minimal"

[daemon]
enabled = true
sync_interval = 300  # seconds

[plugins.sync.github]
repo = "owner/repo"

[compact]
default_days = 7
```

## Index (JSONL)

The brief index in `briefs/index.jsonl` is auto-generated for fast queries:

```jsonl
{"id":"b-7f2a3b1","title":"User Authentication","status":"in_progress","type":"shapeup","created":"2025-01-16T10:30:00Z"}
{"id":"b-8c3d2e1","title":"Search Redesign","status":"proposed","type":"minimal","created":"2025-01-16T11:00:00Z"}
```

This file is:
- Git-ignored (regenerated from markdown files)
- Rebuilt on demand when stale
- Used for fast listing without parsing all markdown

## Cache (SQLite)

The `.cache/shape.db` SQLite database provides:
- Full-text search across briefs and tasks
- Fast queries without scanning JSONL
- Temporary data (not committed to git)

### Manage Cache

```bash
shape cache build     # Rebuild from source files
shape cache clear     # Delete cache
shape cache analyze   # Show cache stats
```

## Sync State

The `sync/` directory stores ID mappings for external tools:

```jsonl
{"local":"b-7f2a3b1","remote":"123","plugin":"github","synced":"2025-01-16T10:30:00Z"}
{"local":"b-7f2a3b1.1","remote":"124","plugin":"github","synced":"2025-01-16T10:31:00Z"}
```

This is git-ignored because:
- Remote IDs are environment-specific
- Different team members may have different permissions
- Sync state can be regenerated

## Merge Driver

Shape includes a custom git merge driver for `tasks.jsonl` conflicts.

### Setup

```bash
shape merge-setup
```

This adds to `.gitattributes`:

```
.shape/tasks.jsonl merge=shape-tasks
```

And configures the merge driver in `.git/config`.

### Conflict Resolution

When conflicts occur, the merge driver:
1. Parses both versions
2. Identifies conflicting tasks by ID
3. Uses last-write-wins based on `updated` timestamp
4. Preserves all unique tasks from both branches

### Manual Conflicts

If the merge driver can't resolve automatically:
1. The file is left with conflict markers
2. Manually edit to resolve
3. Run `shape cache build` to rebuild index

## File Locking

Shape uses file locking (via `fs2`) for concurrent access:
- Prevents corruption when multiple processes write
- Short-lived locks (released immediately after write)
- Graceful fallback if locking unavailable

## Backup and Recovery

### Backup

Everything important is in git:
```bash
git add .shape/
git commit -m "Backup shape data"
```

### Recovery

If files are corrupted:
```bash
# Restore from git
git checkout HEAD -- .shape/

# Rebuild cache
shape cache build
```

### Index Recovery

If the brief index is corrupted:
```bash
# Delete and let Shape rebuild
rm .shape/briefs/index.jsonl
shape brief list  # Triggers rebuild
```

## Direct Editing

You can edit files directly:

### Briefs

Edit markdown files in any editor:
```bash
vim .shape/briefs/b-7f2a3b1.md
```

Then rebuild index:
```bash
shape cache build
```

### Tasks

Editing `tasks.jsonl` directly is possible but not recommended. Use CLI commands instead.

If you must edit:
1. Back up the file
2. Edit carefully (each line must be valid JSON)
3. Run `shape cache build`

## Portability

The `.shape/` directory is fully portable:
- Copy to another machine
- Check into any git repository
- No external dependencies for data

Only `.cache/` and `sync/` are environment-specific and git-ignored.
