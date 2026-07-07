# Float & Sight-Size Frame Support — Implementation Plan

Status: **Phase 1 (sight-size) shipped & deployed to web (2026-07-07)**; XY
assembly-clearance shipped; iOS port + Phase 2 (float) pending. Live status:
[`FLOAT_FRAME_TODO.md`](./FLOAT_FRAME_TODO.md). Original spec (2026-07-06) below.

## 1. Goal

Support two framing styles the current app can't express, both requested by
woodworkers:

- **Sight-size (zero overlap):** the visible/sight opening equals the artwork
  exactly — no frame lip covering any of the image. Common for full-bleed
  prints/photos and panel art.
- **Float frame:** the opening is *larger* than the art, leaving a visible gap
  ("reveal") all around; the art (stretched canvas / cradled panel) is mounted
  **from behind**, with no lip over the face. No glass, no mat.

### The unifying model — a signed "art reveal"

Both are the same knob as our existing rabbet lip, extended past zero:

| Reveal `r` (per side) | Style | Opening vs art |
|---|---|---|
| `r > 0` | **Rabbet** (traditional) | `opening = art − 2r`  ← the only case we support today, clamped `r ≥ 1/16"` |
| `r = 0` | **Sight-size** | `opening = art` |
| `r < 0` | **Float** | `opening = art + 2|r|` (the gap) |

