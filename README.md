# ReferenceFrame

A cross-platform picture frame design calculator with support for Web, iOS, and Android.

![ReferenceFrame Web Interface](docs/images/main_interface.png)

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
- **Web**: Rust → WASM via wasm-bindgen (~314 KB payload, 50-100× smaller than PyScript)
- **Mobile** (planned): Rust → FFI via flutter_rust_bridge
- **Visualization**: Pure SVG generation in Rust (vector graphics, crisp at any zoom)
- **PDF Export**: jsPDF + svg2pdf.js with embedded vector diagrams

## ✨ Web App Features

- ✅ Full frame design calculations (dimensions, cut list, depth analysis)
- ✅ Interactive SVG visualizations (plan + section views)
- ✅ Professional PDF export with embedded vector diagrams and QR codes
- ✅ Text export for cut lists
- ✅ Unit conversion (inches ↔ mm)
- ✅ Shareable URLs (28-byte encoded designs)
- ✅ Saved configurations (localStorage)
- ✅ Aspect ratio locking
- ✅ Responsive design (desktop + mobile)

## 🚢 Deployment

**Web App:** Automatically deployed to GitHub Pages on push to `main` branch

**Deployment Process:**
1. GitHub Actions checks out code
2. Sets up Rust toolchain and wasm-pack
3. Builds WASM binaries from `platforms/web/wasm_bindings/`
4. Deploys static files (HTML, CSS, JS, WASM) to GitHub Pages

**Key Details:**
- URL: https://glarue.github.io/ReferenceFrame
- Workflow: `.github/workflows/deploy.yml`
- Build time: ~45 seconds
- No server required (fully static)

**Legacy PyScript Version:** Archived in `legacy/pyscript/` (reference implementation, replaced by WASM in Jan 2026)

## 📄 License

MIT OR Apache-2.0
