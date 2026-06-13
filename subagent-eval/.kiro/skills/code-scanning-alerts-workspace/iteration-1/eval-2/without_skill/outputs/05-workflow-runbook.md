# Trivy Alert Triage Runbook

Complete workflow for triaging Trivy code-scanning alerts in this repository.

---

## Prerequisites

- `gh` CLI authenticated with `security-events: read` (and `write` for dismissals)
- `jq` installed for JSON processing
- Repository: `perezjoseph/Gestion-inmobiliaria`

---

## Workflow Steps

### Step 1: Fetch All Open Trivy Alerts

```bash
gh api --paginate \
  "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  > trivy-alerts-raw.json
```

### Step 2: Understand the Alert Categories

This project uploads Trivy results under three SARIF categories:

| Category | Source Workflow | Scan Target |
|---|---|---|
| `trivy-backend-image` | `containers.yml` | `ghcr.io/.../realestate-backend@sha256:...` |
| `trivy-frontend-image` | `containers.yml` | `ghcr.io/.../realestate-frontend@sha256:...` |
| `trivy-iac` | `security.yml` | `infra/docker/` (filesystem misconfig scan) |

### Step 3: Classify Each Alert

Apply the decision tree:

```
1. Category == "trivy-iac"           → OUR CODE (Dockerfile misconfiguration)
2. Package type == OS (apk/deb)      → BASE IMAGE
3. Path == /usr/local/bin/realestate-backend → OUR CODE (Rust deps in binary)
4. Path == /usr/bin/caddy            → OUR CODE (Go deps we build)
5. Path under /srv/ or /app/         → OUR CODE
6. Go module vulnerability           → OUR CODE (Caddy build)
7. Everything else                   → BASE IMAGE
```

Run `03-classify-alerts.sh` to automate this classification.

### Step 4: Prioritize Our Code Alerts

For alerts classified as "Our Code", prioritize by:

1. **CRITICAL severity** — fix immediately
2. **HIGH severity** — fix within current sprint
3. **MEDIUM severity** — schedule for next sprint

Fixes:
- **Rust crate vuln**: `cargo update -p <vulnerable-crate>`, rebuild, redeploy
- **Go module vuln in Caddy**: Update the `go get` version pins in `infra/docker/Dockerfile.frontend` caddy-builder stage
- **Dockerfile misconfig**: Edit the Dockerfile directly (e.g., add missing security directives)

### Step 5: Handle Base Image Alerts

For alerts classified as "Base Image":

1. **Check if a fix exists**: Look at `fixedVersion` in the alert details
2. **If fix available**: Bump the Alpine base image tag or add `apk upgrade` in Dockerfile
3. **If no fix**: Dismiss with reason "won't fix" and comment explaining it's upstream
4. **If not exploitable**: Dismiss with reason "used in tests" or "won't fix" with exploitability note

### Step 6: Track Dismissed Alerts for Re-evaluation

Periodically (e.g., weekly via scheduled workflow or manual check):

```bash
# Find dismissed base-image alerts
gh api --paginate \
  "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=dismissed&tool_name=Trivy&per_page=100" \
  --jq '[.[] | select(.dismissed_comment // "" | test("base image|Base image|alpine"; "i"))] | length'
```

When upgrading base images, re-open and re-scan to clear fixed ones.

---

## Quick Reference: Base Images in This Project

| Dockerfile | Final Runtime Base | Packages We Add |
|---|---|---|
| `Dockerfile.backend` | `alpine:3.22` | `ca-certificates`, `libssl3`, `libpq`, `curl` |
| `Dockerfile.frontend` | `alpine:3.23` | `ca-certificates`, `mailcap`, `wget` |

### Attack Surface Reduction Opportunities

- Remove `curl` from backend if healthcheck can use a static binary or wget
- Remove `wget` from frontend if healthcheck can use caddy's built-in health endpoint
- Fewer packages = fewer base-image CVEs to triage

---

## Example: Full Triage Session

```bash
# 1. Fetch
gh api --paginate \
  "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  > trivy-alerts-raw.json

# 2. Count by category
jq 'group_by(.most_recent_instance.category) | map({category: .[0].most_recent_instance.category, count: length})' trivy-alerts-raw.json

# 3. Classify
./03-classify-alerts.sh

# 4. Review our-code alerts
jq '.[] | {number, severity: .rule.severity, rule: .rule.id, path: .most_recent_instance.location.path}' classified-our-code.json

# 5. Fix what we can, dismiss what we can't
./04-dismiss-base-image-alerts.sh

# 6. Verify counts after triage
gh api "/repos/perezjoseph/Gestion-inmobiliaria/code-scanning/alerts?state=open&tool_name=Trivy" \
  --jq 'length'
```

---

## Automation Opportunities

1. **Nightly scheduled workflow** that runs this triage and posts a summary to a Slack channel or GitHub issue
2. **PR comment** on Dockerfile changes showing new vs resolved Trivy findings
3. **Dependabot-style auto-PR** when a base image has a security update available
4. **Auto-dismiss** base image alerts older than 90 days with no upstream fix (with team approval)
