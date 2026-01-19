# Contributing to Shape CLI

Thank you for your interest in contributing to Shape CLI! This guide will help you get started.

## Code of Conduct

Be respectful, inclusive, and constructive. We're all here to build something useful.

## Getting Started

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs))
- Git

### Clone and Build

```bash
git clone https://github.com/shape-cli/shape.git
cd shape
cargo build
```

### Run Tests

```bash
cargo test
```

### Run with Debug Output

```bash
cargo run -- --verbose ready
```

## Development Workflow

### 1. Find or Create an Issue

- Check existing issues for something to work on
- For new features, open an issue first to discuss

### 2. Create a Branch

```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/my-fix
```

### 3. Make Changes

- Write code
- Add tests for new functionality
- Update documentation if needed

### 4. Test

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### 5. Format and Lint

```bash
cargo fmt
cargo clippy
```

### 6. Commit

Write clear commit messages:

```
Add brief type validation

- Validate frontmatter fields
- Check required sections for brief types
- Add tests for validation edge cases
```

### 7. Push and Open PR

```bash
git push origin feature/my-feature
```

Open a pull request with:
- Description of changes
- Link to related issue
- Any breaking changes noted

## Project Structure

```
shape-cli/
├── src/
│   ├── main.rs           # Entry point
│   ├── cli/              # Command-line interface
│   │   ├── app.rs        # CLI definition (clap)
│   │   ├── brief.rs      # Brief commands
│   │   ├── task.rs       # Task commands
│   │   ├── agent.rs      # Agent coordination commands
│   │   ├── context.rs    # Context export
│   │   ├── daemon.rs     # Background daemon
│   │   └── tui/          # Terminal UI
│   ├── domain/           # Core business logic
│   │   ├── brief.rs      # Brief model
│   │   ├── task.rs       # Task model
│   │   ├── graph.rs      # Dependency graph
│   │   └── id.rs         # ID generation
│   ├── storage/          # Persistence
│   │   ├── markdown.rs   # Markdown parsing
│   │   ├── jsonl.rs      # JSONL read/write
│   │   ├── project.rs    # Project structure
│   │   └── cache.rs      # SQLite cache
│   └── plugin/           # Plugin system
│       ├── protocol.rs   # JSON protocol
│       ├── loader.rs     # Plugin discovery
│       └── brief_type.rs # Brief type plugins
├── tests/                # Integration tests
├── docs/                 # Documentation
└── Cargo.toml
```

## Code Style

### Rust

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for lints
- Prefer explicit error handling over `.unwrap()`

### Documentation

- Document public APIs with rustdoc
- Keep README and docs/ in sync with features
- Use clear, concise language

### Commits

- One logical change per commit
- Present tense ("Add feature" not "Added feature")
- Reference issues when relevant

## Testing

### Unit Tests

Place in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

### Integration Tests

Place in `tests/`:

```rust
// tests/brief_test.rs
#[test]
fn test_brief_creation() {
    // ...
}
```

### Test Coverage

Aim for good coverage of:
- Happy paths
- Error cases
- Edge cases (empty inputs, special characters, etc.)

## Documentation

### Code Documentation

```rust
/// Creates a new brief with the given title.
///
/// # Arguments
///
/// * `title` - The brief title
/// * `brief_type` - Optional brief type (defaults to "minimal")
///
/// # Returns
///
/// The created brief, or an error if creation fails.
pub fn create_brief(title: &str, brief_type: Option<&str>) -> Result<Brief> {
    // ...
}
```

### User Documentation

- Update `docs/` for new features
- Keep examples working and realistic
- Link related documentation

## Pull Request Checklist

Before submitting:

- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (if needed)
- [ ] Commit messages are clear
- [ ] PR description explains the change

## Release Process

Maintainers handle releases:

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Tag release: `git tag v0.x.y`
4. Push tag: `git push --tags`
5. CI builds and publishes packages

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: Open a GitHub Issue
- **Security**: Email security@shape-cli.dev (do not open public issues)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
