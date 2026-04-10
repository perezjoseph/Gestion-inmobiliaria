#!/bin/bash
# Reads JSON from STDIN, extracts the file path, and runs rustfmt on .rs files
INPUT=$(cat)
echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
ops = data.get('tool_input', {}).get('operations', [])
for op in ops:
    path = op.get('path', '')
    if path.endswith('.rs'):
        print(path)
" | while read -r filepath; do
    if [ -f "$filepath" ]; then
        rustfmt "$filepath" 2>/dev/null
    fi
done
