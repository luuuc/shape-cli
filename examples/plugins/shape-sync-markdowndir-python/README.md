# shape-sync-markdowndir-python

A sync plugin for Shape CLI that stores briefs as individual markdown files with YAML frontmatter. Implemented in Python using only standard library dependencies.

## Requirements

- Python 3.7+

## Installation

```bash
# Make executable
chmod +x shape-sync-markdowndir-python

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-markdowndir-python ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Configuration

Set the output directory via environment variable:

```bash
export SHAPE_SYNC_MARKDOWN_DIR="./my-pitches"
```

Default: `./pitches`

## Usage

The plugin syncs briefs to individual markdown files with YAML frontmatter.

```bash
# Test the plugin
shape sync test shape-sync-markdowndir-python

# Push briefs to markdown files
shape sync push shape-sync-markdowndir-python

# Pull briefs from markdown files
shape sync pull shape-sync-markdowndir-python

# Check sync status
shape sync status shape-sync-markdowndir-python
```

## File Format

Each brief is stored as a markdown file with YAML frontmatter:

```markdown
---
id: b-123
title: "My Feature"
status: proposed
appetite: 2-week
synced_at: 2024-01-15T10:30:00Z
---

The body content of the brief goes here...
```

Files are named using the pattern: `{id}-{slugified-title}.md`

Example: `b-123-my-feature.md`

## Protocol

### Manifest

```bash
./shape-sync-markdowndir-python --manifest
```

Returns:
```json
{"name":"shape-sync-markdowndir-python","version":"1.0.0","description":"Sync briefs to markdown directory (Python)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-markdowndir-python
```

#### push

Writes briefs as individual markdown files.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test Brief","status":"proposed","appetite":"2-week","body":"# Problem\n\nDescription here..."}]}}' | ./shape-sync-markdowndir-python
```

#### pull

Reads briefs from markdown files in the output directory.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-markdowndir-python
```

#### status

Returns the count of synced briefs.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-markdowndir-python
```

## Why Python?

This plugin showcases Python's strengths for scripting:

1. **Ubiquity**: Python is pre-installed on most systems
2. **Readable**: Clear, explicit code that's easy to understand and modify
3. **Zero dependencies**: Uses only Python standard library (`json`, `os`, `re`, `sys`, `datetime`)
4. **Cross-platform**: Works on Windows, macOS, and Linux without changes
