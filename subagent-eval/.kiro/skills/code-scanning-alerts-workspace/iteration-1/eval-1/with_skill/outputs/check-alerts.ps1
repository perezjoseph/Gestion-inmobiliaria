# check-alerts.ps1
# Complete workflow to check open code scanning alerts and group by severity.
# Based on the code-scanning-alerts skill instructions.
#
# IMPORTANT: This script uses `gh api` which requires:
#   - GitHub CLI (`gh`) installed and authenticated
#   - Repository must have code scanning enabled (CodeQL, Trivy, or Semgrep)

# --- Step 1: Get the repository slug ---
Write-Host "=== Step 1: Determining repository ===" -ForegroundColor Cyan
$repoSlug = gh repo view --json nameWithOwner -q .nameWithOwner
Write-Host "Repository: $repoSlug"

# --- Step 2: Fetch all open alerts to a local JSON file ---
Write-Host "`n=== Step 2: Fetching open code scanning alerts ===" -ForegroundColor Cyan
$alertsFile = "alerts.json"
gh api "/repos/$repoSlug/code-scanning/alerts?state=open&per_page=100" --paginate > $alertsFile
Write-Host "Alerts saved to $alertsFile"

# --- Step 3: Parse and display grouped by severity ---
Write-Host "`n=== Step 3: Parsing alerts ===" -ForegroundColor Cyan
$alerts = Get-Content $alertsFile | ConvertFrom-Json

Write-Host "`nTotal open alerts: $($alerts.Count)" -ForegroundColor Yellow

Write-Host "`n--- Grouped by Severity ---" -ForegroundColor Green
$alerts | Group-Object {$_.rule.security_severity_level} |
  Sort-Object @{Expression={
    switch ($_.Name) {
      'critical' { 0 }
      'high'     { 1 }
      'medium'   { 2 }
      'low'      { 3 }
      default    { 4 }
    }
  }} |
  Select-Object Name, Count |
  Format-Table -AutoSize

Write-Host "--- Grouped by Tool ---" -ForegroundColor Green
$alerts | Group-Object {$_.tool.name} |
  Select-Object Name, Count |
  Format-Table -AutoSize

# --- Step 3d: Show critical and high alerts in detail ---
$criticalHigh = $alerts | Where-Object {$_.rule.security_severity_level -in 'critical','high'}
if ($criticalHigh.Count -gt 0) {
  Write-Host "--- Critical & High Severity Details ---" -ForegroundColor Red
  $criticalHigh |
    Select-Object number,
      @{n='rule';e={$_.rule.id}},
      @{n='severity';e={$_.rule.security_severity_level}},
      @{n='path';e={$_.most_recent_instance.location.path}},
      @{n='line';e={$_.most_recent_instance.location.start_line}},
      @{n='tool';e={$_.tool.name}} |
    Format-Table -AutoSize -Wrap
} else {
  Write-Host "`nNo critical or high severity alerts found." -ForegroundColor Green
}

# --- Step 4: Clean up ---
Write-Host "`n=== Step 4: Cleanup ===" -ForegroundColor Cyan
Remove-Item $alertsFile
Write-Host "Removed $alertsFile"
Write-Host "`nDone." -ForegroundColor Green
