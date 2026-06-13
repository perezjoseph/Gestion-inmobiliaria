#!/usr/bin/env bash
# ============================================================================
# Step 3: Classify Trivy alerts into Base Image vs Our Code
# ============================================================================
# This script processes the raw alerts JSON and classifies each one.
# Requires: jq, gh CLI (assumes trivy-alerts-raw.json already fetched)
#
# NOTE: Do NOT run — reference commands only.
# ============================================================================

OWNER="perezjoseph"
REPO="Gestion-inmobiliaria"
INPUT="trivy-alerts-raw.json"
OUTPUT_BASE_IMAGE="classified-base-image.json"
OUTPUT_OUR_CODE="classified-our-code.json"
OUTPUT_SUMMARY="triage-summary.md"

# ─────────────────────────────────────────────────────────────────────────────
# Classification via jq
# ─────────────────────────────────────────────────────────────────────────────

# Classify as OUR CODE: IaC findings, Rust crates, Go modules in Caddy, app paths
jq '[.[] | select(
  (.most_recent_instance.category == "trivy-iac")
  or (.rule.description // "" | test("rust|cargo|crate"; "i"))
  or (.most_recent_instance.location.path // "" | test("^/usr/local/bin/realestate-backend"))
  or (.most_recent_instance.location.path // "" | test("^/usr/bin/caddy"))
  or (.most_recent_instance.location.path // "" | test("^/srv/"))
  or (.most_recent_instance.location.path // "" | test("^/app/"))
  or (.most_recent_instance.location.path // "" | test("^/etc/caddy/"))
  or (.rule.tags // [] | any(test("lang-go|lang-rust")))
)]' "$INPUT" > "$OUTPUT_OUR_CODE"

# Classify as BASE IMAGE: everything else (OS packages, system paths)
jq '[.[] | select(
  (.most_recent_instance.category != "trivy-iac")
  and ((.rule.description // "" | test("rust|cargo|crate"; "i")) | not)
  and ((.most_recent_instance.location.path // "" | test("^/usr/local/bin/realestate-backend")) | not)
  and ((.most_recent_instance.location.path // "" | test("^/usr/bin/caddy")) | not)
  and ((.most_recent_instance.location.path // "" | test("^/srv/")) | not)
  and ((.most_recent_instance.location.path // "" | test("^/app/")) | not)
  and ((.most_recent_instance.location.path // "" | test("^/etc/caddy/")) | not)
  and ((.rule.tags // [] | any(test("lang-go|lang-rust"))) | not)
)]' "$INPUT" > "$OUTPUT_BASE_IMAGE"

# ─────────────────────────────────────────────────────────────────────────────
# Generate summary report
# ─────────────────────────────────────────────────────────────────────────────

TOTAL=$(jq 'length' "$INPUT")
OUR_CODE_COUNT=$(jq 'length' "$OUTPUT_OUR_CODE")
BASE_IMAGE_COUNT=$(jq 'length' "$OUTPUT_BASE_IMAGE")

cat > "$OUTPUT_SUMMARY" << EOF
# Trivy Alert Triage Summary

**Total Open Alerts:** ${TOTAL}
**Our Code:** ${OUR_CODE_COUNT}
**Base Image:** ${BASE_IMAGE_COUNT}

## Our Code Alerts (Action Required)

| # | Severity | Rule | Location | Category |
|---|----------|------|----------|----------|
$(jq -r '.[] | "| \(.number) | \(.rule.severity // "unknown") | \(.rule.id) | \(.most_recent_instance.location.path // "N/A") | \(.most_recent_instance.category) |"' "$OUTPUT_OUR_CODE")

## Base Image Alerts (Upstream)

| # | Severity | Rule | Location | Category |
|---|----------|------|----------|----------|
$(jq -r '.[] | "| \(.number) | \(.rule.severity // "unknown") | \(.rule.id) | \(.most_recent_instance.location.path // "N/A") | \(.most_recent_instance.category) |"' "$OUTPUT_BASE_IMAGE")

## Recommended Actions

### For Our Code alerts:
- Rust binary vulns: \`cargo update -p <crate>\` then rebuild
- Caddy Go module vulns: Update pinned versions in Dockerfile.frontend caddy-builder stage
- IaC misconfigs: Fix the Dockerfile directly

### For Base Image alerts:
- Bump alpine tag in Dockerfiles if patches are available
- For unpatched CVEs: assess exploitability, dismiss with justification if not reachable
EOF

echo "Triage complete. See ${OUTPUT_SUMMARY}"
