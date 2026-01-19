# Plugins

Shape CLI is extensible through plugins. This guide covers using and developing plugins.

## Overview

Plugins communicate with Shape via JSON over stdin/stdout. This means you can write plugins in any language.

**Plugin types:**

| Type | Purpose | Binary naming |
|------|---------|---------------|
| **Brief Type** | Custom document templates | `shape-brief-<name>` |
| **Sync** | Bidirectional sync with external tools | `shape-sync-<name>` |

## Using Plugins

### List Available Plugins

```bash
shape advanced plugin list
```

### Show Plugin Details

```bash
shape advanced plugin show shape-sync-github
```

### Plugin Discovery

Shape finds plugins in:
1. `.shape/plugins/` in the project directory
2. Directories in `$PATH`

Plugins must:
- Be executable
- Follow naming convention: `shape-{type}-{name}`
- Support `--manifest` flag for capability discovery

## Brief Type Plugins

Brief type plugins define custom document templates.

### Built-in Types

- `minimal` — Basic title and status
- `shapeup` — Full ShapeUp pitch template

### Creating a Brief Type Plugin

A brief type plugin must support these operations:

#### Manifest

```bash
shape-brief-rfc --manifest
```

Response:

```json
{
  "name": "rfc",
  "version": "1.0.0",
  "description": "RFC (Request for Comments) brief type",
  "operations": ["template", "validate"]
}
```

#### Template

Returns the markdown template for new briefs.

Request (via stdin):

```json
{
  "operation": "template",
  "params": {
    "title": "API Versioning Strategy"
  }
}
```

Response:

```json
{
  "success": true,
  "template": "---\nid: {{id}}\ntitle: {{title}}\nstatus: proposed\ntype: rfc\n---\n\n# {{title}}\n\n## Summary\n\nBrief description of the proposal.\n\n## Motivation\n\nWhy are we doing this?\n\n## Detailed Design\n\nExplain the design in detail.\n\n## Alternatives Considered\n\nWhat other approaches were considered?\n\n## Unresolved Questions\n\nWhat is still TBD?\n"
}
```

#### Validate

Validates brief content against type-specific rules.

Request:

```json
{
  "operation": "validate",
  "params": {
    "content": "---\nid: b-123\ntitle: Test\nstatus: proposed\ntype: rfc\n---\n\n# Test\n\n## Summary\n\nA test RFC.\n"
  }
}
```

Response:

```json
{
  "success": true,
  "valid": true,
  "errors": []
}
```

Or with errors:

```json
{
  "success": true,
  "valid": false,
  "errors": [
    "Missing required section: Motivation",
    "Missing required section: Detailed Design"
  ]
}
```

### Example: RFC Plugin (Python)

```python
#!/usr/bin/env python3
import json
import sys

MANIFEST = {
    "name": "rfc",
    "version": "1.0.0",
    "description": "RFC brief type",
    "operations": ["template", "validate"]
}

TEMPLATE = """---
id: {{id}}
title: {{title}}
status: proposed
type: rfc
created: {{created}}
---

# {{title}}

## Summary

Brief description of the proposal.

## Motivation

Why are we doing this?

## Detailed Design

Explain the design in detail.

## Alternatives Considered

What other approaches were considered?

## Unresolved Questions

What is still TBD?
"""

REQUIRED_SECTIONS = ["Summary", "Motivation", "Detailed Design"]

def handle_template(params):
    return {"success": True, "template": TEMPLATE}

def handle_validate(params):
    content = params.get("content", "")
    errors = []
    for section in REQUIRED_SECTIONS:
        if f"## {section}" not in content:
            errors.append(f"Missing required section: {section}")
    return {
        "success": True,
        "valid": len(errors) == 0,
        "errors": errors
    }

def main():
    if "--manifest" in sys.argv:
        print(json.dumps(MANIFEST))
        return

    request = json.loads(sys.stdin.read())
    operation = request.get("operation")
    params = request.get("params", {})

    if operation == "template":
        response = handle_template(params)
    elif operation == "validate":
        response = handle_validate(params)
    else:
        response = {"success": False, "error": f"Unknown operation: {operation}"}

    print(json.dumps(response))

if __name__ == "__main__":
    main()
```

