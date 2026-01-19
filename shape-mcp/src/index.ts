#!/usr/bin/env node

/**
 * Shape MCP Server
 *
 * Exposes Shape CLI commands as MCP tools for AI agents.
 * Run with: npx shape-mcp
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import {
  checkShapeAvailable as checkShapeAvailableImpl,
  getReadyTasks,
  getBlockedTasks,
  getStatus,
  getContext,
  startTask,
  completeTask,
  addTask,
  listAnchors,
  showAnchor,
  type ShapeResult,
} from "./shape.js";

// Cache the shape availability check - only check once per session
let shapeAvailabilityCache: ShapeResult<{ version: string }> | null = null;

function getCachedShapeAvailability(): ShapeResult<{ version: string }> {
  if (shapeAvailabilityCache === null) {
    shapeAvailabilityCache = checkShapeAvailableImpl();
  }
  return shapeAvailabilityCache;
}

const server = new Server(
  {
    name: "shape-mcp",
    version: "0.1.0",
  },
  {
    capabilities: {
      resources: {},
      tools: {},
    },
  }
);

// Tool definitions with descriptions for LLM discoverability
const TOOLS = [
  {
    name: "shape_ready",
    description:
      "Get tasks that are ready to work on (no blocking dependencies). Returns task IDs, titles, and anchor IDs. Use this to find what to work on next.",
    inputSchema: {
      type: "object" as const,
      properties: {
        anchor: {
          type: "string",
          description: "Optional anchor ID to filter tasks (e.g., 'a-1234567')",
        },
      },
    },
  },
  {
    name: "shape_blocked",
    description:
      "Get tasks that are blocked by incomplete dependencies. Shows what's blocking each task. Useful for understanding task dependencies.",
    inputSchema: {
      type: "object" as const,
      properties: {
        anchor: {
          type: "string",
          description: "Optional anchor ID to filter tasks",
        },
      },
    },
  },
  {
    name: "shape_status",
    description:
      "Get project overview: total anchors, task counts by status (todo, in_progress, done), and ready/blocked counts. Good for understanding project state.",
    inputSchema: {
      type: "object" as const,
      properties: {},
    },
  },
  {
    name: "shape_context",
    description:
      "Get AI-optimized project context including anchors, ready tasks, in-progress tasks, blocked tasks, and recently completed work. Best for getting full project understanding.",
    inputSchema: {
      type: "object" as const,
      properties: {
        compact: {
          type: "boolean",
          description: "Use compact format for fewer tokens (default: true)",
          default: true,
        },
        anchor: {
          type: "string",
          description: "Optional anchor ID to filter context",
        },
        days: {
          type: "number",
          description: "Days of completed tasks to include (default: 7)",
          default: 7,
        },
      },
    },
  },
  {
    name: "shape_task_start",
    description:
      "Mark a task as in progress. Call this when you begin working on a task. Takes a task ID like 'a-1234567.1'.",
    inputSchema: {
      type: "object" as const,
      properties: {
        id: {
          type: "string",
          description: "Task ID to start (e.g., 'a-1234567.1')",
        },
      },
      required: ["id"],
    },
  },
  {
    name: "shape_task_done",
    description:
      "Mark a task as complete. Call this when you finish a task. Takes a task ID like 'a-1234567.1'.",
    inputSchema: {
      type: "object" as const,
      properties: {
        id: {
          type: "string",
          description: "Task ID to complete (e.g., 'a-1234567.1')",
        },
      },
      required: ["id"],
    },
  },
  {
    name: "shape_task_add",
    description:
      "Create a new task under an anchor or as a subtask. Parent can be an anchor ID ('a-1234567') for top-level tasks or a task ID ('a-1234567.1') for subtasks.",
    inputSchema: {
      type: "object" as const,
      properties: {
        parent: {
          type: "string",
          description: "Parent ID - anchor ID for top-level tasks, task ID for subtasks",
        },
        title: {
          type: "string",
          description: "Task title describing what needs to be done",
        },
      },
      required: ["parent", "title"],
    },
  },
  {
    name: "shape_anchor_list",
    description:
      "List all anchors (pitches, RFCs, etc.) in the project. Can filter by status. Returns anchor IDs, titles, and statuses.",
    inputSchema: {
      type: "object" as const,
      properties: {
        status: {
          type: "string",
          description: "Filter by status: proposed, in_progress, complete, archived",
        },
      },
    },
  },
  {
    name: "shape_anchor_show",
    description:
      "Show detailed information about an anchor including its content, metadata, and associated tasks. Use this to understand what a specific anchor is about.",
    inputSchema: {
      type: "object" as const,
      properties: {
        id: {
          type: "string",
          description: "Anchor ID to show (e.g., 'a-1234567')",
        },
      },
      required: ["id"],
    },
  },
];

// Resource definitions
const RESOURCES = [
  {
    uri: "shape://context",
    name: "Shape Project Context",
    description: "Full project context optimized for AI consumption",
    mimeType: "application/json",
  },
  {
    uri: "shape://ready",
    name: "Ready Tasks",
    description: "Tasks ready to be worked on",
    mimeType: "application/json",
  },
  {
    uri: "shape://status",
    name: "Project Status",
    description: "Project overview with task counts",
    mimeType: "application/json",
  },
];

// Format tool result for MCP response
function formatResult<T>(result: ShapeResult<T>): { content: Array<{ type: "text"; text: string }> } {
  if (result.success) {
    return {
      content: [
        {
          type: "text" as const,
          text: JSON.stringify(result.data, null, 2),
        },
      ],
    };
  } else {
    return {
      content: [
        {
          type: "text" as const,
          text: `Error: ${result.error}`,
        },
      ],
    };
  }
}

// List available tools
server.setRequestHandler(ListToolsRequestSchema, async () => {
  // Check if shape is available first
  const check = getCachedShapeAvailability();
  if (!check.success) {
    // Return tools but they'll fail with helpful error
    return { tools: TOOLS };
  }
  return { tools: TOOLS };
});

// Handle tool calls
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  // Check shape availability
  const check = getCachedShapeAvailability();
  if (!check.success) {
    return formatResult(check);
  }

  switch (name) {
    case "shape_ready": {
      const anchor = (args as { anchor?: string })?.anchor;
      return formatResult(getReadyTasks(anchor));
    }

    case "shape_blocked": {
      const anchor = (args as { anchor?: string })?.anchor;
      return formatResult(getBlockedTasks(anchor));
    }

    case "shape_status": {
      return formatResult(getStatus());
    }

    case "shape_context": {
      const { compact = true, anchor, days = 7 } = args as {
        compact?: boolean;
        anchor?: string;
        days?: number;
      };
      return formatResult(getContext(compact, anchor, days));
    }

    case "shape_task_start": {
      const { id } = args as { id: string };
      if (!id) {
        return formatResult({ success: false, error: "Task ID is required" });
      }
      return formatResult(startTask(id));
    }

    case "shape_task_done": {
      const { id } = args as { id: string };
      if (!id) {
        return formatResult({ success: false, error: "Task ID is required" });
      }
      return formatResult(completeTask(id));
    }

    case "shape_task_add": {
      const { parent, title } = args as { parent: string; title: string };
      if (!parent) {
        return formatResult({ success: false, error: "Parent ID is required" });
      }
      if (!title) {
        return formatResult({ success: false, error: "Task title is required" });
      }
      return formatResult(addTask(parent, title));
    }

    case "shape_anchor_list": {
      const status = (args as { status?: string })?.status;
      return formatResult(listAnchors(status));
    }

    case "shape_anchor_show": {
      const { id } = args as { id: string };
      if (!id) {
        return formatResult({ success: false, error: "Anchor ID is required" });
      }
      return formatResult(showAnchor(id));
    }

    default:
      return formatResult({ success: false, error: `Unknown tool: ${name}` });
  }
});

// List available resources
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  const check = getCachedShapeAvailability();
  if (!check.success) {
    return { resources: [] };
  }
  return { resources: RESOURCES };
});

// Read resource content
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const { uri } = request.params;

  const check = getCachedShapeAvailability();
  if (!check.success) {
    return {
      contents: [
        {
          uri,
          mimeType: "text/plain",
          text: `Error: ${check.error}`,
        },
      ],
    };
  }

  switch (uri) {
    case "shape://context": {
      const result = getContext(true);
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: result.success
              ? JSON.stringify(result.data, null, 2)
              : `Error: ${result.error}`,
          },
        ],
      };
    }

    case "shape://ready": {
      const result = getReadyTasks();
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: result.success
              ? JSON.stringify(result.data, null, 2)
              : `Error: ${result.error}`,
          },
        ],
      };
    }

    case "shape://status": {
      const result = getStatus();
      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: result.success
              ? JSON.stringify(result.data, null, 2)
              : `Error: ${result.error}`,
          },
        ],
      };
    }

    default:
      return {
        contents: [
          {
            uri,
            mimeType: "text/plain",
            text: `Unknown resource: ${uri}`,
          },
        ],
      };
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("Shape MCP server running on stdio");
}

main().catch((error) => {
  console.error("Failed to start server:", error);
  process.exit(1);
});
