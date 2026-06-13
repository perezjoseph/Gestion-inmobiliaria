#!/usr/bin/env bash
# ============================================================================
# Step 4: Dismiss base-image alerts that are not actionable
# ============================================================================
# After triage, base image alerts with no upstream fix can be dismissed.
# Only dismiss alerts where:
#   - The CVE has no fix available in Alpine repos
#   - The vulnerable code path is not reachable from our application
#
# NOTE: Do NOT run — reference commands only.
# ============================================================================

OWNER="perezjoseph"
REPO="Gestion-inmobiliaria"

# ─────────────────────────────────────────────────────────────────────────────
# Dismiss a single alert as "won't fix" (base image, no upstream patch)
# ─────────────────────────────────────────────────────────────────────────────

dismiss_alert() {
  local alert_number="$1"
  local reason="$2"

  gh api --method PATCH \
    "/repos/${OWNER}/${REPO}/code-scanning/alerts/${alert_number}" \
    -f state="dismissed" \
    -f dismissed_reason="won't fix" \
    -f dismissed_comment="Base image vulnerability (alpine:3.22/3.23). ${reason}"
}

# ─────────────────────────────────────────────────────────────────────────────
# Bulk dismiss all base-image alerts that have no fix available
# ─────────────────────────────────────────────────────────────────────────────

# Extract alert numbers from classified base image file where no fix exists
# (Trivy reports fixedVersion="" when no fix is available)
ALERT_NUMBERS=$(jq -r '.[] | select(
  (.rule.description // "" | test("No fix available|fixed in: <none>"; "i"))
  or (.rule.full_description // "" | test("No fix available"; "i"))
) | .number' classified-base-image.json)

for alert_num in $ALERT_NUMBERS; do
  echo "Dismissing alert #${alert_num} — base image, no upstream fix"
  dismiss_alert "$alert_num" "No fix available in Alpine repos. Will resolve when base image is updated."
done

# ─────────────────────────────────────────────────────────────────────────────
# Re-open previously dismissed alerts if a fix becomes available
# ─────────────────────────────────────────────────────────────────────────────

reopen_alert() {
  local alert_number="$1"

  gh api --method PATCH \
    "/repos/${OWNER}/${REPO}/code-scanning/alerts/${alert_number}" \
    -f state="open"
}

# To check dismissed alerts that might now have fixes:
# gh api --paginate \
#   "/repos/${OWNER}/${REPO}/code-scanning/alerts?state=dismissed&tool_name=Trivy&per_page=100" \
#   --jq '[.[] | select(.dismissed_comment | test("base image"; "i"))]'
