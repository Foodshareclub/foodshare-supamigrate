# Supamigrate

[![Pipeline](https://github.com/foodshare-club/supamigrate/actions/workflows/pipeline.yml/badge.svg)](https://github.com/foodshare-club/supamigrate/actions/workflows/pipeline.yml)
[![Crates.io](https://img.shields.io/crates/v/supamigrate.svg)](https://crates.io/crates/supamigrate)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A fast, cross-platform CLI tool for migrating Supabase projects — database schema, data, storage, and edge functions.

## Features

- 🚀 **Full Migration** — Schema, data, storage, and edge functions in one command
- ⚡ **Edge Functions** — Backup and restore Deno edge functions
- 📦 **Storage Sync** — Parallel bucket transfers with progress bars
- 💾 **Backup & Restore** — Compressed backups with metadata
- 🔧 **Flexible** — Schema-only, data-only, or selective migrations
- 🌍 **Cross-platform** — Linux, macOS, and Windows binaries
- 🔒 **Secure** — SBOM, attestations, supply chain security

## Installation

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/foodshare-club/supamigrate/releases):

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/supamigrate-darwin-aarch64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/supamigrate-darwin-x86_64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/supamigrate-linux-x86_64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/

# Linux (ARM64)
curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/supamigrate-linux-aarch64.tar.gz | tar xz
sudo mv supamigrate /usr/local/bin/
```

### Verify Download

```bash
curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/SHA256SUMS.txt -o SHA256SUMS.txt
sha256sum -c SHA256SUMS.txt --ignore-missing
```

### From Cargo

```bash
cargo install supamigrate
```

### Prerequisites

PostgreSQL client tools required:

```bash
# macOS
brew install postgresql

# Ubuntu/Debian
sudo apt install postgresql-client
```

## Quick Start

### 1. Initialize Configuration

```bash
supamigrate config init
```

Creates `supamigrate.toml`:

```toml
[projects.production]
project_ref = "abcdefghijklmnop"
db_password = "your-db-password"
service_key = "eyJhbGciOiJIUzI1NiIs..."

[projects.staging]
project_ref = "qrstuvwxyz123456"
db_password = "your-db-password"
service_key = "eyJhbGciOiJIUzI1NiIs..."
```

### 2. Migrate

```bash
# Full migration
supamigrate migrate --from production --to staging

# Include storage
supamigrate migrate --from production --to staging --include-storage

# Schema only
supamigrate migrate --from production --to staging --schema-only
```

### 3. Backup & Restore

```bash
# Backup database only
supamigrate backup --project production

# Full backup (database + storage + edge functions)
supamigrate backup --project production --include-storage --include-functions

# Restore
supamigrate restore --from ./backup/production_20240115_120000 --to staging

# Restore with functions
supamigrate restore --from ./backup/production_20240115_120000 --to staging --include-functions
```

## What Gets Backed Up

| Component | Included | Flag |
|-----------|----------|------|
| Tables, views, indexes | ✅ Always | - |
| Functions & triggers | ✅ Always | - |
| RLS policies | ✅ Always | - |
| Storage buckets & files | Optional | `--include-storage` |
| Edge Functions (Deno) | Optional | `--include-functions` |

## Commands

| Command | Description |
|---------|-------------|
| `migrate` | Migrate between projects |
| `backup` | Backup to local disk |
| `restore` | Restore from backup |
| `storage list` | List buckets |
| `storage sync` | Sync storage between projects |
| `config init` | Create config file |
| `config list` | List configured projects |

Run `supamigrate <command> --help` for details.

## Configuration

### Config Locations

1. `./supamigrate.toml`
2. `~/.config/supamigrate/config.toml`
3. `~/.supamigrate.toml`

### Environment Variables

```bash
export SUPAMIGRATE_CONFIG=/path/to/config.toml
export SUPAMIGRATE_SOURCE=production
export SUPAMIGRATE_TARGET=staging
```

## CI/CD

### GitHub Actions

```yaml
name: Sync Staging
on:
  schedule:
    - cron: '0 2 * * *'
jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - name: Install
        run: |
          curl -fsSL https://github.com/foodshare-club/supamigrate/releases/latest/download/supamigrate-linux-x86_64.tar.gz | tar xz
          sudo mv supamigrate /usr/local/bin/
          sudo apt-get install -y postgresql-client

      - name: Config
        run: |
          cat > supamigrate.toml << EOF
          [projects.production]
          project_ref = "${{ secrets.PROD_PROJECT_REF }}"
          db_password = "${{ secrets.PROD_DB_PASSWORD }}"
          service_key = "${{ secrets.PROD_SERVICE_KEY }}"
          [projects.staging]
          project_ref = "${{ secrets.STAGING_PROJECT_REF }}"
          db_password = "${{ secrets.STAGING_DB_PASSWORD }}"
          service_key = "${{ secrets.STAGING_SERVICE_KEY }}"
          EOF

      - name: Sync
        run: supamigrate migrate --from production --to staging -y
```

### GitHub Secrets

| Secret | Required | Description |
|--------|----------|-------------|
| `CARGO_REGISTRY_TOKEN` | For crates.io | https://crates.io/settings/tokens |
| `CODECOV_TOKEN` | Optional | Code coverage |
| `SLACK_WEBHOOK_URL` | Optional | Release notifications |

### Pipeline Stages

```
Validation → Test → Build → Release → Notify
    │          │       │        │        │
    ├─ Lint    ├─ 3 OS ├─ 6 bin ├─ GitHub└─ Summary
    ├─ Security├─ Cover │  SBOM  │  crates  Slack
    └─ SAST    └─ Fuzz  └─ Attest└─────────────────
```

## Development

### Building

```bash
git clone https://github.com/foodshare-club/supamigrate
cd supamigrate
cargo build
cargo test
```

### Creating a Release

```bash
# Update version in Cargo.toml
git add -A
git commit -m "chore: release v0.1.0"
git tag v0.1.0
git push origin main --tags
```

The pipeline automatically builds 6 platform binaries, generates SBOM/attestations, creates GitHub Release, and publishes to crates.io.

### Pull Request Checklist

- [ ] Code follows project style (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Tests pass (`cargo test`)
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (for user-facing changes)

## Security

### Reporting Vulnerabilities

**DO NOT** create public GitHub issues for security vulnerabilities.

1. Email: security@foodshare.club
2. Or use GitHub's private vulnerability reporting (Security → Advisories → New draft)

**Response Timeline:**
- 24 hours: Initial acknowledgment
- 72 hours: Preliminary assessment
- 7 days: Detailed response with remediation plan
- 90 days: Coordinated public disclosure

### Supply Chain Security

- SBOM included in releases
- Build provenance attestations (SLSA Level 3)
- cargo-audit vulnerability scanning
- cargo-deny license compliance
- Secret scanning (TruffleHog, Gitleaks)
- SAST (CodeQL, Semgrep)

### Best Practices

1. **Verify downloads** using SHA256SUMS.txt
2. **Never commit** `supamigrate.toml` with credentials
3. **Use environment variables** in CI/CD
4. **Rotate credentials** regularly

## Code of Conduct

We pledge to make participation in our project a harassment-free experience for everyone. We expect positive behavior: welcoming language, respect for differing viewpoints, and graceful acceptance of criticism. Unacceptable behavior includes trolling, harassment, and publishing others' private information. See [Contributor Covenant](https://www.contributor-covenant.org/version/2/0/code_of_conduct.html) for full details.

## Changelog

### [Unreleased]
- Initial release
- Database migration between Supabase projects (schema, data, triggers, RLS policies)
- Edge Functions backup and restore via Supabase Management API
- Storage bucket sync with parallel transfers
- Backup and restore functionality with compression
- TOML configuration file support
- Cross-platform binaries (Linux, macOS, Windows)
- Enterprise CI/CD pipeline with SBOM and attestations

## License

Apache-2.0 — see [LICENSE](LICENSE).

## Credits

Inspired by [Supa-Migrate](https://github.com/mansueli/Supa-Migrate) by [@mansueli](https://github.com/mansueli).
