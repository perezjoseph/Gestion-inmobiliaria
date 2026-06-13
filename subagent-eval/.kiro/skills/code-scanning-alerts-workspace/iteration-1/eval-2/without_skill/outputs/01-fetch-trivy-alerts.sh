#!/usr/bin/env bash
# ============================================================================
# Step 1: Fetch all Trivy code-scanning alerts from GitHub
# ============================================================================
# This script fetches alerts from the three Trivy SARIF categories uploaded
# by the CI pipeline:
#   - trivy-backend-image  (container scan of backend image)
#   - trivy-frontend-image (container scan of frontend image)
#   - trivy-iac            (IaC misconfiguration scan of infra/docker/)
#
# NOTE: Do NOT run — these are the exact commands for reference.
# ============================================================================

OWNER="perezjoseph"
REPO="Gestion-inmobiliaria"

# Fetch ALL open Trivy alerts (tool_name filter = Trivy)
gh api --paginate \
  "/repos/${OWNER}/${REPO}/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  > trivy-alerts-raw.json

# Alternatively, fetch by specific SARIF category:
# Backend container image alerts
gh api --paginate \
  "/repos/${OWNER}/${REPO}/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  --jq '[.[] | select(.rule.tags // [] | any(. == "trivy-backend-image") or .most_recent_instance.category == "trivy-backend-image")]' \
  > trivy-backend-image-alerts.json

# Frontend container image alerts
gh api --paginate \
  "/repos/${OWNER}/${REPO}/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  --jq '[.[] | select(.most_recent_instance.category == "trivy-frontend-image")]' \
  > trivy-frontend-image-alerts.json

# IaC misconfiguration alerts
gh api --paginate \
  "/repos/${OWNER}/${REPO}/code-scanning/alerts?state=open&tool_name=Trivy&per_page=100" \
  --jq '[.[] | select(.most_recent_instance.category == "trivy-iac")]' \
  > trivy-iac-alerts.json

# Get detailed info for a single alert (substitute ALERT_NUMBER)
# gh api "/repos/${OWNER}/${REPO}/code-scanning/alerts/{ALERT_NUMBER}"
# gh api "/repos/${OWNER}/${REPO}/code-scanning/alerts/{ALERT_NUMBER}/instances"
