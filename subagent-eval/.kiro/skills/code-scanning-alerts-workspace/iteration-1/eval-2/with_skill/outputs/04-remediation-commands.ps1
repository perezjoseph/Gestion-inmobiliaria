# ============================================================================
# Step 3: Remediation Commands
# ============================================================================
# After classification, these are the exact gh api commands and workflows
# you would run to act on each category of Trivy alert.
# ============================================================================

# ────────────────────────────────────────────────────────────────────────────
# A) DISMISS base-image alerts that have no upstream fix yet
# ────────────────────────────────────────────────────────────────────────────

# Example: Dismiss a base-image OS package alert (no fix available upstream)
gh api -X PATCH "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/<ALERT_NUMBER>" `
  -f state=dismissed `
  -f dismissed_reason="won't_fix" `
  -f dismissed_comment="OS-package CVE in alpine:3.22 base image. No patched version available upstream yet. Will resolve on next base image bump."

# Example: Dismiss a bundled-npm alert (unreachable at runtime)
gh api -X PATCH "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/<ALERT_NUMBER>" `
  -f state=dismissed `
  -f dismissed_reason="won't_fix" `
  -f dismissed_comment="CVE in npm bundled dep (node:24-alpine). npm only runs at build time, vulnerable code path not reachable at runtime."

# ────────────────────────────────────────────────────────────────────────────
# B) GET DETAILS on a specific alert for deeper investigation
# ────────────────────────────────────────────────────────────────────────────

# Fetch full detail for a single alert
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/<ALERT_NUMBER>"

# Fetch all instances (occurrences) of an alert
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/<ALERT_NUMBER>/instances?per_page=100" --paginate

# ────────────────────────────────────────────────────────────────────────────
# C) BATCH DISMISS all base-image alerts (after manual review)
# ────────────────────────────────────────────────────────────────────────────

# Load triage results and dismiss all BASE_IMAGE_OS_PACKAGE alerts
$results = Get-Content trivy-triage-results.json | ConvertFrom-Json
$baseImageAlerts = $results | Where-Object { $_.Category -eq "BASE_IMAGE_OS_PACKAGE" }

foreach ($alert in $baseImageAlerts) {
    Write-Host "Dismissing alert #$($alert.AlertNumber): $($alert.RuleId)"
    # UNCOMMENT TO EXECUTE:
    # gh api -X PATCH "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts/$($alert.AlertNumber)" `
    #   -f state=dismissed `
    #   -f dismissed_reason="won't_fix" `
    #   -f dismissed_comment="Base image OS package CVE. Fix by bumping FROM tag in $($alert.Dockerfile)."
}

# ────────────────────────────────────────────────────────────────────────────
# D) GENERATE a fix PR for base image bumps
# ────────────────────────────────────────────────────────────────────────────

# For alpine-based images: bump FROM alpine:3.22 -> alpine:3.23
# Files to update:
#   - infra/docker/Dockerfile.backend (final stage)
#   - infra/docker/Dockerfile.frontend (final stage)

# For node-based images: bump to latest patch
# Files to update:
#   - baileys-service/Dockerfile (all 3 stages: deps, build, runtime)

# For python-based images: bump to latest patch
# Files to update:
#   - ocr-service/Dockerfile (both builder and final stage)

# ────────────────────────────────────────────────────────────────────────────
# E) CHECK if alerts auto-resolve after rebuild
# ────────────────────────────────────────────────────────────────────────────

# After pushing a Dockerfile change and the containers workflow runs,
# Trivy will re-scan the new image. Alerts for fixed CVEs will auto-transition
# to state=fixed. Verify:

gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=fixed&tool_name=Trivy&per_page=100" `
  --paginate > fixed-trivy-alerts.json

$fixed = Get-Content fixed-trivy-alerts.json | ConvertFrom-Json
Write-Host "Trivy alerts auto-fixed after rebuild: $($fixed.Count)"
