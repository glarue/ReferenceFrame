# Float & Sight-Size Frames — Implementation TODO / Status

Living checklist for the sight-size + float feature. Design spec:
[`FLOAT_FRAME_PLAN.md`](./FLOAT_FRAME_PLAN.md). Last updated: 2026-07-07.

**Phasing:** Phase 1 = sight-size (zero overlap) end-to-end + forward-compatible
data model. Phase 2 = float (gap/reveal + tray rendering). Phase 3 = Z-reveal
(canvas proud). See the plan for the full rationale.

---

## Phase 1 — Sight-size  ·  status: **core + web SHIPPED & DEPLOYED (live); mobile TODO**

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

### ✅ Mobile / iOS port (`platforms/mobile/`) — DONE 2026-07-07 (commits `fbbfefd` bridge, `71a66f6` app)
- [x] **Bridge**: `float_reveal` in the numeric `update_frame_design`; `update_frame_design_string()` for `frame_style`; `get_fitted_component_dimensions`; spline/hanging JSON getters; `show_spline`/`show_hanging` through the SVG generators. Bridge-side tests cover the new routing.
- [x] **Bridge regenerated** (`flutter_rust_bridge_codegen generate`, frb 2.11.1).
- [x] **`state/design_state.dart`**: `getStringField`/`updateStringField`, fitted-dims + spline/hanging wrappers, persisted `showSpline`/`showHanging` prefs threaded into all SVG wrappers.
- [x] **`screens/calculator_screen.dart`**: Rabbet/Sight-size `SegmentedButton` in Advanced (with per-style hint; rabbet inputs stay editable per the superseded-housekeeping note), "Cut to fit" row + "Glazing & Backing" card, "Joinery & Hanging" results card.
- [x] **`services/export_service.dart`**: `frame_style`+`float_reveal` in share params. Also fixed two share-link bugs: `frame_depth` read a nonexistent `frame_thickness` design field (encoded 0) and `blade_width` now comes from defaults.
- [x] **Layer toggles**: Spline Slots / Hanging Hardware switches in the Diagram Detail sheet (persisted, default off).
- [x] **Build + boot verified**: `flutter build ios --simulator` clean; app installs, launches, and renders on the iPhone simulator. Remaining: hands-on device pass (toggle layers, sight-size section, share/QR round-trip).

### Housekeeping
- [x] ~~Grey out "Rabbet Width" under Sight-size~~ — **superseded**: `rabbet_width` IS meaningful for sight-size (it's how far the lip grabs the oversized glazing). Keep it; maybe add a per-style hint clarifying its role.
- [x] **Root repo** committed + **deployed to live web** (2026-07-07): core + web + wasm + goldens. Service worker → v12, cache-bust `20260707-sightlip`.
- [x] **Mobile repo** committed 2026-07-07 (`fbbfefd` bridge, `71a66f6` app).
- [ ] iOS build: mobile changes ship on the next `fastlane beta`/`release`.
- [ ] `release.sh` version bump (`feat:` → core minor) — when cutting the mobile build / a core release; not needed for the (already-live) web.

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
- [x] Web **PDF** export: "Cut to fit" line added to the Matboard block when clearance applies (2026-07-07).
- [x] No-mat / canvas path: "Glazing & Backing" card shows seat (rabbet opening) + cut to fit — makes the oversized-for-sight-size component size explicit.

### Sight-size section view — retaining lip (fix, 2026-07-07) · DONE
Reviewer flagged the section looked like nothing held the stack. The rabbet was
always modelled (glazing/backing = art + 2×rabbet_width, held by the lip); only
the *drawing* collapsed the lip. Fixed: section always draws the lip, glazing/
backing seat under it, artwork inset to the sight line (`art_inset = rabbet_w −
lip_over_art`, so traditional frames stay byte-identical). Deployed.
- [x] **Mobile**: mirrored via core SVG (section renders through FFI; done with the mobile port).

---

## Key design notes (for whoever picks this up)
- **One signed knob:** `lip_over_art()` unifies the three styles — `>0` rabbet (opening<art), `=0` sight-size (opening=art), `<0` float (opening>art, Phase 2). Everything downstream reads this, not `rabbet_width` directly, for the art overlap.
- **Rabbet path is untouched:** for `FrameStyle::Rabbet`, `lip_over_art() == rabbet_width`, so all math/rendering is byte-identical (goldens prove it). The feature is purely additive.
- **Float is inert in Phase 1:** the enum value + `float_reveal` field serialize (URL v2) but behave like sight-size and aren't offered in the UI — so Phase 2 is UI + rendering only, no data-model/format churn.
- **`rabbet_width` still exists** as the physical rabbet/channel (used for the section's material illustration width and depth), decoupled from the art-lip. Don't conflate the two.
- **camelCase trap:** `WasmFrameDesign` JS fields are camelCase (`frameStyle`, `floatReveal`); snake_case silently no-ops. See memory `wasm-camelcase-getters`.
