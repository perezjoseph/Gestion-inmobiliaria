# Code Scanning Alerts — Complete Command Workflow

## Objective

Check all open code scanning alerts in the repository and group them by severity.

---

## Step 1: Get Repository Slug

Determine the owner/repo for API calls:

```powershell
gh repo view --json nameWithOwner -q .nameWithOwner
```

**Expected output:** Something like `myorg/realestate`

This value is used in subsequent API URLs as `<OWNER>/<REPO>`.

---

## Step 2: Fetch All Open Alerts to a Local File

Dump all open code scanning alerts (paginated) to a JSON file for local parsing:

```powershell
gh api "/repos/<OWNER>/<REPO>/code-scanning/alerts?state=open&per_page=100" --paginate > alerts.json
```

**Why a file?** The SKILL.md explicitly states that long `-q` jq pipelines break on Windows/PowerShell due to quote escaping. Dumping to a file and parsing locally is more robust.

---

## Step 3: Parse and Group by Severity (PowerShell)

### 3a. Total count of open alerts

```powershell
$alerts = Get-Content alerts.json | ConvertFrom-Json
$alerts.Count
```

### 3b. Group by severity level

```powershell
$alerts | Group-Object {$_.rule.security_severity_level} | Select-Object Name, Count
```

**Expected output format:**

```
Name      Count
----      -----
critical      2
high          5
medium       12
low           3
none          1
```

### 3c. (Bonus) Group by tool for additional context

```powershell
$alerts | Group-Object {$_.tool.name} | Select-Object Name, Count
```

### 3d. Show critical and high alerts in detail

```powershell
$alerts | Where-Object {$_.rule.security_severity_level -in 'critical','high'} |
  Select-Object number,
    @{n='rule';e={$_.rule.id}},
    @{n='severity';e={$_.rule.security_severity_level}},
    @{n='path';e={$_.most_recent_instance.location.path}},
    @{n='tool';e={$_.tool.name}} |
  Format-Table -AutoSize -Wrap
```

---

## Step 4: Clean Up

Remove the temporary JSON dump after triage:

```powershell
Remove-Item alerts.json
```

---

## Key Fields Reference

| Field | Description |
|-------|-------------|
| `number` | Alert ID |
| `rule.id` | CVE or rule name (e.g. `rust/hard-coded-cryptographic-value`) |
| `rule.security_severity_level` | `critical` / `high` / `medium` / `low` / `none` |
| `tool.name` | `CodeQL`, `Trivy`, or `Semgrep` |
| `most_recent_instance.location.path` | Affected file path |
| `state` | `open`, `dismissed`, or `fixed` |

---

## Triage Notes (from SKILL.md)

- Trivy alerts on `usr/local/lib/node_modules/npm/**` or `app/node_modules/**` are from base container images — fix by bumping the base image tag in the Dockerfile.
- Trivy alerts with path equal to `<owner>/<image-name>` (no subpath) are OS-package CVEs — fix by rebuilding on a newer base.
- CodeQL alerts under `backend/` or `frontend/` are real source findings — fix the code.
- Dockerfile misconfigs (DS-xxxx rules) are authoring issues — fix directly.