Save as `.shape/plugins/shape-brief-rfc`, make executable, then:

```bash
shape brief new "API Versioning" --type rfc
```

## Sync Plugins

Sync plugins enable bidirectional sync with external tools.

### Running Sync

```bash
shape advanced sync run github
shape advanced sync status
```

### Creating a Sync Plugin

A sync plugin must support these operations:

#### Manifest

```bash
shape-sync-github --manifest
```

Response:

```json
{
  "name": "github",
  "version": "1.0.0",
  "description": "Sync with GitHub Issues",
  "operations": ["push", "pull", "status"],
  "config_schema": {
    "repo": {"type": "string", "required": true},
    "token": {"type": "string", "required": true, "env": "GITHUB_TOKEN"}
  }
}
```

#### Push

Push local changes to external tool.

Request:

```json
{
  "operation": "push",
  "params": {
    "briefs": [...],
    "tasks": [...],
    "config": {
      "repo": "owner/repo",
      "token": "ghp_..."
    }
  }
}
```

Response:

```json
{
  "success": true,
  "pushed": {
    "briefs": 2,
    "tasks": 5
  },
  "mappings": [
    {"local": "b-7f2a3b1", "remote": "123"},
    {"local": "b-7f2a3b1.1", "remote": "124"}
  ]
}
```

#### Pull

Pull changes from external tool.

Request:

```json
{
  "operation": "pull",
  "params": {
    "config": {
      "repo": "owner/repo",
      "token": "ghp_..."
    },
    "since": "2025-01-15T00:00:00Z"
  }
}
```

Response:

```json
{
  "success": true,
  "briefs": [...],
  "tasks": [...],
  "mappings": [...]
}
```

#### Status

Check sync status.

Request:

```json
{
  "operation": "status",
  "params": {
    "config": {...}
  }
}
```

Response:

```json
{
  "success": true,
  "connected": true,
  "last_sync": "2025-01-16T10:30:00Z",
  "pending_push": 2,
  "pending_pull": 0
}
```

### Sync State

Shape stores sync state in `.shape/sync/`:

```
.shape/
├── sync/
│   ├── github.jsonl    # ID mappings for GitHub
│   └── linear.jsonl    # ID mappings for Linear
```

These files map local IDs to remote IDs and are git-ignored by default.

## Plugin Configuration

Configure plugins in `.shape/config.toml`:

```toml
[plugins.sync.github]
repo = "owner/repo"
# token read from GITHUB_TOKEN env var

[plugins.sync.linear]
team = "ENG"
# api_key read from LINEAR_API_KEY env var

[plugins.brief.rfc]
require_motivation = true
```

## Error Handling

Plugins should return errors in a consistent format:

```json
{
  "success": false,
  "error": "Authentication failed: invalid token",
  "code": "AUTH_ERROR"
}
```

Error codes help Shape handle errors appropriately:
- `AUTH_ERROR` — Prompt for credentials
- `RATE_LIMIT` — Retry with backoff
- `NOT_FOUND` — Resource doesn't exist
- `VALIDATION` — Invalid input

## Testing Plugins

```bash
# Test plugin discovery
shape advanced plugin list

# Test plugin manifest
shape advanced plugin show shape-sync-github

# Test connectivity (for sync plugins)
shape advanced sync status

# Dry run
shape advanced sync run github --dry-run
```

## Plugin Ideas

**Brief Types:**
- `adr` — Architecture Decision Records
- `epic` — Agile epics with acceptance criteria
- `bug` — Bug report template
- `spike` — Research/investigation template

**Sync:**
- `linear` — Linear.app
- `jira` — Atlassian Jira
- `notion` — Notion databases
- `github` — GitHub Issues
- `gitlab` — GitLab Issues
- `asana` — Asana tasks
