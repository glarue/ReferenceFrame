# ReferenceFrame

A cross-platform picture frame design calculator with support for Web, iOS, and Android.

## 🏗️ Architecture

ReferenceFrame uses a **shared Rust core** architecture for maximum code reuse across platforms:

```
ReferenceFrame/
├── core/                      # 🎯 Pure Rust business logic (platform-agnostic)
│   ├── src/                   # Frame calculations, validation, SVG generation
│   └── Cargo.toml
│
├── platforms/
│   ├── web/                   # 🌐 Web app (WASM)
│   │   ├── wasm_bindings/     # Thin WASM wrapper
│   │   ├── index.html         # Web UI
│   │   ├── styles.css
│   │   ├── pkg/               # Generated WASM output
│   │   └── build.sh           # Build script
│   │
│   └── mobile/                # 📱 Flutter app (iOS + Android) - Planned
│       └── README.md
│
└── legacy/                    # Archived code
    └── pyscript/              # Original PyScript version
```

## 🚀 Quick Start

### Web App (Production)

**Live URL:** https://glarue.github.io/ReferenceFrame

**Local Development:**
```bash
cd platforms/web
./build.sh              # Build WASM bindings
python serve.py         # Start local server
# Open http://localhost:8887
```

### Development

**Core Library** (platform-agnostic Rust):
```bash
cd core
cargo build
cargo test
```

**WASM Bindings** (web platform):
```bash
cd platforms/web/wasm_bindings
wasm-pack build --target web --out-dir ../pkg
```

## 📦 What's Where

- **`core/`** - All business logic in pure Rust (no platform dependencies)
  - Frame design calculations
  - Input parsing (fractional dimensions)
  - Validation rules
  - SVG visualization generation

- **`platforms/web/`** - Web application
  - `wasm_bindings/` - Thin wrapper adding `#[wasm_bindgen]` to core types
  - `index.html` - Web UI
  - `pkg/` - Generated WASM (created by build script)

- **`platforms/mobile/`** - Future Flutter app (iOS + Android)

## 🎯 Key Benefits

1. **Maximum Code Sharing**: 90%+ of code in platform-agnostic `core/`
2. **Consistent Logic**: Same calculations across web and mobile
3. **Easy Testing**: Core logic tested independently of UI
4. **Future-Proof**: Easy to add new platforms (desktop, CLI, etc.)

## 📝 Development Workflow

1. Make changes to **core logic** in `core/src/`
2. Run tests: `cd core && cargo test`
3. **For web**: Rebuild WASM with `cd platforms/web && ./build.sh`
4. **For mobile** (future): Regenerate FFI bindings

## 🔧 Tech Stack

- **Core**: Rust (pure, no platform dependencies)
- **Web**: Rust → WASM via wasm-bindgen (~220 KB payload)
- **Mobile** (planned): Rust → FFI via flutter_rust_bridge
- **Visualization**: SVG generation in Rust

## 🚢 Deployment

**Web App:** Automatically deployed to GitHub Pages on push to `main` branch
- Deploys from: `platforms/web/`
- URL: https://glarue.github.io/ReferenceFrame
- Workflow: `.github/workflows/deploy.yml`

**Legacy PyScript Version:** Archived in `legacy/pyscript/` (reference only)

## 📄 License

MIT OR Apache-2.0
