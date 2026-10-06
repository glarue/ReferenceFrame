# Wood-Grain Frame Rendering Plan

> **Status (2026-10-06):** Active. The prototype generator and parameters for 17 species plus a synthetic generic wood are done. They live in the private `tools/wood-fit/` repo (`chosen.json`, `gen_fit.py`, README). The Rust port has not started.

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
2. **Color streaks**: soft-edged (three nested bands) and patchy (they pinch off along the length), with capped opacity, so they never read as full-length stripes.
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
7. **Sheen gradient** across the width, plus **miter seam** lines.

**Anti-repetition rules** (from a two-agent panel review plus by-eye review):
- Every layer draws from its own hash-derived random stream (lowbias32 or splitmix of seed + layer tag), so a count change in one layer, such as during a dimension change, never reshuffles another.
- fBm uses a detuned lacunarity (about 2.03) and per-octave offsets.
- Nothing is periodic: no fixed spacings and no repeating dash lists.
- All four sides are cut consecutively from one virtual board.

**Determinism:** geometry uses basic float operations plus `sqrt`. `exp` appears only in lognormal draws, and output is rounded to 0.1 px. Output is identical on a given platform; bit-identical output across platforms is not required.

**Pixel floors:** lines are at least 0.45 px and pores at least 0.4 px, so features survive preview scale.

## Data: `presets.json`

Add an `appearance` block to each `materials.woods.<key>`.

```json
"appearance": {
  "palette": {"base": "#…", "late": "#…", "streak": "#…", "alt": "#…"},
  "structure": {"figure": "cathedral", "line_mode": "none", "zone_side": 1, "pores": true, "flecks": false},
  "params": {"rings_per_in": 3.9, "pore_len_in": 0.075, "…": "…"},
  "per_frame_range": {"ring_cv": [0.38, 0.69], "…": "…"},
  "source": "palette and texture statistics fitted to reference photos"
}
```

- An exporter script in `tools/wood-fit/` writes these blocks from `chosen.json` (`final_fixed`).
- **Covered:** red oak, white oak (plain-sawn, the default for `white_oak`), white ash, black walnut, Honduran mahogany, eastern white pine, ponderosa pine, Douglas-fir, western red cedar, yellow poplar, basswood, red alder, soft maple, hard maple, black cherry, and yellow birch. White oak (quartersawn) is a figure variant.
- **Generic wood:** a single synthetic appearance, not a real species: a neutral medium-brown palette with hard-maple-inspired texture (fine continuous lines, no visible pores, straight grain). `generic`, `generic_hardwood` and `generic_softwood` all render with it; the hardwood and softwood entries exist only for their weight densities. It is the default look for users who never pick a species.
- **Still open:** `mdf_moulding` and `finger_jointed_pine` are usually painted (see open questions).

## Integration

- **Core:** a new `core/src/visualization/wood/` module (noise, rings, ribbon, pores, flecks, streaks, path encoding). Entry point: `side_grain(appearance, seed, length_in, width_in, scale) -> String`.
- **Options:** `DiagramOptions` gains `wood: Option<WoodRender { species: String, lod: WoodLod /* Off | Flat | Grain */, reshuffle: u32 }>`. The default is off, so **every existing golden SVG stays unchanged**.
- **Plan view (`plan_svg.rs`):** a new frame-face layer under the existing strokes: four clipped sides, sheen, and seams.
- **Section view:** fill `MaterialPatterns.frame` with the species' base color. This is a cheap, immediate win; end grain comes later.
- **Seeding:** hash(species key, reshuffle counter); each side gets a sub-seed from its index.
- **Bindings:** the WASM layer and the mobile bridge (`api/simple.rs`) pass species and LOD into the diagram and preview calls.
- **Web:** a toggle in the Advanced panel, reusing the existing `#wood-species` picker.
- **iOS:**
  - A "Wood grain" item in the Layers & Detail sheet, reusing `woodKey`.
  - `FramePreview` passes `Flat` while `_morphController` animates and `Grain` at rest.
  - The Diagrams tab uses `Grain`.
- **Dark mode:** wood colors don't invert; only sheen and seam opacity change.

## Payload budget

- **Prototype sizes:** 3″×6″ boards at 150 px/in ran 45–190 KB. Variable-width traces add about 25%, and roughly double the size on dense softwood boards.
- **Budget:** preview canvases are far smaller (about 18–40 px/in), but set a budget and test it: **≤ 40 KB per preview SVG at `Grain`**.
- **Levers:**
  - relative path commands with the `s` shorthand, which is lossless for Catmull-Rom segments
  - 0.1 px precision
  - point thinning that keeps apexes
  - culling of off-face runs
  - dropping pores and flecks when they fall below the pixel floor

## Tests

- **Determinism:** fixed-seed snapshots per species, added to the golden matrix behind the new option.
- **Payload:** assert each species' preview SVG is within budget.
- **Guards** (cheap statistics on the generated geometry):
  - ring-width CV within [0.25, 0.7]
  - no loops with the pith on the face for straight-grained species
  - ring-porous species emit no ring-boundary strokes
- **iOS:** on the oldest supported device class (iOS 15), measure how long the switch from flat color to grain takes once the morph settles.

## Phases

1. Core generator, `appearance` schema and exporter, off by default, with tests.
2. Plan-view face layer and section-view species color.
3. Web toggle.
4. iOS: Layers toggle, preview LOD, device performance check.
5. Later:
   - species in shareable URLs (format v3, appending one byte)
   - per-frame variation within `per_frame_range`, plus a "reshuffle" button
   - figured variants (curly, quartersawn)
   - end grain in the section view
   - more species via the pipeline

## Open questions

- Should grain be on by default in the plan view (a technical drawing, where dimension callouts must stay legible), or only in the preview by default?
- Painted, MDF and finger-jointed stock: a paint-color picker, or a neutral finish?
- Should per-frame variation within the reviewed range be on by default?
- Should shareable URLs carry the species (format v3)?
