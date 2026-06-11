# ReferenceFrame Codebase Audit

**Date:** 2026-06-10
**Scope:** Rust core, web platform (WASM), Flutter iOS app, build/release tooling
**Method:** Four parallel deep-review passes (core, web, mobile, tooling/cross-platform), with high-severity findings independently verified against source.

> **Remediation status (2026-06-10):** All High and Medium findings fixed, plus L1, L3, L6–L9.
> Deferred by design: L4 (monolith split), L5 (epsilon comparisons), L10 (CDN vendoring), L11 (JSON round-trip optimization) — see each item's rationale.
> Drift detection (H3) lives at `scripts/check_presets_drift.py`, wired into CI; note that CI checkouts lack the nested mobile repo, so the Dart palette check runs on dev machines only.
> Bonus gaps found while archiving the Python tests (H4), worth porting as Rust tests later: round-half-up behavior at the 0.5 boundary, the mm/inches toggle misinterpretation regression, and rejection of zero/negative artwork dimensions.

> A prior cleanup pass (`be87c7e refactor(core): codebase audit`) already addressed surface-level issues. This audit focuses on what remains: architectural debt, cross-platform drift, and future-proofing gaps.
>
> Excluded by design: `estimate_text_width` calibration (already tracked in TODO.md).

---

## Executive Summary

The codebase is in good shape overall — clean module boundaries in core, no meaningful dead code in the Flutter app, working CI for Rust tests. The three structural risks worth real investment are:

1. **A dead bridge crate** (`platforms/mobile/rust_bridge/`, ~2,400 lines) that looks active but isn't — verified zero references outside its own directory.
2. **The "single source of truth" (presets.json) is aspirational, not enforced.** Colors, defaults, validation limits, and aspect ratios are hand-duplicated in 3–5 places per platform with no drift detection.
3. **No serialization versioning** on shareable URLs, history JSON, or localStorage/SharedPreferences schemas — any schema evolution risks silently breaking saved user data.

---

## High Priority

### H1. Delete the dead `rust_bridge/` crate ✅ verified
**Where:** `platforms/mobile/rust_bridge/` (mobile repo)
The active FFI path is `platforms/mobile/rust/` (crate `rust_lib_referenceframe`, 62 exported functions, wired via pubspec → rust_builder). The `rust_bridge/` crate (`referenceframe_mobile`, 41 functions, missing history/interpolation/previews/shareable URLs) has **zero references** in pubspec.yaml, rebuild.sh, lib/, or iOS config — confirmed by grep for both the directory name and the crate name. It's a stale snapshot of early scaffolding that will mislead future work (e.g., someone "fixing" the wrong bridge).
**Action:** `rm -rf platforms/mobile/rust_bridge/` (plus its `build_ios.sh`). Commit to the mobile repo.

### H2. presets.json drift: extended palette colors exist only in platform code ✅ verified
**Where:** `core/data/presets.json` (7-color palette) vs `platforms/web/styles.css` (`--rf-flag-red`, `--rf-dark-cyan`, `--rf-air-force-blue` + variants), `platforms/web/index.html` (COLOR_PALETTE / SEMANTIC_CATEGORIES arrays), `platforms/mobile/lib/models/color_category.dart:96-150` (10-color Dart consts).
The user-customizable palette has grown to 10 colors, but presets.json still defines 7. The extra three are hand-coded per platform with nothing checking they match. Hex values can silently diverge (and light/dark variants are independently derived per platform).
**Action:**
1. Add `flag_red`, `dark_cyan`, `air_force_blue` to `presets.json` → `colors.palette` (with light/dark variants like the existing 7).
2. Add a drift test (see H3) so this can't recur.

### H3. No cross-platform consistency tests — add a drift-detection CI job
**Where:** `.github/workflows/test.yml` runs only `cargo test --lib`.
Nothing validates that presets.json values match the hand-duplicated copies in `styles.css`, `index.html` inline JS, `color_category.dart`, or `aspect_ratio_presets.dart`. Every "single source of truth" violation in this report would have been caught by one script.
**Action:** Add `scripts/check_presets_drift.py` (or a Rust test) that:
- Loads presets.json
- Regex-extracts hex colors from styles.css, color_category.dart, and the inline JS arrays in index.html
- Extracts preset numeric arrays / defaults where duplicated
- Fails on mismatch
Run it in CI alongside `cargo test`. This is the highest-leverage single change in this report — it converts the SSOT convention into a guarantee.

### H4. Stale PyScript-era Python tests in root `tests/` ✅ verified
**Where:** `tests/test_frame_calculations.py`, `test_conversions.py`, `test_unit_conversion_bugs.py`, etc. (~1,500 LOC).
These import a Python `FrameDesign` that no longer exists (PyScript era), are run by nothing (CI is Rust-only), and duplicate logic now tested in `core/src` (265 passing Rust tests).
**Action:** Delete, or move to `legacy/pyscript/tests/` alongside the archived PyScript app. Spot-check first for any edge-case scenarios worth porting to Rust tests (the unit-conversion bug regression tests are the most likely candidates).

