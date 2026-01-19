# Publishing Guide

This document describes the release process for Shape CLI.

## Overview

Shape CLI is distributed through multiple package managers:

| Package Manager | Package Name | Registry |
|-----------------|--------------|----------|
| Cargo | `shape-cli` | crates.io |
| npm | `shape-cli` | npmjs.com |
| pip | `shape-cli` | PyPI |
| gem | `shape-cli` | RubyGems |
| Homebrew | `shape-cli` | Custom tap |

All packages share the same version number and are released simultaneously.

## Prerequisites

### Required Secrets

Configure these secrets in your GitHub repository settings:

| Secret | Description |
|--------|-------------|
| `NPM_TOKEN` | npm access token with publish permissions |
| `PYPI_TOKEN` | PyPI API token |
| `RUBYGEMS_API_KEY` | RubyGems API key |
| `CARGO_REGISTRY_TOKEN` | crates.io API token (optional, for Cargo publish) |

### Local Setup

For testing releases locally:

```bash
# npm
npm login

# PyPI
pip install twine build
# Configure ~/.pypirc or use TWINE_USERNAME/TWINE_PASSWORD

# RubyGems
gem signin
```

## Release Process

### 1. Prepare the Release

1. Update version in `Cargo.toml`:
   ```toml
   version = "X.Y.Z"
   ```

2. Update CHANGELOG.md (if maintained)

3. Commit the version bump:
   ```bash
   git add Cargo.toml
   git commit -m "Bump version to X.Y.Z"
   ```

### 2. Create the Release

Tag and push:

```bash
git tag vX.Y.Z
git push origin main --tags
```

### 3. Automated Release

The GitHub Actions workflow (`.github/workflows/release.yml`) automatically:

1. **Builds binaries** for all platforms:
   - macOS (x64, arm64)
   - Linux (x64, arm64, musl variants)
   - Windows (x64)

2. **Creates GitHub Release** with:
   - All binary archives
   - SHA256 checksums

3. **Publishes to package registries**:
   - npm (main package + platform packages)
   - PyPI
   - RubyGems

### 4. Verify the Release

After the workflow completes:

```bash
# Verify npm
npm view shape-cli version
npm install -g shape-cli && shape --version

# Verify PyPI
pip index versions shape-cli
pip install shape-cli && shape --version

# Verify RubyGems
gem search shape-cli
gem install shape-cli && shape --version
```

## Manual Publishing

If automated publishing fails, you can publish manually.

### npm

```bash
cd packages/npm

# Update versions
VERSION=X.Y.Z
for pkg in shape-cli @shape-cli/*; do
  cd $pkg
  npm version $VERSION --no-git-tag-version
  cd ..
done

# Download binaries from GitHub release
# Extract to respective @shape-cli/*/bin/ directories

# Publish platform packages first
for platform in darwin-arm64 darwin-x64 linux-arm64 linux-x64 windows-x64; do
  cd @shape-cli/$platform
  npm publish --access public
  cd ..
done

# Publish main package
cd shape-cli
npm publish --access public
```

### PyPI

```bash
cd packages/python

# Update version
sed -i 's/version = ".*"/version = "X.Y.Z"/' pyproject.toml

# Build
python -m build

# Upload
twine upload dist/*
```

### RubyGems

```bash
cd packages/ruby

# Update version
sed -i 's/spec.version = ".*"/spec.version = "X.Y.Z"/' shape-cli.gemspec

# Build
gem build shape-cli.gemspec

# Push
gem push shape-cli-X.Y.Z.gem
```

## Troubleshooting

### Build Failures

**ARM64 Linux cross-compilation fails:**
- Ensure `gcc-aarch64-linux-gnu` is installed
- Check that `CARGO_TARGET_*_LINKER` environment variables are set

**musl builds fail:**
- Ensure `musl-tools` is installed
- ARM64 musl may need additional setup

### Publish Failures

**npm publish fails:**
- Verify `NPM_TOKEN` is valid and has publish permissions
- Check that package names aren't already taken
- Ensure you're publishing platform packages before the main package

**PyPI publish fails:**
- Verify `PYPI_TOKEN` is valid
- Check that the version doesn't already exist
- Ensure `twine` is installed

**RubyGems publish fails:**
- Verify `RUBYGEMS_API_KEY` is valid
- Check that the version doesn't already exist

### Partial Release Recovery

If some registries published but others failed:

1. Do NOT re-tag or delete the GitHub release
2. Manually publish to the failed registries
3. Document any version skips if needed

## Architecture

### Binary Distribution

```
GitHub Release vX.Y.Z
├── shape-darwin-arm64.tar.gz
├── shape-darwin-x64.tar.gz
├── shape-linux-arm64.tar.gz
├── shape-linux-x64.tar.gz
├── shape-linux-arm64-musl.tar.gz
├── shape-linux-x64-musl.tar.gz
├── shape-windows-x64.zip
└── checksums.txt
```

### Package Structure

**npm:** Uses optional dependencies for platform-specific binaries. The main package contains a shim that delegates to the platform binary.

**pip:** Downloads binary on first use (lazy install). Stores binary in package directory.

**gem:** Downloads binary during gem installation via native extension hook.

## Security

- All binaries are signed with SHA256 checksums
- Checksum verification is mandatory in all wrappers
- Download failures are retried (3 attempts with 2s delay)
- GitHub releases are the single source of truth for binaries
