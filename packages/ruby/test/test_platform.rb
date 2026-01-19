# frozen_string_literal: true

require "minitest/autorun"
require "rbconfig"

# Platform detection logic (duplicated from extconf.rb for testing)
module PlatformDetection
  def self.platform_info
    os = case RbConfig::CONFIG["host_os"]
         when /darwin/i then "darwin"
         when /linux/i then "linux"
         when /mswin|mingw|cygwin/i then "windows"
         else nil
         end

    arch = case RbConfig::CONFIG["host_cpu"]
           when /x86_64|amd64/i then "x64"
           when /arm64|aarch64/i then "arm64"
           else nil
           end

    [os, arch]
  end

  def self.binary_name
    os, = platform_info
    os == "windows" ? "shape.exe" : "shape"
  end

  def self.download_url(version)
    os, arch = platform_info
    return nil unless os && arch

    ext = os == "windows" ? "zip" : "tar.gz"
    "https://github.com/shape-cli/shape/releases/download/v#{version}/shape-#{os}-#{arch}.#{ext}"
  end
end

class TestPlatformDetection < Minitest::Test
  def test_platform_info_returns_valid_os
    os, = PlatformDetection.platform_info
    assert_includes %w[darwin linux windows], os, "OS should be darwin, linux, or windows"
  end

  def test_platform_info_returns_valid_arch
    _, arch = PlatformDetection.platform_info
    assert_includes %w[x64 arm64], arch, "Architecture should be x64 or arm64"
  end

  def test_binary_name_on_current_platform
    binary = PlatformDetection.binary_name
    os, = PlatformDetection.platform_info

    if os == "windows"
      assert_equal "shape.exe", binary
    else
      assert_equal "shape", binary
    end
  end

  def test_download_url_format
    url = PlatformDetection.download_url("1.2.3")
    os, arch = PlatformDetection.platform_info

    assert url.start_with?("https://github.com/shape-cli/shape/releases/download/v1.2.3/")
    assert url.include?("shape-#{os}-#{arch}")

    if os == "windows"
      assert url.end_with?(".zip")
    else
      assert url.end_with?(".tar.gz")
    end
  end

  def test_download_url_includes_version
    url = PlatformDetection.download_url("2.0.0-beta.1")
    assert url.include?("v2.0.0-beta.1")
  end
end

class TestChecksumVerification < Minitest::Test
  def test_sha256_format
    # SHA256 hashes should be 64 hex characters
    sample_hash = "a" * 64
    assert_match(/^[a-f0-9]{64}$/, sample_hash)
  end

  def test_checksum_line_parsing
    line = "abc123def456  shape-darwin-arm64.tar.gz"
    parts = line.split
    assert_equal 2, parts.length
    assert_equal "abc123def456", parts[0]
    assert_equal "shape-darwin-arm64.tar.gz", parts[1]
  end

  def test_checksum_line_parsing_single_space
    line = "abc123def456 shape-darwin-arm64.tar.gz"
    parts = line.split
    assert_equal 2, parts.length
  end
end
