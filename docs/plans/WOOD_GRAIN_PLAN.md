# Wood-Grain Frame Rendering Plan

> **Status (2026-10-07):** Active. **Phases 1–4 are done**, plus tones and new boards (Phase 4b, iOS; not yet on TestFlight). iOS is on TestFlight (1.12.0, latest build 52; the user tested on device: "works well overall"). Not yet pushed or released: web deploy, App Store submission (see "Resume here"). The generator is in `core/src/visualization/wood/`, its data (17 species plus a synthetic generic wood) is in `core/data/wood_appearance.json`, and the core diagrams draw it when `DiagramOptions::wood` is set (plan view, corner-detail inset, section colour, live-preview shape). With `wood` unset the output is unchanged. The web app has a "Wood grain" toggle; the iOS app has a "Wood Grain" layer switch. The parameters come from the private `tools/wood-fit/` repo (`chosen.json` and the `gen_fit.py` reference prototype).

**Goal:** draw the frame face with species-specific procedural wood grain, replacing today's outline-only plan view and flat brown section fill, in the plan view and live preview on both web and iOS. The species is the one users already pick for the weight estimate (`materials.woods`). The aim is "close in spirit" rather than photoreal: deterministic, vector, and one implementation in `core/` for both platforms.

---

## Resume here (state as of 2026-10-07)

