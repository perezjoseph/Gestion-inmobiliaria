# Workflow: Check Open Code Scanning Alerts Grouped by Severity

## Overview

This workflow uses the GitHub CLI (`gh`) to query the GitHub Code Scanning API, retrieve all open alerts, and organize them by severity level.

## Files Produced

| File | Purpose |
|------|---------|
| `commands.md` | Step-by-step individual commands with explanations |
| `scan-alerts.sh` | Ready-to-run bash script that executes the full workflow |

## Workflow Steps

### 1. Authenticate
Verify `gh` CLI is authenticated against the target repository.

### 2. Fetch Open Alerts
Query `GET /repos/{owner}/{repo}/code-scanning/alerts?state=open` with pagination to retrieve all open alerts regardless of tool (CodeQL, Semgrep, Trivy, etc.).

### 3. Group by Severity
Use `jq` to aggregate alerts into severity buckets:
- **Security alerts** use `rule.security_severity_level` → `critical`, `high`, `medium`, `low`
- **Code quality alerts** use `rule.severity` → `error`, `warning`, `note`

### 4. Display Detailed Listings per Severity
For each severity level, list the individual alerts showing:
- Alert number
- Scanning tool name
- Rule ID
- Affected file path

## API Endpoints Used

| Endpoint | Purpose |
|----------|---------|
| `GET /repos/{owner}/{repo}/code-scanning/alerts` | List all code scanning alerts |

### Key Query Parameters

| Parameter | Value | Purpose |
|-----------|-------|---------|
| `state` | `open` | Only fetch unresolved alerts |
| `severity` | `critical\|high\|medium\|low` | Filter by severity (optional) |
| `per_page` | `100` | Maximum page size for efficiency |

## Severity Mapping

```
GitHub Code Scanning Severity Levels:
├── Security Alerts (rule.security_severity_level)
│   ├── critical  — Exploitable vulnerabilities with severe impact
│   ├── high      — Exploitable vulnerabilities with significant impact
│   ├── medium    — Potential vulnerabilities or risky patterns
│   └── low       — Informational security findings
└── Code Quality Alerts (rule.severity)
    ├── error     — Definite bugs or critical issues
    ├── warning   — Likely problems or bad practices
    └── note      — Suggestions and minor issues
```

## Expected Output Format

```
============================================
 Code Scanning Alerts — Grouped by Severity
============================================

--- Summary: Alert Count by Severity ---

high: 5 alert(s)
medium: 12 alert(s)
low: 3 alert(s)

--- Critical Severity ---

  (none)

--- High Severity ---

  #14 [CodeQL] js/sql-injection — backend/src/handlers/search.rs
  #11 [Semgrep] rust.lang.security.unsafe-block — backend/src/services/crypto.rs
  ...

--- Medium Severity ---

  #22 [CodeQL] js/missing-rate-limiting — backend/src/routes.rs
  ...
```
