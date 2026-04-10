#!/bin/bash
# Reads JSON from STDIN, determines which crate was modified, runs clippy on that crate
INPUT=$(cat)
CRATES=$(echo "$INPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
ops = data.get('tool_input', {}).get('operations', [])
seen = set()
for op in ops:
    path = op.get('path', '')
    if path.endswith('.rs'):
        if 'backend' in path and 'backend' not in seen:
            seen.add('backend')
            print('realestate-backend')
        elif 'frontend' in path and 'frontend' not in seen:
            seen.add('frontend')
            print('realestate-frontend')
" 2>/dev/null)

for crate in $CRATES; do
    cargo clippy -p "$crate" -- -D warnings 2>&1
done
