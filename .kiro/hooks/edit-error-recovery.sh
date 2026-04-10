#!/bin/bash
INPUT=$(cat)
RESPONSE=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
resp = data.get('tool_response', {})
success = resp.get('success', True) if isinstance(resp, dict) else True
result = str(resp.get('result', '')) if isinstance(resp, dict) else str(resp)
if not success or 'not found' in result.lower() or 'error' in result.lower() or 'failed' in result.lower():
    print('FAILED')
else:
    print('OK')
" 2>/dev/null)

if [ "$RESPONSE" = "FAILED" ]; then
    echo "[EDIT FAILED - RECOVERY REQUIRED] Re-read the target file, then retry with fresh content." >&2
fi
exit 0