**Next steps (all need the user's go-ahead):**
1. Delete the local branch `backup/pre-source-scrub` (old history that names the photo source; must never be pushed).
2. Before App Store submission of 1.12.0: move the local tags `core-v1.12.0`, `app-v1.12.0`, `bridge-v1.8.0` to the current commits (they point at the build-48 commits; otherwise `release.sh` proposes 1.13.0). Nothing is pushed, so this is safe.
3. Push the root repo (`git push --follow-tags`), which deploys the web app via CI. The mobile and wood-fit repos have no remote.
4. App Store: `fastlane submit build:52 version:1.12.0` (or a later build); draft notes with `/release-notes`. Never name the wood photo source in notes or UI. Phase 4b (tones, new boards) is committed after build 52: ship it as a new TestFlight build first (it's a `feat`, so `release.sh` would propose 1.13.0 unless the 1.12.0 tags are moved past it).
5. Optional: photo-as-artwork preview (assessment under Phase 5).

**Done:**
- **Phase 1:** the generator in core, its data file, and tests.
- **Payload optimization:** compound paths.
- **Phase 2:** `DiagramOptions::wood` (`WoodRender`) wired into the plan view, corner-detail inset and section view, with golden cases. Reviewed by eye with the user.
- **Phase 3 (web):** "Wood grain" toggle in the View section's layer toggles; diagram, print and PDF export. Deploys on the next push to `main`.
- **Phase 4 (iOS):** "Wood Grain" layer switch, swatch species picker, preview/diagram/PDF. TestFlight 1.12.0 builds 48–52.
- **Phase 4b (tones and new boards, iOS):** Lighter / Natural / Darker per species, and a "New board" shuffle (with "Original" to go back), in the wood picker. See Phase 4b below.
- **Section-view layout overhaul (not wood-specific, done alongside):** one fixed scale whatever the face width (3" reference, break above 3"), the stack total as a "Total" row of the label column, wrapped labels, even spacing in the combined view, collision-free rabbet caption. See ARCHITECTURE.md "Section view scale" / "Combined view layout"; commits `59ae86a`…`bcc8c8e`.
- **Source hygiene:** no public file or commit names the reference-photo source (unpushed history was rewritten 2026-10-06; final tree verified identical). The weight-estimate density citations in `presets.json` are separate and already public.

**Where things live:**

| What | Where |
|---|---|
| Generator | `core/src/visualization/wood/`: `mod.rs` (API), `grain.rs` (layers), `path.rs` (encoding), `noise.rs`, `appearance.rs` (data types and loader) |
| Public API | `visualization::{WoodRender, WoodLod, WoodTone, DepthCues, seed_for, frame_face_svg, corner_face_svg, wood_swatch_svg, wood_looks, FrameFace, FrameFaceSvg, FaceDepths, wood_appearance, board_svg}` |
| Diagram wiring | `DiagramOptions::wood` (`visualization/types.rs`); plan face + corner inset in `plan_svg.rs`; section colour in `svg.rs::generate_diagram_with_style` |
| Data (generated, don't hand-edit) | `core/data/wood_appearance.json` |
| Dev utility | `cargo run --release --example wood_grain -- sizes \| boards <dir> [n] \| frames <dir> \| diagrams <dir>` (`diagrams` = the Phase 2 review cases, full diagrams light/dark/preview) |
| Offline tooling (private nested repo, local only) | `tools/wood-fit/`; see its README for the fit → blind review → `chosen.json` → `export_appearance.py` pipeline and `validate_rust.py` |
| Reference photos (third-party, never committed or shipped) | `tools/wood-fit/photos/` (git-ignored; see the wood-fit README) |

**Decisions (made with the user; don't re-litigate):**
- **Look:**
  - Flat-faced pieces, with no rounded-profile shading.
  - Vertical pieces about 5% darker than horizontal ones.
  - Depth cues: inner shadow in the plan view; inner plus wall shadow in the live preview.
- **Species:**
  - The term is "plain-sawn", not "flat-sawn".
  - One synthetic **generic wood** (neutral medium brown, maple-like texture) covers `generic`, `generic_hardwood` and `generic_softwood`. The hardwood and softwood keys stay in the weight picker for their densities.
  - Ring-porous woods (oak, ash) have no ring-boundary line.
  - The user prefers finer pores with light-tone variation on oaks.
- **Review process:** species looks are approved by blind by-eye review. Metrics are guards, not judges; DISTS was tested and dropped.
- **Budget:** at most 60 KB per preview frame (80 KB for quartersawn) at 20 px/in; at most 100 KB per full diagram (125 KB quartersawn). The `Flat` LOD is for animation frames.
- **Wood is purely additive.** Turning it on hides no other display element (dashed content/mat boundaries, dimensions, overlays all stay, in the preview too). The one exception: the semi-transparent rabbet-overlap tint is dropped over wood (it read as a muddy stripe); the dashed content boundary still marks the overlap.
- **Where it can be toggled:** live preview, plan view (Diagrams tab / web diagram) and PDF output. **One switch drives all of them** (user decision 2026-10-06), on each platform. Context: the user may later add a separate photo-style "preview" mode with a user-loaded image as the artwork placeholder (Phase 5); wood is still a toggle everywhere else.
- **Corner-detail inset:** gets grain too (two short boards at the inset's zoom, same per-side seeds as the full face's bottom and left pieces; they don't match the main view's corner exactly).
- **Dark mode:** as is. Wood colours don't invert, the frame outline stays light gray, and the wall shadow is simply invisible on dark surfaces (no light edge).
- **Rejected:** cubic Bézier fitting (bigger files), `<pattern>` tiles (flutter_svg limits, visible repeats), and SVG filters (unsupported on iOS).

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

**Determinism:** geometry uses basic float operations plus `sqrt`/`hypot`; `exp`, `powf` and one `sin` use the platform libm, and output is rounded to 0.1 px. Output is identical on a given platform; bit-identical output across platforms is not required. The wood golden SVGs are generated on macOS and checked on Linux CI, so a libm last-bit difference that flips a 0.1 px rounding would show up there (judged very unlikely; if it happens, switch those calls to the `libm` crate, which WASM already uses under the hood).

**Pixel floors:** lines are at least 0.45 px and pores at least 0.4 px, so features survive preview scale.

**Scale LOD** (inactive at validation scale; at preview scale these cut payload 2–4×):
- Rings closer than 2.5 px are decimated: every k-th ring is drawn, and its line, zone and pore opacity are raised by k^0.75 so overall darkness holds.
- Pore rows are capped to stay at least 1.2 px apart, with the same opacity compensation.
- Every run is simplified with Ramer–Douglas–Peucker at 0.25 px, keeping arch apexes, and drawn as a polyline. Within that tolerance, a smooth curve adds bytes but nothing visible.
- Variable-width outlines are used only where the trace is at least 1 px wide; thinner traces are strokes.
- Colour-streak outlines are sampled about once per inch but at least 24 px apart, so long sides at small scales don't pay for detail their soft edges can't show (this halved an 80×80″ frame's diagram, 96 → 49 KB; inactive at ≥ 24 px/in).

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
      "per_frame_range": {"ring_cv": [0.38, 0.69], "…": "…"},
      "tones": {"light": {"base": "#…", "…": "…"}, "dark": {"…": "…"}}
    },
    "white_oak": {"…": "…", "variants": {"quartersawn": {"…": "…"}}}
  }
}
```

- Tests check that every `materials.woods` key resolves to a look, that every appearance key is a real wood key, and that every look has tones.
- **`per_frame_range`:** each knob's span over the top-3 by-eye picks. A reshuffled board (`reshuffle` ≠ 0) draws each knob within it (see Phase 4b).
- **`tones`:** lighter and darker palettes, computed by `tools/wood-fit/tones.py` (see Phase 4b).
- **Covered:** red oak, white oak (plain-sawn, the default for `white_oak`), white ash, black walnut, Honduran mahogany, eastern white pine, ponderosa pine, Douglas-fir, western red cedar, yellow poplar, basswood, red alder, soft maple, hard maple, black cherry, and yellow birch. White oak (quartersawn) is a figure variant.
- **Generic wood:** a single synthetic appearance, not a real species: a neutral medium-brown palette with hard-maple-inspired texture (fine continuous lines, no visible pores, straight grain). `generic`, `generic_hardwood` and `generic_softwood` all render with it; the hardwood and softwood entries exist only for their weight densities. It is the default look for users who never pick a species.
- **Still open:** `mdf_moulding` and `finger_jointed_pine` are usually painted (see open questions). Until that's settled, they fall back to the generic wood.

## Integration

- **Core (Phase 1, done):** `core/src/visualization/wood/`, made up of `noise`, `path` (compact relative encoding), `grain` (per-piece layers), `appearance` (data) and `mod`.
  - Entry point: `frame_face_svg(&FrameFace { appearance, x, y, width, height, frame_width, px_per_in, seed, lod, depth, depths, id_prefix })`.
  - It returns three fragments in drawing order: `under` (the wall shadow), `face` (the four clipped pieces plus tone and seams), and `inner_shadow` (drawn over the mat, under the opening's stroke).
  - `board_svg` renders a reference board for validation.
  - `examples/wood_grain.rs` reports sizes and writes boards and frames.
- **Options (done):** `DiagramOptions.wood: Option<WoodRender>`; JSON `"wood": {"species": "red_oak", "variant": null, "tone": "light" | "natural" | "dark", "lod": "grain" | "flat", "depth": "none" | "inner" | "inner_and_wall", "reshuffle": 0}`, every field but `species` optional (defaults `natural`, `grain`, `inner`, 0). `null` or omitted = off, byte-identical to before. `DiagramRequest` flattens the options, so the web and iOS JSON entry points take it as is.
- **Plan view (done, `plan_svg.rs`):** a `<g id="wood">` layer right after the SVG header, under every stroke, overlap fill, axis-break mask and dimension: wall shadow (if `inner_and_wall`), the four clipped pieces with grain-direction tone and seams, then the inner shadow. Axis-break ribbons (background-filled) mask the grain in the gap. Clip/gradient ids get a per-diagram prefix (`wg<hash>-`, corner inset `wd<hash>-`).
- **Corner-detail inset (done):** grain at the inset's scale via `WoodRender::corner_face` / `corner_face_svg`.
- **Section view (done):** `generate_diagram_with_style` clones the style and sets `material_patterns.frame` to the species' base colour (profile and legend swatch). End grain comes later.
- **Seeding (done):** `seed_for(species, variant, reshuffle)` = FNV-1a of the key (plus `/variant`) mixed with `reshuffle`; each side gets a sub-seed from its index. The tone doesn't change the seed (same boards, different colour).
- **Bindings:** the WASM layer and the mobile bridge (`api/simple.rs`) pass species and LOD into the diagram and preview calls.
- **Web:** a toggle in the Advanced panel, reusing the existing `#wood-species` picker.
- **iOS:**
  - A "Wood grain" item in the Layers & Detail sheet, reusing `woodKey`.
  - `FramePreview` passes `Flat` while `_morphController` animates and `Grain` at rest. Both depth cues (inner and wall shadow) stay on throughout.
  - The Diagrams tab uses `Grain`.
- **Dark mode (decided):** wood colours don't invert; nothing else changes (see Decisions).

## Payload budget

- **Measured** (18×22″ frame with 1.5″ moulding at 18 px/in, `Grain` LOD): 35–49 KB and 60–134 elements per species, and 60 KB / 132 elements for quartersawn oak. The `Flat` LOD is 2.6 KB.
- **Compound paths:** within each piece, same-style elements of a layer merge into one path. This cut element counts by about half (518 → 132 for quartersawn) and bytes by 5–15%; elements are what drive parse time on iOS and layout on web.
  - Random opacity, width and dash choices are drawn from 3 levels (or 4 shared dash lists) so that elements can share a style.
  - Each dashed run starts at a random interior point and draws outward both ways. Dash patterns restart per sub-path, so without this the merged rows would line up in columns.
  - Filled outlines are normalized to one winding direction, so overlaps union instead of cancelling into holes.
  - What remains is mostly fixed per-frame structure: clips, groups, shadows, seams, and 24 streak bands, which stay separate so their nested soft edges don't collapse.
- **Tried and rejected: cubic Bézier fitting** (Schneider's algorithm on the full-density points). It came out 25–100% *larger* than the simplified polylines, even at twice the tolerance. The noise-warped grain lines wiggle every few px, so a cubic (6 numbers) rarely replaces more than about 3 polyline vertices (2 numbers each).
- Before the scale LOD and encoding work, the same frames were 56–216 KB; a straight port of the prototype was up to 768 KB.
- **Full diagrams (tested, Phase 2):** at most **100 KB** per species (**125 KB** quartersawn) on the 800×600 canvas across 8×10, 16×20 matted, 8×60 (breaks), 80×80 and 4×6 frames, with and without callouts. Worst measured: Douglas-fir 8×60 at 88 KB (the corner inset adds 12–20 KB); quartersawn 80×80 at 117 KB.
- **Budget (tested):** at most **60 KB** per species and at most **80 KB** for quartersawn, at 20 px/in. The earlier 40 KB was a guess. The evidence for this budget is the flutter_svg device test, which measured about 8.5 ms to parse 100 KB of grain, and that happens once when the preview settles.
- **Levers in use:**
  - relative path commands, implicit command repetition, and `c` then `s` for streak curves
  - 0.1 px precision
  - RDP polylines
  - culling of off-face runs
  - ring and pore-row decimation by scale
  - compound paths per style

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
- **Phase 2 (in place):**
  - golden cases: `wood_red_oak_matted_16x20_plan_inches` and `wood_douglas_fir_standard_8x10_plan_inches` (grain), `wood_generic_tall_8x60_plan_inches` (grain + axis breaks + textured corner inset), `wood_flat_matted_16x20_both_inches` (section colour, combined view), `wood_flat_portrait_16x20_preview` (no callouts, wall shadow)
  - full-diagram payload budget (`wood::tests::diagram_payload_within_budget`)
  - JSON defaults and bad values (`diagram_request::tests::wood_takes_documented_defaults`); `wood: null` and omitted render identically for all three views
  - seed stability; flutter_svg-safe output includes the corner inset
- **Guard still to add:** no loops with the pith on the face for straight-grained species.
- **iOS:** on the oldest supported device class (iOS 15), measure how long the switch from flat color to grain takes once the morph settles.

## Phases

### Phase 1: core generator (DONE 2026-10-06)
Root commits `d87d943`, `0ac8f51` (compound paths) and docs; tools repo `d60a926`.

### Phase 2: wire into the core diagrams (DONE 2026-10-06)
Root commit: `feat(core): draw wood grain in diagrams via DiagramOptions::wood`. What shipped is described under Integration, Payload budget and Tests. Notes from the build:
- The wall shadow is drawn as offset rects *minus the frame rectangle* (evenodd), not full rects: the diagrams have no opaque mat, so full rects darkened the whole opening.
- The rabbet-overlap tint is skipped over wood (plan view and corner inset); everything else is unchanged.
- The combined (`Both`) view renders the plan twice (probe + final), so grain is generated twice; fast enough, not optimized.
- Review renders: `cargo run --release --example wood_grain -- diagrams <dir>`, rasterize with `resvg` (the wood-fit venv has `resvg_py`).

### Phase 3: web (DONE 2026-10-06)
Root commits `dcbd945` (core: the combined view's sizing probe skips the grain; halves its cost) and `585749e` (web).
- **Toggle:** `#show-wood` "Wood grain" in the View section beside Spline slots / Hanging hardware (more reachable than the Advanced panel). Species = the existing Materials › Wood picker (subtitle now "Materials", hint "Weight estimate and the wood-grain look"). Persisted as `STORAGE_KEYS.SHOW_WOOD` (`rf_show_wood`, `"true"`/`"false"`) and carried in backups as `preferences.show_wood` (backup format stays 1.1: optional field).
- **Requests:** `woodOption(depth)` in `index.html` builds `{species, lod: "grain", depth}` or null. The on-screen diagram uses `depth: "inner"`; browser print reuses the on-screen render; the jsPDF/svg2pdf export uses `depth: "none"`, because svg2pdf ignores gradient stop-opacity and drew the inner shadow as a flat gray band.
- **Measured (headless Edge):** WASM render with wood about 5–8 ms (combined view, warm; 23 ms for the first render), plan view 8 ms. The toggle persists across reloads. The PDF export took 0.1–0.5 s and was about 250–340 KB.
- **Browser check harness:** a CDP script drove headless Edge (`/Applications/Microsoft Edge.app`) against `serve.py`; Node 25 has `WebSocket` built in, so no Playwright is needed. jsPDF's `save` lives on `jsPDF.API`; hook it there to capture the export.
- **Not done:** history thumbnails don't render diagrams, so nothing to do there.

### Phase 4: iOS (built 2026-10-06; on-device check and release pending)
Root `62078f3` (core fix below); mobile repo `edefb7a` (bridge) and the `feat(app)` commit after it.
- **One switch:** "Wood Grain" in Layers & Detail (`lib/widgets/layers_detail_sheet.dart`, hint "Frame in your Materials wood, here and in the preview"), persisted as `pref_show_wood`. It drives the live preview, the Diagrams tab and the PDF. The species is the existing Materials wood picker (`woodKey`). The Layers button counts wood as an active layer (dot + VoiceOver value "… and wood grain on").
- **Diagrams tab and PDF:** `DesignState.woodOption()` adds `'wood': {species, lod: 'grain', depth: 'inner'}` to the combined-view request (screen and PDF alike). `VisualizationScreen._diagramKey` includes `showWood` and `woodKey`.
- **Live preview:** the bridge's `generate_preview_svg` / `generate_interpolated_preview_svg` take an optional `wood_json` (core `WoodRender` JSON; bindings regenerated with `flutter_rust_bridge_codegen generate`). `FramePreview` sends `lod: 'flat'` for the interpolated morph frames and `'grain'` for the settled SVG, both with `depth: 'inner_and_wall'`; its `Selector` now watches `(designJson, showWood, woodKey)`. When wood is on it precaches the settled SVG with `SvgStringLoader(svg).loadBytes(context)` (same cache key `SvgPicture.string` uses), so parsing starts while the flat morph plays.
- **PDF finding (fixed in core):** the Dart `pdf` package resolves `clip-path` in the parent's coordinate space, ignoring the element's own `transform`, so every frame piece was clipped away. `Piece::draw` now emits `<g transform=…><g clip-path=…>` (equivalent everywhere). The `pdf` package renders the inner-shadow gradient correctly, so the iOS PDF keeps `depth: 'inner'` (only the web's svg2pdf needed `none`).
- **Checked off-device:** flutter_svg renders WASM-generated preview SVGs correctly (grain, clips, tone, seams, both shadows; flat and grain match in tone) in a widget test; the `pdf` package renders the PDF diagram (probe scripts in the session scratchpad, not committed). The full Flutter suite passes; analyzer shows only 3 pre-existing infos.
- **Preview payload is higher than the Phase 1 budget assumed:** the preview canvas is 2× the container (about 30 px/in, not 20), so the settled grain SVG is up to ~104 KB (Douglas-fir, 16×20″ with mat); 41–86 KB for other sizes. Generation is 3–11 ms (WASM; native is faster). It's generated once per change and parsed in the background, so it should be fine, but **measure on device**. Levers if needed: render the preview at 1.5× instead of 2×, or tighten the LOD floors for the preview.
- **TestFlight 1.12.0 (48)** carried the above; the user tried it on device: "works well overall".
- **Species picker (after 1.12.0 (48)):** the wood is chosen from a bottom sheet of generated swatches (`wood_swatch_svg`, a 2" square of the species' board, grain horizontal; 5–11 KB at 192 px; cached in `DesignState`). The grid shows `wood_looks()` (generic + species with their own look); the weight-only woods (generic hardwood/softwood, MDF, finger-jointed) are listed apart under "Weight only" (user: listing them among the looks is confusing). It opens from the Materials row (swatch + name, replacing the dropdown) and from a species row under Wood Grain in Layers & Detail. Glazing and backing became chips. iOS only for now (user); the web keeps its `<select>`.
- **Simulator UI checks without tapping:** a temporary `integration_test` + `integration_test_driver_extended` driver (`onScreenshot` writes PNGs), run with `flutter drive -d <simulator>`; not committed.
- **Status:** simulator-checked; on TestFlight (1.12.0 builds 48–52) and tried on device by the user. Release steps are under "Resume here". If the flat-to-grain settle swap ever proves slow on old devices, the fallback is a binary display list over FFI drawn by a `CustomPainter`.

### Phase 4b: tones and new boards (iOS, built 2026-10-07)
Not yet on TestFlight.
- **Tones (`WoodRender.tone`, `WoodTone::{Light, Natural, Dark}`):** each species' palette comes from the "oiled" target, a 50/50 CIELAB blend of the sanded and finished photos of one sample. `tools/wood-fit/tones.py` takes the per-role change from that blend to the sanded photo (lighter: paler, less amber) and to the finished photo (darker: richer).
  - **Lightness step:** the photo's step, clamped to 6–12 L\*. A finish changes pines and birch more in chroma than in lightness, so the clamp keeps every tone visibly lighter or darker. Roles move in proportion to their headroom, which keeps the grain contrast and never clips.
  - **Chroma:** the photo's per-role ratio, applied at constant hue (adding the photo's a\*/b\* deltas turned pale woods pink or grey). It is scaled up with a boosted lightness step, so a darker pale wood doesn't just grey.
  - **Saturation cap:** C\*/L\* is held to at most 1.15× the natural colour's, so dark cherry and alder stay brown instead of orange.
  - **Other cases:** out-of-gamut colours give up chroma at constant hue. The generic wood takes the mean shift.
  - Reviewed by eye with `review_tones.py` (local-only sheet). Core just picks `tones.light`/`tones.dark` (`WoodAppearance::palette_for`); the section fill follows the tone.
- **New boards (`reshuffle` ≠ 0):** `WoodAppearance::styled(tone, vary)` draws each `per_frame_range` knob uniformly within its reviewed span (integer knobs round), plus ring spacing within ±10% (`RINGS_SPREAD`), deterministically from the seed. `reshuffle` 0 is exactly the reviewed look (borrowed, byte-identical; goldens unchanged). The payload test covers reshuffles 1–3.
- **iOS:**
  - **Storage:** tone and reshuffle are kept **per wood key** in the `pref_wood_looks` JSON pref (`{key: {tone, reshuffle}}`). The species is itself a global preference, not part of the saved design, so the look sits next to it rather than per design.
  - **Picker:** the sheet now opens with a panel for the selected wood: a 64 pt swatch, the name, New board / Original, a Lighter·Natural·Darker segmented control and a one-line hint. Tapping a swatch selects it and keeps the sheet open; Done closes it.
  - **Swatches** use each wood's own tone and board (bridge `wood_swatch_svg(species, tone, reshuffle, size_px)`).
  - **Labels and rebuilds:** the Materials row and the Layers sheet show e.g. "Black Walnut · Darker". The preview and diagram rebuild keys use `woodLookKey`.
- Web unchanged: it sends no tone or reshuffle, so it gets the natural look.

### Phase 5: later
- **Photo-as-artwork preview** (user idea; assessed 2026-10-06, not started). A markup-free "Preview" page (frame + mat + the user's photo), optionally the photo in the live preview and a PDF page.
  - **Approach:** a core "presentation" diagram option (no dimensions/dashed lines, filled mat with bevel line, frame face, shadows) plus a bridge call returning the visible window rect. On iOS draw the photo as a native image layered under the SVG, clipped to that window (embedding base64 in the SVG would be re-parsed every animation frame). Both flutter_svg and the Dart `pdf` package do decode `data:` `<image>`s, so embedding is fine for static outputs (PDF, PNG export, web).
  - **iOS:** `image_picker` (PHPicker: no permission prompt; native downscale + HEIC→JPEG, ~1600 px). Keep one current photo in app storage. Fill + centre-crop by default; a "Match artwork size to photo" action.
  - **Must update:** `PRIVACY_POLICY.md` and the web privacy page (they say the app "does not access your photos"); add a photo-library usage string. Privacy label stays "no data collected" (local only).
  - **Effort:** ~2–3 days iOS-only (pick + preview page + share), ~4–5 with web and PDF. Open: storage per design or single; mat colour picker scope; PDF page 2 vs inset.
- **Web species picker with swatches** (iOS has one; web keeps its `<select>` for now).
- Species in shareable URLs (format v3, appending one byte).
- Tones and new boards on the web (core and WASM already take `tone`/`reshuffle`).
- Figured variants (curly, quartersawn) in the UI. The quartersawn white oak data is already in `wood_appearance.json` under `variants`.
- End grain in the section view.
- More species via the `tools/wood-fit` pipeline.
- Cutting the four sides from one virtual board, so grain flows around the corners.
- Ragged streak tips for close zoom.
- A test guard against loops from the pith landing on the face for straight-grained species.

## Open questions

- Should grain be on by default anywhere, or off everywhere until the user turns it on?
- One wood switch for preview, diagrams and PDF, or a switch per surface?
- Painted, MDF and finger-jointed stock: a paint-color picker, or a neutral finish? These currently fall back to generic wood.
- Should per-frame variation within the reviewed range be on by default?
- Should shareable URLs carry the species (format v3)?
