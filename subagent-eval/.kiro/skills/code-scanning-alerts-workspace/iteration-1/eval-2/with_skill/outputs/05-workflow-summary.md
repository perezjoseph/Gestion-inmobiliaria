# Trivy Alert Triage — Workflow Summary

## Complete Workflow Steps

### Step 1: Fetch Alerts

```powershell
# Get repo slug
gh repo view --json nameWithOwner -q .nameWithOwner

# Dump all open alerts to file (paginated)
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=open&per_page=100" --paginate > all-alerts.json

# Filter to Trivy only
$allAlerts = Get-Content all-alerts.json | ConvertFrom-Json
$trivyAlerts = $allAlerts | Where-Object { $_.tool.name -eq "Trivy" }
$trivyAlerts | ConvertTo-Json -Depth 10 | Set-Content trivy-alerts.json
```

### Step 2: Classify Each Alert

Run `02-triage-classification.ps1` which applies the decision tree:

| Rule | Path Pattern | Classification |
|------|-------------|----------------|
| `DS\d{4}` | Any | OUR_DOCKERFILE_MISCONFIG |
| Any | `owner/image` (bare ref) | BASE_IMAGE_OS_PACKAGE |
| Any | `usr/local/lib/node_modules/npm/**` | BASE_IMAGE_NPM_BUNDLED |
| Any | `app/node_modules/**` | OUR_DEPENDENCY |
| Any | `usr/local/lib/python*/site-packages/**` | OUR_DEPENDENCY |
| Any | `*go/pkg/mod*` or `usr/bin/caddy` | OUR_DEPENDENCY |
| Any | `*intel-igc*\|*libigdgmm*\|*intel-opencl*` | OUR_DEPENDENCY |
| Any | `usr/lib\|lib/\|usr/share` (line 1) | BASE_IMAGE_OS_PACKAGE |
| Any | `backend/\|frontend/\|baileys-service/src/\|ocr-service/` | OUR_CODE |

### Step 3: Prioritize

1. **Critical/High OUR_DEPENDENCY** → Fix immediately (bump versions)
2. **Critical/High OUR_DOCKERFILE_MISCONFIG** → Fix Dockerfile
3. **Critical BASE_IMAGE_OS_PACKAGE** → Bump FROM tag, rebuild
4. **Medium/Low BASE_IMAGE** → Track, dismiss with comment if no upstream fix

### Step 4: Act

- **OUR_DEPENDENCY/OUR_CODE:** Create a fix PR targeting the vulnerable package version
- **BASE_IMAGE:** Create a Dockerfile PR bumping the FROM tag
- **Won't fix:** Dismiss via API with documented reason

### Step 5: Verify

After the fix PR merges and the containers workflow re-scans:

```powershell
# Check alerts transitioned to "fixed"
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=fixed&tool_name=Trivy&per_page=100" --paginate > fixed-alerts.json
$fixed = Get-Content fixed-alerts.json | ConvertFrom-Json
Write-Host "Auto-fixed: $($fixed.Count)"
```

---

## Key API Endpoints Used

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/repos/{owner}/{repo}/code-scanning/alerts` | GET | List alerts (supports `state`, `tool_name`, `per_page`, pagination) |
| `/repos/{owner}/{repo}/code-scanning/alerts/{number}` | GET | Full alert detail |
| `/repos/{owner}/{repo}/code-scanning/alerts/{number}` | PATCH | Dismiss or reopen alert |
| `/repos/{owner}/{repo}/code-scanning/alerts/{number}/instances` | GET | All instances of an alert |

## Key Fields for Classification

- `tool.name` — Filter to `"Trivy"` (vs CodeQL, Semgrep)
- `rule.id` — CVE ID or Trivy rule (e.g., `CVE-2024-XXXXX`, `DS0002`)
- `rule.security_severity_level` — `critical`, `high`, `medium`, `low`
- `most_recent_instance.location.path` — **Primary classification signal**
- `most_recent_instance.location.start_line` — `1` for package-level findings
- `most_recent_instance.message.text` — Contains CVE link and fixed-in version

## Output Files

| File | Purpose |
|------|---------|
| `01-fetch-alerts.ps1` | Commands to retrieve alerts from GitHub API |
| `02-triage-classification.ps1` | Full classification logic with PowerShell functions |
| `03-triage-heuristics.md` | Detailed heuristic documentation and decision tree |
| `04-remediation-commands.ps1` | Dismissal, investigation, and fix commands |
| `05-workflow-summary.md` | This file — end-to-end workflow overview |
