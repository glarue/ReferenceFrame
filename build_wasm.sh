#!/bin/bash
# Build script for ReferenceFrame WASM
# Ensures WASM is built to the correct location every time
#
# IMPORTANT: There is only ONE pkg directory: platforms/web/pkg/
# This is where index.html loads from (via ./pkg/ relative path)
# Do not create platforms/pkg/ or any other pkg directories!

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building ReferenceFrame WASM...${NC}"

# Setup PATH for Rust toolchain
export PATH="/home/glarue/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:/home/glarue/.cargo/bin:/usr/bin:/bin:$PATH"

# Build WASM
echo -e "${BLUE}Running wasm-pack build...${NC}"
wasm-pack build --target web platforms/web/wasm_bindings --out-dir ../pkg

# Verify output
if [ ! -f "platforms/web/pkg/referenceframe_wasm_bg.wasm" ]; then
    echo "ERROR: WASM build failed - file not found"
    exit 1
fi

echo -e "${GREEN}✓ WASM built successfully to platforms/web/pkg/${NC}"
echo -e "${BLUE}Build timestamp: $(date)${NC}"
ls -lh platforms/web/pkg/*.wasm
