# ============================================================================
# Step 1: Fetch all open Trivy alerts from GitHub Code Scanning API
# ============================================================================
# This script fetches all open code scanning alerts filtered to the Trivy tool
# and dumps them to a local JSON file for offline triage.
#
# Prerequisites:
#   - gh CLI authenticated with repo scope
#   - Repository: perezjoseph/Gestion-inmobiliaria
# ============================================================================

# Get the repo slug (verify we're in the right repo)
gh repo view --json nameWithOwner -q .nameWithOwner
# Expected output: perezjoseph/Gestion-inmobiliaria

# Fetch ALL open code scanning alerts (paginated, all tools)
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=open&per_page=100" --paginate > all-alerts.json

# Filter to Trivy-only alerts using PowerShell
$allAlerts = Get-Content all-alerts.json | ConvertFrom-Json
$trivyAlerts = $allAlerts | Where-Object { $_.tool.name -eq "Trivy" }
$trivyAlerts | ConvertTo-Json -Depth 10 | Set-Content trivy-alerts.json

# Summary counts
Write-Host "Total open alerts: $($allAlerts.Count)"
Write-Host "Trivy alerts: $($trivyAlerts.Count)"

# Group by severity
$trivyAlerts | Group-Object { $_.rule.security_severity_level } | Select-Object Name, Count | Format-Table

# Group by SARIF category (tells us which scan produced it)
# Categories in this repo: trivy-backend-image, trivy-frontend-image, trivy-iac
$trivyAlerts | Group-Object { $_.tool.name } | Select-Object Name, Count | Format-Table
