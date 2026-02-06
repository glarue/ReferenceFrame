# ReferenceFrame Architecture

**Last Updated**: 2026-02-06

## Directory Structure (OFFICIAL)

This document defines the **canonical** directory structure. Any deviations from this structure should be treated as errors.

```
ReferenceFrame/
├── core/                       # ✅ OFFICIAL: Pure Rust business logic
│   ├── src/
│   │   ├── lib.rs
│   │   ├── frame.rs            # Frame calculation engine
│   │   ├── conversions.rs      # Unit conversions
│   │   ├── defaults.rs         # Default values
│   │   ├── aspect_ratio.rs     # Aspect ratio locking
│   │   ├── shareable_url.rs    # URL encoding/decoding
│   │   └── visualization/
│   │       ├── mod.rs
│   │       ├── svg.rs          # ⚠️ CRITICAL: SVG generation (inline arrow polygons)
│   │       ├── types.rs
│   │       ├── style.rs
│   │       ├── geometry.rs
│   │       ├── callouts.rs
│   │       └── layout.rs
│   ├── Cargo.toml              # Pure rlib (no platform dependencies)
│   └── tests/
│
├── platforms/                  # ✅ OFFICIAL: Platform-specific implementations
│   ├── web/                    # Web platform (WASM)
│   │   ├── wasm_bindings/      # Thin WASM wrapper
│   │   │   ├── src/lib.rs      # wasm-bindgen annotations
│   │   │   └── Cargo.toml      # Depends on: path = "../../../core"
│   │   ├── pkg/                # Generated WASM output (gitignored)
│   │   ├── index.html          # Web UI
│   │   ├── styles.css
│   │   ├── serve.py
│   │   └── build.sh            # ⚠️ CRITICAL: Use this to build!
│   │
│   └── mobile/                 # ✅ PRODUCTION: Flutter iOS app (separate git repo)
│       ├── lib/                # Dart source
│       ├── rust/               # FFI bridge to core (Cargo.toml)
│       ├── ios/                # Xcode project + Fastlane
│       └── pubspec.yaml        # App version (semver+build)
│
├── legacy/                     # 📦 ARCHIVED: Old implementations
│   └── pyscript/               # Original PyScript version
│
├── src/                        # 📦 TRACKED: Current PyScript version (pre-migration)
│   └── *.py                    # Will be moved to legacy/ after WASM migration
│
├── docs/                       # Documentation
├── tests/                      # Integration tests
├── .github/                    # CI/CD workflows
├── README.md
├── CLAUDE.md
└── ARCHITECTURE.md             # ⬅️ This file

```

## ❌ WRONG DIRECTORIES (Do Not Use)

These directories exist due to exploratory work but should NOT be edited:

- `rust-flutter/` - Experimental, wrong architecture (has wasm-bindgen in core)
- `web-wasm/` - Old experimental web implementation
- Any other untracked Rust directories

**Action**: Move to `legacy/experiments/` to avoid confusion.

## Critical Build Paths

### WASM Build (Web Platform)

**Location**: `/home/glarue/code/ReferenceFrame/platforms/web/`

**Build Script**: `./build.sh`

**Manual Build**:
```bash
cd platforms/web/wasm_bindings
wasm-pack build --target web --out-dir ../pkg
```

**Output**: `platforms/web/pkg/` (this is what index.html loads)

**Core Library Path**: `../../../core` (relative from wasm_bindings/Cargo.toml)

### Core Library Testing

**Location**: `/home/glarue/code/ReferenceFrame/core/`

**Commands**:
```bash
cd core
cargo test
cargo build
```

## Guardrails

### 1. Build Script Validation

The `platforms/web/build.sh` script includes path validation:
```bash
# Verify we're building from the correct core
if [ ! -f "../../../core/Cargo.toml" ]; then
    echo "ERROR: Core library not found at expected path!"
    echo "Expected: /home/glarue/code/ReferenceFrame/core/"
    exit 1
fi
```

