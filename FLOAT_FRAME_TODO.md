# Float & Sight-Size Frames — Implementation TODO / Status

Living checklist for the sight-size + float feature. Design spec:
[`FLOAT_FRAME_PLAN.md`](./FLOAT_FRAME_PLAN.md). Last updated: 2026-07-07.

**Phasing:** Phase 1 = sight-size (zero overlap) end-to-end + forward-compatible
data model. Phase 2 = float (gap/reveal + tray rendering). Phase 3 = Z-reveal
(canvas proud). See the plan for the full rationale.

---

## Phase 1 — Sight-size  ·  status: **core + web DONE & verified; mobile TODO**

### ✅ Core (`core/`) — done, tested
- [x] `FrameStyle` enum `{ Rabbet, SightSize, Float }` (serde `snake_case`), re-exported from crate root — `frame.rs`, `lib.rs`
- [x] `FrameDesign` fields `frame_style` (default Rabbet) + `float_reveal` (default 0), both `#[serde(default)]`-covered so old JSON loads
- [x] `lip_over_art()` helper — the signed reveal: Rabbet→`rabbet_width`, Sight/Float→0 (Float's negative reveal is Phase 2)
- [x] `has_mat()` is style-aware (non-Rabbet ⇒ no mat, so opening=art and depth stack excludes matboard)
- [x] `get_visible_dimensions()` no-mat branch uses `lip_over_art()`
- [x] `enforce_constraints()` clamps `float_reveal ≥ 0`; `interpolate()` lerps reveal, style uses destination
- [x] +10 unit tests (opening per style, serde snake_case, old-JSON default, interpolation)

### ✅ Validation (`core/src/validation.rs`) — done
- [x] Skip the rabbet-**width** lip-retention checks (min-lip / min_rabbet / max) for non-Rabbet styles (back-mounted → no front lip). Rabbet-**depth** checks still apply.

### ✅ Serialization (`core/src/shareable_url.rs`) — done, tested
- [x] Shareable URL **format v2** (39 bytes): `frame_style` packed into the reserved flag bits 3-2; `float_reveal` appended as uint16. `FORMAT_VERSION = 2`, `V2_LEN = 39`.
- [x] v0/v1 links still decode (→ rabbet, reveal 0); version cross-checked vs payload length. +3 round-trip tests.
- [x] History: no change needed (serdes whole `FrameDesign`; `#[serde(default)]` handles it).

### ✅ Visualization (`core/src/visualization/`) — done, golden-verified
- [x] Section: decoupled **lip width** (`lip_over_art()`, →0 for sight-size) from **material illustration width** (physical `rabbet_width`) so materials stay visible with no lip — `geometry/section.rs`
- [x] Section: style-aware rabbet label ("Sight-size — no lip · depth …") — `section_svg.rs`
- [x] Plan: content-area extension + overlap fill use `lip_over_art()`; corner-detail skipped when no lip — `geometry/plan.rs`, `plan_svg.rs`
- [x] +3 golden cases (`sight_size_11x14_{plan,section,both}_inches.svg`); **all existing Rabbet goldens byte-identical** (no regression)

### ✅ WASM binding + Web UI — done, smoke-tested
- [x] `WasmFrameDesign` `frameStyle` (string) + `floatReveal` getters/setters (camelCase); `getDefaults` includes them — `platforms/web/wasm_bindings/src/lib.rs`
- [x] Web: "Frame Style" `<select>` (Rabbet / Sight-size), recalc-on-change, `design.frameStyle` on calculate — `platforms/web/index.html`
- [x] Web: `frame_style`+`float_reveal` in all param builders — share-link, PDF/QR, saved-config save+load; decode/apply reads them
- [x] Web: cache-bust bumped to `?v=20260706-framestyle` (styles + wasm imports)
- [x] Verified: `./build_wasm.sh` clean; 7/7 live WASM→JS smoke checks (sight-size opening = art; permalink round-trips style)

### ⬜ Mobile / iOS port (`platforms/mobile/`) — TODO
The flagship; keep it at least as polished as web. Steps:
- [ ] **Bridge** (`rust/src/api/simple.rs`): route `float_reveal` through the numeric `update_frame_design`; add a string setter for `frame_style` (e.g. `update_frame_design_string()`, mirroring `update_frame_design_bool`). Mirror `frame_style`+`float_reveal` into the bridge `ShareableParams` build.
- [ ] **Regenerate the bridge** (flutter_rust_bridge codegen) so the new fn(s) surface in Dart. (Needs `flutter_rust_bridge_codegen`; the design already flows as JSON, so `FrameDesign` fields ride along via serde — only the enum setter needs new plumbing.)
- [ ] **`state/design_state.dart`**: route `frame_style` (string) + `float_reveal` in get/update; extend the `getRustDefault` / `getAllRustDefaults` maps (frame_style→"rabbet", float_reveal→0).
- [ ] **`screens/calculator_screen.dart`**: `SegmentedButton` (Rabbet / Sight-size), recalc on change; optionally grey the rabbet-width input under sight-size.
- [ ] **`services/export_service.dart`**: add `frame_style`+`float_reveal` to `_buildShareableParamsJson` (and PDF dimension list, optional).
- [ ] **Tests**: `design_state` routing (prefer Rust-free helpers — note the `flutter test`/`RustLib` limitation blocks anything constructing `DesignState`).
- [ ] **Build + verify**: `./rebuild.sh run`; confirm sight-size opening = art, section shows no lip, share/QR round-trips.

### ⬜ Housekeeping (when shipping)
- [ ] Optional web polish: grey/disable the "Rabbet Width" input when Sight-size is selected (it no longer affects the opening, only the section channel depth).
- [ ] Commit — **root repo**: core + web + wasm + goldens; **mobile repo**: bridge + Flutter. (Nothing committed yet.)
- [ ] Web deploy: bump `CACHE_NAME` + `RUNTIME_CACHE` in `platforms/web/sw.js` (currently v10 → v11); `./build_wasm.sh` already done; then deploy.
- [ ] iOS build: mobile changes ship on the next `fastlane beta`/`release`.
- [ ] `/release-notes` + version bumps (`release.sh`) — `feat:` (minor) for core/app/bridge.

---

## Phase 2 — Float frame (future)
- [ ] Wire `lip_over_art()` for `Float` to `-float_reveal` (opening = art + 2·reveal); add `float_reveal` UI (input + presets in presets.json) and expose **Float** in the style pickers.
- [ ] Section rendering: draw the perimeter **gap** and canvas standing in a deeper channel (all 4 axis-break variants); plan: show the reveal gap around the art.
- [ ] Canvas defaults / a "Canvas float" preset (deep rabbet, no glass/mat, thick artwork = canvas depth); depth warning when the channel can't swallow the canvas.
- [ ] Validation: `min_reveal`/`max_reveal` limits (presets.json + settings screen).
- [ ] "Mounted from behind" build note. No URL format change needed (v2 already carries `float_reveal`).

## Phase 3 — Z-reveal (optional, future)
- [ ] Model the canvas sitting proud of / flush with / recessed below the frame face (`float_depth_offset` + section-view support). The one genuinely new dimension; deferred.

---

## Related / backlog (independent of the phases above)

### Assembly clearance in XY (build margin for component fit) — reviewer request 2026-07-07 · **core+web DONE**
**Decided** (approved 2026-07-07): broaden the existing, already-user-editable
`assembly_margin` to mean assembly clearance in *every* direction — depth (as
before) **and** XY — rather than a new field or a rename. Apply by **undersizing**
the loose parts that seat in the rabbet (glazing, backing, matboard-outer),
leaving the frame moulding and all art-based/visual dimensions exact. No URL
change (`assembly_margin` already serialized since v1). No diagram change (viz
still uses the exact rabbet opening).
- [x] Core: `get_fitted_component_dimensions()` = rabbet opening − 2×`assembly_margin`; `xy_assembly_margin()` capped at `rabbet_width` so parts stay under the lip; +3 tests. `get_matboard_dimensions()` unchanged (the exact reference).
- [x] WASM `getFittedComponentDimensions`; web Matboard table shows **"Cut to fit"** + **"Outside (exact)"** (single "Outside" row when margin 0). Cache-bust → `20260707-fitmargin`. 5/5 WASM smoke checks.
- [ ] Web **PDF** export: mirror the "cut to fit" size (PDF currently shows the exact matboard size only — `index.html:~3149/3209`).
- [ ] No-mat / canvas path: optionally surface a glazing/backing "cut to fit" line (component W×H only shows when a mat is present today).
- [ ] **Mobile**: mirror in the calculator results (rolls into the mobile-port item above).

---

## Key design notes (for whoever picks this up)
- **One signed knob:** `lip_over_art()` unifies the three styles — `>0` rabbet (opening<art), `=0` sight-size (opening=art), `<0` float (opening>art, Phase 2). Everything downstream reads this, not `rabbet_width` directly, for the art overlap.
- **Rabbet path is untouched:** for `FrameStyle::Rabbet`, `lip_over_art() == rabbet_width`, so all math/rendering is byte-identical (goldens prove it). The feature is purely additive.
- **Float is inert in Phase 1:** the enum value + `float_reveal` field serialize (URL v2) but behave like sight-size and aren't offered in the UI — so Phase 2 is UI + rendering only, no data-model/format churn.
- **`rabbet_width` still exists** as the physical rabbet/channel (used for the section's material illustration width and depth), decoupled from the art-lip. Don't conflate the two.
- **camelCase trap:** `WasmFrameDesign` JS fields are camelCase (`frameStyle`, `floatReveal`); snake_case silently no-ops. See memory `wasm-camelcase-getters`.