### H5. No version byte in shareable URL binary format ✅ verified
**Where:** `core/src/shareable_url.rs` (30-byte format; no "version" anywhere in the file).
The decoder special-cases a 28-byte legacy format by length, but there's no forward path: adding any field (new rabbet style, asymmetric mats — already planned per `SAVED_CONFIGS_PLAN.md`) breaks every previously shared URL, and length-sniffing won't scale past two formats.
**Action:** The flags byte uses only ~2 of 8 bits — reserve the top bits as a format version now (current = 0), and make the decoder switch on it. Cheap today, impossible to retrofit after more formats ship.

### H6. No schema versioning/migration for persisted user data
**Where:**
- `core/src/history.rs:186-193` — `to_json`/`from_json` raw serde, no version field
- `platforms/web/storage.js` — localStorage keys unversioned; export claims `version: 1.0` but import has no migration path
- `platforms/mobile/lib/state/storage.dart:322` — mentions `'version': 3` but no migration logic
Saved configs and history are user-created data; a schema change (field rename, new required field) silently drops or corrupts them.
**Action:** Minimum viable fix, in order of value:
1. Add `#[serde(default)]` to all `FrameDesign` fields so old JSON loads with defaults for new fields (one attribute, covers most evolution).
2. Add a `version` field to `DesignHistory` and to each localStorage payload.
3. Write a `migrate(old, fromVersion)` stub on each platform now, while version 1→1 is a no-op — so the pattern exists when needed.

---

## Medium Priority

### M1. Division-by-zero / NaN propagation in core math
**Where:** `core/src/aspect_ratio.rs:79` ✅ verified — `known_value / ratio` with no zero guard (sibling functions at lines 59 and 88 do guard). Also `core/src/input_parser.rs:136` — `DimensionInput::divide(scalar)` unguarded.
A zero ratio produces `inf`/`NaN` that flows into dimension fields and SVG geometry rather than failing visibly.
**Action:** Guard both (return the known value / an Err or no-op) and add a regression test feeding 0 through the public calculate path.

### M2. Dimension formatting implemented in ~5 places (mobile)
**Where:** `design_state.dart:488-509`, `dimension_input.dart:76-97`, `preset_dimension_input.dart:64-84` (+2 more sites), `saved_sizes_sheet.dart:15-19`, and Rust `formatDimensionBridge()`.
Each copy has slightly different trim/fallback behavior; CLAUDE.md already mandates "display format must be respected consistently in all value displays" — this fragmentation is how that breaks.
**Action:** Make Rust's `formatDimensionBridge()` the only formatter (extend its signature to cover the decimal-display special case), or at minimum extract one Dart utility in `lib/utils/formatting.dart` and delete the copies.

### M3. Web defaults hardcoded in HTML `value=` attributes
**Where:** `platforms/web/index.html:137-206` vs `presets.json` defaults.
The page already fetches defaults from WASM (`getDefaults()`); the HTML attributes are a second, drift-prone copy that flashes before JS overwrites them.
**Action:** Strip the hardcoded `value=` attributes and set inputs programmatically from `getDefaults()` during init.

### M4. Service worker strategy vs `?v=` cache-busting
**Where:** `platforms/web/sw.js:52-78` + `index.html` `?v=` convention.
Two cache-invalidation mechanisms coexist (network-first SW + query-param busting). They mostly work, but a stale SW + slow network can serve old HTML referencing old `?v=` params.
**Action:** Pick one strategy and document it in sw.js: simplest is bumping `CACHE_NAME` on every deploy (could be automated from the core version in `build_wasm.sh`), keeping `?v=` only as a belt-and-suspenders for non-SW browsers.

### M5. `DesignState` god object + broad rebuilds (mobile)
**Where:** `design_state.dart` (820 lines, 26 `notifyListeners()` calls); full `Consumer<DesignState>` in `calculator_screen.dart:178` and `frame_preview.dart:275`.
Past perf work already moved MaterialApp and UnitToggleButton to `Selector` — the same medicine applies to the remaining full Consumers. Also: `setUnits()` (async, hits SharedPreferences) is called un-awaited in `calculator_screen.dart:45`.
**Action:** Convert the two remaining full Consumers to `Selector` on the fields they render; batch `notifyListeners()` in multi-step updates; await `setUnits()`.

### M6. Silent error swallowing at FFI/WASM boundaries
**Where:** `platforms/mobile/rust/src/api/simple.rs:89-113` (`unwrap_or_default()` on parse failures → empty state, no error to Dart); `index.html:2779` (PDF export failure logs to console only, no user feedback).
**Action:** Return `Result<T, String>` from bridge functions so Dart can surface errors; add a visible failure message to the PDF export path.

