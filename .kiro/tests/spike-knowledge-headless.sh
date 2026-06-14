#!/usr/bin/env bash
set -euo pipefail

RESULT_FILE="${RUNNER_TEMP:-/tmp}/spike-knowledge-result.txt"
KB_SEARCH_MARKER="${RUNNER_TEMP:-/tmp}/spike-kb-search-ok"
KB_STORE_MARKER="${RUNNER_TEMP:-/tmp}/spike-kb-store-ok"
rm -f "$RESULT_FILE" "$KB_SEARCH_MARKER" "$KB_STORE_MARKER"

kiro-cli settings chat.enableKnowledge true --global
kiro-cli settings knowledge.indexType Fast --global

STORE_PROMPT="Use the knowledge tool to store the following: '[artifact: spike-test] [diag_sig: spike_validation] This is a validation entry from the headless knowledge spike. Fix that worked: confirmed store operation functions in headless mode.' After storing, confirm by saying STORE_OK."

set +e
STORE_OUTPUT=$(kiro-cli chat --agent autofix --no-interactive --trust-all-tools "$STORE_PROMPT" 2>&1)
STORE_EXIT=$?
set -e

if [ $STORE_EXIT -eq 0 ] && echo "$STORE_OUTPUT" | grep -qi "STORE_OK"; then
  touch "$KB_STORE_MARKER"
fi

SEARCH_PROMPT="Use the knowledge tool to search for 'spike-test spike_validation'. Report what you find. If you find the spike validation entry, say SEARCH_OK."

set +e
SEARCH_OUTPUT=$(kiro-cli chat --agent autofix --no-interactive --trust-all-tools "$SEARCH_PROMPT" 2>&1)
SEARCH_EXIT=$?
set -e

if [ $SEARCH_EXIT -eq 0 ] && echo "$SEARCH_OUTPUT" | grep -qi "SEARCH_OK"; then
  touch "$KB_SEARCH_MARKER"
fi

XDG_DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
KB_PATHS_FOUND=""
for candidate in \
  "$XDG_DATA/kiro-cli/knowledge_bases" \
  "$HOME/.local/share/kiro-cli/knowledge_bases" \
  "$HOME/.local/share/kiro/knowledge_bases" \
  "$HOME/.kiro/knowledge_bases" \
  "/opt/kiro/knowledge_bases"; do
  if [ -d "$candidate" ]; then
    KB_PATHS_FOUND="${KB_PATHS_FOUND}${candidate}\n"
  fi
done

if [ -z "$KB_PATHS_FOUND" ]; then
  KB_PATHS_FOUND=$(find "$HOME" /opt /var -maxdepth 4 -type d -name "knowledge_bases" 2>/dev/null || true)
fi

{
  echo "=== SPIKE: kiro-cli knowledge tool headless validation ==="
  echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo ""
  echo "--- Store operation ---"
  if [ -f "$KB_STORE_MARKER" ]; then
    echo "RESULT: PASS"
  else
    echo "RESULT: FAIL"
    echo "Exit code: $STORE_EXIT"
    echo "Output (last 40 lines):"
    echo "$STORE_OUTPUT" | tail -40
  fi
  echo ""
  echo "--- Search operation ---"
  if [ -f "$KB_SEARCH_MARKER" ]; then
    echo "RESULT: PASS"
  else
    echo "RESULT: FAIL"
    echo "Exit code: $SEARCH_EXIT"
    echo "Output (last 40 lines):"
    echo "$SEARCH_OUTPUT" | tail -40
  fi
  echo ""
  echo "--- KB store path discovery ---"
  if [ -n "$KB_PATHS_FOUND" ]; then
    echo "Discovered path(s):"
    printf '%b\n' "$KB_PATHS_FOUND"
  else
    echo "No knowledge_bases directory found on this image."
  fi
  echo ""
  echo "=== GATE DECISION ==="
  if [ -f "$KB_STORE_MARKER" ] && [ -f "$KB_SEARCH_MARKER" ]; then
    echo "PASS — knowledge tool functions headless. Proceed with F10."
  else
    echo "FAIL — knowledge tool does NOT function headless. Do NOT proceed with F10 infra work."
  fi
} | tee "$RESULT_FILE"

rm -f "$KB_SEARCH_MARKER" "$KB_STORE_MARKER"

if grep -q "^PASS" <<< "$(grep "GATE DECISION" -A1 "$RESULT_FILE" | tail -1)"; then
  exit 0
else
  exit 1
fi