### 2. Git Ignore Rules

`.gitignore` prevents committing build artifacts and experimental directories:
```
# Build outputs
/platforms/web/pkg/
/target/
/*/target/

# Experimental directories (do not commit)
/rust-flutter/
/web-wasm/
```

### 3. Cargo.toml Dependency Check

`platforms/web/wasm_bindings/Cargo.toml` MUST have:
```toml
[dependencies]
referenceframe_core = { path = "../../../core" }  # ⚠️ MUST point to core/
```

If this path is wrong, the build will compile the wrong code!

## Common Mistakes

### ❌ WRONG: Editing rust-flutter/rust_core/src/
```bash
# This edits the WRONG directory!
vim /home/glarue/code/ReferenceFrame/rust-flutter/rust_core/src/visualization/svg.rs
```

### ✅ CORRECT: Editing core/src/
```bash
# This edits the OFFICIAL core library
vim /home/glarue/code/ReferenceFrame/core/src/visualization/svg.rs
```

### ❌ WRONG: Building with wrong output path
```bash
cd platforms/web
wasm-pack build --target web --out-dir pkg wasm_bindings  # Wrong! Outputs to wasm_bindings/pkg/
```

### ✅ CORRECT: Building with correct output path
```bash
cd platforms/web
./build.sh  # Uses --out-dir ../pkg from wasm_bindings/
```

## Verification Commands

### Check which core is being compiled:
```bash
cd platforms/web/wasm_bindings
cargo metadata --format-version 1 | grep -o '"core[^"]*"' | head -1
```

### Check WASM output location:
```bash
ls -la platforms/web/pkg/referenceframe_wasm.js  # Should exist
ls -la platforms/web/wasm_bindings/pkg/          # Should NOT exist
```

### Verify HTML loads correct WASM:
```bash
grep "from.*pkg/" platforms/web/index.html
# Should show: './pkg/referenceframe_wasm.js'
```

## Release & Versioning

Three independently versioned scopes, managed by `./release.sh`:

| Scope | Version file | Tag format | Repo |
|-------|-------------|------------|------|
| core | `core/Cargo.toml` | `core-v1.1.0` | root |
| app | `platforms/mobile/pubspec.yaml` | `app-v1.1.0` | mobile |
| bridge | `platforms/mobile/rust/Cargo.toml` | `bridge-v1.0.0` | mobile |

**Conventional commits** enforced by `hooks/commit-msg` (shared via `core.hooksPath`):
- `feat:` → minor, `fix:/perf:` → patch, `feat!:` → major
- `docs: style: refactor: test: build: ci: chore: revert:` → no version bump
- Build numbers (`pubspec.yaml +N`) managed by Fastlane, not release.sh

**Workflow**: `./release.sh` (dry run) → `./release.sh --apply` → `git push --follow-tags`

## Platform Status

- **WASM Web**: Production at https://glarue.github.io/ReferenceFrame
- **iOS Mobile**: Production on App Store (Fastlane deployment)
- **PyScript**: Archived in `legacy/pyscript/`

## Key Files for PDF Export Feature

The current PDF export work involves these files:

1. **`core/src/visualization/svg.rs`** ⚠️ PRIMARY FILE
   - Line 102-135: `generate_arrow_polygon()` - Creates inline polygon arrows
   - Line 153-184: `generate_line_with_arrows()` - Generates lines with arrows
   - Line 1772: `generate_defs()` - No longer includes marker definitions

2. **`platforms/web/index.html`**
   - Line 395: WASM import with cache-busting parameter
   - Line 1583-1780: PDF export JavaScript code

3. **`platforms/web/wasm_bindings/src/lib.rs`**
   - Line 17: Version string for debugging

## Remember

**ALWAYS** edit files in `core/` for business logic changes, **NEVER** in experimental directories!
