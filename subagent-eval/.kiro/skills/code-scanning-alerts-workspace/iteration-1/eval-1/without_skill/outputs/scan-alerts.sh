#!/usr/bin/env bash
# scan-alerts.sh — Fetch open code scanning alerts and group by severity
# Usage: ./scan-alerts.sh
# Prerequisites: gh CLI installed and authenticated (gh auth login)

set -euo pipefail

echo "============================================"
echo " Code Scanning Alerts — Grouped by Severity"
echo "============================================"
echo ""

# Step 1: Verify authentication
echo "Checking GitHub CLI authentication..."
if ! gh auth status &>/dev/null; then
  echo "ERROR: Not authenticated. Run 'gh auth login' first."
  exit 1
fi
echo "Authenticated OK."
echo ""

# Step 2: Fetch all open alerts and group by severity
echo "--- Summary: Alert Count by Severity ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&per_page=100" \
  --jq '
    [.[] | .rule.security_severity_level // .rule.severity // "unknown"]
    | group_by(.)
    | map({severity: .[0], count: length})
    | sort_by(.count)
    | reverse
    | .[]
    | "\(.severity): \(.count) alert(s)"
  '

echo ""
echo "--- Critical Severity ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=critical&per_page=100" \
  --jq '.[] | "  #\(.number) [\(.tool.name)] \(.rule.id) — \(.most_recent_instance.location.path)"' \
  || echo "  (none or API error)"

echo ""
echo "--- High Severity ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=high&per_page=100" \
  --jq '.[] | "  #\(.number) [\(.tool.name)] \(.rule.id) — \(.most_recent_instance.location.path)"' \
  || echo "  (none or API error)"

echo ""
echo "--- Medium Severity ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=medium&per_page=100" \
  --jq '.[] | "  #\(.number) [\(.tool.name)] \(.rule.id) — \(.most_recent_instance.location.path)"' \
  || echo "  (none or API error)"

echo ""
echo "--- Low Severity ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&severity=low&per_page=100" \
  --jq '.[] | "  #\(.number) [\(.tool.name)] \(.rule.id) — \(.most_recent_instance.location.path)"' \
  || echo "  (none or API error)"

echo ""
echo "--- Warning/Note (non-security, code quality) ---"
echo ""
gh api \
  --method GET \
  --paginate \
  "/repos/{owner}/{repo}/code-scanning/alerts?state=open&per_page=100" \
  --jq '
    [.[] | select((.rule.security_severity_level // "") == "")]
    | group_by(.rule.severity)
    | .[]
    | "\(.[0].rule.severity // "unknown") (\(length) alerts):"
  ' \
  || echo "  (none or API error)"

echo ""
echo "Done."
