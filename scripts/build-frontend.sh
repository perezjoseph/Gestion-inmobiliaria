#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_DIR="$ROOT_DIR/frontend"
DIST_DIR="$FRONTEND_DIR/dist"

WASM_OPT="${WASM_OPT:-}"
if [ -z "$WASM_OPT" ]; then
  if command -v wasm-opt &>/dev/null; then
    WASM_OPT="wasm-opt"
  else
    CACHE_DIR="${LOCALAPPDATA:-$HOME/.cache}/trunkrs/trunk/cache"
    WASM_OPT=$(find "$CACHE_DIR" -name "wasm-opt*" -type f 2>/dev/null | head -1 || true)
  fi
fi

echo "[1/2] Building frontend with Trunk (release)..."
cd "$FRONTEND_DIR"
trunk build --release

echo "[2/2] Optimizing WASM binary with wasm-opt..."
WASM_FILE=$(find "$DIST_DIR" -name "*.wasm" -type f | head -1)
if [ -z "$WASM_FILE" ]; then
  echo "ERROR: No .wasm file found in $DIST_DIR"
  exit 1
fi

if [ -n "$WASM_OPT" ]; then
  BEFORE_SIZE=$(wc -c < "$WASM_FILE" | tr -d ' ')
  "$WASM_OPT" -Oz \
    --enable-bulk-memory \
    --enable-nontrapping-float-to-int \
    --output "$WASM_FILE.opt" \
    "$WASM_FILE"
  mv "$WASM_FILE.opt" "$WASM_FILE"
  AFTER_SIZE=$(wc -c < "$WASM_FILE" | tr -d ' ')
  echo "  Before: $((BEFORE_SIZE / 1024)) KB → After: $((AFTER_SIZE / 1024)) KB"
else
  echo "  wasm-opt not found, skipping (Cargo release profile still applies)"
fi

echo "Done! Output in $DIST_DIR"
