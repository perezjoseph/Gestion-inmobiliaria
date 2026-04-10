#!/bin/bash
INPUT=$(cat)
TOOL=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data.get('tool_name', ''))
" 2>/dev/null)

if [ "$TOOL" != "use_subagent" ] && [ "$TOOL" != "subagent" ]; then
    exit 0
fi

RESPONSE=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
resp = data.get('tool_response', {})
output = resp.get('result', '') if isinstance(resp, dict) else str(resp)
print(output[:2000])
" 2>/dev/null)

if echo "$RESPONSE" | grep -qiE 'error|failed|not found|not available|denied|rejected'; then
    echo "[SUBAGENT DELEGATION FAILED - RETRY REQUIRED]" >&2
    echo "Verify agent name is one of: optimization-agent, security-hardener-agent, explore-agent, lint-fixer-agent" >&2
    echo "Ensure prompt includes all 6 sections. Re-delegate with the specific error." >&2
fi
exit 0
