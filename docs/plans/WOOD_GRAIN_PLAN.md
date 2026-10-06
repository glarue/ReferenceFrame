# Wood-Grain Frame Rendering Plan

> **Status (2026-10-06):** Active. **Phase 1 is done.** The generator is ported to `core/src/visualization/wood/`, and its data (17 species plus a synthetic generic wood) is in `core/data/wood_appearance.json`. Nothing calls it yet, so the diagrams are unchanged. Next is Phase 2 (plan view and section view). The parameters come from the private `tools/wood-fit/` repo (`chosen.json` and the `gen_fit.py` reference prototype).

**Goal:** draw the frame face with species-specific procedural wood grain, replacing today's outline-only plan view and flat brown section fill, in the plan view and live preview on both web and iOS. The species is the one users already pick for the weight estimate (`materials.woods`). The aim is "close in spirit" rather than photoreal: deterministic, vector, and one implementation in `core/` for both platforms.

---

## Decisions (and the evidence behind them)

- **Procedural vector SVG generated in `core/`.**
  - iOS renders diagrams with flutter_svg, which has no SVG filters (`feTurbulence` etc.), so the grain must be explicit geometry.
  - A harness pinned to the app's versions (flutter_svg 2.2.3, vector_graphics_compiler 1.1.20) rendered the prototype the same as WebKit: clip paths, gradients, transforms, dash arrays and filled outlines all match.
  - **Percentage lengths throw** (`width="100%"`).
  - `<pattern>` ignores `patternTransform`, so don't use it.
- **Flat color while the preview's spring morph runs, grain once it settles.**
  - flutter_svg parses each new SVG string on a fresh isolate. That costs about 4.7 ms fixed, plus about 3–7 ms per 50–130 KB of grain SVG (measured on an M5 Pro in a profile build).
  - vector_graphics drops a load that a newer one supersedes, so any load slower than one frame freezes the animation instead of stuttering it.
  - The app runs at 120 Hz on ProMotion devices (`CADisableMinimumFrameDurationOnPhone`). At 120 Hz, grain showed 16 of 144 frames, against 138 of 144 for today's 1.3 KB preview.
  - The rotation animation reuses one static SVG, so grain is fine there.
- **Per-species numbers come from an offline pipeline.** It fits to photos, then a person ranks blinded variants by eye (`tools/wood-fit/`). This repo only receives the resulting parameters. The photos are used only to extract colors and texture statistics, and are never committed or shipped.

## Generator (port of `tools/wood-fit/gen_fit.py`)

Each moulding side is drawn in local coordinates: u runs along the length, v across the width. Geometry is in inches, converted to px with the diagram scale, and each side is clipped to its miter trapezoid. Layers, bottom to top:

1. **Base fill**: the palette's `base` color.
2. **Color streaks**: soft-edged (two nested bands) and patchy (they pinch off along the length), with capped opacity, so they never read as full-length stripes. Outlines are sampled about once per inch and smoothed. Follow-up: when zoomed in close, a pinched streak's tips can look too smooth, almost almond-shaped; the prototype's denser sampling looked more ragged.
3. **Growth-zone bands**: one subtle band per ring.
   - Placed on the earlywood side for ring-porous species (where it is the pore band), or on the latewood side for softwoods (`zone_side`).
   - Can be broken into long irregular segments.
   - Opacity capped at 0.12. In review, this layer was the main source of "stripey" results.
4. **Ring traces**:
   - **Ring geometry.** Rings sit around a tilted, eccentric, drifting pith, controlled by `pith_offset_in`, `pith_depth_in`, `pith_tilt` and `depth_noise_in`. Ring widths follow AR(1) (φ ≈ 0.6, CV ≈ 0.4, with an occasional narrow year).
   - **Joined apexes.** The two sides of each arch are joined through the exact apex, interpolated where R equals depth. Earlier versions left the peaks broken.
   - **Variable-width traces.** Traces are filled outlines, not strokes. Width = base × min(cap, R/x) × slow noise along the length. Each ring is a shell between nested cones, and the board face cuts it in a conic section whose apparent width grows toward the vertex. The cap is 2; zone bands get half the widening.
   - **Line modes.** `continuous` (diffuse-porous species and softwoods), `broken` (walnut, mahogany), or `none` (oak and ash, whose rings show only as pore bands; a full-height boundary line looks wrong).
5. **Pores**: rows of short dashes with aperiodic gaps, inside each ring's earlywood band.
   - The row count scales with ring width, so narrow rings don't become dense clusters.
   - Rows are jittered, with occasional dropouts and per-row width variation.
   - No `stroke-dashoffset`; its flutter_svg support is unverified.
