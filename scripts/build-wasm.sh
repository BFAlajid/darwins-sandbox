#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$PROJECT_DIR/crates/simulation"

echo "Building WASM..."
cd "$CRATE_DIR"
wasm-pack build --target web --out-dir pkg

# Copy artifacts to public/wasm and src/workers/wasm
echo "Copying WASM artifacts..."
mkdir -p "$PROJECT_DIR/public/wasm"
cp pkg/simulation_bg.wasm "$PROJECT_DIR/public/wasm/"

# Only copy JS glue and types if src/workers/wasm exists (after Next.js setup)
if [ -d "$PROJECT_DIR/src/workers/wasm" ]; then
    cp pkg/simulation.js "$PROJECT_DIR/src/workers/wasm/"
    cp pkg/simulation.d.ts "$PROJECT_DIR/src/workers/wasm/"
fi

echo "WASM build complete."
ls -lh "$PROJECT_DIR/public/wasm/simulation_bg.wasm"
