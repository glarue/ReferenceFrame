#!/bin/bash
# Build script for ReferenceFrame web platform

set -euo pipefail  # Exit on error, unset vars, and pipeline failures

# Always run from platforms/web/ (where this script lives), regardless of cwd
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Ensure cargo and wasm-pack are in PATH
export PATH="$HOME/.cargo/bin:$PATH"

echo "🔍 Validating build environment..."

# Sanity check: this script should live in platforms/web/ alongside wasm_bindings/
if [ ! -f "wasm_bindings/Cargo.toml" ]; then
    echo "❌ ERROR: wasm_bindings/Cargo.toml not found next to this script"
    echo "Current directory: $(pwd)"
    exit 1
fi

# Verify core library exists at expected path
CORE_PATH="../../core"
if [ ! -f "$CORE_PATH/Cargo.toml" ]; then
    echo "❌ ERROR: Core library not found at expected path!"
    echo "Expected: $(cd .. && cd .. && pwd)/core/"
    echo ""
    echo "The build MUST use the official core library at:"
    echo "  $(cd ../.. && pwd)/core/"
    echo ""
    echo "NOT experimental directories like rust-flutter/rust_core/"
    exit 1
fi

# Verify wasm_bindings depends on correct core
if ! grep -q 'path = "\.\./\.\./\.\./core"' wasm_bindings/Cargo.toml; then
    echo "❌ ERROR: wasm_bindings/Cargo.toml has wrong core dependency path!"
    echo "Expected: path = \"../../../core\""
    grep "referenceframe_core" wasm_bindings/Cargo.toml || true
    exit 1
fi

echo "✅ Build environment validated"
echo ""
echo "🔨 Building WASM bindings..."
cd wasm_bindings
wasm-pack build --target web --out-dir ../pkg
cd ..

echo "✅ Build complete! WASM package generated at platforms/web/pkg/"
echo ""
echo "To run the web app:"
echo "  python serve.py"
echo "  Then open http://localhost:8000"
