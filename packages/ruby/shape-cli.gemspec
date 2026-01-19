# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = "shape-cli"
  spec.version = "0.0.0"
  spec.authors = ["Shape CLI Team"]
  spec.email = ["hello@shape-cli.dev"]

  spec.summary = "A local-first task management CLI for software teams"
  spec.description = <<~DESC
    Shape CLI helps you organize work around "anchors" (pitches, RFCs, PRDs)
    with dependent tasks and AI-optimized context export.
  DESC
  spec.homepage = "https://github.com/shape-cli/shape"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 2.7.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/shape-cli/shape"
  spec.metadata["changelog_uri"] = "https://github.com/shape-cli/shape/releases"

  spec.files = Dir[
    "lib/**/*",
    "bin/*",
    "ext/**/*",
    "LICENSE",
    "README.md"
  ]
  spec.bindir = "bin"
  spec.executables = ["shape"]
  spec.require_paths = ["lib"]

  spec.extensions = ["ext/extconf.rb"]
end
