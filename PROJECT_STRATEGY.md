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
- **Payload:** ~314 KB WASM binary (50-100× smaller than PyScript's ~30 MB)
- **First load:** <100ms (cached after initial build)
- **Performance:** Near-native (no interpreter overhead)
- **Platforms:** Web browser (desktop/mobile browsers)
- **Code sharing:** 90%+ shared with future mobile platforms

### Deployment Details
- **URL:** https://glarue.github.io/ReferenceFrame
- **CI/CD:** GitHub Actions automatically builds and deploys on push to `main`
- **Build Process:**
  1. Sets up Rust toolchain and wasm-pack
  2. Compiles Rust core to WASM (~45 seconds)
  3. Deploys static files (HTML, CSS, JS, WASM) to GitHub Pages
- **No server required:** Fully static site, zero hosting costs

### Features
- ✅ Full frame calculations (dimensions, cut list, depth analysis)
- ✅ Interactive visualizations with dimension callouts (plan + section views)
- ✅ Professional PDF export with embedded vector SVG diagrams and QR codes
- ✅ Text export for cut lists
- ✅ Shareable URLs (28-byte base64-encoded designs)
- ✅ Saved custom sizes (localStorage)
- ✅ Aspect ratio locking with orientation toggle
- ✅ Unit conversion (inches ↔ mm)
- ✅ Responsive design (desktop + mobile browsers)

### Maintenance Strategy
**Active development** - Continue adding features and improvements. This is the primary implementation going forward.

---

## 2. Mobile Platforms (Planned)

**Location:** `platforms/mobile/` directory

**Status:** 📋 **Planned** - Core library complete, mobile UI not yet started

### Approach

**Shared Rust Core** (`core/` directory):
- ✅ All business logic: calculations, validation, SVG generation
- ✅ 100+ tests passing
- ✅ Platform-agnostic (no dependencies on web, mobile, etc.)
- ✅ Already powering the web app

**Platform-Specific UI** (future work):
- iOS: Native Swift app using Rust via FFI
- Android: Native Kotlin app using Rust via FFI
- Or: Single Flutter app for both platforms

**Code Sharing Estimate:** 90%+ (core logic) + 10% (platform UI/integration)

### Benefits
- Same calculations and visualization across web and mobile
- Native mobile performance (no hybrid web wrapper)
- Smaller app size (no embedded browser engine)
- Offline-first design
- Professional PDF export on mobile devices

### Next Steps
1. Choose mobile framework (Native Swift/Kotlin vs Flutter)
2. Generate FFI bindings to Rust core
3. Implement mobile UI (forms, results, visualization rendering)
4. Add platform-specific features (share sheets, file access, etc.)
5. App Store / Play Store submission

---

## 3. Architecture Philosophy

### Shared Rust Core Pattern

The project uses a **platform-agnostic core library** (`core/`) containing all business logic:

```
core/                           # Platform-agnostic Rust library
├── src/
│   ├── frame.rs               # Frame design calculations
│   ├── conversions.rs         # Unit conversion (inches ↔ mm)
│   ├── input_parser.rs        # Parse fractional dimensions ("12 3/4")
│   ├── validation.rs          # Input validation rules
│   ├── aspect_ratio.rs        # Aspect ratio utilities
│   ├── shareable_url.rs       # URL encoding/decoding
│   └── visualization/         # SVG generation
│       ├── geometry.rs        # Layout calculations
│       ├── svg.rs             # SVG rendering
│       └── callouts.rs        # Dimension arrows & labels
└── Cargo.toml

platforms/web/                  # Web-specific UI (10% of code)
├── wasm_bindings/             # Thin wrapper for WASM
├── index.html                 # Web UI
└── styles.css

platforms/mobile/               # Future: Mobile UI (10% of code)
└── (iOS/Android apps using core via FFI)
```

**Key Principles:**
1. **Business logic in Rust** - All calculations, validation, and visualization in `core/`
2. **Platform-specific UI only** - HTML/CSS for web, Swift/Kotlin for mobile
3. **No duplication** - Same calculations across all platforms
4. **Easy testing** - Core library tested independently (100+ tests)

### Why Rust + WASM?

**Replaced PyScript (Jan 2026) due to:**
- **50-100× smaller payload** (~314 KB vs ~30 MB)
- **Faster load times** (<100ms vs 10-30s first load)
- **Better performance** (no Python interpreter overhead)
- **Mobile path** (same core can power native iOS/Android apps via FFI)
- **Maintained parity** (all PyScript features ported, verified via regression tests)

**PyScript preserved as reference** in `legacy/pyscript/` directory.

---

## 4. Future Roadmap

### Web App (Current Focus)
- ✅ Deployed and production-ready (Jan 2026)
- 🔄 Ongoing improvements and feature additions
- 📋 Potential additions:
  - Saved configurations UI (data model already exists)
  - Standard artwork sizes picker
  - Print-optimized visualization layouts
  - Advanced mat calculations (multiple mats, v-groove)

### Mobile Apps (Planned)
**Timeline:** TBD (depends on demand and resources)

**Approach:**
1. Choose platform: Native (Swift/Kotlin) vs Flutter
2. Generate FFI bindings to Rust core
3. Build mobile UI (~2-3 weeks per platform)
4. Add platform-specific features (share sheets, camera integration)
5. Submit to App Store / Play Store

**Effort estimate:** 2-3 weeks for iOS-only (Swift), 3-4 weeks for iOS + Android (Flutter)

---

## 5. Documentation

- **Project README:** `/README.md` - Quick start and overview
- **Architecture:** `/ARCHITECTURE.md` - Detailed technical architecture
- **Deployment Guide:** `/DEPLOYMENT_AND_IOS_REVIEW.md` - Deployment options and iOS planning
- **Session Notes:** `/CLAUDE.md` - Development session history
- **Legacy PyScript:** `/legacy/pyscript/README.md` - Archived implementation reference

---

## 6. Decision History

### December 2024
- ✅ Built Rust core library (frame calculations, validation, SVG generation)
- ✅ Created WASM web implementation
- ✅ Verified feature parity with PyScript via regression tests
- ⏸️ Paused mobile development (prioritized web deployment)

### January 2026
- ✅ **Deployed WASM web app** to production (https://glarue.github.io/ReferenceFrame)
- ✅ **Replaced PyScript** as primary implementation
- ✅ **Archived PyScript** to `legacy/pyscript/` (preserved as reference)
- ✅ **Enhanced PDF export** with professional layout, embedded vector diagrams, QR codes
- ✅ **Automated deployment** via GitHub Actions (builds WASM on every push)
- 📋 Mobile development remains planned (pending demand/resources)

### Key Decisions
- **Why replace PyScript?** 50-100× smaller payload, faster load, mobile path via shared core
- **Why not parallel deployment?** WASM provides same features with better UX, no need for both
- **Why preserve PyScript?** Algorithm reference, educational value, migration validation

---

**Last Updated:** 2026-01-06
**Status:** Production WASM web app deployed, mobile platforms planned
