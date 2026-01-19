# shape-sync-jsonfile-nodejs

A minimal sync plugin for Shape CLI implemented in Node.js. This plugin demonstrates the language-agnostic nature of the plugin protocol using idiomatic JavaScript with only built-in modules.

## Requirements

- Node.js 14+

## Installation

```bash
# Make executable
chmod +x shape-sync-jsonfile-nodejs

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-jsonfile-nodejs ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs and tasks to a local `shape-sync.json` file.

```bash
# Test the plugin
shape sync test shape-sync-jsonfile-nodejs

# Push briefs to JSON file
shape sync push shape-sync-jsonfile-nodejs

# Pull briefs from JSON file
shape sync pull shape-sync-jsonfile-nodejs

# Check sync status
shape sync status shape-sync-jsonfile-nodejs
```

## Protocol

### Manifest

```bash
./shape-sync-jsonfile-nodejs --manifest
```

Returns:
```json
{"name":"shape-sync-jsonfile-nodejs","version":"1.0.0","description":"Sync briefs to local JSON file (Node.js)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-jsonfile-nodejs
```

#### push

Saves briefs and tasks to `shape-sync.json`.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test"}],"tasks":[]}}' | ./shape-sync-jsonfile-nodejs
```

#### pull

Reads briefs and tasks from `shape-sync.json`.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-jsonfile-nodejs
```

#### status

Returns sync status information.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-jsonfile-nodejs
```

## Why Node.js?

This plugin exists as an example for JavaScript developers who want to create Shape plugins. It demonstrates:

1. **Zero npm dependencies**: Uses only Node.js built-in modules (`fs`, `readline`)
2. **CommonJS format**: Maximum compatibility without transpilation
3. **Synchronous I/O**: Simpler and clearer for this use case
4. **Familiar patterns**: Easy to extend with npm packages if needed later
