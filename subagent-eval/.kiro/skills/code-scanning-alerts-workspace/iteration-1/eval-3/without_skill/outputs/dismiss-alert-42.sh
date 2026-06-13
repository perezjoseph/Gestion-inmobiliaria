#!/bin/bash
# Dismiss code scanning alert #42 as a false positive because it's in a test file

# Step 1: Get alert details to confirm it exists and check its current state
gh api \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/{owner}/{repo}/code-scanning/alerts/42

# Step 2: Dismiss alert #42 as false positive with reason
gh api \
  --method PATCH \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/{owner}/{repo}/code-scanning/alerts/42 \
  -f state='dismissed' \
  -f dismissed_reason='false positive' \
  -f dismissed_comment='This alert is in a test file and does not represent a real security risk.'
