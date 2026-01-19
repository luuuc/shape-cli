/**
 * Tests for Shape CLI wrapper
 *
 * These tests verify the shape wrapper module functions correctly.
 * They require the Shape CLI to be installed and available.
 */

import { execSync } from "child_process";
import { mkdtempSync, rmSync, existsSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";

import {
  execShape,
  checkShapeAvailable,
  getReadyTasks,
  getBlockedTasks,
  getStatus,
  getContext,
  startTask,
  completeTask,
  addTask,
  listAnchors,
  showAnchor,
} from "./shape.js";

// Check if shape CLI is available for testing
function isShapeAvailable(): boolean {
  try {
    execSync("shape --version", { encoding: "utf-8", stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
}

const SHAPE_AVAILABLE = isShapeAvailable();

// Helper to create a temp Shape project
function createTempProject(): string {
  const dir = mkdtempSync(join(tmpdir(), "shape-mcp-test-"));
  execSync("shape init", { cwd: dir, encoding: "utf-8", stdio: "pipe" });
  return dir;
}

// Helper to cleanup temp project
function cleanupTempProject(dir: string): void {
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
  }
}

describe("Shape CLI wrapper", () => {
  // Skip all tests if shape is not available
  const describeIfShape = SHAPE_AVAILABLE ? describe : describe.skip;

  describeIfShape("when shape CLI is available", () => {
    let tempDir: string;

    beforeEach(() => {
      tempDir = createTempProject();
    });

    afterEach(() => {
      cleanupTempProject(tempDir);
    });

    describe("checkShapeAvailable", () => {
      it("returns success when shape and .shape exist", () => {
        const result = checkShapeAvailable(tempDir);
        expect(result.success).toBe(true);
      });

      it("returns error when .shape directory is missing", () => {
        const emptyDir = mkdtempSync(join(tmpdir(), "shape-mcp-empty-"));
        try {
          const result = checkShapeAvailable(emptyDir);
          expect(result.success).toBe(false);
          expect(result.error).toContain(".shape");
        } finally {
          rmSync(emptyDir, { recursive: true, force: true });
        }
      });
    });

    describe("getStatus", () => {
      it("returns project status", () => {
        const result = getStatus(tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toHaveProperty("anchors");
        expect(result.data).toHaveProperty("tasks");
        expect(result.data?.anchors).toHaveProperty("total");
        expect(result.data?.tasks).toHaveProperty("total");
      });
    });

    describe("listAnchors", () => {
      it("returns empty list for new project", () => {
        const result = listAnchors(undefined, tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toEqual([]);
      });
    });

    describe("getReadyTasks", () => {
      it("returns empty list for new project", () => {
        const result = getReadyTasks(undefined, tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toEqual([]);
      });
    });

    describe("getBlockedTasks", () => {
      it("returns empty list for new project", () => {
        const result = getBlockedTasks(undefined, tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toEqual([]);
      });
    });

    describe("getContext", () => {
      it("returns context in compact mode", () => {
        const result = getContext(true, undefined, 7, tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toHaveProperty("anchors");
        expect(result.data).toHaveProperty("ready");
        expect(result.data).toHaveProperty("in_progress");
        expect(result.data).toHaveProperty("blocked");
      });
    });

    describe("task operations with anchor", () => {
      let anchorId: string;

      beforeEach(() => {
        // Create an anchor first
        const anchorResult = execShape<{ id: string }>(
          'anchor new "Test Anchor"',
          tempDir
        );
        expect(anchorResult.success).toBe(true);
        anchorId = anchorResult.data!.id;
      });

      it("can add a task to an anchor", () => {
        const result = addTask(anchorId, "Test task", tempDir);
        expect(result.success).toBe(true);
        expect(result.data).toHaveProperty("id");
        expect(result.data?.title).toBe("Test task");
      });

      it("can start and complete a task", () => {
        // Add task
        const addResult = addTask(anchorId, "Task to complete", tempDir);
        expect(addResult.success).toBe(true);
        const taskId = addResult.data!.id;

        // Start task
        const startResult = startTask(taskId, tempDir);
        expect(startResult.success).toBe(true);
        expect(startResult.data?.status).toBe("in_progress");

        // Complete task
        const doneResult = completeTask(taskId, tempDir);
        expect(doneResult.success).toBe(true);
        expect(doneResult.data?.status).toBe("done");
      });

      it("can show anchor details", () => {
        const result = showAnchor(anchorId, tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.id).toBe(anchorId);
        expect(result.data?.title).toBe("Test Anchor");
      });

      it("ready tasks includes task without dependencies", () => {
        // Add a task
        const addResult = addTask(anchorId, "Ready task", tempDir);
        expect(addResult.success).toBe(true);

        // Check ready tasks
        const readyResult = getReadyTasks(undefined, tempDir);
        expect(readyResult.success).toBe(true);
        expect(readyResult.data?.length).toBeGreaterThan(0);
      });
    });
  });

  describe("when shape CLI is not available", () => {
    describe("execShape", () => {
      it("returns error for invalid command", () => {
        // Use an invalid command that should fail
        const result = execShape("nonexistent-command-xyz");
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });
    });
  });
});

describe("execShape function", () => {
  it("handles JSON parse errors gracefully", () => {
    // This should fail because echo doesn't produce JSON
    const result = execShape("--help");
    // Help output isn't valid JSON, so it should fail
    expect(result.success).toBe(false);
  });
});

describe("error cases", () => {
  const describeIfShape = isShapeAvailable() ? describe : describe.skip;

  describeIfShape("with shape CLI available", () => {
    let tempDir: string;

    beforeEach(() => {
      tempDir = mkdtempSync(join(tmpdir(), "shape-mcp-error-test-"));
      execSync("shape init", { cwd: tempDir, encoding: "utf-8", stdio: "pipe" });
    });

    afterEach(() => {
      if (existsSync(tempDir)) {
        rmSync(tempDir, { recursive: true, force: true });
      }
    });

    describe("startTask", () => {
      it("returns error for non-existent task ID", () => {
        const result = startTask("a-0000000.999", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });

      it("returns error for invalid task ID format", () => {
        const result = startTask("invalid-id", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });
    });

    describe("completeTask", () => {
      it("returns error for non-existent task ID", () => {
        const result = completeTask("a-0000000.999", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });
    });

    describe("showAnchor", () => {
      it("returns error for non-existent anchor ID", () => {
        const result = showAnchor("a-0000000", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });

      it("returns error for invalid anchor ID format", () => {
        const result = showAnchor("invalid", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });
    });

    describe("addTask", () => {
      it("returns error for non-existent parent anchor", () => {
        const result = addTask("a-0000000", "Test task", tempDir);
        expect(result.success).toBe(false);
        expect(result.error).toBeDefined();
      });
    });

    describe("addTask shell escaping", () => {
      let anchorId: string;

      beforeEach(() => {
        const anchorResult = execShape<{ id: string }>(
          'anchor new "Shell Test"',
          tempDir
        );
        anchorId = anchorResult.data!.id;
      });

      it("handles titles with single quotes", () => {
        const result = addTask(anchorId, "Don't break", tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.title).toBe("Don't break");
      });

      it("handles titles with double quotes", () => {
        const result = addTask(anchorId, 'Say "hello"', tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.title).toBe('Say "hello"');
      });

      it("handles titles with backticks", () => {
        const result = addTask(anchorId, "Use `code` here", tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.title).toBe("Use `code` here");
      });

      it("handles titles with dollar signs", () => {
        const result = addTask(anchorId, "Cost is $100", tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.title).toBe("Cost is $100");
      });

      it("handles titles with special shell characters", () => {
        const result = addTask(anchorId, "Test & verify | check", tempDir);
        expect(result.success).toBe(true);
        expect(result.data?.title).toBe("Test & verify | check");
      });
    });
  });
});
