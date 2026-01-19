# Shape CLI

A local-first task management tool for software teams. Organize work around "anchors" (pitches, RFCs, PRDs) with dependent tasks and AI-optimized context export.

## Installation

### npm (Node.js)

```bash
npm install -g shape-cli
# or use without installing
npx shape-cli ready
```

### pip (Python)

```bash
pip install shape-cli
```

### gem (Ruby)

```bash
gem install shape-cli
# or add to Gemfile
gem "shape-cli"
```

### Cargo (Rust)

```bash
cargo install shape-cli
```

### Homebrew (macOS/Linux)

```bash
brew install shape-cli/tap/shape-cli
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/shape-cli/shape/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `shape-darwin-arm64.tar.gz` |
| macOS (Intel) | `shape-darwin-x64.tar.gz` |
| Linux (x64) | `shape-linux-x64.tar.gz` |
| Linux (ARM64) | `shape-linux-arm64.tar.gz` |
| Linux (Alpine/musl) | `shape-linux-x64-musl.tar.gz` |
| Windows (x64) | `shape-windows-x64.zip` |

## Quick Start

```bash
# Initialize a project
shape init

# Create an anchor (pitch/RFC/etc)
shape anchor new "My Feature Pitch" --type shapeup

# Add tasks to the anchor
shape task add a-1234567 "Build the API"
shape task add a-1234567 "Write tests"

# Set dependencies
shape task dep a-1234567.2 a-1234567.1

# See what's ready to work on
shape ready

# Export context for AI
shape context --compact
```

## Commands

| Command | Description |
|---------|-------------|
| `shape init` | Initialize a new shape project |
| `shape anchor new "Title"` | Create a new anchor |
| `shape anchor list` | List all anchors |
| `shape anchor show <id>` | Show anchor details |
| `shape anchor status <id> <status>` | Update anchor status |
| `shape task add <parent> "Title"` | Add a task |
| `shape task list <anchor>` | List tasks for an anchor |
| `shape task show <id>` | Show task details |
| `shape task start <id>` | Mark task in progress |
| `shape task done <id>` | Mark task complete |
| `shape task dep <task> <depends-on>` | Add dependency |
| `shape task undep <task> <depends-on>` | Remove dependency |
| `shape task meta <id> <key> <value>` | Set task metadata |
| `shape ready` | Show unblocked tasks |
| `shape blocked` | Show blocked tasks |
| `shape status` | Project overview |
| `shape context` | Export for AI |
| `shape plugin list` | List plugins |
| `shape plugin test <name>` | Test plugin connectivity |
| `shape sync run <plugin>` | Sync with external tool |
| `shape sync status` | Show sync status |
| `shape sync link <local> <remote>` | Link IDs manually |

## Anchor Types

### Minimal (default)

Basic anchor with title and status.

### ShapeUp

Full ShapeUp pitch template with:
- Problem statement
- Appetite (6-weeks, 2-weeks, 1-week)
- Solution overview
- Rabbit holes to avoid
- No-gos (out of scope)

```bash
shape anchor new "My Pitch" --type shapeup
```

## Storage

Data is stored in `.shape/`:
- `anchors/*.md` - Markdown files with YAML frontmatter
- `tasks.jsonl` - Task data in JSON Lines format
- `config.toml` - Project configuration

Anchors are human-editable markdown files.

## AI Context Export

Export project state in a format optimized for AI agents:

```bash
# Full context
shape context

# Compact (minimal tokens)
shape context --compact

# Single anchor
shape context --anchor a-1234567

# Include older completed tasks
shape context --days 14
```

## Sync Plugins

Shape supports bidirectional sync with external tools via plugins.

```bash
# Sync with GitHub (requires shape-sync-github plugin)
shape sync run github

# Check sync status
shape sync status

# Manually link a local anchor to a remote issue
shape sync link a-1234567 123 --plugin github
```

## Plugin Development

Plugins communicate via JSON over stdin/stdout. Two plugin types are supported:

### Anchor Type Plugins

Create custom anchor templates and validation. Binary name format: `shape-anchor-<name>`

### Sync Plugins

Bidirectional sync with external tools. Binary name format: `shape-sync-<name>`

Plugin discovery:
1. `.shape/plugins/` in project directory
2. Directories in `$PATH`

## AI Integration

Shape CLI is designed for AI agent consumption.

### Quick Setup

```bash
shape agent-setup  # Auto-configure CLAUDE.md, .cursorrules, etc.
```

This detects existing AI config files and adds Shape CLI instructions. Supports:
- `CLAUDE.md` (Claude Code)
- `.cursorrules` (Cursor)
- `.windsurfrules` (Windsurf)
- `AGENTS.md` (generic)

Options:
- `--show` - Preview instructions without writing
- `--claude` - Only configure CLAUDE.md
- `--cursor` - Only configure .cursorrules
- `--windsurf` - Only configure .windsurfrules

### Manual Integration

Add to your AI config file:
- Check `shape ready` for available tasks
- Use `shape context --compact` for token-efficient project state
- Mark tasks with `shape task start/done`

### Output Formats

All commands support `--format json` for machine parsing:

```bash
shape ready --format json
shape context --compact  # Already optimized for AI
```

### Example Workflow

```bash
# AI agent checks what's ready
shape ready --format json

# Starts working on a task
shape task start a-abc1234.1

# Completes the task
shape task done a-abc1234.1

# Gets full context if needed
shape context --compact
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Install Shape CLI
  run: npm install -g shape-cli

- name: Check ready tasks
  run: shape ready --format json
```

### GitLab CI

```yaml
install_shape:
  script:
    - pip install shape-cli
    - shape ready
```

### Generic (download binary)

```bash
curl -fsSL https://github.com/shape-cli/shape/releases/latest/download/shape-linux-x64.tar.gz | tar -xz
./shape ready
```

## Building from Source

```bash
# Development
cargo build

# Release
cargo build --release

# Tests
cargo test
```

## License

MIT
