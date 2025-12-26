#!/bin/bash
# Build script for the WASM engine

set -e

echo "Building editor-engine WASM module..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Build the WASM module
wasm-pack build --target web --out-dir ../src/lib/engine-wasm

echo "Build complete! Output in src/lib/engine-wasm/"
