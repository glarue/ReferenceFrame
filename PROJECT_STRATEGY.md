# ReferenceFrame Project Strategy

## Overview

ReferenceFrame uses a **shared Rust core** architecture to support multiple platforms from a single codebase. The web implementation transitioned from PyScript to WASM in January 2026.

---

## 1. WASM Web App (Production)

**Location:** `platforms/web/`

**Status:** ✅ **Production** - Deployed, actively maintained

**Live URL:** https://glarue.github.io/ReferenceFrame

### Technology Stack
- **Core:** Rust (compiled to WASM)
- **Visualization:** SVG generation in Rust
- **UI:** HTML/CSS/JavaScript
- **Storage:** Browser localStorage
- **Deployment:** GitHub Pages (static site)

### Characteristics
- **Payload:** ~220 KB (50-100× smaller than PyScript)
- **First load:** <100ms (cached)
- **Performance:** Near-native (no interpreter overhead)
- **Platforms:** Web browser (desktop/mobile browsers)
- **Code sharing:** 79% shared with mobile platforms

### Features
- ✅ Full frame calculations
- ✅ Interactive visualizations with dimension callouts
- ✅ Vector PDF export (SVG embedded via svg2pdf.js)
- ✅ Text export
- ✅ Shareable URLs
- ✅ Saved custom sizes (localStorage)
- ✅ Aspect ratio locking
- ✅ Unit conversion (inches/mm)
- ✅ Progressive Web App (installable)

### Maintenance Strategy
**Keep as-is** - This is the proven, working implementation. No plans to replace or deprecate. Continue bug fixes and minor improvements as needed.

---

## 2. Rust Multi-Platform (Development)

**Location:** `/rust-flutter/` directory

**Status:** 🚧 **Development** - Core complete, platform implementations in progress

### Technology Stack
- **Core Library:** Rust (compiled to native/WASM)
- **Visualization:** Pure SVG generation in Rust
- **Web:** WASM + TypeScript
- **iOS:** Swift + Rust FFI
- **Android:** Kotlin + Rust FFI

### Code Sharing Architecture

```
┌─────────────────────────────────────┐
│   Rust Core Library (~2,400 LOC)   │
│  ✅ All calculations                │
│  ✅ Unit conversions                │
│  ✅ URL encoding/decoding           │
│  ✅ Aspect ratio management         │
│  ✅ SVG visualization generation    │
└─────────────────────────────────────┘
           │           │           │
    ┌──────┴────┐  ┌───┴────┐  ┌──┴─────┐
    │   Web     │  │  iOS   │  │Android │
    │(WASM +    │  │(Swift +│  │(Kotlin+│
    │TypeScript)│  │ FFI)   │  │  FFI)  │
    │~400 LOC   │  │~400 LOC│  │~400 LOC│
    │UI/render  │  │UI/render│ │UI/render│
    └───────────┘  └────────┘  └────────┘
```

**Code Sharing:** ~79% (core logic) + 21% (platform UI)

### Characteristics
- **Payload (Web):** ~220 KB (50-100× smaller than PyScript)
- **Load time (Web):** <100ms (cached)
- **Performance:** Near-native (no Python interpreter overhead)
- **Platforms:** Web (WASM), iOS (native), Android (native)

### Implementation Status

#### Phase 1: Rust Core Library ✅ COMPLETE
- **Status:** 108 tests passing
- **Modules:**
  - `defaults.rs` - Constants
  - `conversions.rs` - Unit conversion/formatting
  - `frame.rs` - Frame calculations
  - `aspect_ratio.rs` - Aspect ratio utilities
  - `shareable_url.rs` - URL encoding/decoding
  - `visualization/` - SVG generation (56 tests)
- **Location:** `/rust-flutter/rust_core/`

#### Phase 2: Web (WASM) ✅ COMPLETE
- **Status:** Built, tested, demo working
- **Output:** ~220 KB WASM binary
- **Demo:** `/web-wasm/demo.html`
- **Integration:** Documented in `/web-wasm/INTEGRATION_OPTIONS.md`
- **Deployment:** Ready (separate from PyScript app)

#### Phase 3: Mobile (Flutter) ⏸️ MOCKUP READY
- **Status:** FFI bindings complete, Flutter SDK setup pending
- **FFI Tests:** 3/3 passing
- **Mockup:** `/rust-flutter/flutter_mockup/`
- **Blocker:** Flutter SDK installation needed
- **Setup Guide:** `/rust-flutter/FLUTTER_SETUP.md`

