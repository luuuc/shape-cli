# shape-cli

A local-first task management CLI for software teams. Organize work around "anchors" (pitches, RFCs, PRDs) with dependent tasks and AI-optimized context export.

## Installation

```bash
pip install shape-cli
```

## Quick Start

```bash
# Initialize a project
shape init

# Create an anchor (pitch/RFC/etc)
shape anchor new "My Feature Pitch" --type shapeup

# Add tasks to the anchor
shape task add a-1234567 "Build the API"
shape task add a-1234567 "Write tests"

# See what's ready to work on
shape ready

# Export context for AI
shape context --compact
```

## Documentation

See the [main repository](https://github.com/shape-cli/shape) for full documentation.

## Supported Platforms

- macOS (Apple Silicon and Intel)
- Linux (x64 and ARM64)
- Windows (x64)

## License

MIT
