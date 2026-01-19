#!/usr/bin/env node

/**
 * Post-install script that verifies the binary is available.
 * The actual binary is provided by platform-specific optional dependencies.
 */

const fs = require("fs");
const path = require("path");

const platform = process.platform;
const arch = process.arch;

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

function getPlatformPackage() {
  const archMap = platformMap[platform];
  if (!archMap) {
    return null;
  }
  return archMap[arch] || null;
}

function verifyInstallation() {
  const packageName = getPlatformPackage();

  if (!packageName) {
    console.warn(`Warning: Unsupported platform (${platform}/${arch})`);
    console.warn("shape-cli may not work correctly on this platform.");
    console.warn("Supported platforms: macOS, Linux, Windows (x64 and arm64)");
    return false;
  }

  try {
    const packagePath = require.resolve(`${packageName}/package.json`);
    const packageDir = path.dirname(packagePath);
    const binaryName = platform === "win32" ? "shape.exe" : "shape";
    const binaryPath = path.join(packageDir, "bin", binaryName);

    if (!fs.existsSync(binaryPath)) {
      console.warn(`Warning: Binary not found at ${binaryPath}`);
      console.warn("The platform package was installed but the binary is missing.");
      console.warn("Try reinstalling: npm install shape-cli");
      return false;
    }

    // Verify binary is executable (Unix only)
    if (platform !== "win32") {
      try {
        fs.accessSync(binaryPath, fs.constants.X_OK);
      } catch {
        // Try to make it executable
        try {
          fs.chmodSync(binaryPath, 0o755);
        } catch (e) {
          console.warn(`Warning: Could not make binary executable: ${e.message}`);
        }
      }
    }

    return true;
  } catch (e) {
    // Optional dependency wasn't installed
    if (process.env.npm_config_optional === "false") {
      // User explicitly disabled optional deps
      console.warn("Warning: Optional dependencies disabled.");
      console.warn("shape-cli requires platform-specific binaries.");
      console.warn("Re-run without --no-optional flag.");
    } else {
      console.warn(`Warning: Platform package ${packageName} not installed.`);
      console.warn("This may happen during development or in unsupported environments.");
      console.warn("Try reinstalling: npm install shape-cli");
    }
    return false;
  }
}

// Run verification
const success = verifyInstallation();

if (success) {
  // Silent success - no output on successful install
  process.exit(0);
} else {
  // Warnings were printed, but don't fail the install
  // The main bin/shape script will provide a better error at runtime
  process.exit(0);
}
