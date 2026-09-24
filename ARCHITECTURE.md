# ReferenceFrame Architecture

A picture-frame design calculator: one pure-Rust core, two thin platform
shells. All calculation, validation, formatting, and diagram (SVG) generation
lives in the core, so the web app and the iOS app compute identical numbers
and draw identical diagrams.

```
core/  (Rust, pure lib)
 ├── WASM   ──> platforms/web/wasm_bindings  ──> platforms/web       (GitHub Pages)
 └── FFI    ──> platforms/mobile/rust        ──> platforms/mobile    (Flutter, iOS App Store)
```

## Repositories

| Repo | Location | Contents | Remote |
|---|---|---|---|
| Root | `/` | core, web, build/release tooling, shared git hooks, docs | GitHub (public) |
| Mobile | `platforms/mobile/` | Flutter app, FRB bridge crate, Fastlane | none (local only) |

The root repo gitignores `platforms/mobile/`, so root CI never sees the mobile
code. Both repos use the shared `hooks/commit-msg` (conventional commits) via
`git config core.hooksPath`; see README.md for setup.

## Crates

| Crate | Path | Type | Role |
|---|---|---|---|
| `referenceframe_core` | `core/` | rlib | All business logic; no platform dependencies |
| `referenceframe_wasm` | `platforms/web/wasm_bindings/` | cdylib | `wasm-bindgen` wrapper; JS-facing names are camelCase |
| `rust_lib_referenceframe` | `platforms/mobile/rust/` | cdylib + staticlib | `flutter_rust_bridge` API (`src/api/simple.rs`); Dart bindings generated into `lib/src/rust/` |

Both wrappers depend on the core by path (`../../../core`). Only the WASM and
bridge crates' `Cargo.lock` files are tracked; the core library's is not.

## Core modules (`core/src/`)

| Module | Responsibility |
|---|---|
| `frame.rs` | `FrameDesign` (every input, in inches) and derived dimensions: opening, rabbet, matboard, fitted component sizes, depth stack, cut list, total wood |
| `constraints.rs` | Input-constraint policy (`apply_input_constraints`): the one place both platforms clamp input; each clamp returns a user-facing, unit-aware notice |
| `validation.rs` | `validate_design`: errors/warnings for whatever is still out of range, against a user-editable `ValidationConfig` |
| `conversions.rs`, `input_parser.rs` | inches/mm conversion; fraction, decimal, and mm formatting; parsing input like `12 3/4` |
| `aspect_ratio.rs` | Ratio display names and aspect-lock math |
| `joinery.rs` | Spline (corner key) slot planning for mitered corners |
| `hanging.rs` | D-ring placement and picture-wire sizing |
| `weight.rs` | Weight estimate from sourced material densities (with error bounds) and wire tension |
| `shareable_url.rs` | Compact binary share-link format (see below) |
| `overlay_params.rs` | Spline/hanging/weight overlay parameters and the JSON helpers both bindings call (one parse per request) |
| `diagram_request.rs` | `DiagramRequest` = `DiagramOptions` + theme (light/dark/pdf): the single options-object entry point for diagram SVGs on both platforms |
| `history.rs` | Design-history entries (versioned, capped list) |
| `presets.rs` | Loads `core/data/presets.json` (compiled in with `include_str!`) |
| `version.rs` | Version info from Cargo.toml |
| `visualization/` | SVG generation: plan view (`plan_svg.rs`), section view (`section_svg.rs`), combined/entry points (`svg.rs`), geometry (`geometry/{mod,plan,section}.rs`), callouts/layout/collision, overlays (spline/hanging), `style.rs` (`DiagramStyle`, incl. `for_dark()`/`for_pdf()`) |

### Key concepts

- **Units.** Everything is inches internally. mm is converted on input and
  formatted on display.
- **Single source of truth.** `core/data/presets.json` holds defaults, presets,
  validation limits, aspect ratios, and the 10-color palette with light/dark
  variants. Values that platform code must duplicate (CSS variables, Dart
  constants, inline JS) are checked by `scripts/check_presets_drift.py` in CI.
  The Dart checks run only where the mobile repo is checked out.
