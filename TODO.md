# TODO

Future improvements worth doing but not yet prioritized.

---

## Client-side text width calibration

**File:** `core/src/visualization/geometry.rs` + `style.rs`

**Problem:** `estimate_text_width` uses a character-class heuristic (average char width × count). This is systematically off for the actual font rendered on device/browser. Text width errors flow into:
- Mat cut label extent in `compute_plan_viewbox` → affects horizontal centering
- Thumbnail label extent → affects horizontal centering (portrait frames)
- Horizontal callout `label_bounds` widths → affects collision detection accuracy

**Proposed fix:** Add a `text_width_scale: f64` field to `DiagramStyle` (default `1.0`). On the client, after loading the font, measure a handful of representative strings (e.g., `"10 1/4\""`, `"37 1/4\""`, `"Mat Cut: 2 3/8\""`) via `getBoundingClientRect()` (web) or `TextPainter.width` (Flutter). Compute `actual / estimated`, average the ratios, store the result, and pass it as `text_width_scale` when constructing `DiagramStyle`. The core multiplies all `estimate_text_width` results by this factor.

**Notes:**
- A single scalar works reasonably well because the estimation error is largely a constant proportional bias (not per-character-class)
- Recalibrate if the font stack changes
- Thread `style` (or the scale factor) through `estimate_text_width` call sites — moderate refactor
- This won't fix asymmetries caused by one-sided callout stacking or overlay placement, but it improves the accuracy of all text-dependent bounds

