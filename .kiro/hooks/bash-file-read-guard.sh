#!/bin/bash
INPUT=$(cat)
CMD=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data.get('tool_input', {}).get('command', ''))
" 2>/dev/null)

if echo "$CMD" | grep -qP '^\s*(cat|head|tail|less|more)\s+[^|&;]+$'; then
    echo "BLOCKED: Use the read tool instead of '${CMD}'. The read tool provides line numbers and better context for editing." >&2
    exit 2
fi
exit 0
