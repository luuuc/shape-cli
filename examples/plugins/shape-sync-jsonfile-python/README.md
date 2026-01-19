# shape-sync-jsonfile-python

A minimal sync plugin for Shape CLI implemented in Python. This plugin demonstrates the language-agnostic nature of the plugin protocol using idiomatic Python with only standard library dependencies.

## Requirements

- Python 3.7+

## Installation

```bash
# Make executable
chmod +x shape-sync-jsonfile-python

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-jsonfile-python ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs and tasks to a local `shape-sync.json` file.

```bash
# Test the plugin
shape sync test shape-sync-jsonfile-python

# Push briefs to JSON file
shape sync push shape-sync-jsonfile-python

# Pull briefs from JSON file
shape sync pull shape-sync-jsonfile-python

# Check sync status
shape sync status shape-sync-jsonfile-python
```

## Protocol

### Manifest

```bash
./shape-sync-jsonfile-python --manifest
```

Returns:
```json
{"name":"shape-sync-jsonfile-python","version":"1.0.0","description":"Sync briefs to local JSON file (Python)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-jsonfile-python
```

#### push

Saves briefs and tasks to `shape-sync.json`.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test"}],"tasks":[]}}' | ./shape-sync-jsonfile-python
```

#### pull

Reads briefs and tasks from `shape-sync.json`.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-jsonfile-python
```

#### status

Returns sync status information.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-jsonfile-python
```

## Why Python?

This plugin exists as an example for Python developers who want to create Shape plugins. It demonstrates:

1. **Zero pip dependencies**: Uses only Python standard library (`json`, `sys`, `datetime`)
2. **Idiomatic Python**: Clean, readable code that follows Python conventions
3. **Easy to modify**: Simple procedural style that's easy to understand and adapt
4. **Widely accessible**: Python is one of the most popular languages for scripting and tooling
