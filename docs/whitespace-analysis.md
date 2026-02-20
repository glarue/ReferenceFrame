# Whitespace Analysis for Cut Pieces Integration

## Current Layout Budget (Combined View)

### Overall Structure
```
┌─────────────────────────────────────────────┐
│ [Title Block]              (95px if present)│
│─────────────────────────────────────────────│
│                                             │
│ [Plan View]                (58% of avail)  │
│                                             │
│─────────────────────────────────────────────│
│ [Gap]                      (40px)          │  ← COMPRESSION TARGET #1
│─────────────────────────────────────────────│
│                                             │
│ [Section View]             (42% of avail)  │
│                                             │
│ [Legend gap]               (6px)           │  ← COMPRESSION TARGET #2
│ [Legend]                   (25px)          │
│─────────────────────────────────────────────│
│ [Bottom margin]            (~8px)          │
└─────────────────────────────────────────────┘

Available height = canvas_height - gap_between_views - title_height
```

## Current Margin Values

### DiagramStyle (core/src/visualization/style.rs)

| Parameter | Web/Screen | PDF | Notes |
|-----------|-----------|-----|-------|
| `margin` | 8.0 | 4.0 | General padding around diagram |
| `dimension_offset_base` | 22.0 | 28.0 | First dimension line offset from geometry |
| `dimension_offset_step` | 18.0 | 23.0 | Spacing between stacked dimensions |
| `label_font_size` | 13.0 | 24.0 | Dimension label text size |

### Section View Layout (core/src/visualization/geometry.rs)

| Element | Value | Previous | Notes |
|---------|-------|----------|-------|
| `legend_gap` | 6.0 | 10.0 | **Already reduced** from 10px |
| `legend_height` | 25.0 | - | Fixed legend size |
| `dim_line_offset` (left) | 18.0 | 30.0 | **Already reduced** from 30px |
| `base_offset` (right) | 18.0 | 35.0 | **Already reduced** from 35px |

### Combined View Gap (core/src/visualization/svg.rs)

| Element | Value | Notes |
|---------|-------|-------|
| `gap_between_views` | 40.0 | Gap between plan and section views |
| `title_height` | 95.0 | If title block present |

## Space Recovery Opportunities

### 1. Gap Between Views (PRIMARY TARGET)
**Location:** `svg.rs` line 405
**Current:** 40.0 px
**Proposed:** 25.0 px
**Savings:** 15 px

**Rationale:**
- Originally increased to 40px to prevent mat cut label overlap (see comment line 404)
- Mat cut label has `mat_cut_offset = style.extension_line_overshoot + style.label_font_size / 2.0 + style.dimension_offset_base`
- With tighter layout, we can reduce this gap
- **Risk:** May need to verify no callout collisions with tape measure format

**Code change:**
```rust
// Before:
let gap_between_views = 40.0; // Increased to prevent overlaps

// After:
let gap_between_views = 25.0; // Reduced for cut pieces space
```

### 2. Section Legend Gap (SECONDARY TARGET)
**Location:** `geometry.rs` line 391
**Current:** 6.0 px
**Proposed:** 3.0 px
**Savings:** 3 px

**Rationale:**
- Already reduced from 10px to 6px
- Can go tighter (3px minimum for visual separation)
- Legend is small text, doesn't need much breathing room

**Code change:**
```rust
// Before:
let legend_gap = 6.0;  // Reduced from 10.0 to recover space for title

// After:
let legend_gap = 3.0;  // Further reduced for cut pieces space
```

### 3. Plan View Top Margin (TERTIARY TARGET)
**Location:** Dynamic viewBox calculation in `build_plan_svg`
**Current:** `style.margin` (8px web, 4px PDF) at top of viewBox
**Proposed:** Reduce effective top padding by 3-5px
**Savings:** 3-5 px

**Rationale:**
- Plan view uses dynamic viewBox with padding calculated from positioned callouts
- Line 634: `let padding = style.margin;`
- Could apply asymmetric padding (less on top, more on sides/bottom)
- **Risk:** May clip top dimension callouts if not careful

**Code change (more complex):**
```rust
// Current:
let padding = style.margin;
(min_x - padding, min_y - padding, max_x - min_x + 2.0 * padding, max_y - min_y + 2.0 * padding)

// Proposed:
let padding_h = style.margin;
let padding_v_top = style.margin * 0.5; // 50% reduction at top
let padding_v_bottom = style.margin;
(min_x - padding_h, min_y - padding_v_top, max_x - min_x + 2.0 * padding_h, max_y - min_y + padding_v_top + padding_v_bottom)
```

## Total Space Available

### Conservative Estimate (Targets 1 & 2 only)
- Gap between views: **15 px**
- Legend gap: **3 px**
- **Total: 18 px**

