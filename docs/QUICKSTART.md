# Quick Start

Get up and running with Shape CLI in 5 minutes.

## Install

```bash
cargo install shape-cli
```

Or see [other installation options](../README.md#installation).

## Initialize a Project

```bash
cd your-project
shape init
```

This creates a `.shape/` directory with:
- `briefs/` — Your markdown documents
- `tasks.jsonl` — Task data
- `config.toml` — Project settings

## Your First Brief

A **brief** is a document that drives work. Create one:

```bash
shape brief new "Add user authentication"
# → Created brief b-7f2a3b1
```

Edit the brief in your editor — it's just markdown:

```bash
$EDITOR .shape/briefs/b-7f2a3b1.md
```

## Add Tasks

Tasks belong to briefs. Add some:

```bash
shape task add b-7f2a3b1 "Research OAuth providers"
shape task add b-7f2a3b1 "Implement OAuth flow"
shape task add b-7f2a3b1 "Add login endpoint"
shape task add b-7f2a3b1 "Write authentication tests"
```

List them:

```bash
shape task list b-7f2a3b1
# → b-7f2a3b1.1  Todo  Research OAuth providers
# → b-7f2a3b1.2  Todo  Implement OAuth flow
# → b-7f2a3b1.3  Todo  Add login endpoint
# → b-7f2a3b1.4  Todo  Write authentication tests
```

## Set Dependencies

Some tasks block others. The OAuth flow needs research first:

```bash
shape task dep b-7f2a3b1.2 b-7f2a3b1.1   # flow depends on research
shape task dep b-7f2a3b1.3 b-7f2a3b1.2   # endpoint depends on flow
shape task dep b-7f2a3b1.4 b-7f2a3b1.3   # tests depend on endpoint
```

## Check What's Ready

```bash
shape ready
# → b-7f2a3b1.1  Research OAuth providers
```

Only unblocked tasks appear. As you complete them, more become ready.

## Work on Tasks

```bash
# Start working
shape task start b-7f2a3b1.1

# When done
shape task done b-7f2a3b1.1

# Check what's ready now
shape ready
# → b-7f2a3b1.2  Implement OAuth flow
```

## Export for AI

Get project state in a format optimized for AI agents:

```bash
shape context --compact
```

This outputs minimal tokens while preserving essential context.

## What's Next?

- [Commands Reference](COMMANDS.md) — All available commands
- [Briefs Guide](BRIEFS.md) — Document types and templates
- [AI Integration](AI_INTEGRATION.md) — Set up Claude, Cursor, or Windsurf
- [Multi-Agent](MULTI_AGENT.md) — Coordinate multiple agents

## Quick Reference

```bash
shape brief new "Title"          # Create brief
shape brief list                 # List briefs
shape task add <brief> "Title"   # Add task
shape task dep <task> <blocker>  # Set dependency
shape ready                      # Show unblocked tasks
shape task start <task>          # Start working
shape task done <task>           # Complete task
shape context --compact          # Export for AI
shape tui                        # Interactive UI
```
