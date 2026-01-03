# Contributing to Supamigrate

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

1. **Install Rust** (1.75+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone the repo**
   ```bash
   git clone https://github.com/Foodshareclub/supamigrate
   cd supamigrate
   ```

3. **Build**
   ```bash
   cargo build
   ```

4. **Run tests**
   ```bash
   cargo test
   ```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Add tests for new functionality
- Update documentation for user-facing changes

## Pull Request Process

1. Fork the repo and create a feature branch
2. Make your changes with clear commit messages
3. Ensure CI passes (fmt, clippy, tests)
4. Open a PR with a description of changes

## Commit Messages

Use conventional commits:

```
feat: add bucket filtering to storage sync
fix: handle empty buckets gracefully
docs: update installation instructions
test: add integration tests for backup
```

## Testing

### Unit tests
```bash
cargo test
```

### Integration tests (requires Supabase)
```bash
# Set up test projects first
export TEST_SOURCE_PROJECT=...
export TEST_TARGET_PROJECT=...
cargo test --features integration
```

## Releasing

Releases are automated via GitHub Actions when a tag is pushed:

```bash
git tag v0.2.0
git push origin v0.2.0
```

## Questions?

Open an issue or start a discussion!
