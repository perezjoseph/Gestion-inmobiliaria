#!/bin/bash
# preToolUse hook for write tool — warns if overwriting existing file
# Exit 0 = allow (warning only), stderr shown to user
INPUT=$(cat)
FILEPATH=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
ops = data.get('tool_input', {}).get('operations', [])
for op in ops:
    path = op.get('path', '')
    if path:
        print(path)
        break
" 2>/dev/null)

if [ -n "$FILEPATH" ] && [ -f "$FILEPATH" ]; then
    echo "WARNING: Writing to existing file '$FILEPATH'. Ensure you have read it first. Prefer targeted edits over full overwrites." >&2
fi
exit 0
