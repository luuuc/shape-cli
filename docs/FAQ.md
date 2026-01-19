# FAQ

Frequently asked questions about Shape CLI.

## General

### What is Shape CLI?

Shape is a local-first, git-backed task management tool designed for software teams and AI agents. It organizes work around "briefs" (documents like epics, PRDs, RFCs) with dependent tasks and AI-optimized context export.

### How is Shape different from Beads?

Both are git-backed task trackers for AI agents. Key differences:

| Aspect | Shape | Beads |
|--------|-------|-------|
| Core abstraction | Briefs → Tasks | Issues only |
| Document storage | Markdown | JSONL |
| Methodology support | Pluggable (ShapeUp, RFC, etc.) | Generic |
| Mental model | Documents drive work | Flat task list |

Shape is better if your workflow starts with documents (pitches, RFCs, PRDs). Beads is leaner if you just want task tracking.

### How is Shape different from Linear/Jira/GitHub Issues?

- **Local-first** — Data lives in your repo, not a vendor's cloud
- **Git-backed** — Version control, branching, merging built-in
- **AI-optimized** — Context export designed for minimal tokens
- **Pluggable** — Sync with external tools, don't replace them (including Linear/Jira/GitHub Issues)

Shape can sync with these tools while keeping your source of truth local.

### Is Shape free?

Yes. Shape is open source under the MIT license.

## Installation

### What platforms are supported?

- macOS (Apple Silicon and Intel)
- Linux (x64, ARM64, musl)
- Windows (x64)

### How do I update Shape?

Same as installation:

```bash
cargo install shape-cli        # Cargo
brew upgrade shape-cli         # Homebrew
npm update -g shape-cli        # npm
pip install -U shape-cli       # pip
gem update shape-cli           # gem
```

### Can I use Shape without Rust/Cargo?

Yes. Use npm, pip, gem, Homebrew, or download pre-built binaries from GitHub Releases.

## Briefs

### What's a brief?

A brief is a document that drives work — the "why" behind your tasks. It can be:
- An Epic (Agile/Scrum)
- A PRD (Product teams)
- An RFC (Engineering)
- An ADR (Architecture)
- A Pitch (ShapeUp)

Briefs are stored as markdown files you can edit directly.

### Do I have to use briefs?

No. You can create standalone tasks:

```bash
shape task add "Fix typo in README"
```

Standalone tasks work without any brief.

### Can I have tasks without a brief?

Yes. Standalone tasks are created when you omit the brief ID:

```bash
shape task add "Quick fix"  # No brief ID = standalone
```

### How do I convert standalone tasks to a brief?

Currently, you'd need to:
1. Create the brief
2. Recreate tasks under that brief
3. Delete standalone tasks

A migration command is planned for future releases.

## Tasks

### What's the difference between `shape ready` and `shape next`?

- `shape ready` — Lists all unblocked tasks
- `shape next` — Suggests the single best task to work on (considering priority, age, dependencies)

### How do dependencies work?

```bash
# Task 2 depends on Task 1
shape task dep b-7f2a3b1.2 b-7f2a3b1.1
```

This means:
- Task 2 won't appear in `shape ready` until Task 1 is done
- Task 2 will appear in `shape blocked`

### Can dependencies create cycles?

No. Shape detects cycles and rejects them:

```bash
shape task dep b-7f2a3b1.1 b-7f2a3b1.2
# Error: Would create dependency cycle
```

### What are dependency types?

| Type | Flag | Affects ready queue? |
|------|------|---------------------|
| Blocks | `--blocks` (default) | Yes |
| Provenance | `--from` | No |
| Related | `--related` | No |
| Duplicates | `--duplicates` | No |

Only `blocks` dependencies affect which tasks are ready.

## AI Integration

### Which AI tools does Shape support?

- Claude Code (via CLAUDE.md)
- Cursor (via .cursorrules)
- Windsurf (via .windsurfrules)
- Any MCP-compatible client (via MCP server)
- Any tool that can run shell commands

### How do I set up Claude Code?

```bash
shape agent-setup --claude
```

Or manually add Shape instructions to `CLAUDE.md`.

### What's the MCP server?

MCP (Model Context Protocol) is a standard for AI tools to interact with external services. Shape's MCP server lets AI agents call Shape commands natively without shell access.

### How do I minimize token usage?

```bash
shape context --compact
```

This exports minimal context (< 2000 tokens for typical projects).

For long-running projects:

```bash
shape compact --days 7
```

This summarizes tasks older than 7 days.

## Multi-Agent

### Can multiple agents work on the same project?

Yes. Use claims to coordinate:

```bash
shape claim b-7f2a3b1.1 --agent claude
# Other agents see this task as claimed
```

### What happens if two agents claim the same task?

The second claim fails unless `--force` is used:

```bash
shape claim b-7f2a3b1.1 --force --reason "Taking over"
```

### How do I hand off work between agents?

```bash
shape handoff b-7f2a3b1.1 "Need frontend expertise" --to cursor
```

## Storage

### Where is data stored?

In `.shape/` in your project directory:
- `briefs/*.md` — Brief markdown files
- `tasks.jsonl` — Task data
- `config.toml` — Configuration

### Is data stored in the cloud?

No. Shape is local-first. All data stays in your repository.

### Can I edit files directly?

Yes. Briefs are markdown files — edit them in any text editor.

For tasks, use CLI commands. Direct editing of `tasks.jsonl` is possible but not recommended.

### How do I back up my data?

Commit to git:

```bash
git add .shape/
git commit -m "Backup shape data"
```

### What if I get merge conflicts?

Shape includes a merge driver for `tasks.jsonl`:

```bash
shape merge-setup
```

This enables automatic conflict resolution using last-write-wins.

## Plugins

### What plugins are available?

Built-in brief types:
- `minimal` — Basic brief
- `shapeup` — ShapeUp pitch template

Sync plugins are community-developed. Check the repository for available plugins.

### How do I create a plugin?

See [Plugins](PLUGINS.md). Plugins communicate via JSON over stdin/stdout, so you can use any language.

### Can I sync with Linear/Jira/GitHub?

The sync plugin interface exists, but first-party plugins are in development. Community contributions welcome!

## Troubleshooting

### `shape ready` shows nothing but I have tasks

Check:
1. Are tasks blocked by dependencies? (`shape blocked`)
2. Are tasks already done? (`shape task list`)
3. Are tasks claimed? (`shape task show <id>`)

### Commands are slow

Rebuild the cache:

```bash
shape cache build
```

### Brief index seems stale

Delete and rebuild:

```bash
rm .shape/briefs/index.jsonl
shape brief list
```

### Merge conflicts in tasks.jsonl

Set up the merge driver:

```bash
shape merge-setup
```

Then retry the merge.

### "File locked" errors

Another process is writing. Wait and retry. If persistent, check for stuck processes:

```bash
shape daemon stop  # If daemon is running
```

## Getting Help

### Where do I report bugs?

GitHub Issues: https://github.com/shape-cli/shape/issues

### Where do I ask questions?

GitHub Discussions: https://github.com/shape-cli/shape/discussions

### How do I contribute?

See [CONTRIBUTING.md](../CONTRIBUTING.md) for development setup and guidelines.
