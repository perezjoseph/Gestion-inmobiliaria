# ============================================================================
# Step 2: Triage Classification — Base Image vs Our Code
# ============================================================================
# This script classifies each Trivy alert into one of these categories:
#
#   1. BASE_IMAGE_OS_PACKAGE  — OS-level CVE in the base image layer
#   2. BASE_IMAGE_NPM_BUNDLED — CVE in npm bundled with the Node base image
#   3. OUR_DOCKERFILE_MISCONFIG — Dockerfile authoring issue (DS-xxxx rules)
#   4. OUR_DEPENDENCY — Vulnerability in a package WE installed
#   5. OUR_CODE — Source code finding (unlikely for Trivy, but possible via IaC)
#
# The classification is based on the `most_recent_instance.location.path` field
# from the GitHub Code Scanning API response.
# ============================================================================

# Load the Trivy alerts
$trivyAlerts = Get-Content trivy-alerts.json | ConvertFrom-Json

# ============================================================================
# CLASSIFICATION FUNCTION
# ============================================================================
function Get-AlertClassification {
    param(
        [Parameter(Mandatory)]
        $Alert
    )

    $path = $Alert.most_recent_instance.location.path
    $ruleId = $Alert.rule.id
    $message = $Alert.most_recent_instance.message.text
    $startLine = $Alert.most_recent_instance.location.start_line

    # ── Heuristic 1: Dockerfile misconfigurations (DS-xxxx rules) ──
    # These are OUR authoring issues in Dockerfiles we maintain.
    if ($ruleId -match "^DS\d{4}") {
        return [PSCustomObject]@{
            Category    = "OUR_DOCKERFILE_MISCONFIG"
            Reason      = "Dockerfile misconfiguration rule $ruleId in $path"
            Action      = "Fix the Dockerfile directly"
            Dockerfile  = $path
        }
    }

    # ── Heuristic 2: Path is <owner>/<image-name> with no subpath ──
    # e.g., "perezjoseph/realestate-backend" or "library/alpine"
    # These are OS-package CVEs reported against the image manifest itself.
    if ($path -match "^[^/]+/[^/]+$" -and $path -notmatch "\.\w+$") {
        return [PSCustomObject]@{
            Category    = "BASE_IMAGE_OS_PACKAGE"
            Reason      = "OS-package CVE in image manifest path: $path"
            Action      = "Bump FROM tag or add 'apk upgrade'/'apt-get upgrade' in Dockerfile"
            Dockerfile  = Get-DockerfileForImage -ImagePath $path
        }
    }

    # ── Heuristic 3: npm bundled with Node base image ──
    # Paths like: usr/local/lib/node_modules/npm/**
    # These come from the npm binary shipped inside node:xx-alpine images.
    if ($path -match "^usr/local/lib/node_modules/npm/") {
        return [PSCustomObject]@{
            Category    = "BASE_IMAGE_NPM_BUNDLED"
            Reason      = "CVE in npm bundled with Node base image at: $path"
            Action      = "Bump node base image tag (node:24-alpine) or add 'npm install -g npm@latest' in Dockerfile"
            Dockerfile  = "baileys-service/Dockerfile"
        }
    }

    # ── Heuristic 4: node_modules inside the app layer ──
    # Paths like: app/node_modules/** — these are OUR dependencies installed by npm ci.
    if ($path -match "^app/node_modules/") {
        return [PSCustomObject]@{
            Category    = "OUR_DEPENDENCY"
            Reason      = "CVE in app dependency at: $path"
            Action      = "Update package-lock.json — run 'npm audit fix' or bump the vulnerable package"
            Dockerfile  = "baileys-service/Dockerfile"
        }
    }

    # ── Heuristic 5: Python site-packages from base image or our install ──
    # Paths like: usr/local/lib/python3.12/site-packages/**
    if ($path -match "^usr/local/lib/python[\d.]+/site-packages/") {
        # Check if it's a package listed in our requirements.txt
        return [PSCustomObject]@{
            Category    = "OUR_DEPENDENCY"
            Reason      = "CVE in Python package at: $path (check requirements.txt)"
            Action      = "Bump version in ocr-service/requirements.txt"
            Dockerfile  = "ocr-service/Dockerfile"
        }
    }

    # ── Heuristic 6: System library paths (base image OS packages) ──
    # Paths under /usr/lib/, /lib/, /usr/share/, etc. with start_line = 1
    if ($path -match "^(usr/(lib|share|bin)|lib/)" -and $startLine -le 1) {
        return [PSCustomObject]@{
            Category    = "BASE_IMAGE_OS_PACKAGE"
            Reason      = "OS library CVE at system path: $path"
            Action      = "Rebuild with newer base image or add OS package upgrade step"
            Dockerfile  = Get-DockerfileForSystemPath -Path $path -Message $message
        }
    }

    # ── Heuristic 7: Go binaries vendored in caddy-builder stage ──
    # Paths in the Go module cache or /usr/bin/caddy
    if ($path -match "(go/pkg/mod|usr/bin/caddy)") {
        return [PSCustomObject]@{
            Category    = "OUR_DEPENDENCY"
            Reason      = "CVE in Go dependency (Caddy build): $path"
            Action      = "Update Go dependency versions in Dockerfile.frontend caddy-builder stage"
            Dockerfile  = "infra/docker/Dockerfile.frontend"
        }
    }

    # ── Heuristic 8: Intel GPU / compute-runtime packages ──
    if ($path -match "(intel-igc|libigdgmm|intel-opencl)") {
        return [PSCustomObject]@{
            Category    = "OUR_DEPENDENCY"
            Reason      = "CVE in Intel compute-runtime package: $path"
            Action      = "Bump Intel compute-runtime release URLs in ocr-service/Dockerfile"
            Dockerfile  = "ocr-service/Dockerfile"
        }
    }

    # ── Fallback: Unclassified ──
    return [PSCustomObject]@{
        Category    = "UNCLASSIFIED"
        Reason      = "Could not auto-classify path: $path (rule: $ruleId)"
        Action      = "Manual review required"
        Dockerfile  = "unknown"
    }
}

