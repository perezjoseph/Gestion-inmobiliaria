# Summary: Check Open Code Scanning Alerts Grouped by Severity

## What Was Produced

1. **commands.md** — Step-by-step documentation of the exact `gh api` and PowerShell commands to run, with explanations and expected output formats.
2. **check-alerts.ps1** — A ready-to-run PowerShell script that executes the full workflow end-to-end.

## Workflow Overview

| Step | Action | Command |
|------|--------|---------|
| 1 | Get repo slug | `gh repo view --json nameWithOwner -q .nameWithOwner` |
| 2 | Fetch open alerts to file | `gh api "/repos/{slug}/code-scanning/alerts?state=open&per_page=100" --paginate > alerts.json` |
| 3 | Parse and group by severity | PowerShell `Group-Object` on `$_.rule.security_severity_level` |
| 4 | Clean up | `Remove-Item alerts.json` |

## Key Decisions (from SKILL.md)

- **No `gh code-scanning` subcommand** — it doesn't exist. Must use `gh api` against the REST endpoint.
- **Dump to file first** — long `-q` jq expressions break on Windows/PowerShell due to quote escaping issues. The skill explicitly recommends dumping raw JSON and parsing locally.
- **Always paginate** — pass `--paginate` and `per_page=100` to ensure all alerts are retrieved.
- **Severity field** — use `rule.security_severity_level` (not `rule.severity`) for the canonical severity grouping. Values: `critical`, `high`, `medium`, `low`, `none`.

## Output Grouping

The primary grouping is by severity level, sorted from most critical to least:

```
critical → high → medium → low → none
```

A secondary grouping by tool (`CodeQL`, `Trivy`, `Semgrep`) is also provided for context.