- **Frame styles.** `FrameStyle { Rabbet, SightSize, Float }` sits behind one
  signed quantity, `lip_over_art()`:
  - `> 0` for Rabbet: the lip overlaps the art.
  - `= 0` for SightSize: the opening equals the art, and the lip holds the
    oversized glazing and backing.
  - Float (a negative reveal) is groundwork only. It is carried in the data
    model and share links but behaves like sight-size and isn't offered in
    the UI. See `FLOAT_FRAME_PLAN.md`.
- **Constrain, then validate.** Input first passes through the constraint
  policy, which fixes what can be fixed and reports each change. Validation
  then reports whatever is still out of range. Neither platform clamps on its
  own.
- **Assembly clearance.** `assembly_margin` undersizes the parts that drop into
  the rabbet (glazing, backing, mat outer edge) to a "cut to fit" size. The
  exact rabbet opening is still what the diagrams draw.
- **Shareable URLs.** A design is packed into a fixed-point binary payload,
  base64url-encoded, and carried in `?d=`.
  - The current format is **v2** (39 bytes; adds frame style and float reveal).
  - Links in formats v0 and v1 still decode.
  - New fields are appended and the version bumped.
  - Both platforms encode links: web share links, and the QR code in the
    PDFs. Only the web app opens them.

## Platforms

**Web (`platforms/web/`).**
- The UI is `index.html`: a single file of inline JS importing `pkg/referenceframe_wasm.js`, plus `styles.css`.
- `storage.js` handles localStorage (saved configs, history, settings) and has Node tests in `storage.test.mjs`.
- `sw.js` is the service worker (network-first for app files).
- PDF export uses jsPDF and svg2pdf.js from a CDN.

**iOS (`platforms/mobile/`).**
- Flutter, with the `lib/` layout:
  - `state/design_state.dart`: the app state, which wraps the bridge
  - `screens/`, `widgets/`, `services/`: export, colors, haptics
  - `lib/src/rust/`: the generated bindings (don't edit)
- `frame_preview.dart` draws the live animated preview natively. Every other diagram comes from the core SVG.

## Build, test, release

| Task | Command |
|---|---|
| Core tests (unit + edge + golden SVG + doctests) | `cd core && cargo test` |
| Palette/preset drift check | `python3 scripts/check_presets_drift.py` |
| Web storage tests | `node --test platforms/web/storage.test.mjs` |
| Build WASM (always from the repo root; output only to `platforms/web/pkg/`) | `./build_wasm.sh` |
| Local web server (`:8887`, WASM MIME + no-cache) | `cd platforms/web && python3 serve.py` |
| iOS dev run (rebuilds Rust when core/bridge sources or `presets.json` change) | `cd platforms/mobile && ./rebuild.sh run` |
| Regenerate Dart bindings after bridge API changes | `cd platforms/mobile && flutter_rust_bridge_codegen generate` |
| Mobile tests | `cd platforms/mobile && flutter test` and `cd rust && cargo test` |

**CI (root repo, GitHub Actions).**
- `test.yml` runs on every push and PR: core tests, the drift check, and the storage tests.
- `deploy.yml` runs on push to `main`:
  1. It reuses `test.yml` and deploys only if the tests pass.
  2. It builds the WASM with a pinned wasm-pack.
  3. It stamps the commit SHA into the deployed copy of `index.html` (every `?v=`) and `sw.js` (cache names).
  4. It publishes to GitHub Pages. The Pages source must stay set to **GitHub Actions**.

**Versioning.**
- `./release.sh` (dry run) and `./release.sh --apply` read conventional commits and bump three scopes independently:
  - `core`: `core/Cargo.toml`, tag `core-v*`
  - `app`: `pubspec.yaml`, tag `app-v*`
  - `bridge`: `rust/Cargo.toml`, tag `bridge-v*`
- Bump rules:
  - `feat` → minor
  - other shipping types → patch
  - `docs`/`test` and build-number-only commits → no bump
- iOS build numbers belong to Fastlane.
- The iOS release flow (`bump_build` → `beta` → `submit`) is documented in `platforms/mobile/RELEASING.md`.