# ============================================================================
# HELPER: Map image manifest path to Dockerfile
# ============================================================================
function Get-DockerfileForImage {
    param([string]$ImagePath)

    switch -Regex ($ImagePath) {
        "realestate-backend"  { return "infra/docker/Dockerfile.backend" }
        "realestate-frontend" { return "infra/docker/Dockerfile.frontend" }
        "baileys"             { return "baileys-service/Dockerfile" }
        "ocr"                 { return "ocr-service/Dockerfile" }
        "actions-runner"      { return "infra/docker/Dockerfile.runner" }
        "alpine"              { return "infra/docker/Dockerfile.backend (alpine:3.22 base)" }
        "node"                { return "baileys-service/Dockerfile (node:24-alpine base)" }
        "python"              { return "ocr-service/Dockerfile (python:3.12-slim base)" }
        "golang"              { return "infra/docker/Dockerfile.frontend (golang:1.26.3-alpine caddy-builder)" }
        "rust"                { return "infra/docker/Dockerfile.backend (rust:1.88-alpine build stage)" }
        default               { return "unknown — investigate image: $ImagePath" }
    }
}

# ============================================================================
# HELPER: Map system library path to likely Dockerfile
# ============================================================================
function Get-DockerfileForSystemPath {
    param([string]$Path, [string]$Message)

    # Use message text to identify which image the CVE was found in
    if ($Message -match "realestate-backend") { return "infra/docker/Dockerfile.backend" }
    if ($Message -match "realestate-frontend") { return "infra/docker/Dockerfile.frontend" }
    if ($Message -match "node|baileys") { return "baileys-service/Dockerfile" }
    if ($Message -match "python|ocr") { return "ocr-service/Dockerfile" }

    # Fallback: guess based on path patterns
    if ($Path -match "musl|alpine") { return "infra/docker/Dockerfile.backend (alpine)" }
    if ($Path -match "debian|apt") { return "ocr-service/Dockerfile or infra/docker/Dockerfile.runner" }

    return "unknown — check which image scan produced this alert"
}

# ============================================================================
# RUN CLASSIFICATION ON ALL ALERTS
# ============================================================================
$results = $trivyAlerts | ForEach-Object {
    $classification = Get-AlertClassification -Alert $_
    [PSCustomObject]@{
        AlertNumber = $_.number
        RuleId      = $_.rule.id
        Severity    = $_.rule.security_severity_level
        Path        = $_.most_recent_instance.location.path
        Category    = $classification.Category
        Reason      = $classification.Reason
        Action      = $classification.Action
        Dockerfile  = $classification.Dockerfile
    }
}

# ============================================================================
# OUTPUT: Summary by category
# ============================================================================
Write-Host "`n====== TRIAGE SUMMARY ======"
$results | Group-Object Category | Select-Object Name, Count | Format-Table -AutoSize

# ============================================================================
# OUTPUT: Base image alerts (not directly fixable in our code)
# ============================================================================
Write-Host "`n====== BASE IMAGE ALERTS (fix by bumping FROM tag) ======"
$results | Where-Object { $_.Category -in "BASE_IMAGE_OS_PACKAGE", "BASE_IMAGE_NPM_BUNDLED" } |
    Format-Table AlertNumber, Severity, RuleId, Path, Dockerfile -AutoSize -Wrap

# ============================================================================
# OUTPUT: Our code / dependency alerts (fix in source)
# ============================================================================
Write-Host "`n====== OUR CODE / DEPENDENCY ALERTS (fix in source) ======"
$results | Where-Object { $_.Category -in "OUR_DEPENDENCY", "OUR_DOCKERFILE_MISCONFIG", "OUR_CODE" } |
    Format-Table AlertNumber, Severity, RuleId, Path, Action -AutoSize -Wrap

# Export full results to JSON for further processing
$results | ConvertTo-Json -Depth 5 | Set-Content trivy-triage-results.json
Write-Host "`nFull results written to trivy-triage-results.json"
