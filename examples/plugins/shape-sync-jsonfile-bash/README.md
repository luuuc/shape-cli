# shape-sync-jsonfile-bash

A minimal sync plugin for Shape CLI implemented in pure Bash. This plugin demonstrates the language-agnostic nature of the plugin protocol by using only standard Unix tools.

## Requirements

- Bash 4.0+
- `jq` (JSON processor)

## Installation

```bash
# Make executable
chmod +x shape-sync-jsonfile-bash

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-jsonfile-bash ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs and tasks to a local `shape-sync.json` file.

```bash
# Test the plugin
shape sync test shape-sync-jsonfile-bash

# Push briefs to JSON file
shape sync push shape-sync-jsonfile-bash

# Pull briefs from JSON file
shape sync pull shape-sync-jsonfile-bash

# Check sync status
shape sync status shape-sync-jsonfile-bash
```

## Protocol

### Manifest

```bash
./shape-sync-jsonfile-bash --manifest
```

Returns:
```json
{"name":"shape-sync-jsonfile-bash","version":"1.0.0","description":"Sync briefs to local JSON file (Bash)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-jsonfile-bash
```

#### push

Saves briefs and tasks to `shape-sync.json`.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test"}],"tasks":[]}}' | ./shape-sync-jsonfile-bash
```

#### pull

Reads briefs and tasks from `shape-sync.json`.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-jsonfile-bash
```

#### status

Returns sync status information.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-jsonfile-bash
```

## Why Bash?

This plugin exists as a proof-of-concept that the Shape plugin protocol is simple enough to implement in any language, including shell scripts. It demonstrates:

1. **Zero dependencies**: No runtime installation required beyond standard Unix tools
2. **Protocol simplicity**: If it works in Bash, the protocol is well-designed
3. **Quick prototyping**: Sketch plugin ideas before porting to a more robust language
