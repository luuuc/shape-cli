# shape-sync-markdowndir-ruby

A sync plugin for Shape CLI that stores briefs as individual markdown files with YAML frontmatter. Implemented in Ruby using only standard library dependencies.

## Requirements

- Ruby 2.7+

## Installation

```bash
# Make executable
chmod +x shape-sync-markdowndir-ruby

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-markdowndir-ruby ~/.local/bin/

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
shape sync test shape-sync-markdowndir-ruby

# Push briefs to markdown files
shape sync push shape-sync-markdowndir-ruby

# Pull briefs from markdown files
shape sync pull shape-sync-markdowndir-ruby

# Check sync status
shape sync status shape-sync-markdowndir-ruby
```

## File Format

Each brief is stored as a markdown file with YAML frontmatter:

```markdown
---
id: b-123
title: My Feature
status: proposed
appetite: 2-week
synced_at: '2024-01-15T10:30:00Z'
---

The body content of the brief goes here...
```

Files are named using the pattern: `{id}-{slugified-title}.md`

Example: `b-123-my-feature.md`

## Protocol

### Manifest

```bash
./shape-sync-markdowndir-ruby --manifest
```

Returns:
```json
{"name":"shape-sync-markdowndir-ruby","version":"1.0.0","description":"Sync briefs to markdown directory (Ruby)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-markdowndir-ruby
```

#### push

Writes briefs as individual markdown files.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test Brief","status":"proposed","appetite":"2-week","body":"# Problem\n\nDescription here..."}]}}' | ./shape-sync-markdowndir-ruby
```

#### pull

Reads briefs from markdown files in the output directory.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-markdowndir-ruby
```

#### status

Returns the count of synced briefs.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-markdowndir-ruby
```

## Why Ruby?

This plugin showcases Ruby's strengths for text processing:

1. **Excellent YAML support**: Ruby's built-in YAML library makes frontmatter parsing clean
2. **String manipulation**: Ruby's regex and string methods are powerful and readable
3. **Rails community appeal**: Ruby is the language of Basecamp, where Shape Up originated
4. **Zero dependencies**: Uses only Ruby standard library (`json`, `yaml`, `fileutils`, `time`)