### Development Strategy
Build **separate apps** for each platform that share the Rust core:
- **Web:** WASM-based app (deploy separately from PyScript)
- **iOS:** Native Swift app using Rust via FFI
- **Android:** Native Kotlin app using Rust via FFI

**No migration planned** - These are new implementations, not replacements.

---

## Why Two Implementations?

### PyScript Advantages
- ✅ **Proven and deployed** - Production-ready, working now
- ✅ **Python ecosystem** - Direct use of matplotlib, familiar language
- ✅ **Rapid development** - Python in browser, no compilation
- ✅ **Zero server costs** - Static deployment
- ✅ **Already works** - No need to fix what isn't broken

### Rust Multi-Platform Advantages
- ✅ **Cross-platform** - Web + iOS + Android from single codebase
- ✅ **Performance** - Near-native speed, minimal overhead
- ✅ **Payload size** - 50-100× smaller for web (220 KB vs 30 MB)
- ✅ **Code sharing** - 79% shared across all platforms
- ✅ **Native mobile** - True iOS/Android apps, not web wrappers
- ✅ **Battery efficient** - No interpreter overhead on mobile

### Use Case Fit

| Use Case | Best Implementation |
|----------|-------------------|
| Quick web calculator | PyScript (already deployed) |
| Mobile app (iOS/Android) | Rust (only option) |
| Offline-first web app | PyScript (service worker ready) |
| Performance-critical web app | Rust WASM (faster, smaller) |
| Embedded in other web apps | Rust WASM (tiny payload) |

---

## Relationship Between Implementations

### Shared
- ✅ **Core algorithms** - Identical calculations (validated via tests)
- ✅ **Default values** - Same constants
- ✅ **Unit conventions** - All calculations in inches internally
- ✅ **Output format** - Same fractional display (12 3/4")

### Independent
- **Codebases** - No code dependencies between implementations
- **Deployment** - Separate sites/apps
- **Updates** - Can evolve independently
- **Features** - May diverge based on platform needs

### Validation
- **Regression tests** - Rust has 26 regression tests against Python behavior
- **Algorithm parity** - Verified identical calculations
- **No drift** - Core logic locked via tests

---

## Future Roadmap

### PyScript App (Short-term)
- Continue minor improvements and bug fixes
- Maintain as primary web offering
- No major architectural changes

### Rust Multi-Platform (Medium-term)

**Web WASM:**
- Deploy demo as alternative web calculator
- Gather performance metrics
- Optional: Offer as "lightweight mode" alongside PyScript

**Mobile Apps:**
1. Install Flutter SDK
2. Complete iOS app
3. Complete Android app
4. Deploy to App Store / Play Store

**Long-term:**
- Maintain both implementations indefinitely
- Users choose based on needs (web-only vs mobile)
- Possible convergence if WASM clearly superior for web

---

## Documentation References

### PyScript App
- **Main README:** `/README.md`
- **User Guide:** `/docs/`
- **Development Guide:** `/CLAUDE.md`
- **Deployment:** `/docs/DEPLOYMENT.md`

### Rust Multi-Platform
- **Core Library:** `/rust-flutter/README.md`
- **Implementation Status:** `/rust-flutter/IMPLEMENTATION_STATUS.md`
- **Implementation Complete:** `/rust-flutter/IMPLEMENTATION_COMPLETE.md`
- **Visualization Design:** `/rust-flutter/VISUALIZATION_PLAN.md`
- **Web Integration:** `/web-wasm/INTEGRATION_OPTIONS.md`
- **Flutter Setup:** `/rust-flutter/FLUTTER_SETUP.md`

---

## Decision History

### December 2024
- ✅ Decided to keep PyScript app as-is (production)
- ✅ Decided to build separate Rust implementation for multi-platform
- ✅ Completed Rust core library
- ✅ Completed WASM build and demo
- ✅ Completed visualization system in Rust
- ⏸️ Paused mobile development (Flutter SDK needed)

### Pending Decisions
- [ ] Deploy WASM demo publicly?
- [ ] Invest in Flutter SDK for mobile development?
- [ ] Eventually deprecate PyScript in favor of WASM? (no current plans)

---

**Last Updated:** 2025-12-31  
**Status:** Two parallel implementations, both functional, serving different platforms
