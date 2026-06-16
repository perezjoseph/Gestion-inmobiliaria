---
name: code-scanning-alerts
description: >
  Triage, list, and manage GitHub code scanning alerts (CodeQL, Trivy, Semgrep)
  from the CLI. Use when asked to check security alerts, triage vulnerabilities,
  dismiss false positives, list open code scanning results, or investigate CVEs
  in the repository.
---

# Code Scanning Alerts

Workflow for listing and triaging open GitHub code scanning alerts (CodeQL + Trivy + Semgrep) from the CLI.

## Important

- `gh code-scanning` does **not** exist as a subcommand. Use `gh api` against the REST endpoint.
- The `alerts` endpoint is paginated. Always pass `--paginate` and `per_page=100`.
- On Windows / cmd / PowerShell, escape double quotes inside `-q` jq expressions with `\"` or save the raw JSON to a file first and filter with PowerShell. Long `-q` pipelines with `|` often break because the shell splits the argument.

## Workflow

### Step 1: Get repo slug

```powershell
gh repo view --json nameWithOwner -q .nameWithOwner
```

### Step 2: List open alerts (raw JSON to file)

Prefer dumping to a file and parsing locally. It's more robust than long `-q` expressions.

```powershell
gh api "/repos/<OWNER>/<REPO>/code-scanning/alerts?state=open&per_page=100" --paginate > alerts.json
```

Valid `state` values: `open`, `closed`, `dismissed`, `fixed`.

### Step 3: Parse with PowerShell

```powershell
$alerts = Get-Content alerts.json | ConvertFrom-Json

# Total count
$alerts.Count

# Group by severity
$alerts | Group-Object {$_.rule.security_severity_level} | Select-Object Name, Count

# Group by tool
$alerts | Group-Object {$_.tool.name} | Select-Object Name, Count

# Critical + high only
$alerts | Where-Object {$_.rule.security_severity_level -in 'critical','high'} |
  Select-Object number,
    @{n='rule';e={$_.rule.id}},
    @{n='path';e={$_.most_recent_instance.location.path}},
    @{n='tool';e={$_.tool.name}} |
  Format-Table -AutoSize -Wrap
```

Clean up the dump file after triage: `Remove-Item alerts.json`.

### Step 4: Parse with jq (short queries only)

```powershell
gh api "/repos/<OWNER>/<REPO>/code-scanning/alerts?state=open&per_page=100" --paginate -q "length"
```

For anything longer than `length` or a single field selector, use the PowerShell approach.

## Key Fields

- `number` — alert ID, use with `gh api /repos/.../code-scanning/alerts/{number}` for full detail.
- `rule.id` — CVE or CodeQL rule name (e.g. `rust/hard-coded-cryptographic-value`).
- `rule.security_severity_level` — `critical` / `high` / `medium` / `low` / `none`.
- `tool.name` — `CodeQL`, `Trivy`, `Semgrep`.
- `most_recent_instance.location.path` — file or image path.
- `most_recent_instance.location.start_line` — line number (1 for container/package alerts).
- `most_recent_instance.message.text` — human-readable description with CVE link and fixed version.
- `state` — `open`, `dismissed`, `fixed`.

## Triage Heuristics

- Trivy alerts on `usr/local/lib/node_modules/npm/**` and `app/node_modules/**` come from base container images, not our code. Fix by bumping the base image tag in the matching Dockerfile, not by editing lockfiles.
- Trivy alerts with path equal to `<owner>/<image-name>` (no subpath) are OS-package CVEs in the image. Fix by rebuilding on a newer base (`apt-get update && apt-get upgrade` during image build, or bump `FROM` tag).
- CodeQL alerts with path under `backend/` or `frontend/` are real source findings. Fix the code.
- Dockerfile misconfigs (DS-xxxx rules) are authoring issues in our own Dockerfiles. Fix directly.

## Dismissing False Positives

```powershell
gh api -X PATCH "/repos/<OWNER>/<REPO>/code-scanning/alerts/<NUMBER>" `
  -f state=dismissed `
  -f dismissed_reason=false_positive `
  -f dismissed_comment="<reason>"
```

Valid `dismissed_reason`: `false_positive`, `won't_fix`, `used_in_tests`.
