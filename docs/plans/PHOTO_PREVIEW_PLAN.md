# Photo Preview Plan

Show the frame as it will look, with the user's own photo as the artwork. iOS first; web later.

**Status:** iOS built 2026-10-10 (not yet on TestFlight); web not started. Replaces the "Photo-as-artwork preview" item in `WOOD_GRAIN_PLAN.md` Phase 5.

**As built (iOS):**
- **Core:** `generate_presentation` + `PhotoFit::place` + `preview_photo_layout` (`visualization/presentation.rs`), `mat_colors` in `presets.json`, `DesignLook` (`look.rs`) on `HistoryEntry.look` (an entry matches only with the same mat, wood and photo; a match takes the new crop). Review renders: `cargo run --example presentation -- <dir> [photo.jpg]`.
- **Bridge:** `generate_presentation_json`, `preview_photo_layout`, `place_photo`; `add_to_history_with_unit` takes `look_json`.
- **App:**
  - `PhotoStore` (`photos/<id>.jpg`; picked with `image_picker`, then orientation baked, resized and re-encoded as JPEG q85 in an isolate). The photo size setting is in Settings › Data & History.
  - `PhotoPreviewScreen` (tap the live preview, the Diagrams app bar, or Preview in either action menu): the photo is a layer under the SVG; drag/pinch, Fill/Fit, Match Artwork to Photo, mat swatches, and share as PNG.
  - The live preview draws the photo over the diagram's window.
  - The PDF gets a second page when the design has a photo (the JPEG sits under the render, so it stays small).
  - Unreferenced photo files are pruned at launch and after photo or history changes.
  - Backups embed history photos (base64) and are shared as a `.json` file.
- **Differences from the plan:**
  - Saved configs are frame setups without the artwork, so they keep the mat colour and wood but not the photo; the photo is per history entry.
  - The current design's mat colour and photo persist as `pref_design_look`.
- **Not verified on device:** the native photo picker flow (the simulator checks injected a photo).

## Decisions (user, 2026-10-10)

- **Pop-up, not a tab.** A full-screen preview opened from the live preview and from the Diagrams tab.
- **One photo per design.** The goal is a record of which photo went with which frame, so photos travel with history entries (saved configs keep the mat and wood only; see "As built").
- **PDF:** the clean render goes in its own page or section; the measured diagram is unchanged.
- **Mat colour:** a selector with preset common mat colours (no free colour picker).
- **Wood grain:** the preview respects the Wood Grain layer switch. With it off, the frame is drawn flat (no species is forced).

## Pieces

### 1. Core: presentation render (~1 day)
- A `DiagramOptions` presentation mode (plan view only), which leaves out dimensions, callouts, dashed outlines, the overlap zone and labels.
- What it draws:
  - the frame face (wood grain if `wood` is set, else a flat frame colour)
  - a filled mat in the chosen colour, with a thin bevel line at the opening
  - the frame's inner shadow on the mat, and a drop shadow on the wall
  - the window left **transparent**, so a photo can sit underneath
- The window is the mat opening, or the frame's sight opening when there's no mat. It is returned in the result (px rect), next to the SVG.
- **Static outputs** (PDF, PNG, web) can instead embed the photo as a `data:` `<image>` clipped to the window. flutter_svg and the Dart `pdf` package both decode these.
- **Mat colours:** a preset list in `presets.json` (white, off-white, antique white, cream, light grey, charcoal, black); `FrameDesign.mat_color` stores the key (`serde(default)`).
- **Tests:** goldens for the presentation render (mat and no mat, wood on and off), plus window-rect checks.

### 2. Per-design photo storage (~1–1.5 days)
- **Files:** `Documents/photos/<uuid>.jpg`.
  - `image_picker` uses PHPicker, so there's no permission prompt.
  - It downscales to about 2000 px on the long side and converts HEIC to JPEG at roughly 0.85 quality.
- **Current design:** carries a `photo_id`, which is persisted with the working design.
- **Saved configs:** store `photo_id`. Loading a config restores its photo.
- **History:** `HistoryEntry.photo_id: Option<String>` in core (`serde(default)`).
  - The dedup key includes it, so the same frame with a different photo is a new entry.
  - Bridge: `add_to_history*` takes the id.
  - History rows show a thumbnail.
- **Cleanup:** on launch, and after a history trim or delete, remove photo files that nothing references (current design, configs or history).
- **Stored resolution (user, 2026-10-10):** each photo is kept as one downscaled JPEG, sized by a setting: "Photo size" Small 800 px / Medium 1200 px (default) / Large 2000 px on the long side. That's about 80 / 180 / 450 KB.
- **Backup export/import (`exportAll`):** embeds each referenced photo as base64 in the backup JSON, so the photo travels with its design. Import writes the files back and keeps the ids.
- **Why files, not base64 in the records:** the history and configs live in SharedPreferences (NSUserDefaults). 50 entries × ~180 KB inline would load about 10 MB at every launch.
- **Share links:** carry no photo.
- **Wood look per design (user, 2026-10-10):** species, tone and board are saved with configs and history entries, and restored on load. The global preference stays as the working value.

### 3. iOS preview pop-up (~1–1.5 days)
- **Layout:** the photo as a native `Image` under the SVG in a `Stack`, positioned and clipped to the window rect. This avoids re-parsing a base64 photo every frame.
- **Controls:**
  - choose / replace / remove photo
  - fill or fit, plus drag and pinch to reposition (crop stored normalised per design)
  - "Match artwork size to photo" (keeps the long side, sets the other from the photo's aspect)
  - mat colour swatches (hidden when there's no mat)
  - Share / Save image (`RepaintBoundary.toImage` → share sheet)
- **No photo:** a neutral placeholder in the window, with a "Choose photo" prompt.

### 4. Live calculator preview (~0.5 day)
- The same layering. The bridge preview calls also return the window rect, so the photo follows the morph animation.

### 5. PDF (~0.5 day)
- A second page (or a section), "Preview": the clean render with the photo embedded, plus a caption (size, wood, mat colour).
- Only when the design has a photo (user, 2026-10-10).

### 6. Privacy and release (~0.5 day)
- Update `PRIVACY_POLICY.md` / `platforms/web/privacy-policy.html:85`, which currently say the app doesn't access photos. New wording: you choose photos with the system picker, and they're stored only on the device.
- Add `NSPhotoLibraryUsageDescription` to Info.plist.
- The App Store privacy label stays "Data Not Collected".

### 7. Web (later, ~1–1.5 days)
- `<input type=file>`, the photo embedded in the presentation SVG, and per-design storage in IndexedDB.

## Estimate
- **iOS:** about 4.5–5.5 days (per-design storage adds about a day to the single-photo scope).
- **Web:** about 1–1.5 more.

## Order
1. Core (presentation mode, `mat_color`, `HistoryEntry.photo_id`, window rect) → goldens.
2. Bridge.
3. Photo storage and cleanup (with unit tests).
4. Preview pop-up.
5. Live preview.
6. PDF page.
7. Privacy.
8. TestFlight.
