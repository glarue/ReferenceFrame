# Photo Preview Plan

Show the frame as it will look, with the user's own photo as the artwork. iOS first; web later.

**Status:** scoped 2026-10-10, not started. Replaces the "Photo-as-artwork preview" item in `WOOD_GRAIN_PLAN.md` Phase 5.

## Decisions (user, 2026-10-10)

- **Pop-up, not a tab.** A full-screen preview opened from the live preview and from the Diagrams tab.
- **One photo per design.** The goal is a record of which photo went with which frame, so photos travel with saved configs and history entries.
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
- **Backup export/import (`exportAll`):** JSON only, so photos aren't included at first (note this in the UI). Later this could become a zip.
- **Share links:** carry no photo.
- **Open:** should the wood look (species, tone, board) also be stored per design, so a history entry restores the full look? Today it is a global preference.

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
- Only when the design has a photo, or always (with a placeholder)? Proposed: only with a photo.

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
