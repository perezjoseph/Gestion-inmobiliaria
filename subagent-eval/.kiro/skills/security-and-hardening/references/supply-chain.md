# Supply Chain Security

Dependency auditing, CI pipeline security, and practices for managing third-party code in this Rust workspace.

## Table of Contents

1. [Tooling Overview](#tooling-overview)
2. [cargo-audit](#cargo-audit)
3. [cargo-deny Configuration](#cargo-deny-configuration)
4. [CI Pipeline Integration](#ci-pipeline-integration)
5. [Dependency Review Process](#dependency-review-process)
6. [Handling Advisories](#handling-advisories)
7. [Secrets in CI](#secrets-in-ci)

---

## Tooling Overview

- **`cargo audit`** — Scan Cargo.lock against RustSec Advisory Database. Run in CI (scheduled) and pre-release.
- **`cargo deny check`** — Enforce license, advisory, ban, and source policies. Run on every PR.
- **`cargo outdated`** — Find dependencies with newer versions available. Run periodically.
- **`gitleaks`** — Detect secrets accidentally committed. Run via pre-commit hook and CI.

---

## cargo-audit

Checks `Cargo.lock` against the [RustSec Advisory Database](https://rustsec.org/).

### Commands

```bash
# Basic scan
cargo audit

# Fail on warnings too (strict mode for CI)
cargo audit --deny warnings

# JSON output for automated processing
cargo audit --json

# Auto-fix by updating Cargo.lock where safe
cargo audit fix

# Ignore specific advisories (prefer deny.toml for persistent ignores)
cargo audit --ignore RUSTSEC-2023-0071
```

### Syncing with deny.toml

The project maintains advisory ignores in `deny.toml` under `[advisories].ignore`. Each entry must have a comment explaining:
- Why it's ignored (transitive dep, not exercised, no patch available)
- What would need to change for it to be resolved
- When it was last reviewed

Ignores (review periodically):
```toml
[advisories]
ignore = [
    "RUSTSEC-2023-0071",  # rsa Marvin Attack — we use HMAC not RSA
    "RUSTSEC-2024-0370",  # proc-macro-error unmaintained — yew transitive
    "RUSTSEC-2025-0141",  # bincode unmaintained — yew transitive
    "RUSTSEC-2021-0140",  # rusttype unmaintained — genpdf transitive
    "RUSTSEC-2026-0097",  # rand unsoundness — no patch available
]
```

---

## cargo-deny Configuration

The project's `deny.toml` enforces four policy dimensions:

### Advisories
- `vulnerability = "deny"` — hard fail on known vulnerabilities
- Ignored advisories documented with justification

### Licenses
- Allowlist of permissive licenses (MIT, Apache-2.0, BSD, ISC, etc.)
- Exceptions for specific crates (actix-governor GPL, genpdf EUPL)
- Private workspace crates exempt from license checks

### Bans
- `multiple-versions = "warn"` — alerts on duplicate crate versions
- `wildcards = "deny"` — no `*` version specs

### Sources
- `unknown-registry = "deny"` — only crates.io allowed
- `unknown-git = "deny"` — no git dependencies without explicit allow
- `allow-git = []` — currently no git deps allowed

### Running

```bash
# All checks
cargo deny check

# Individual checks
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources

# Generate a report
cargo deny check 2>&1 | tee deny-report.txt
```

---

## CI Pipeline Integration

### Scheduled Security Audit

```yaml
# .github/workflows/security-audit.yml
name: Security Audit
on:
  schedule:
    - cron: '0 6 * * *'  # Runs on schedule — advisories appear continuously
  push:
    paths:
      - 'Cargo.lock'
      - 'deny.toml'

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

### PR Check (Every Pull Request)

```yaml
# .github/workflows/pr-checks.yml (add to existing)
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check
          arguments: --all-features

  gitleaks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## Dependency Review Process

### Before Adding a New Dependency

1. **Check necessity** — Can this be done with std or existing deps?
2. **Check maintenance** — Last publish date, open issues, bus factor
3. **Check security** — Run `cargo audit` after adding
4. **Check license** — Will `cargo deny check licenses` pass?
5. **Check size** — How many transitive deps does it pull in? (`cargo tree -p new-crate`)
6. **Pin version** — Use exact versions for security-critical deps

### Security-Critical Dependencies (Pin Exact Versions)

These crates handle authentication, cryptography, or data integrity. Pin to exact versions and review changelogs before updating:

```toml
# Cargo.toml — security-critical deps
argon2 = "=0.5.3"           # Password hashing
jsonwebtoken = "=10.3.0"    # JWT encode/decode (>=10.3.0 required for CVE fix)
```

### General Dependencies (Use Compatible Versions)

```toml
# Cargo.toml — general deps (semver-compatible ranges OK)
actix-web = "4"
sea-orm = "1"
serde = "1"
```

---

## Handling Advisories

### Decision Tree

```
Advisory reported
├── Severity: Critical/High
│   ├── Is the vulnerable code path exercised?
│   │   ├── YES → Fix immediately (update, patch, or replace)
│   │   └── NO → Add to deny.toml ignore with justification, fix within 2 weeks
│   └── Is a fix available?
│       ├── YES → Update Cargo.lock (`cargo update -p affected-crate`)
│       └── NO → Document in deny.toml, monitor for patch, consider alternatives
├── Severity: Medium
│   ├── Exercised in production? → Fix in next release cycle
│   └── Dev-only or transitive? → Track, fix when convenient
└── Severity: Low
    └── Fix during regular dependency updates
```

### Adding an Ignore Entry

```toml
[advisories]
ignore = [
    # RUSTSEC-YYYY-NNNN: <crate-name> — <vulnerability title>
    # Impact: <why this doesn't affect us>
    # Resolution: <what needs to happen for us to remove this ignore>
    # Added: YYYY-MM-DD, Review by: YYYY-MM-DD
    "RUSTSEC-YYYY-NNNN",
]
```

### Removing an Ignore Entry

When a patch becomes available:
1. Run `cargo update -p affected-crate`
2. Verify the advisory no longer triggers: `cargo deny check advisories`
3. Remove the ignore entry from `deny.toml`
4. Run full test suite to confirm no regressions

---

## Secrets in CI

### GitHub Actions Secrets

- Store all credentials in GitHub Secrets (Settings → Secrets and variables → Actions)
- Never echo secrets in workflow logs
- Use `${{ secrets.NAME }}` syntax — GitHub masks these in logs automatically

### Required Secrets for This Project

- **`DATABASE_URL`** — Test database connection. Rotation: per environment.
- **`JWT_SECRET`** — Token signing in integration tests. Rotation: periodically.
- **`GITHUB_TOKEN`** — Auto-provided, used for audit reports. Rotation: automatic.

### Secret Scanning

The project uses `gitleaks` (configured in `.gitleaks.toml`) to prevent accidental commits of:
- API keys and tokens
- Database connection strings with passwords
- Private keys (PEM, RSA)
- AWS/GCP/Azure credentials

Pre-commit hook (if not already configured):
```bash
# .git/hooks/pre-commit
#!/bin/sh
gitleaks protect --staged --config .gitleaks.toml
```

---

## Useful Commands Reference

```bash
# Full security check (run before releases)
cargo audit && cargo deny check

# Check what a new dependency pulls in
cargo tree -p new-crate-name

# Find duplicate crate versions
cargo deny check bans

# List outdated dependencies
cargo outdated --root-deps-only

# Check for yanked crates in Cargo.lock
cargo deny check advisories

# Scan for secrets in git history
gitleaks detect --config .gitleaks.toml

# Update a specific crate (after advisory fix)
cargo update -p crate-name
```
