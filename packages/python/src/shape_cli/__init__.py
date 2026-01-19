"""
shape-cli: A local-first task management CLI for software teams.

This package provides a Python wrapper for the shape-cli binary.
The binary is downloaded on first use based on your platform.
"""

import os
import platform
import subprocess
import sys
from pathlib import Path

__version__ = "0.0.0"

GITHUB_REPO = "shape-cli/shape"
BINARY_NAME = "shape.exe" if platform.system() == "Windows" else "shape"


def get_platform_info():
    """Get the current platform and architecture."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    # Normalize platform names
    if system == "darwin":
        os_name = "darwin"
    elif system == "linux":
        os_name = "linux"
    elif system == "windows":
        os_name = "windows"
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")

    # Normalize architecture names
    if machine in ("x86_64", "amd64"):
        arch = "x64"
    elif machine in ("arm64", "aarch64"):
        arch = "arm64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return os_name, arch


def get_binary_dir():
    """Get the directory where the binary should be stored."""
    return Path(__file__).parent / "bin"


def get_binary_path():
    """Get the path to the shape binary, downloading if necessary."""
    binary_dir = get_binary_dir()
    binary_path = binary_dir / BINARY_NAME

    if not binary_path.exists():
        from ._download import install_binary
        install_binary(__version__)

    if not binary_path.exists():
        raise FileNotFoundError(
            "shape binary not found after download attempt. "
            "Try reinstalling: pip install --force-reinstall shape-cli"
        )

    return str(binary_path)


def main():
    """Entry point for the shape command."""
    try:
        binary_path = get_binary_path()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    # Make sure the binary is executable (Unix only)
    if platform.system() != "Windows":
        os.chmod(binary_path, 0o755)

    # Execute the binary with the provided arguments
    try:
        result = subprocess.run(
            [binary_path] + sys.argv[1:],
            env=os.environ,
        )
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)
    except Exception as e:
        print(f"Error executing shape: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
