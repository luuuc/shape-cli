# frozen_string_literal: true

require_relative "shape_cli/version"

module ShapeCli
  class Error < StandardError; end

  class << self
    def binary_path
      # Check for binary in gem's lib directory
      gem_binary = File.join(__dir__, "shape_cli", "bin", binary_name)
      return gem_binary if File.exist?(gem_binary)

      # Fall back to system PATH
      system_binary = which("shape")
      return system_binary if system_binary

      raise Error, "shape binary not found. Try reinstalling: gem install shape-cli"
    end

    def binary_name
      Gem.win_platform? ? "shape.exe" : "shape"
    end

    def run(*args)
      exec(binary_path, *args)
    end

    private

    def which(cmd)
      exts = ENV["PATHEXT"] ? ENV["PATHEXT"].split(";") : [""]
      ENV["PATH"].split(File::PATH_SEPARATOR).each do |path|
        exts.each do |ext|
          exe = File.join(path, "#{cmd}#{ext}")
          return exe if File.executable?(exe) && !File.directory?(exe)
        end
      end
      nil
    end
  end
end
