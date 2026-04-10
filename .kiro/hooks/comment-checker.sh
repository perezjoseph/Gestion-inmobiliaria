#!/bin/bash
# Reads JSON from STDIN, extracts .rs file paths, checks for AI slop comments
# Exits 0 always (informational only, doesn't block writes)
INPUT=$(cat)
FILES=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
ops = data.get('tool_input', {}).get('operations', [])
for op in ops:
    path = op.get('path', '')
    if path.endswith('.rs'):
        print(path)
" 2>/dev/null)

for filepath in $FILES; do
    if [ -f "$filepath" ]; then
        # Check for common AI slop comment patterns
        SLOP=$(grep -n -E '^\s*//' "$filepath" | grep -iE \
            'this function|this method|this struct|handle the|import the|we need to|let.s|here we|now we|finally we|first we|TODO|FIXME|HACK|XXX' \
            2>/dev/null)
        if [ -n "$SLOP" ]; then
            echo "COMMENT_CHECK: $filepath has potential AI slop comments:"
            echo "$SLOP"
        fi
    fi
done