### Aggressive Estimate (All three targets)
- Gap between views: **15 px**
- Legend gap: **3 px**
- Plan top margin: **4 px**
- **Total: 22 px**

## Cut Pieces Space Requirements

### Estimated Height Needed (8×12 artwork example)

**Given:**
- Frame width: 0.75"
- Two pieces (horizontal + vertical) stacked
- Scale from plan view: ~40 pixels/inch (typical)

**Calculation:**
```
Horizontal piece height: 0.75" × 40 px/inch = 30 px
Gap between pieces: 10 px
Vertical piece height: 0.75" × 40 px/inch = 30 px
Top margin (callouts): 15 px
Bottom margin (callouts): 15 px

Total: 30 + 10 + 30 + 15 + 15 = 100 px
```

### With Smaller Frame (0.5" width)
```
Horizontal: 0.5" × 40 = 20 px
Gap: 10 px
Vertical: 0.5" × 40 = 20 px
Margins: 30 px

Total: 80 px
```

## Problem: Space Recovery vs. Requirement

**Space needed:** ~80-100 px
**Space recoverable:** ~18-22 px

**Gap:** We're **58-82 px SHORT** even with all compressions!

## Alternative Layout Strategies

### Option A: Reduce Section View Height Allocation
**Current:** Plan 58%, Section 42%
**Proposed:** Plan 55%, Section 35%, Cut Pieces 10%

This gives cut pieces a dedicated slice of the canvas height budget.

**Trade-off:** Section view will be slightly smaller

### Option B: Horizontal Placement (Side-by-side)
Place cut pieces to the RIGHT of section view instead of below.

**Pros:**
- Section view is narrower than plan view (typically ~40% of width)
- Unused horizontal space to the right of section view
- No vertical compression needed

**Cons:**
- Cut pieces would need independent scale (can't match plan view scale)
- More complex layout logic

### Option C: Increase Overall Canvas Height
**Current typical:** 600-800 px
**Proposed:** Add 100 px dedicated to cut pieces
**New canvas height:** 700-900 px

**Pros:**
- No compression of existing views
- Cut pieces get full space needed

**Cons:**
- Increases overall diagram size
- May require scrolling on smaller screens
- Affects PDF export page layout

### Option D: Toggle/Collapsible Cut Pieces View
Show cut pieces on demand (user toggle or separate tab).

**Pros:**
- No layout constraints
- Full space available when shown

**Cons:**
- More complex UI
- Not always visible

## Tape Measure Format Impact

When `use_tape_segments = true`, labels like:
- `"23/32"` → `"3/4 - 1/32"` (more characters)
- `"1 7/8"` → `"2 - 1/8"` (same length)

**Impact on layout:**
- Longer labels increase horizontal extent of dimension callouts
- Label height calculation: `style.label_font_size * 1.2` (line 597 svg.rs)
- **No vertical impact** (label height stays same)
- Horizontal bounds may expand, but plan view uses dynamic viewBox so it auto-fits
- **Conclusion:** Tape measure format does NOT significantly impact vertical space budget

## Recommended Approach

### Phase 1: Implement Option A (Height Reallocation)
1. Adjust combined view split:
   - Plan: 55% (down from 58%)
   - Section: 35% (down from 42%)
   - Cut pieces: 10% (new)
2. Apply compression targets #1 and #2:
   - `gap_between_views` 40 → 25 px
   - `legend_gap` 6 → 3 px
3. Integrate cut pieces into combined view layout

### Phase 2: Fine-tune Based on Real Data
1. Test with various frame sizes (small, medium, large, wide frames)
2. Verify no callout collisions
3. Adjust percentages if needed

### Phase 3: Consider Option C (Canvas Height) if Needed
If section view becomes too cramped at 35%, increase overall canvas height by 50-100px.

## Implementation Notes

### Files to Modify
1. **`svg.rs`**
   - Line 405: Reduce `gap_between_views`
   - Line 412-416: Adjust height allocation percentages
   - Add cut pieces rendering in combined view

2. **`geometry.rs`**
   - Line 391: Reduce `legend_gap`
   - Add `CutPieceGeometry` integration

3. **`types.rs`**
   - Update `ViewOption` enum to handle cut pieces in combined view

### Testing Checklist
- [ ] Small frame (4×6) with narrow molding (0.5")
- [ ] Medium frame (8×12) with standard molding (0.75")
- [ ] Large frame (16×20) with wide molding (1.5")
- [ ] Square frame (12×12) - verify x4 quantity label
- [ ] Tape measure format enabled (verify no vertical overflow)
- [ ] PDF export (verify all three views fit on page)
- [ ] Web responsive (800×600, 1024×768, 1920×1080)
