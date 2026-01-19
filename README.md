# Shape CLI

Turn documents into tasks. Turn tasks into AI context.
Git-backed, local-first project management for humans and AI agents.

[![Crates.io](https://img.shields.io/crates/v/shape-cli.svg)](https://crates.io/crates/shape-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## The Problem

Your documents (epics, PRDs, RFCs, Pitches) live in one place. Your tasks live in another. AI agents can't see either. You copy-paste between tools and lose context.

## The Solution

Shape stores **briefs** alongside the **tasks** they spawn. Both are git-backed. Both are AI-readable.

**Briefs** can be any document that drives work:
- Epics, User Stories (Agile/Scrum)
- PRDs (Product teams)
- RFCs, ADRs (Engineering)
- Pitches (ShapeUp)

```
Brief (Markdown)  →  Tasks (JSONL)  →  AI Context
"The Why"            "The What"        "What Agents See"
```

## Quick Start

```bash
# Install
cargo install shape-cli

# Initialize
shape init

# Create a brief and add tasks
shape brief new "User Authentication"
# → Created brief b-7f2a3b1

shape task add b-7f2a3b1 "Set up OAuth provider"
shape task add b-7f2a3b1 "Build login endpoint"
shape task add b-7f2a3b1 "Add session management"

# Set dependencies
shape task dep b-7f2a3b1.2 b-7f2a3b1.1   # login waits for OAuth
shape task dep b-7f2a3b1.3 b-7f2a3b1.2   # sessions wait for login

# See what's ready
shape ready
# → b-7f2a3b1.1  Set up OAuth provider

# Export for AI
shape context --compact
```

## Installation

### Cargo (Rust)

```bash
cargo install shape-cli
```

### Homebrew (macOS/Linux)

```bash
brew install shape-cli/tap/shape-cli
```

<details>
<summary>Other installation options</summary>

### npm (Node.js)

```bash
npm install -g shape-cli
```

### pip (Python)

```bash
pip install shape-cli
```

### gem (Ruby)

```bash
gem install shape-cli
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/shape-cli/shape/releases):

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | `shape-darwin-arm64.tar.gz` |
| macOS (Intel) | `shape-darwin-x64.tar.gz` |
| Linux (x64) | `shape-linux-x64.tar.gz` |
| Linux (ARM64) | `shape-linux-arm64.tar.gz` |
| Windows (x64) | `shape-windows-x64.zip` |

</details>

## Features

### Core
- **Briefs** — Human-editable markdown documents with YAML frontmatter
- **Tasks** — Machine-readable JSONL with dependency tracking
- **Ready queue** — `shape ready` shows unblocked tasks
- **Search** — Full-text search across briefs and tasks

### AI Integration
- **Context export** — `shape context --compact` for minimal tokens
- **Agent setup** — `shape agent-setup` configures Claude, Cursor, Windsurf
- **MCP server** — Native Model Context Protocol support
- **JSON output** — `--format json` on all commands

### Multi-Agent Coordination
- **Claims** — `shape claim` / `shape unclaim` for task ownership
- **Next task** — `shape next` suggests optimal task to work on
- **Handoffs** — `shape handoff` transfers work between agents
- **History** — `shape history` shows task timeline
- **Notes & links** — Attach context, commits, PRs to tasks

### Infrastructure
- **TUI viewer** — `shape tui` for interactive browsing
- **Background daemon** — `shape daemon` for automatic git sync
- **Memory compaction** — `shape compact` summarizes old tasks
- **Merge driver** — Conflict resolution for concurrent edits

## Essential Commands

| Command | Description |
|---------|-------------|
| `shape ready` | Show tasks ready to work on |
| `shape next` | Suggest the best next task |
| `shape task start <id>` | Mark task in progress |
| `shape task done <id>` | Mark task complete |
| `shape context --compact` | Export state for AI |
| `shape tui` | Interactive terminal UI |

## Documentation

- [Quick Start](docs/QUICKSTART.md) — Your first 5 minutes
- [Commands](docs/COMMANDS.md) — Full reference
- [Briefs Guide](docs/BRIEFS.md) — Document types and templates
- [AI Integration](docs/AI_INTEGRATION.md) — Claude, Cursor, MCP setup
- [Multi-Agent](docs/MULTI_AGENT.md) — Coordination for teams
- [Plugins](docs/PLUGINS.md) — Build your own
- [Storage](docs/STORAGE.md) — File formats and structure
- [FAQ](docs/FAQ.md) — Common questions

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)
