"""Tests for shape_cli download functionality."""

import hashlib
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

from shape_cli._download import (
    ChecksumError,
    DownloadError,
    get_download_url,
    get_platform_info,
    verify_checksum,
)


class TestGetPlatformInfo:
    """Tests for platform detection."""

    @patch("platform.system")
    @patch("platform.machine")
    def test_darwin_arm64(self, mock_machine, mock_system):
        mock_system.return_value = "Darwin"
        mock_machine.return_value = "arm64"
        assert get_platform_info() == ("darwin", "arm64")

    @patch("platform.system")
    @patch("platform.machine")
    def test_darwin_x64(self, mock_machine, mock_system):
        mock_system.return_value = "Darwin"
        mock_machine.return_value = "x86_64"
        assert get_platform_info() == ("darwin", "x64")

    @patch("platform.system")
    @patch("platform.machine")
    def test_linux_x64(self, mock_machine, mock_system):
        mock_system.return_value = "Linux"
        mock_machine.return_value = "x86_64"
        assert get_platform_info() == ("linux", "x64")

    @patch("platform.system")
    @patch("platform.machine")
    def test_linux_arm64(self, mock_machine, mock_system):
        mock_system.return_value = "Linux"
        mock_machine.return_value = "aarch64"
        assert get_platform_info() == ("linux", "arm64")

    @patch("platform.system")
    @patch("platform.machine")
    def test_windows_x64(self, mock_machine, mock_system):
        mock_system.return_value = "Windows"
        mock_machine.return_value = "AMD64"
        assert get_platform_info() == ("windows", "x64")

    @patch("platform.system")
    @patch("platform.machine")
    def test_unsupported_os(self, mock_machine, mock_system):
        mock_system.return_value = "FreeBSD"
        mock_machine.return_value = "x86_64"
        with pytest.raises(RuntimeError, match="Unsupported operating system"):
            get_platform_info()

    @patch("platform.system")
    @patch("platform.machine")
    def test_unsupported_arch(self, mock_machine, mock_system):
        mock_system.return_value = "Linux"
        mock_machine.return_value = "riscv64"
        with pytest.raises(RuntimeError, match="Unsupported architecture"):
            get_platform_info()


class TestGetDownloadUrl:
    """Tests for download URL generation."""

    @patch("shape_cli._download.get_platform_info")
    def test_url_format_unix(self, mock_platform):
        mock_platform.return_value = ("linux", "x64")
        url = get_download_url("1.2.3")
        assert url == "https://github.com/shape-cli/shape/releases/download/v1.2.3/shape-linux-x64.tar.gz"

    @patch("shape_cli._download.get_platform_info")
    def test_url_format_windows(self, mock_platform):
        mock_platform.return_value = ("windows", "x64")
        url = get_download_url("1.2.3")
        assert url == "https://github.com/shape-cli/shape/releases/download/v1.2.3/shape-windows-x64.zip"


class TestVerifyChecksum:
    """Tests for checksum verification."""

    def test_valid_checksum(self):
        """Test that valid checksum passes."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"test content")
            f.flush()
            file_path = Path(f.name)

        try:
            expected_hash = hashlib.sha256(b"test content").hexdigest()
            checksums = f"{expected_hash}  test-file.tar.gz\n".encode()
            # Should not raise
            verify_checksum(file_path, checksums, "test-file.tar.gz")
        finally:
            file_path.unlink()

    def test_invalid_checksum(self):
        """Test that invalid checksum raises error."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"test content")
            f.flush()
            file_path = Path(f.name)

        try:
            wrong_hash = "a" * 64
            checksums = f"{wrong_hash}  test-file.tar.gz\n".encode()
            with pytest.raises(ChecksumError, match="Checksum mismatch"):
                verify_checksum(file_path, checksums, "test-file.tar.gz")
        finally:
            file_path.unlink()

    def test_missing_checksum(self):
        """Test that missing checksum raises error."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"test content")
            f.flush()
            file_path = Path(f.name)

        try:
            checksums = b"abc123  other-file.tar.gz\n"
            with pytest.raises(ChecksumError, match="No checksum found"):
                verify_checksum(file_path, checksums, "test-file.tar.gz")
        finally:
            file_path.unlink()

    def test_checksum_format_with_single_space(self):
        """Test checksum parsing with single space separator."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"test content")
            f.flush()
            file_path = Path(f.name)

        try:
            expected_hash = hashlib.sha256(b"test content").hexdigest()
            # Single space instead of double
            checksums = f"{expected_hash} test-file.tar.gz\n".encode()
            verify_checksum(file_path, checksums, "test-file.tar.gz")
        finally:
            file_path.unlink()

    def test_checksum_case_insensitive(self):
        """Test that checksum comparison is case-insensitive."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"test content")
            f.flush()
            file_path = Path(f.name)

        try:
            expected_hash = hashlib.sha256(b"test content").hexdigest().upper()
            checksums = f"{expected_hash}  test-file.tar.gz\n".encode()
            verify_checksum(file_path, checksums, "test-file.tar.gz")
        finally:
            file_path.unlink()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
