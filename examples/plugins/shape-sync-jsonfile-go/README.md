# shape-sync-jsonfile-go

A minimal sync plugin for Shape CLI implemented in Go. This plugin demonstrates the language-agnostic nature of the plugin protocol using idiomatic Go with only standard library packages.

## Requirements

- Go 1.21+

## Installation

```bash
cd examples/plugins/shape-sync-jsonfile-go
go build -o shape-sync-jsonfile-go

# Option 1: Add to PATH
cp shape-sync-jsonfile-go ~/.local/bin/

# Option 2: Add plugin directory to Shape config
shape config set plugin_dirs "$(pwd)"
```

## Usage

The plugin syncs briefs and tasks to a local `shape-sync.json` file.

```bash
# Test the plugin
shape sync test shape-sync-jsonfile-go

# Push briefs to JSON file
shape sync push shape-sync-jsonfile-go

# Pull briefs from JSON file
shape sync pull shape-sync-jsonfile-go

# Check sync status
shape sync status shape-sync-jsonfile-go
```

## Protocol

### Manifest

```bash
./shape-sync-jsonfile-go --manifest
```

Returns:
```json
{"name":"shape-sync-jsonfile-go","version":"1.0.0","description":"Sync briefs to local JSON file (Go)","type":"sync","operations":["push","pull","status","test"]}
```

### Operations

All operations receive JSON on stdin and return JSON on stdout.

#### test

Tests that the plugin is working.

```bash
echo '{"operation":"test","params":{}}' | ./shape-sync-jsonfile-go
```

#### push

Saves briefs and tasks to `shape-sync.json`.

```bash
echo '{"operation":"push","params":{"briefs":[{"id":"b-123","title":"Test"}],"tasks":[]}}' | ./shape-sync-jsonfile-go
```

#### pull

Reads briefs and tasks from `shape-sync.json`.

```bash
echo '{"operation":"pull","params":{}}' | ./shape-sync-jsonfile-go
```

#### status

Returns sync status information.

```bash
echo '{"operation":"status","params":{}}' | ./shape-sync-jsonfile-go
```

## Why Go?

This plugin exists as an example for Go developers who want to create Shape plugins. It demonstrates:

1. **Single binary distribution**: No runtime dependencies, just copy and run
2. **Cross-compilation**: Easy to build for Linux/macOS/Windows from any platform
3. **Strong typing**: Catches protocol errors at compile time
4. **Fast startup**: Important for plugins called frequently
5. **Standard library only**: No external dependencies
