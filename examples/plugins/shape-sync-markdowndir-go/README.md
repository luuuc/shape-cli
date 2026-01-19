# shape-sync-markdowndir-go

A sync plugin for Shape CLI that exports briefs as individual markdown files with YAML frontmatter. Implemented in Go with only standard library packages.

## Requirements

- Go 1.21+

## Installation

```bash
cd examples/plugins/shape-sync-markdowndir-go
go build -o shape-sync-markdowndir-go

# Option 1: Add to PATH
cp shape-sync-markdowndir-go ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs to a directory of markdown files (default: `./pitches`).

```bash
# Test the plugin
shape sync test shape-sync-markdowndir-go

# Push briefs to markdown files
shape sync push shape-sync-markdowndir-go

# Pull briefs from markdown files
shape sync pull shape-sync-markdowndir-go

# Check sync status
shape sync status shape-sync-markdowndir-go
```

## Configuration

Set the output directory via environment variable:

```bash
export SHAPE_SYNC_MARKDOWN_DIR="./my-pitches"
```

## File Format

Each brief is saved as `{id}-{slugified-title}.md`:

```markdown
---
id: b-123
title: "My Brief Title"
status: proposed
appetite: 2-weeks
synced_at: 2024-01-15T10:30:00Z
---

Brief body content here...
```

## Protocol

### Manifest

```bash
./shape-sync-markdowndir-go --manifest
```

Returns:
```json
{"name":"shape-sync-markdowndir-go","version":"1.0.0","description":"Sync briefs to markdown directory (Go)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-markdowndir-go
```

#### push

Saves briefs as individual markdown files.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test Brief","status":"proposed","body":"Content here"}]}}' | ./shape-sync-markdowndir-go
```

#### pull

Reads briefs from markdown files in the output directory.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-markdowndir-go
```

#### status

Returns the count of markdown files in the output directory.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-markdowndir-go
```

## Why Go?

This plugin demonstrates Go patterns for Shape plugin development:

1. **Single binary distribution**: No runtime dependencies
2. **File system operations**: Creating directories, writing multiple files
3. **Text processing**: Slugifying titles, generating YAML frontmatter
4. **Standard library only**: No external dependencies
