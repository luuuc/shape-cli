/**
 * Shape CLI wrapper - executes shape commands and returns structured results
 */

import { execSync, type ExecSyncOptions } from "child_process";

export interface ShapeResult<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface ReadyTask {
  id: string;
  title: string;
  anchor_id: string;
}

export interface BlockedTask {
  id: string;
  title: string;
  blocked_by: string[];
}

export interface StatusResult {
  anchors: {
    total: number;
    active: number;
    complete: number;
  };
  tasks: {
    total: number;
    todo: number;
    in_progress: number;
    done: number;
    ready: number;
    blocked: number;
  };
}

export interface ContextResult {
  anchors: Array<{
    id: string;
    title: string;
    status: string;
  }>;
  ready: string[];
  in_progress: string[];
  blocked: string[];
  recently_done: string[];
}

export interface TaskResult {
  id: string;
  status: string;
  completed_at?: string;
}

export interface TaskAddResult {
  id: string;
  title: string;
  status: string;
}

export interface Anchor {
  id: string;
  title: string;
  status: string;
  type?: string;
  body?: string;
  meta?: Record<string, unknown>;
  tasks?: Array<{ id: string; title: string; status: string }>;
}

/**
 * Execute a shape CLI command and return structured result
 */
export function execShape<T>(command: string, cwd?: string): ShapeResult<T> {
  const options: ExecSyncOptions = {
    encoding: "utf-8",
    maxBuffer: 10 * 1024 * 1024, // 10MB buffer
    timeout: 5000, // 5 second timeout - local CLI should be fast
  };

  if (cwd) {
    options.cwd = cwd;
  }

  try {
    const output = execSync(`shape ${command} --format json`, options);
    const data = JSON.parse(output as string) as T;
    return { success: true, data };
  } catch (error) {
    const err = error as Error & { stderr?: Buffer; stdout?: Buffer };

    // Try to parse error message
    let errorMessage = err.message;
    if (err.stderr) {
      errorMessage = err.stderr.toString().trim() || errorMessage;
    }

    return { success: false, error: errorMessage };
  }
}

/**
 * Check if shape CLI is available and .shape directory exists
 */
export function checkShapeAvailable(cwd?: string): ShapeResult<{ version: string }> {
  try {
    // Check if shape command exists
    const options: ExecSyncOptions = {
      encoding: "utf-8",
      timeout: 5000,
    };
    if (cwd) {
      options.cwd = cwd;
    }

    execSync("shape --version", options);

    // Check if .shape directory exists by trying status
    const statusResult = execShape<StatusResult>("status", cwd);
    if (!statusResult.success) {
      return {
        success: false,
        error: "No .shape directory found. Run 'shape init' to create a project.",
      };
    }

    return { success: true, data: { version: "0.1.0" } };
  } catch {
    return {
      success: false,
      error: "Shape CLI not found. Install it from https://github.com/shape-cli/shape",
    };
  }
}

// Tool implementations

export function getReadyTasks(anchor?: string, cwd?: string): ShapeResult<ReadyTask[]> {
  const anchorArg = anchor ? ` --anchor ${anchor}` : "";
  return execShape<ReadyTask[]>(`ready${anchorArg}`, cwd);
}

export function getBlockedTasks(anchor?: string, cwd?: string): ShapeResult<BlockedTask[]> {
  const anchorArg = anchor ? ` --anchor ${anchor}` : "";
  return execShape<BlockedTask[]>(`blocked${anchorArg}`, cwd);
}

export function getStatus(cwd?: string): ShapeResult<StatusResult> {
  return execShape<StatusResult>("status", cwd);
}

export function getContext(compact: boolean = true, anchor?: string, days: number = 7, cwd?: string): ShapeResult<ContextResult> {
  let args = compact ? "--compact" : "";
  if (anchor) {
    args += ` --anchor ${anchor}`;
  }
  args += ` --days ${days}`;
  return execShape<ContextResult>(`context ${args}`.trim(), cwd);
}

export function startTask(id: string, cwd?: string): ShapeResult<TaskResult> {
  return execShape<TaskResult>(`task start ${id}`, cwd);
}

export function completeTask(id: string, cwd?: string): ShapeResult<TaskResult> {
  return execShape<TaskResult>(`task done ${id}`, cwd);
}

/**
 * Escape a string for safe use in shell single quotes.
 * Single quotes prevent all shell interpretation except for the quote itself.
 */
function shellEscape(str: string): string {
  // Replace single quotes with: end quote, escaped single quote, start quote
  // 'don't' becomes 'don'\''t'
  return "'" + str.replace(/'/g, "'\\''") + "'";
}

export function addTask(parent: string, title: string, cwd?: string): ShapeResult<TaskAddResult> {
  const escapedTitle = shellEscape(title);
  return execShape<TaskAddResult>(`task add ${parent} ${escapedTitle}`, cwd);
}

export function listAnchors(status?: string, cwd?: string): ShapeResult<Anchor[]> {
  const statusArg = status ? ` --status ${status}` : "";
  return execShape<Anchor[]>(`anchor list${statusArg}`, cwd);
}

export function showAnchor(id: string, cwd?: string): ShapeResult<Anchor> {
  return execShape<Anchor>(`anchor show ${id}`, cwd);
}