One control, three legitimate styles. Sight-size is literally the boundary
case (`r = 0`); today it's blocked by a hard `rabbet_width ≥ 1/16"` clamp
(`core/src/frame.rs:122-126`) and a `min_rabbet` validation error
(`core/src/validation.rs:429-436`, default 1/8").

## 2. "Is a float frame just a deeper rabbet?" — no, but the cut list barely changes

For the **cut list** (moulding lengths — the app's core output) almost nothing
changes: every dimension flows from the inside opening, and the only difference
is the *sign* of the reveal offset. `outside = inside + 2·frame_width`, total
wood length, and miters are all identical formulas.

What genuinely differs for a float build:

1. **No lip over the face.** The L-notch profile becomes a plain channel
   (lip → 0). This is a *drawing/profile* change, not a cut-length change.
2. **Rabbet depth sized to the canvas** (¾"–1½"). This is the one real depth
   change — and it already works numerically if you set `artwork_thickness` to
   the canvas depth (`get_rabbet_z_depth_required()`, `frame.rs:224-230`;
   `max_artwork` is 2.0" in presets, so canvas fits).
3. **No glass / no mat** in the stack (already settable: glazing = 0, mat = 0).
4. **Rear retention** — held from behind, not by a front lip. A build *note*,
   not a computed dimension.
5. **(Optional) Z-reveal** — canvas sitting proud of / flush with / recessed
   below the frame face. Not currently modeled; see §10 open decisions.

## 3. Data-model decision (recommended)

Add to `FrameDesign` (`core/src/frame.rs:33-61`):

```rust
pub frame_style: FrameStyle,   // enum { Rabbet, SightSize, Float }
pub float_reveal: f64,         // gap per side, ≥ 0; only meaningful for Float
```

- Keep `rabbet_width` **unsigned** (it stays the physical frame lip; keeps the
  URL packing simple and the "rabbet" mental model intact).
- `FrameStyle::Rabbet` → today's behavior exactly (default; back-compat).
- The opening logic in `get_visible_dimensions()` (`frame.rs:153-165`, no-mat
  branch) becomes a `match frame_style`:
  - `Rabbet`   → `art − 2·rabbet_width`
  - `SightSize`→ `art`
  - `Float`    → `art + 2·float_reveal`
- **Mat is only valid for `Rabbet`.** Float/sight are canvas/panel work — guard
  so a non-Rabbet style forces `has_mat() == false` (ignore/zero mat fields).

Rendering collapses to **two modes**: *lip* (Rabbet) vs *no-lip* (SightSize =
reveal 0, Float = reveal > 0). We keep three UI styles because the woodworkers
name all three, but the section/plan code only branches on lip-vs-no-lip plus
the reveal magnitude.

> Alternative considered: repurpose `rabbet_width` as a single *signed* field.
> Rejected — negative "rabbet" is semantically confusing, breaks the min-lip
> validation and the URL's unsigned packing, and `rabbet_width` still has an
> independent meaning in matted designs (the lip the *mat* tucks under). An
> explicit style enum keeps those concerns separate.

## 4. Core changes (`core/`)

| File | Change |
|---|---|
| `src/frame.rs:33-61` | Add `frame_style`, `float_reveal` fields (+ `#[serde(default)]` already on struct). |
| `src/frame.rs:63-86` | `Default`: `frame_style: Rabbet`, `float_reveal: 0.0` from presets. |
| `src/frame.rs:153-165` | `get_visible_dimensions()` — `match frame_style` for the opening (see §3). With-mat path stays Rabbet-only. |
| `src/frame.rs:102-137` | `enforce_constraints()` — **exempt** the `rabbet_width`/`min_rabbet` floors when style ≠ Rabbet; clamp `float_reveal ≥ 0` and to a sane max (e.g. ≤ ½ min-artwork-dim). Keep forcing mat off for non-Rabbet. |
| `src/frame.rs:285-309` | `interpolate()` (animation) — lerp `float_reveal`; `frame_style` uses destination (like other enums/bools). |
| `data/presets.json` | Add defaults `frame_style: "rabbet"`, `float_reveal: 0.0`; optional `float_reveal` preset list (e.g. `[0.125, 0.1875, 0.25, 0.375]`); consider a `min_reveal`/`max_reveal` limit pair. A ready-made "Canvas float" preset (deep rabbet, no glass/mat) would be a nice touch. |
| `src/presets.rs:83-164` | Mirror new default/limit fields in `Defaults` / `ValidationLimits` structs. |

## 5. Validation (`core/src/validation.rs`)

- **Skip** the min-lip / `min_rabbet` checks (`~419-465`) when style ≠ Rabbet —
  a floater has no front lip by design; those checks exist to guarantee lip
  retention (`min_lip_width`, `min_rabbet`).
- Add float-specific checks: `float_reveal` within `[0, max_reveal]`; warn if
  the rabbet/channel **depth** can't swallow the canvas
  (`get_rabbet_z_depth_required()` already computes the needed depth — surface
  it as a warning for Float since canvas is thick).
- Keep messages unit-aware (they already take `use_mm`).

## 6. Serialization

### Shareable URL → format **v2** (`core/src/shareable_url.rs`)
The flags byte reserves bits 4..2 (`shareable_url.rs:73-78`). Plan:
- Pack `frame_style` (3 variants → 2 bits) into those **reserved flag bits** —
  no new byte for the enum.
- Append **`float_reveal` as a uint16** (2 bytes, ×10000) → **v2 = 39 bytes**.
- Bump `FORMAT_VERSION = 2`; add a `V2_LEN = 39` arm to the length→version
  match (`~197-251`); v0/v1 decode with `frame_style = Rabbet`, `float_reveal =
  0` (exactly the existing default-fill pattern). Add `ShareableParams` fields
  (`:14-36`) with `#[serde(default)]`.
- New roundtrip tests mirroring `test_v1_new_fields_roundtrip`.

### History (`core/src/history.rs`)
No code change — it serdes the whole `FrameDesign`; `#[serde(default)]` makes
old snapshots load with the new fields defaulted.

## 7. Visualization (`core/src/visualization/`) — the largest chunk

The lip/overlap assumption is **moderately baked in** but cleanly isolated.
Add a no-lip branch keyed on `frame_style`:

**Section view** (`section_svg.rs`, `geometry/section.rs`)
- `geometry/section.rs:250-333` — material stack is positioned at
  `content_x = origin_x + frame_width − rabbet_w` and rests on `lip_y`. For
  no-lip: `content_x` moves to the frame's inner wall and the stack sits in the
  channel (no lip step). For Float: inset the art by the reveal and show the gap.
- `section_svg.rs:485-500` (+ axis-break variants at `~192-483`) — the L-shape
  frame polygon. No-lip styles draw a **plain rectangular channel** (no inward
  step). Note: the step logic is duplicated across the no-break / v-break /
  h-break / both-break cases, so the no-lip branch touches all four.
- `section_svg.rs:894-939` — the "Rabbet: W × D" label + clearance line becomes
  "Float gap: r" / "Sight-size (no lip)".

**Plan view** (`plan_svg.rs`, `geometry/plan.rs`, `callouts.rs`)
- `geometry/plan.rs:68-75` — `content_area` extends *inward* by `rabbet_width`.
  For Float it extends *outward* by the reveal; for Sight-size it equals the art.
- `plan_svg.rs:434-462` — the `rabbet-overlap` fill: hide for Sight-size; for
  Float, render the **gap** instead of the overlap.
- `plan_svg.rs:22-243` — `render_corner_detail()` is rigid (designed for lip
  overlap). **Skip it** for non-Rabbet styles (or redesign to show the gap).
- `callouts.rs:59-127` — opening callout uses `get_frame_inside_dimensions()`,
  so it updates for free; the mat-cut callout is Rabbet-only anyway.

**Golden tests:** ~13 section/`both` SVGs regenerate
(`core/tests/golden_svgs/`, driven by `golden_svg_matrix.rs`). Add a
float + a sight-size case to the matrix.

## 8. Bindings & UI

### WASM (`platforms/web/wasm_bindings/src/lib.rs`)
- Add **camelCase** getter/setter `frameStyle` (string) + `floatReveal`
  (`~87-115` pattern). ⚠️ camelCase `js_name` only — snake_case silently no-ops
  (see the `wasm-camelcase-getters` gotcha).
- Include both in `getDefaults()` (`~570-590`).

### Web UI (`platforms/web/index.html`)
- Add a **style selector** (Rabbet / Sight-size / Float); show the reveal input
  only for Float, hide the lip input for non-Rabbet.
- Build design: `design.frameStyle = ...; design.floatReveal = ...`
  (`~1479-1509`). Add to `buildShareableParams()` (`~2086-2095`) and the decode
  path (`~2554-2569`). Relabel "visible" → **"sight size"**.
- Cut-list display: swap the "Rabbet" line for "Float gap"/"Sight-size" per
  style.

### Mobile bridge (`platforms/mobile/rust/src/api/simple.rs`)
- `float_reveal` routes through the existing `update_frame_design` numeric match
  (`~101-126`). `frame_style` is an enum → add a small
  `update_frame_design_string()` (mirrors `update_frame_design_bool`,
  `~129-143`) or a dedicated setter.
- `ShareableParams` mirrors core; auto-serializes.

### Mobile UI (`platforms/mobile/lib/`)
- `screens/calculator_screen.dart` — a `SegmentedButton` for style; reveal
  `DimensionInput` shown only for Float; hide/relabel the rabbet inputs per
  style (rabbet clamp logic at `~648-699`).
- `state/design_state.dart` — route `float_reveal` via `updateField`; add
  string routing for `frame_style` (`getField`/`updateField` at `~145-197`);
  extend the defaults maps (`getRustDefault`/`getAllRustDefaults`, `~750-785`).
- `services/export_service.dart` — add both to `_buildShareableParamsJson()`
  (`~386-408`) and the PDF dimension list (`~120-128`).
- `screens/settings_screen.dart` — optional `min_reveal`/`max_reveal` validation
  fields (`~1382-1389` pattern).

## 9. Tests

- **Core unit** (`frame.rs`): opening for each style; `enforce_constraints`
  exemptions; interpolation of `float_reveal`; mat-forced-off for non-Rabbet.
- **Validation**: min-lip skipped for Float/Sight; canvas-depth warning fires.
- **Serialization**: v2 roundtrip; v0/v1 → Rabbet/reveal-0 back-compat.
- **Golden**: regenerate + add float/sight cases.
- **Dart**: `design_state` field routing (Rust-free helpers where possible; note
  the existing `flutter test` + `RustLib` limitation for anything constructing
  `DesignState`).

## 10. Open decisions

1. **Style enum vs single signed reveal** — recommend the enum (§3).
2. **Keep Sight-size as a distinct style, or collapse into Float `reveal = 0`?**
   Functionally identical opening; they differ only in rendering/vocabulary.
   Recommend keeping both in the UI, two rendering modes under the hood.
3. **Z-reveal (canvas proud/flush/recessed)** — model it now, or defer? It's the
   one genuinely new *dimension* (needs a `float_depth_offset` and section-view
   support). Recommend **defer to a follow-up**; ship the XY reveal + no-lip
   first.
4. **UI control shape** — segmented style picker (recommended) vs a single
   signed slider that crosses zero.

## 11. Phasing & rough effort

- **Phase 1 — Sight-size (zero overlap).** Enum {Rabbet, SightSize}, opening =
  art, relax validation, minimal rendering (lip → 0, flush). Delivers the shared
  "0 overlap" ask for *both* audiences. **~0.5–1 day.** Build the data model to
  fit Float too, so it's not throwaway.
- **Phase 2 — Float.** `float_reveal` gap, no-lip-with-gap section/plan
  rendering (all axis-break variants), canvas-depth defaults + warning, URL v2,
  mobile SegmentedButton, "Canvas float" preset, mounting note. **~2–3 days.**
- **Phase 3 (optional) — Z-reveal** (canvas proud), richer float section
  drawing. **~1 day.**

Full feature touches every layer (core, viz, WASM, web, bridge, mobile) but
needs **no refactoring** — the seams (`frame_style` branch in
`get_visible_dimensions`, a no-lip rendering branch) are clean.
