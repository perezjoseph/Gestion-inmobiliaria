# Code Scanning Alerts - Command Workflow

## Objective
Check all open code scanning alerts in the repository and group them by severity.

---

## Prerequisites

```bash
# Ensure gh CLI is installed and authenticated
gh auth status
```

---

## Step 1: Fetch All Open Code Scanning Alerts

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&per_page=100" \
  --jq '.[] | {number, rule: .rule.id, severity: .rule.security_severity_level // .rule.severity, tool: .tool.name, description: .rule.description, path: .most_recent_instance.location.path, created_at}'
```

**Explanation:** This fetches all open code scanning alerts from the GitHub API. The `--paginate` flag ensures we get all results even if there are more than 100. We extract key fields including the severity (preferring `security_severity_level` for security alerts, falling back to `rule.severity` for code quality alerts).

---

## Step 2: Get Summary Count by Severity

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&per_page=100" \
  --jq '[.[] | .rule.security_severity_level // .rule.severity] | group_by(.) | map({severity: .[0], count: length}) | sort_by(.count) | reverse | .[]'
```

**Explanation:** This groups all open alerts by their severity level and counts them. Results are sorted by count in descending order so the most common severity appears first.

---

## Step 3: Detailed Breakdown - Critical Alerts

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=critical&per_page=100" \
  --jq '.[] | {number, rule: .rule.id, tool: .tool.name, path: .most_recent_instance.location.path, created: .created_at}'
```

---

## Step 4: Detailed Breakdown - High Alerts

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=high&per_page=100" \
  --jq '.[] | {number, rule: .rule.id, tool: .tool.name, path: .most_recent_instance.location.path, created: .created_at}'
```

---

## Step 5: Detailed Breakdown - Medium Alerts

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=medium&per_page=100" \
  --jq '.[] | {number, rule: .rule.id, tool: .tool.name, path: .most_recent_instance.location.path, created: .created_at}'
```

---

## Step 6: Detailed Breakdown - Low Alerts

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=low&per_page=100" \
  --jq '.[] | {number, rule: .rule.id, tool: .tool.name, path: .most_recent_instance.location.path, created: .created_at}'
```

---

## Step 7: Alerts Grouped by Tool AND Severity

```bash
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&per_page=100" \
  --jq 'group_by(.tool.name) | map({tool: .[0].tool.name, alerts: (group_by(.rule.security_severity_level // .rule.severity) | map({severity: .[0].rule.security_severity_level // .[0].rule.severity, count: length}))})'
```

**Explanation:** Groups alerts first by the scanning tool (CodeQL, Semgrep, Trivy, etc.) and then by severity within each tool, giving a two-dimensional view.

---

## Notes

- `{owner}/{repo}` is automatically resolved by `gh` when run inside a git repository.
- The `security_severity_level` field is used for security-related alerts (values: critical, high, medium, low).
- The `rule.severity` field is used for code quality alerts (values: error, warning, note).
- The `--paginate` flag handles repositories with more than 100 alerts.
- If `gh` is not available, equivalent `curl` commands can be used with a GitHub PAT.