6. **Ray flecks** (quartersawn only): tapered ribbons along stacked ∩ arcs, with blue-noise spacing.
7. **Flat faces and miter seams.** Pieces are drawn as flat boards, with no profile shading that would imply a rounded top. The vertical pieces are about 5% darker than the horizontal ones (`grain_tone`): their grain runs at 90° to the others, so they reflect light differently.
8. **Depth cues** (light from the upper left; no filters needed). Shadow length is K = 0.6 per inch of height, and both shadows use heights from the design:
   - **Inner shadow:** the lip's shadow on the mat or art, along the top and left inner edges, drawn as gradient strips. Width = (`frame_material_depth` − `rabbet_depth` + `glazing_thickness`) × K; about 5 px at preview scale with the defaults.
   - **Wall shadow:** the frame standing off the wall, along the bottom and right outer edges, drawn beneath the frame as about 6 stacked offset rectangles at low opacity for a soft edge. Width = `frame_material_depth` × K.
   - The **live preview** uses both. The **plan view** uses the inner shadow only, because the wall shadow would sit where the dimension callouts go.
   - Both cost about 10 elements, so they stay on during the spring animation (`Flat` LOD as well as `Grain`).

**Anti-repetition rules** (from a two-agent panel review plus by-eye review):
- Every layer draws from its own hash-derived random stream (lowbias32 or splitmix of seed + layer tag), so a count change in one layer, such as during a dimension change, never reshuffles another.
- fBm uses a detuned lacunarity (about 2.03) and per-octave offsets.
- Nothing is periodic: no fixed spacings and no repeating dash lists.
- The four sides currently get independent sub-seeds; this matches the prototype that was reviewed. Cutting all four sides consecutively from one virtual board, so grain flows around the corners, is deferred to Phase 5.

**Determinism:** geometry uses basic float operations plus `sqrt`. `exp` appears only in lognormal draws, and output is rounded to 0.1 px. Output is identical on a given platform; bit-identical output across platforms is not required.

**Pixel floors:** lines are at least 0.45 px and pores at least 0.4 px, so features survive preview scale.

**Scale LOD** (inactive at validation scale; at preview scale these cut payload 2–4×):
- Rings closer than 2.5 px are decimated: every k-th ring is drawn, and its line, zone and pore opacity are raised by k^0.75 so overall darkness holds.
- Pore rows are capped to stay at least 1.2 px apart, with the same opacity compensation.
- Every run is simplified with Ramer–Douglas–Peucker at 0.25 px, keeping arch apexes, and drawn as a polyline. Within that tolerance, a smooth curve adds bytes but nothing visible.
- Variable-width outlines are used only where the trace is at least 1 px wide; thinner traces are strokes.

