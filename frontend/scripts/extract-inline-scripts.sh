#!/bin/sh
# Post-build script: extracts Trunk's inline module script into an external file
# so that the CSP can remain strict (no 'unsafe-inline' for scripts).

DIST_DIR="${TRUNK_STAGING_DIR:-dist}"
INDEX_PATH="$DIST_DIR/index.html"

if [ ! -f "$INDEX_PATH" ]; then
    echo "ERROR: index.html not found at $INDEX_PATH" >&2
    exit 1
fi

# Extract the inline module script content and replace it with an external reference
# Uses sed to find the script type="module" block and extract its content
SCRIPT_CONTENT=$(sed -n '/<script type="module">/,/<\/script>/{/<script type="module">/d;/<\/script>/d;p;}' "$INDEX_PATH")

if [ -z "$SCRIPT_CONTENT" ]; then
    echo "WARNING: No inline module script found to extract"
    exit 0
fi

# Write extracted script to external file
printf '%s' "$SCRIPT_CONTENT" > "$DIST_DIR/trunk-init.js"

# Replace the inline script block with an external script tag
sed -i '/<script type="module">/,/<\/script>/c\<script type="module" src="/trunk-init.js"></script>' "$INDEX_PATH"

echo "Extracted inline module script to $DIST_DIR/trunk-init.js"
