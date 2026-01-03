# Supamigrate

[![Pipeline](https://github.com/Foodshareclub/supamigrate/actions/workflows/pipeline.yml/badge.svg)](https://github.com/Foodshareclub/supamigrate/actions/workflows/pipeline.yml)
[![Crates.io](https://img.shields.io/crates/v/supamigrate.svg)](https://crates.io/crates/supamigrate)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Supabase](https://img.shields.io/badge/Built%20for-Supabase-3ECF8E?logo=supabase&logoColor=white)](https://supabase.com)

A fast, cross-platform CLI tool for migrating and backing up [Supabase](https://supabase.com) projects — database schema, data, storage, and edge functions.

## Why Supamigrate?

- **One command** — Migrate entire projects between environments
- **Complete backups** — Database, storage, edge functions, RLS policies
- **CI/CD ready** — Automate daily backups to S3/R2/MinIO
- **Cross-platform** — Pre-built binaries for Linux, macOS, Windows
- **Secure** — SBOM, attestations, no credentials stored in code

## Features

| Feature | Description |
|---------|-------------|
| **Full Migration** | Schema, data, triggers, RLS policies in one command |
| **Edge Functions** | Backup and restore Deno edge functions via Management API |
| **Storage Sync** | Parallel bucket transfers with progress bars |
| **Backup & Restore** | Compressed backups with metadata |
| **Flexible** | Schema-only, data-only, or selective migrations |

## Installation

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/Foodshareclub/supamigrate/releases):

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/Foodshareclub/supamigrate/releases/latest/download/supamigrate-darwin-aarch64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/Foodshareclub/supamigrate/releases/latest/download/supamigrate-darwin-x86_64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/Foodshareclub/supamigrate/releases/latest/download/supamigrate-linux-x86_64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# Linux (ARM64)
curl -fsSL https://github.com/Foodshareclub/supamigrate/releases/latest/download/supamigrate-linux-aarch64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/
```

### From Cargo

```bash
cargo install supamigrate
```

### Prerequisites

PostgreSQL client tools required for database operations:

```bash
# macOS
brew install postgresql

# Ubuntu/Debian
sudo apt install postgresql-client

# Windows (via Chocolatey)
choco install postgresql
```

## Quick Start

### 1. Initialize Configuration

```bash
supamigrate config init
```

This creates `supamigrate.toml` (add to .gitignore!):

```toml
[projects.production]
project_ref = "your-project-ref"           # From Supabase dashboard URL
db_password = "your-db-password"           # Database password
service_key = "eyJhbGciOiJIUzI1NiIs..."    # Service role key (not anon!)
access_token = "sbp_xxxxxxxxxxxxx"         # Personal access token (for edge functions)

[projects.staging]
project_ref = "your-staging-ref"
db_password = "your-staging-password"
service_key = "eyJhbGciOiJIUzI1NiIs..."
access_token = "sbp_xxxxxxxxxxxxx"

[defaults]
parallel_transfers = 4
compress_backups = true
```

> **Where to find these values:**
> - `project_ref`: Your Supabase URL is `https://<project_ref>.supabase.co`
> - `db_password`: Project Settings → Database → Database password
> - `service_key`: Project Settings → API → `service_role` key (not anon!)
> - `access_token`: [Account → Access Tokens](https://supabase.com/dashboard/account/tokens)

### 2. Migrate Between Projects

```bash
# Full migration (production → staging)
supamigrate migrate --from production --to staging

# Include storage buckets
supamigrate migrate --from production --to staging --include-storage

# Schema only (no data)
supamigrate migrate --from production --to staging --schema-only
```

### 3. Backup & Restore

```bash
# Backup database
supamigrate backup --project production

# Full backup (database + storage + edge functions)
supamigrate backup --project production --include-storage --include-functions

# Restore to another project
supamigrate restore --from ./backup/production_20240115_120000 --to staging
```

## What Gets Backed Up

| Component | Included | Flag |
|-----------|----------|------|
| Tables, views, indexes | Always | - |
| Functions & triggers | Always | - |
| RLS policies | Always | - |
| Storage buckets & files | Optional | `--include-storage` |
| Edge Functions (Deno) | Optional | `--include-functions` |

## Commands

| Command | Description |
|---------|-------------|
| `migrate` | Migrate between Supabase projects |
| `backup` | Backup project to local disk |
| `restore` | Restore from backup |
| `storage list` | List storage buckets |
| `storage sync` | Sync storage between projects |
| `config init` | Create config file |
| `config list` | List configured projects |

Run `supamigrate <command> --help` for details.

## CI/CD Integration

### Automated Daily Backups (GitHub Actions)

```yaml
name: Daily Backup
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily
  workflow_dispatch:

jobs:
  backup:
    runs-on: ubuntu-latest
    steps:
      - name: Install supamigrate
        run: |
          curl -fsSL https://github.com/Foodshareclub/supamigrate/releases/latest/download/supamigrate-linux-x86_64.tar.gz | tar xz
          sudo mv supamigrate /usr/local/bin/
          sudo apt-get install -y postgresql-client

      - name: Create config
        run: |
          cat > supamigrate.toml << EOF
          [projects.production]
          project_ref = "${{ secrets.SUPABASE_PROJECT_REF }}"
          db_password = "${{ secrets.SUPABASE_DB_PASSWORD }}"
          service_key = "${{ secrets.SUPABASE_SERVICE_KEY }}"
          access_token = "${{ secrets.SUPABASE_ACCESS_TOKEN }}"
          EOF

      - name: Backup
        run: supamigrate backup --project production --include-functions

      - name: Upload to S3/R2
        run: |
          # Upload backup to your storage
          aws s3 cp ./backup/ s3://your-bucket/backups/ --recursive
```

### Required Secrets

| Secret | Description |
|--------|-------------|
| `SUPABASE_PROJECT_REF` | Project reference (from URL) |
| `SUPABASE_DB_PASSWORD` | Database password |
| `SUPABASE_SERVICE_KEY` | Service role key |
| `SUPABASE_ACCESS_TOKEN` | Personal access token (for edge functions) |

## Configuration

### Config File Locations

Searched in order:
1. `./supamigrate.toml`
2. `~/.config/supamigrate/config.toml`
3. `~/.supamigrate.toml`

### Environment Variables

```bash
export SUPAMIGRATE_CONFIG=/path/to/config.toml
export SUPAMIGRATE_SOURCE=production
export SUPAMIGRATE_TARGET=staging
```

## Development

```bash
git clone https://github.com/Foodshareclub/supamigrate
cd supamigrate
cargo build
cargo test
```

### Creating a Release

```bash
# Update version in Cargo.toml, then:
git add -A
git commit -m "chore: release v0.1.0"
git tag v0.1.0
git push origin main --tags
```

The pipeline automatically builds binaries for 6 platforms, generates SBOM, and publishes to crates.io.

## Security

### Reporting Vulnerabilities

Use GitHub's private vulnerability reporting:
**Security → Advisories → New draft**

### Supply Chain Security

- SBOM included in releases
- Build provenance attestations (SLSA Level 3)
- cargo-audit vulnerability scanning
- Secret scanning (TruffleHog, Gitleaks)

### Best Practices

1. **Never commit** `supamigrate.toml` — it contains credentials
2. **Verify downloads** using SHA256SUMS.txt
3. **Use GitHub Secrets** in CI/CD pipelines
4. **Rotate credentials** regularly

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Apache-2.0 — see [LICENSE](LICENSE).

## Community

Built for the [Supabase](https://supabase.com) community.

- [Report bugs](https://github.com/Foodshareclub/supamigrate/issues)
- [Request features](https://github.com/Foodshareclub/supamigrate/issues)
- [Discussions](https://github.com/Foodshareclub/supamigrate/discussions)

Inspired by [Supa-Migrate](https://github.com/mansueli/Supa-Migrate) by [@mansueli](https://github.com/mansueli).
