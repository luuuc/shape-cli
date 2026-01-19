# shape-mcp

MCP (Model Context Protocol) server for Shape CLI. Exposes Shape task management as AI tools for Claude Code, Cursor, Amp, and other MCP-compatible clients.

## Installation

```bash
npm install -g shape-mcp
```

Or use directly with npx:

```bash
npx shape-mcp
```

## Prerequisites

- [Shape CLI](https://github.com/shape-cli/shape) must be installed and available in PATH
- A Shape project (run `shape init` in your project directory)

## Usage

### Claude Code

Add to your Claude Code MCP configuration (`~/.claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "shape": {
      "command": "npx",
      "args": ["shape-mcp"]
    }
  }
}
```

### Cursor

Add to your Cursor MCP configuration (`.cursor/mcp.json` in your project):

```json
{
  "servers": {
    "shape": {
      "command": "npx",
      "args": ["shape-mcp"]
    }
  }
}
```

### Other MCP Clients

Run the server in stdio mode:

```bash
npx shape-mcp
```

## Available Tools

| Tool | Description | Example Use |
|------|-------------|-------------|
| `shape_ready` | Get tasks ready to work on | Find what to implement next |
| `shape_blocked` | Get blocked tasks | Understand dependencies |
| `shape_status` | Project overview | Check overall progress |
| `shape_context` | AI-optimized context | Full project understanding |
| `shape_task_start` | Mark task in progress | Track what you're working on |
| `shape_task_done` | Mark task complete | Update progress |
| `shape_task_add` | Create a task | Break down work |
| `shape_anchor_list` | List anchors | See all pitches/RFCs |
| `shape_anchor_show` | Show anchor details | Understand requirements |

## Available Resources

| URI | Description |
|-----|-------------|
| `shape://context` | Full project context |
| `shape://ready` | Ready tasks |
| `shape://status` | Project status |

## Tool Details

### shape_ready

Get tasks that are ready to work on (no blocking dependencies).

```json
{
  "anchor": "a-1234567"  // Optional: filter by anchor
}
```

### shape_blocked

Get tasks blocked by incomplete dependencies.

```json
{
  "anchor": "a-1234567"  // Optional: filter by anchor
}
```

### shape_status

Get project overview with task counts. No parameters required.

### shape_context

Get AI-optimized project context.

```json
{
  "compact": true,       // Use compact format (default: true)
  "anchor": "a-1234567", // Optional: filter by anchor
  "days": 7              // Days of completed tasks (default: 7)
}
```

### shape_task_start

Mark a task as in progress.

```json
{
  "id": "a-1234567.1"    // Required: task ID
}
```

### shape_task_done

Mark a task as complete.

```json
{
  "id": "a-1234567.1"    // Required: task ID
}
```

### shape_task_add

Create a new task.

```json
{
  "parent": "a-1234567", // Required: anchor ID or task ID for subtasks
  "title": "Implement feature X"  // Required: task title
}
```

### shape_anchor_list

List all anchors in the project.

```json
{
  "status": "in_progress"  // Optional: filter by status
}
```

### shape_anchor_show

Show detailed anchor information.

```json
{
  "id": "a-1234567"  // Required: anchor ID
}
```

## How It Works

The MCP server is a thin wrapper around the Shape CLI. Each tool call:

1. Executes the corresponding `shape` command with `--format json`
2. Parses the JSON output
3. Returns the result to the MCP client

This design means:

- **Zero state**: Each call is independent
- **Low overhead**: Just CLI execution
- **Always in sync**: Uses the same data as direct CLI use

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test

# Lint
npm run lint
```

### Testing Locally with Claude Code

Before publishing to npm, you can test the MCP server locally:

1. Build the project:
   ```bash
   cd shape-mcp
   npm install && npm run build
   ```

2. Configure Claude Code to use the local build. Add to `~/.claude/claude_desktop_config.json`:
   ```json
   {
     "mcpServers": {
       "shape": {
         "command": "node",
         "args": ["/absolute/path/to/shape-mcp/dist/index.js"]
       }
     }
   }
   ```

3. Restart Claude Code to pick up the new MCP server.

4. In a Shape project directory, test the tools:
   - Ask Claude to "check shape status"
   - Ask Claude to "show ready tasks"

### Testing Locally with Cursor

1. Build the project as above.

2. Add to `.cursor/mcp.json` in your project:
   ```json
   {
     "servers": {
       "shape": {
         "command": "node",
         "args": ["/absolute/path/to/shape-mcp/dist/index.js"]
       }
     }
   }
   ```

3. Restart Cursor to load the MCP server.

## License

MIT