**Validation against the prototype:** the random streams differ (SplitMix64 vs Python's Mersenne Twister), so outputs aren't pixel-identical. The noise functions match bit for bit (a parity test checks this). Texture statistics over 8 seeds per species (grain direction, tonal contrast, line continuity, pores) match within seed-to-seed variance (`tools/wood-fit/validate_rust.py`).

## Data: `core/data/wood_appearance.json`

This is a generated file, written by `tools/wood-fit/export_appearance.py` from `chosen.json` (`final_fixed`). It is separate from `presets.json` for two reasons: `presets.json` is hand-formatted, and the web app loads presets JSON that doesn't need this data. Its keys match `materials.woods`. Keys without their own entry resolve through `aliases` and then `default` (the generic wood), and a figure such as quartersawn sits under `variants`. In core, `wood_appearance(key, variant)` returns a key's look.

```json
{
  "default": "generic",
  "aliases": {"generic_hardwood": "generic", "generic_softwood": "generic"},
  "woods": {
    "red_oak": {
      "palette": {"base": "#…", "late": "#…", "streak": "#…", "alt": "#…"},
      "structure": {"figure": "cathedral", "line_mode": "none", "zone_side": 1, "pores": true, "flecks": false},
      "params": {"rings_per_in": 3.9, "pore_len_in": 0.075, "…": "…"},
      "per_frame_range": {"ring_cv": [0.38, 0.69], "…": "…"}
    },
    "white_oak": {"…": "…", "variants": {"quartersawn": {"…": "…"}}}
  }
}
```

- Tests check that every `materials.woods` key resolves to a look, and that every appearance key is a real wood key.
- **Covered:** red oak, white oak (plain-sawn, the default for `white_oak`), white ash, black walnut, Honduran mahogany, eastern white pine, ponderosa pine, Douglas-fir, western red cedar, yellow poplar, basswood, red alder, soft maple, hard maple, black cherry, and yellow birch. White oak (quartersawn) is a figure variant.
- **Generic wood:** a single synthetic appearance, not a real species: a neutral medium-brown palette with hard-maple-inspired texture (fine continuous lines, no visible pores, straight grain). `generic`, `generic_hardwood` and `generic_softwood` all render with it; the hardwood and softwood entries exist only for their weight densities. It is the default look for users who never pick a species.
- **Still open:** `mdf_moulding` and `finger_jointed_pine` are usually painted (see open questions). Until that's settled, they fall back to the generic wood.

## Integration

- **Core (Phase 1, done):** `core/src/visualization/wood/`, made up of `noise`, `path` (compact relative encoding), `grain` (per-piece layers), `appearance` (data) and `mod`.
  - Entry point: `frame_face_svg(&FrameFace { appearance, x, y, width, height, frame_width, px_per_in, seed, lod, depth, depths, id_prefix })`.
  - It returns three fragments in drawing order: `under` (the wall shadow), `face` (the four clipped pieces plus tone and seams), and `inner_shadow` (drawn over the mat, under the opening's stroke).
  - `board_svg` renders a reference board for validation.
  - `examples/wood_grain.rs` reports sizes and writes boards and frames.
- **Options:** `DiagramOptions` gains `wood: Option<WoodRender { species: String, lod: WoodLod /* Off | Flat | Grain */, reshuffle: u32 }>`. The default is off, so **every existing golden SVG stays unchanged**.
- **Plan view (`plan_svg.rs`):** a new frame-face layer under the existing strokes: four clipped sides, grain-direction tone, seams, and the inner shadow.
- **Section view:** fill `MaterialPatterns.frame` with the species' base color. This is a cheap, immediate win; end grain comes later.
- **Seeding:** hash(species key, reshuffle counter); each side gets a sub-seed from its index.
- **Bindings:** the WASM layer and the mobile bridge (`api/simple.rs`) pass species and LOD into the diagram and preview calls.
- **Web:** a toggle in the Advanced panel, reusing the existing `#wood-species` picker.
- **iOS:**
  - A "Wood grain" item in the Layers & Detail sheet, reusing `woodKey`.
  - `FramePreview` passes `Flat` while `_morphController` animates and `Grain` at rest. Both depth cues (inner and wall shadow) stay on throughout.
  - The Diagrams tab uses `Grain`.
- **Dark mode:** wood colors don't invert. Only the opacity of the seams and shadows changes; the wall shadow may need to become a light edge on a dark surface.

## Payload budget

- **Measured** (18×22″ frame with 1.5″ moulding at 18 px/in, `Grain` LOD): 35–53 KB per species, and 76 KB for quartersawn oak with its hundreds of ray flecks. The `Flat` LOD is 2.6 KB.
- Before the scale LOD and encoding work, the same frames were 56–216 KB; a straight port of the prototype was up to 768 KB.
- **Budget (tested):** at most **60 KB** per species and at most **80 KB** for quartersawn, at 20 px/in. The earlier 40 KB was a guess. The evidence for this budget is the flutter_svg device test, which measured about 8.5 ms to parse 100 KB of grain, and that happens once when the preview settles.
- **Levers in use:**
  - relative path commands, implicit command repetition, and `c` then `s` for streak curves
  - 0.1 px precision
  - RDP polylines
  - culling of off-face runs
  - ring and pore-row decimation by scale
  - grouped shared attributes for pores and flecks

## Tests

- **In place (Phase 1, 23 tests in `wood/`):**
  - noise parity with the prototype
  - determinism per seed
  - output safe for flutter_svg (no `%`, filters or patterns; balanced groups)
  - payload budget per species
  - `Flat` LOD emits no grain
  - depth cues scale with the design's depths
  - ring-width CV within [0.25, 0.7]
  - arch apexes joined
  - ring-porous species emit pores but no ring-boundary lines
  - quartersawn emits flecks
  - every wood key resolves, and aliases and variants work
- **Phase 2:** fixed-seed snapshots per species in the golden matrix, behind the new option.
- **Guard still to add:** no loops with the pith on the face for straight-grained species.
- **iOS:** on the oldest supported device class (iOS 15), measure how long the switch from flat color to grain takes once the morph settles.

## Phases

1. ~~Core generator, `appearance` data and exporter, off by default, with tests.~~ Done 2026-10-06.
2. Plan-view face layer and section-view species color.
3. Web toggle.
4. iOS: Layers toggle, preview LOD, device performance check.
5. Later:
   - species in shareable URLs (format v3, appending one byte)
   - per-frame variation within `per_frame_range`, plus a "reshuffle" button
   - figured variants (curly, quartersawn)
   - end grain in the section view
   - more species via the pipeline
   - cutting the four sides from one virtual board, so grain flows around the corners
   - ragged streak tips for close zoom

## Open questions

- Should grain be on by default in the plan view (a technical drawing, where dimension callouts must stay legible), or only in the preview by default?
- Painted, MDF and finger-jointed stock: a paint-color picker, or a neutral finish?
- Should per-frame variation within the reviewed range be on by default?
- Should shareable URLs carry the species (format v3)?
