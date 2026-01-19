# shape-sync-jsonfile-ruby

A minimal sync plugin for Shape CLI implemented in Ruby. This plugin demonstrates the language-agnostic nature of the plugin protocol using idiomatic Ruby with only standard library dependencies.

## Requirements

- Ruby 2.7+

## Installation

```bash
# Make executable
chmod +x shape-sync-jsonfile-ruby

# Option 1: Add to PATH
ln -s $(pwd)/shape-sync-jsonfile-ruby ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs and tasks to a local `shape-sync.json` file.

```bash
# Test the plugin
shape sync test shape-sync-jsonfile-ruby

# Push briefs to JSON file
shape sync push shape-sync-jsonfile-ruby

# Pull briefs from JSON file
shape sync pull shape-sync-jsonfile-ruby

# Check sync status
shape sync status shape-sync-jsonfile-ruby
```

## Protocol

### Manifest

```bash
./shape-sync-jsonfile-ruby --manifest
```

Returns:
```json
{"name":"shape-sync-jsonfile-ruby","version":"1.0.0","description":"Sync briefs to local JSON file (Ruby)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-jsonfile-ruby
```

#### push

Saves briefs and tasks to `shape-sync.json`.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test"}],"tasks":[]}}' | ./shape-sync-jsonfile-ruby
```

#### pull

Reads briefs and tasks from `shape-sync.json`.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-jsonfile-ruby
```

#### status

Returns sync status information.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-jsonfile-ruby
```

## Why Ruby?

This plugin exists as an example for Ruby developers who want to create Shape plugins. It demonstrates:

1. **Zero gem dependencies**: Uses only Ruby standard library (`json`, `time`)
2. **Idiomatic Ruby**: Clean, readable code that follows Ruby conventions
3. **Easy to modify**: Simple procedural style that's easy to understand and adapt
4. **Rails-friendly**: Ruby is the language of Basecamp, where Shape Up originated
