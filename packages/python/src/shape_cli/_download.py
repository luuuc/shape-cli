"""
Download script for shape-cli binary.

This module handles downloading the correct binary for the current platform
from GitHub releases. Features:
- Retry logic for network resilience
- Mandatory checksum verification for security
"""

import hashlib
import platform
import shutil
import stat
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

GITHUB_REPO = "shape-cli/shape"
MAX_RETRIES = 3
RETRY_DELAY_SECONDS = 2
BINARY_NAME = "shape.exe" if platform.system() == "Windows" else "shape"


class DownloadError(Exception):
    """Raised when download fails after all retries."""
    pass


class ChecksumError(Exception):
    """Raised when checksum verification fails."""
    pass


def get_platform_info():
    """Get the current platform and architecture for download."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "darwin":
        os_name = "darwin"
    elif system == "linux":
        os_name = "linux"
    elif system == "windows":
        os_name = "windows"
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")

    if machine in ("x86_64", "amd64"):
        arch = "x64"
    elif machine in ("arm64", "aarch64"):
        arch = "arm64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return os_name, arch


def get_download_url(version):
    """Get the download URL for the current platform."""
    os_name, arch = get_platform_info()
    platform_name = f"{os_name}-{arch}"
    ext = "zip" if os_name == "windows" else "tar.gz"

    return (
        f"https://github.com/{GITHUB_REPO}/releases/download/"
        f"v{version}/shape-{platform_name}.{ext}"
    )


def get_checksum_url(version):
    """Get the checksum file URL."""
    return (
        f"https://github.com/{GITHUB_REPO}/releases/download/"
        f"v{version}/checksums.txt"
    )


def download_with_retry(url, dest_path):
    """Download a file from a URL with retry logic."""
    last_error = None

    for attempt in range(1, MAX_RETRIES + 1):
        try:
            print(f"Downloading {url} (attempt {attempt}/{MAX_RETRIES})...")
            urllib.request.urlretrieve(url, dest_path)
            return
        except urllib.error.HTTPError as e:
            last_error = e
            if e.code == 404:
                raise DownloadError(f"Release not found: {url}") from e
            print(f"HTTP error {e.code}: {e.reason}")
        except urllib.error.URLError as e:
            last_error = e
            print(f"Network error: {e.reason}")
        except Exception as e:
            last_error = e
            print(f"Download error: {e}")

        if attempt < MAX_RETRIES:
            print(f"Retrying in {RETRY_DELAY_SECONDS} seconds...")
            time.sleep(RETRY_DELAY_SECONDS)

    raise DownloadError(
        f"Failed to download {url} after {MAX_RETRIES} attempts"
    ) from last_error


def verify_checksum(file_path, checksums_content, expected_filename):
    """
    Verify the SHA256 checksum of a file.

    Raises ChecksumError if verification fails.
    """
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha256_hash.update(chunk)
    actual_hash = sha256_hash.hexdigest().lower()

    # Parse checksums file (format: "hash  filename" or "hash filename")
    for line in checksums_content.decode("utf-8").splitlines():
        line = line.strip()
        if not line:
            continue

        # Handle both "hash  filename" and "hash filename" formats
        parts = line.split()
        if len(parts) >= 2 and expected_filename in parts[-1]:
            expected_hash = parts[0].lower()
            if actual_hash == expected_hash:
                print(f"Checksum verified: {expected_filename}")
                return
            raise ChecksumError(
                f"Checksum mismatch for {expected_filename}:\n"
                f"  Expected: {expected_hash}\n"
                f"  Actual:   {actual_hash}"
            )

    raise ChecksumError(f"No checksum found for {expected_filename} in checksums.txt")


def extract_binary(archive_path, dest_dir):
    """Extract the binary from the archive."""
    os_name, _ = get_platform_info()

    if os_name == "windows":
        with zipfile.ZipFile(archive_path, "r") as zf:
            zf.extractall(dest_dir)
    else:
        with tarfile.open(archive_path, "r:gz") as tf:
            tf.extractall(dest_dir)

    binary_path = dest_dir / BINARY_NAME
    if not binary_path.exists():
        raise RuntimeError(f"Binary not found in archive: {binary_path}")

    return binary_path


def install_binary(version):
    """Download and install the shape binary."""
    # Determine installation directory
    package_dir = Path(__file__).parent
    bin_dir = package_dir / "bin"
    bin_dir.mkdir(exist_ok=True)

    dest_binary = bin_dir / BINARY_NAME

    # Skip if already installed
    if dest_binary.exists():
        return str(dest_binary)

    os_name, arch = get_platform_info()
    platform_name = f"{os_name}-{arch}"
    ext = "zip" if os_name == "windows" else "tar.gz"
    archive_name = f"shape-{platform_name}.{ext}"

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Download archive with retry
        archive_path = tmpdir / archive_name
        download_url = get_download_url(version)
        download_with_retry(download_url, archive_path)

        # Download and verify checksum (mandatory)
        checksums_url = get_checksum_url(version)
        checksums_path = tmpdir / "checksums.txt"
        download_with_retry(checksums_url, checksums_path)

        with open(checksums_path, "rb") as f:
            checksums_content = f.read()

        verify_checksum(archive_path, checksums_content, archive_name)

        # Extract binary
        extract_dir = tmpdir / "extract"
        extract_dir.mkdir()
        binary_path = extract_binary(archive_path, extract_dir)

        # Move to final destination
        shutil.move(str(binary_path), str(dest_binary))

        # Make executable (Unix only)
        if os_name != "windows":
            dest_binary.chmod(dest_binary.stat().st_mode | stat.S_IEXEC)

    print(f"Installed shape binary to {dest_binary}")
    return str(dest_binary)


if __name__ == "__main__":
    version = sys.argv[1] if len(sys.argv) > 1 else "0.0.0"
    install_binary(version)