### M7. Validation thresholds hardcoded outside presets.json
**Where:** `core/src/validation.rs:97-143` (`ValidationConfig::default()` — min/max frame width, rabbet limits, etc.).
These are product tuning values of the same kind presets.json exists for, and the web UI persists user overrides of them (`rf_validation_config`), so the defaults matter.
**Action:** Move the limits into a `validation_limits` section of presets.json and have `ValidationConfig::default()` read them (presets.json is already `include_str!`'d, so no runtime cost).

### M8. Stale planning docs and mockups mixed with shipping code
**Where:** `platforms/web/SAVED_CONFIGS_PLAN.md`, `STYLE_MATCHING_PLAN.md`, `UI_REFRESH_PLAN.md`; `docs/corner-detail-*.html`, `cut-pieces-example*.html`.
Partially-implemented plans sitting next to source read as authoritative.
**Action:** Move to `docs/plans/` with a one-line status header (done / partial / abandoned), or delete the completed ones. Same for the HTML mockups (note in docs/ that they're illustrative only).

### M9. WASM bindings crate frozen at version 0.1.0
**Where:** `platforms/web/wasm_bindings/Cargo.toml` vs core 1.5.4.
There's no way to tell which core version a deployed web build contains.
**Action:** Lowest-effort fix: surface `core/src/version.rs` in the web footer (a `getVersion()` binding may already exist — wire it through). Optionally teach `release.sh` to keep the wasm_bindings version mirroring core.

---

## Low Priority

| # | Finding | Where | Action |
|---|---------|-------|--------|
| L1 | Untracked debug test writes SVG to `/tmp`, no assertions | `core/tests/wide_frame_svg_dump.rs` | Delete, or convert to a golden test following `golden_svg_matrix.rs` |
| L2 | `TODO.md` untracked at root | `/TODO.md` | Commit it (it's a useful roadmap doc) or fold into docs/ |
| L3 | Icon-only buttons lack `aria-label`; validation messages lack `role="alert"`; collapsibles lack `aria-expanded` | `index.html` throughout | Add ARIA attributes in a single pass; cheap and makes the PWA screen-reader usable |
| L4 | `index.html` is a 3,681-line monolith (300+ line `calculate()`) | `platforms/web/index.html` | Extract `pdf-export.js` first (most self-contained, ~300 lines); further splitting only if it starts hurting |
| L5 | Exact `== 0.0` float comparisons | `aspect_ratio.rs:26,59,88` | Acceptable for literal-zero guards; switch to epsilon only if near-zero inputs appear in practice |
| L6 | Shareable URL u16 fields max out at 6.5535" | `shareable_url.rs:64-70` | Document the per-field max; add range validation on encode |
| L7 | Build scripts vary in defensiveness (`set -e` vs `set -euo pipefail`; cwd assumptions in `platforms/web/build.sh:12`) | build_wasm.sh, build.sh, rebuild.sh, release.sh | Standardize on `set -euo pipefail` + `SCRIPT_DIR` pattern |
| L8 | `cargo update` failure swallowed in release.sh | `release.sh:260-274` | Drop the `2>/dev/null \|\| true`; fail loudly so the lock-file commit isn't silently skipped |
| L9 | Commit hook requires manual `git config core.hooksPath hooks/` after clone | `hooks/commit-msg` | One-line setup note in README, or a `scripts/setup-dev.sh` |
| L10 | PDF libs (jsPDF, svg2pdf, qrcode) loaded from CDN with no fallback | `index.html:23-33` | Acceptable; optionally vendor them and let the SW cache them |
| L11 | JSON round-trips per keystroke across WASM/FFI boundaries | `wasm_bindings/lib.rs`, `simple.rs` | Not worth changing unless profiling shows it; current design is simple and correct |

### Findings investigated and rejected
- **`frame.rs:109` comment "mismatch"** — the comment ("leave at least 1/4\" visible") is correct: 0.125" is clamped *per side*, so total visible = 2 × 0.125 = 0.25". No bug.
- **Dead code in Flutter lib/** — none found; all widgets/screens are referenced.

---

## Suggested Sequencing

1. **Quick wins (one sitting):** H1 delete rust_bridge · H4 delete/archive Python tests · L1 delete or gitignore wide_frame_svg_dump.rs · L2 commit TODO.md · M8 relocate stale plans
2. **Drift hardening (the big one):** H2 add 3 colors to presets.json → H3 drift-check script in CI → M7 validation limits into presets.json → M3 strip HTML hardcoded defaults
3. **Data durability:** H5 URL version bits · H6 `#[serde(default)]` + version fields + migration stubs
4. **Correctness:** M1 zero guards · M6 FFI/PDF error surfacing
5. **Mobile health (as touched):** M2 formatting consolidation · M5 Selector conversions + await setUnits
6. **Opportunistic:** everything in Low Priority

Items 1–4 are small, independent, and individually committable. Item 2 is the one that pays compounding dividends — it makes the project's core architectural promise (presets.json as single source of truth) self-enforcing.
