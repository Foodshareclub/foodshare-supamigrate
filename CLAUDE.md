# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Supamigrate is a Rust CLI tool for migrating Supabase projects — database schema, data, storage, and edge functions between environments.

## Build & Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test
cargo test --all-features

# Lint & Format
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -A clippy::module_name_repetitions

# Run locally
cargo run -- <command>
```

## Architecture

The codebase follows a modular structure with clear separation of concerns:

```
src/
├── main.rs         # Entry point, tracing setup, command dispatch
├── cli.rs          # Clap-based CLI definitions (Commands, Args structs)
├── config.rs       # TOML config loading from ./supamigrate.toml or ~/.config/supamigrate/
├── error.rs        # Custom error types using thiserror
├── commands/       # Command implementations
│   ├── migrate.rs  # Project-to-project migration
│   ├── backup.rs   # Backup to local disk
│   ├── restore.rs  # Restore from backup
│   ├── storage.rs  # Storage-only operations (list, sync, download, upload)
│   └── config.rs   # Config management (init, add, list, show)
├── db/             # Database operations using pg_dump/psql
│   ├── dump.rs     # pg_dump wrapper
│   ├── restore.rs  # psql restore
│   └── transform.rs# SQL transformations
├── storage/        # Supabase Storage API client
│   ├── client.rs   # HTTP client for storage operations
│   └── transfer.rs # Parallel file transfers with progress
└── functions/      # Edge Functions via Supabase Management API
    └── client.rs   # Backup/restore Deno edge functions
```

## Key Design Patterns

- **Async runtime**: tokio with full features
- **HTTP client**: reqwest with rustls (no OpenSSL dependency for cross-platform builds)
- **CLI parsing**: clap with derive macros and env var support
- **Error handling**: thiserror for library errors, anyhow for application errors
- **Progress display**: indicatif for progress bars on storage transfers
- **Database operations**: Shell out to pg_dump/psql (PostgreSQL client tools required)

## Configuration

Config is loaded from (in order): `./supamigrate.toml`, `~/.config/supamigrate/config.toml`, `~/.supamigrate.toml`

Projects are referenced by alias (e.g., "production", "staging") defined in config.

## CI/CD

The pipeline (.github/workflows/pipeline.yml) builds 6 platform binaries on tag push:
- linux-x86_64, linux-x86_64-musl, linux-aarch64
- darwin-x86_64, darwin-aarch64
- windows-x86_64

Releases are automated: update version in Cargo.toml, tag with `v*`, push.

## Clippy Configuration

The codebase allows certain clippy lints (see main.rs):
- `clippy::uninlined_format_args`
- `clippy::doc_markdown`
- `clippy::cast_precision_loss`
- `clippy::struct_excessive_bools`
- `clippy::too_many_lines`
- `clippy::single_match_else`

## Commit Style

Use conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `chore:`
