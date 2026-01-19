# shape-sync-markdowndir-bash

A sync plugin for Shape CLI that writes briefs to a directory of markdown files. Implemented in pure Bash to demonstrate that the plugin protocol can handle complex operations without sophisticated tooling.

## Requirements

- Bash 4.0+
- `jq` (JSON processor)

## Installation

```bash
# Make executable
chmod +x shape-sync-markdowndir-bash

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-markdowndir-bash ~/.local/bin/

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

```bash
# Test the plugin
shape sync test shape-sync-markdowndir-bash

# Push briefs to markdown files
shape sync push shape-sync-markdowndir-bash

# Pull briefs from markdown files
shape sync pull shape-sync-markdowndir-bash

# Check sync status
shape sync status shape-sync-markdowndir-bash
```

## File Structure

Each brief becomes a markdown file:

```
pitches/
  b-123-my-feature.md
  b-456-another-feature.md
```

Files include YAML frontmatter:

```markdown
---
id: b-123
title: "My Feature"
status: proposed
appetite: 2-weeks
synced_at: 2024-01-15T10:30:00Z
---

The body content goes here...
```

## Protocol

### Manifest

```bash
./shape-sync-markdowndir-bash --manifest
```

Returns:
```json
{"name":"shape-sync-markdowndir-bash","version":"1.0.0","description":"Sync briefs to markdown directory (Bash)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-markdowndir-bash
```

#### push

Creates markdown files for each brief.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"My Feature","status":"proposed","body":"Description here"}]}}' | ./shape-sync-markdowndir-bash
```

#### pull

Reads all markdown files from the output directory.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-markdowndir-bash
```

#### status

Returns the number of synced files.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-markdowndir-bash
```

## Why Bash?

This plugin demonstrates that the Shape plugin protocol handles complex operations:

1. **Directory and file creation**: Creating multiple files in a directory structure
2. **String manipulation**: Generating slugs from titles
3. **Frontmatter handling**: Writing and parsing YAML-like frontmatter
4. **JSON array iteration**: Processing multiple briefs from input

If these operations work in Bash with just `jq`, plugin authors know the protocol is accessible to any language.
