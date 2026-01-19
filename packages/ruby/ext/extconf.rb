#!/usr/bin/env ruby
# frozen_string_literal: true

# This extension downloads the shape binary during gem installation.
# It uses the native extension mechanism to hook into gem install.
#
# Features:
# - Retry logic for network resilience
# - Mandatory checksum verification for security

require "fileutils"
require "net/http"
require "uri"
require "digest"
require "rubygems/package"
require "zlib"

GITHUB_REPO = "shape-cli/shape"
MAX_RETRIES = 3
RETRY_DELAY_SECONDS = 2

class DownloadError < StandardError; end
class ChecksumError < StandardError; end

def platform_info
  os = case RbConfig::CONFIG["host_os"]
       when /darwin/i then "darwin"
       when /linux/i then "linux"
       when /mswin|mingw|cygwin/i then "windows"
       else raise "Unsupported OS: #{RbConfig::CONFIG["host_os"]}"
       end

  arch = case RbConfig::CONFIG["host_cpu"]
         when /x86_64|amd64/i then "x64"
         when /arm64|aarch64/i then "arm64"
         else raise "Unsupported architecture: #{RbConfig::CONFIG["host_cpu"]}"
         end

  [os, arch]
end

def download_url(version)
  os, arch = platform_info
  ext = os == "windows" ? "zip" : "tar.gz"
  "https://github.com/#{GITHUB_REPO}/releases/download/v#{version}/shape-#{os}-#{arch}.#{ext}"
end

def checksums_url(version)
  "https://github.com/#{GITHUB_REPO}/releases/download/v#{version}/checksums.txt"
end

def binary_name
  os, = platform_info
  os == "windows" ? "shape.exe" : "shape"
end

def download_file_with_retry(url)
  last_error = nil

  MAX_RETRIES.times do |attempt|
    puts "Downloading #{url} (attempt #{attempt + 1}/#{MAX_RETRIES})..."

    begin
      uri = URI.parse(url)
      response = nil

      # Follow redirects (GitHub releases redirect to CDN)
      5.times do
        http = Net::HTTP.new(uri.host, uri.port)
        http.use_ssl = uri.scheme == "https"
        http.open_timeout = 30
        http.read_timeout = 60

        request = Net::HTTP::Get.new(uri.request_uri)
        response = http.request(request)

        case response
        when Net::HTTPRedirection
          uri = URI.parse(response["location"])
        when Net::HTTPSuccess
          return response.body
        when Net::HTTPNotFound
          raise DownloadError, "Release not found: #{url}"
        else
          raise DownloadError, "HTTP #{response.code}: #{response.message}"
        end
      end

      raise DownloadError, "Too many redirects for #{url}"
    rescue DownloadError
      raise
    rescue StandardError => e
      last_error = e
      puts "Download error: #{e.message}"

      if attempt < MAX_RETRIES - 1
        puts "Retrying in #{RETRY_DELAY_SECONDS} seconds..."
        sleep(RETRY_DELAY_SECONDS)
      end
    end
  end

  raise DownloadError, "Failed to download #{url} after #{MAX_RETRIES} attempts: #{last_error&.message}"
end

def verify_checksum(data, checksums_content, expected_filename)
  actual_hash = Digest::SHA256.hexdigest(data).downcase

  # Parse checksums file (format: "hash  filename" or "hash filename")
  checksums_content.each_line do |line|
    line = line.strip
    next if line.empty?

    parts = line.split
    next unless parts.length >= 2 && parts.last.include?(expected_filename)

    expected_hash = parts.first.downcase
    if actual_hash == expected_hash
      puts "Checksum verified: #{expected_filename}"
      return
    end

    raise ChecksumError, <<~MSG
      Checksum mismatch for #{expected_filename}:
        Expected: #{expected_hash}
        Actual:   #{actual_hash}
    MSG
  end

  raise ChecksumError, "No checksum found for #{expected_filename} in checksums.txt"
end

def extract_binary(archive_data, dest_dir)
  os, = platform_info

  if os == "windows"
    # For Windows, use system unzip if available
    require "tempfile"
    Tempfile.create(["shape", ".zip"]) do |f|
      f.binmode
      f.write(archive_data)
      f.close
      system("unzip", "-o", f.path, "-d", dest_dir) or raise "Failed to extract zip"
    end
  else
    # Extract tar.gz
    require "stringio"
    io = StringIO.new(archive_data)
    gz = Zlib::GzipReader.new(io)
    tar = Gem::Package::TarReader.new(gz)

    tar.each do |entry|
      if entry.file? && entry.full_name == "shape"
        dest_path = File.join(dest_dir, binary_name)
        File.open(dest_path, "wb") do |f|
          f.write(entry.read)
        end
        File.chmod(0o755, dest_path)
      end
    end
  end
end

def install_binary
  # Get version from gemspec
  gemspec_path = File.expand_path("../../shape-cli.gemspec", __FILE__)
  version = File.read(gemspec_path).match(/spec\.version\s*=\s*"([^"]+)"/)[1]

  # Destination directory
  lib_dir = File.expand_path("../../lib/shape_cli", __FILE__)
  bin_dir = File.join(lib_dir, "bin")
  FileUtils.mkdir_p(bin_dir)

  dest_binary = File.join(bin_dir, binary_name)

  # Skip if already exists
  if File.exist?(dest_binary)
    puts "Binary already installed at #{dest_binary}"
    return
  end

  os, arch = platform_info
  ext = os == "windows" ? "zip" : "tar.gz"
  archive_name = "shape-#{os}-#{arch}.#{ext}"

  puts "Downloading shape binary v#{version}..."

  # Download archive with retry
  archive_data = download_file_with_retry(download_url(version))

  # Download checksums with retry (mandatory)
  checksums_content = download_file_with_retry(checksums_url(version))

  # Verify checksum (mandatory - raises on failure)
  verify_checksum(archive_data, checksums_content, archive_name)

  puts "Extracting binary..."
  extract_binary(archive_data, bin_dir)

  unless File.exist?(dest_binary)
    raise "Failed to extract binary to #{dest_binary}"
  end

  puts "Installed shape binary to #{dest_binary}"
end

begin
  install_binary
rescue DownloadError, ChecksumError => e
  abort "Error: #{e.message}\nInstallation failed. Please check your network connection and try again."
rescue => e
  abort "Error: #{e.message}\nYou may need to install shape manually or via another method."
end

# Create a dummy Makefile (required by Ruby's extension mechanism)
File.write("Makefile", <<~MAKEFILE)
  .PHONY: all install clean

  all:
  \t@echo "shape-cli binary installation complete"

  install:
  \t@echo "Nothing to install"

  clean:
  \t@echo "Nothing to clean"
MAKEFILE
