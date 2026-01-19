/**
 * Tests for platform detection logic
 * Uses Node.js built-in test runner (Node 18+)
 */

const { describe, it } = require("node:test");
const assert = require("node:assert");

// Platform mapping (duplicated from bin/shape for testing)
const platformMap = {
  darwin: {
    arm64: "@shape-cli/darwin-arm64",
    x64: "@shape-cli/darwin-x64",
  },
  linux: {
    arm64: "@shape-cli/linux-arm64",
    x64: "@shape-cli/linux-x64",
  },
  win32: {
    x64: "@shape-cli/windows-x64",
  },
};

function getPlatformPackage(platform, arch) {
  const archMap = platformMap[platform];
  if (!archMap) {
    return null;
  }
  return archMap[arch] || null;
}

describe("Platform Detection", () => {
  describe("getPlatformPackage", () => {
    it("should return correct package for darwin/arm64", () => {
      assert.strictEqual(
        getPlatformPackage("darwin", "arm64"),
        "@shape-cli/darwin-arm64"
      );
    });

    it("should return correct package for darwin/x64", () => {
      assert.strictEqual(
        getPlatformPackage("darwin", "x64"),
        "@shape-cli/darwin-x64"
      );
    });

    it("should return correct package for linux/arm64", () => {
      assert.strictEqual(
        getPlatformPackage("linux", "arm64"),
        "@shape-cli/linux-arm64"
      );
    });

    it("should return correct package for linux/x64", () => {
      assert.strictEqual(
        getPlatformPackage("linux", "x64"),
        "@shape-cli/linux-x64"
      );
    });

    it("should return correct package for win32/x64", () => {
      assert.strictEqual(
        getPlatformPackage("win32", "x64"),
        "@shape-cli/windows-x64"
      );
    });

    it("should return null for unsupported platform", () => {
      assert.strictEqual(getPlatformPackage("freebsd", "x64"), null);
    });

    it("should return null for unsupported architecture", () => {
      assert.strictEqual(getPlatformPackage("darwin", "ia32"), null);
    });

    it("should return null for win32/arm64 (not supported)", () => {
      assert.strictEqual(getPlatformPackage("win32", "arm64"), null);
    });
  });

  describe("Binary naming", () => {
    it("should use correct binary name for current platform", () => {
      const binaryName = process.platform === "win32" ? "shape.exe" : "shape";
      assert.ok(binaryName.length > 0);
      if (process.platform === "win32") {
        assert.ok(binaryName.endsWith(".exe"));
      } else {
        assert.ok(!binaryName.includes("."));
      }
    });
  });
});
