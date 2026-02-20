# Cut Pieces View - Implementation Guide

**Status:** Planning document - DO NOT implement yet
**Created:** 2026-02-11
**Purpose:** Detailed roadmap for integrating cut pieces visualization into combined view

---

## Overview

Add a schematic visualization of cut frame pieces (trapezoids with 45° miters) below the section view, showing actual cut geometry with dimension callouts.

## Space Budget Analysis

### Current Combined View Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Title Block (if present)                        95 px       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Plan View                                   58% of avail    │
│                                                              │
├─────────────────────────────────────────────────────────────┤
│ Gap between views                               40 px       │  ← COMPRESSION TARGET #1
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Section View (frame + materials)                            │
│                                                              │
│ Below-frame extension (rabbet label)        46.6-70.8 px    │  ← COMPRESSION TARGET #2
│ Legend gap                                    5.2-9.6 px    │  ← COMPRESSION TARGET #3
│ Legend                                          ~25 px      │
├─────────────────────────────────────────────────────────────┤
│ [NEW] Cut Pieces View                       ~80-100 px     │  ← TO BE ADDED
└─────────────────────────────────────────────────────────────┘
```

### Space Recovery Opportunities

| Target | Current (web) | Proposed | Savings |
|--------|---------------|----------|---------|
| Gap between views | 40 px | 25 px | **15 px** |
| Rabbet label extension | 46.6 px | 30 px | **16.6 px** |
| Legend gap | 5.2 px | 3 px | **2.2 px** |
| **Total recoverable** | - | - | **33.8 px** |

### Height Reallocation Strategy

**Current:**
- Available height = `canvas_height - gap_between_views - title_height`
- Plan: 58% of available
- Section: 42% of available

**Proposed:**
- Available height = `canvas_height - gap_between_views - title_height` (same)
- Plan: **55%** (down from 58%, -3%)
- Section: **33%** (down from 42%, -9%)
- Cut Pieces: **12%** (new)

**Net space for cut pieces:**
- Space from reallocation: ~12% of available height
- Space from compression: ~34 px
- Typical available height: 600-800 px (web), so 12% ≈ 72-96 px
- **Total: ~72-96 px + 34 px = 106-130 px** ✓ Sufficient!

---

## Implementation Steps

### Phase 1: Core Geometry Module

**File:** `core/src/visualization/geometry.rs`

#### 1.1 Add TrapezoidGeometry struct

```rust
/// Geometry for a single cut frame piece (trapezoid with 45° miters)
#[derive(Debug, Clone)]
pub struct TrapezoidGeometry {
    /// Center position of the piece
    pub center: Point,

    /// Outside edge length (top/long edge)
    pub outside_length: f64,

    /// Inside edge length (bottom/short edge)
    pub inside_length: f64,

    /// Width (height of trapezoid = frame material width)
    pub width: f64,

    /// Four corner points for SVG polygon rendering
    /// Ordered: top-left, top-right, bottom-right, bottom-left
    pub corners: [Point; 4],

    /// Quantity label (2 for rectangular, 4 for square)
    pub quantity: usize,
}

impl TrapezoidGeometry {
    /// Create trapezoid with 45° miters on each end.
    ///
    /// Geometry:
    /// - Left miter slopes \ (down and to the right)
    /// - Right miter slopes / (down and to the left)
    /// - Miter offset is implicit: (outside - inside) / 2 = frame_width
    pub fn new(center: Point, outside: f64, inside: f64, width: f64, quantity: usize) -> Self {
        // Outside edge (top, longer)
        let top_left = Point::new(center.x - outside / 2.0, center.y - width / 2.0);
        let top_right = Point::new(center.x + outside / 2.0, center.y - width / 2.0);

        // Inside edge (bottom, shorter) - centered under outside edge
        let bottom_right = Point::new(center.x + inside / 2.0, center.y + width / 2.0);
        let bottom_left = Point::new(center.x - inside / 2.0, center.y + width / 2.0);

        Self {
            center,
            outside_length: outside,
            inside_length: inside,
            width,
            corners: [top_left, top_right, bottom_right, bottom_left],
            quantity,
        }
    }

    /// Get bounding box for layout calculations
    pub fn bounds(&self) -> Rect {
        let min_x = self.corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let max_x = self.corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = self.corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_y = self.corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}
```

#### 1.2 Add CutPieceGeometry struct

```rust
/// Computed geometry for cut pieces view
#[derive(Debug, Clone)]
pub struct CutPieceGeometry {
    /// Horizontal piece (wider, for left/right sides of frame)
    pub horizontal_piece: TrapezoidGeometry,

    /// Vertical piece (taller, for top/bottom sides of frame)
    pub vertical_piece: TrapezoidGeometry,

    /// Scale factor (inherited from plan view)
    pub scale: f64,

    /// Total height consumed by cut pieces view
    pub total_height: f64,

    /// Whether this is a square frame (shows only one piece with x4)
    pub is_square: bool,
}

impl CutPieceGeometry {
    /// Create cut piece geometry using the plan view scale.
    ///
    /// Layout: Vertically stacked pieces
    /// - Horizontal piece (top)
    /// - Gap
    /// - Vertical piece (bottom) [omitted if square]
    pub fn from_design(
        design: &FrameDesign,
        scale: f64,
        available_width: f64,
        available_height: f64,
    ) -> Self {
        let cut_list = design.get_cut_list();
        let horiz = &cut_list.horizontal_pieces[0];
        let vert = &cut_list.vertical_pieces[0];

        // Check if square frame (within 1/32" tolerance)
        let is_square = (horiz.outside_length - vert.outside_length).abs() < 0.03125;

        // Layout constants
        let gap_between_pieces = 10.0;  // px
        let margin_top = 15.0;  // px for callouts above
        let margin_bottom = 15.0;  // px for callouts below

        // Scale dimensions
        let horiz_scaled = (
            horiz.outside_length * scale,  // outside
            horiz.inside_length * scale,   // inside
            horiz.width * scale,           // width
        );

        let vert_scaled = (
            vert.outside_length * scale,
            vert.inside_length * scale,
            vert.width * scale,
        );

        // Position pieces vertically stacked, centered horizontally
        let center_x = available_width / 2.0;
        let horiz_y = margin_top + horiz_scaled.2 / 2.0;

        let horizontal_piece = TrapezoidGeometry::new(
            Point::new(center_x, horiz_y),
            horiz_scaled.0,  // outside
            horiz_scaled.1,  // inside
            horiz_scaled.2,  // width
            if is_square { 4 } else { 2 },  // quantity
        );

        let (vertical_piece, total_height) = if is_square {
            // Square frame: no vertical piece shown
            let dummy = TrapezoidGeometry::new(
                Point::new(0.0, 0.0), 0.0, 0.0, 0.0, 0
            );
            let height = margin_top + horiz_scaled.2 + margin_bottom;
            (dummy, height)
        } else {
            // Rectangular frame: show both pieces
            let vert_y = horiz_y + horiz_scaled.2 / 2.0 + gap_between_pieces + vert_scaled.2 / 2.0;
            let piece = TrapezoidGeometry::new(
                Point::new(center_x, vert_y),
                vert_scaled.0,
                vert_scaled.1,
                vert_scaled.2,
                2,
            );
            let height = margin_top + horiz_scaled.2 + gap_between_pieces + vert_scaled.2 + margin_bottom;
            (piece, height)
        };

        Self {
            horizontal_piece,
            vertical_piece,
            scale,
            total_height,
            is_square,
        }
    }
}
```

### Phase 2: Callout Generation

**File:** `core/src/visualization/callouts.rs`

#### 2.1 Add new DimensionType variants

In `types.rs::DimensionType`:

```rust
// Cut piece dimensions
CutPieceOutsideLength,
CutPieceInsideLength,
CutPieceWidth,
```

Update `priority()` and `preferred_side()` methods:

```rust
DimensionType::CutPieceOutsideLength => 1,  // Must show
DimensionType::CutPieceInsideLength => 1,   // Must show
DimensionType::CutPieceWidth => 2,          // Nice to have

DimensionType::CutPieceOutsideLength => Side::Top,
DimensionType::CutPieceInsideLength => Side::Bottom,
DimensionType::CutPieceWidth => Side::Left,
```

#### 2.2 Implement callout generation function

```rust
/// Generate dimension callouts for cut pieces view
pub fn generate_cut_piece_callouts(
    design: &FrameDesign,
    geometry: &CutPieceGeometry,
    unit_mm: bool,
    use_tape_segments: bool,
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |value: f64| format_dimension(value, unit, use_tape_segments);

    let cut_list = design.get_cut_list();
    let horiz = &cut_list.horizontal_pieces[0];

    // Horizontal piece callouts
    let h = &geometry.horizontal_piece.corners;

    // Outside length (top edge)
    callouts.push(DimensionCallout::new(
        horiz.outside_length,
        format!("Outside: {}", fmt(horiz.outside_length)),
        DimensionType::CutPieceOutsideLength,
        h[0],  // top-left
        h[1],  // top-right
    ));

    // Inside length (bottom edge)
    callouts.push(DimensionCallout::new(
        horiz.inside_length,
        format!("Inside: {}", fmt(horiz.inside_length)),
        DimensionType::CutPieceInsideLength,
        h[3],  // bottom-left
        h[2],  // bottom-right
    ));

    // Width (left edge)
    callouts.push(DimensionCallout::new(
        horiz.width,
        format!("Width: {}", fmt(horiz.width)),
        DimensionType::CutPieceWidth,
        h[0],  // top-left
        h[3],  // bottom-left
    ));

    // Vertical piece callouts (if not square)
    if !geometry.is_square {
        let vert = &cut_list.vertical_pieces[0];
        let v = &geometry.vertical_piece.corners;

        callouts.push(DimensionCallout::new(
            vert.outside_length,
            format!("Outside: {}", fmt(vert.outside_length)),
            DimensionType::CutPieceOutsideLength,
            v[0], v[1],
        ));

        callouts.push(DimensionCallout::new(
            vert.inside_length,
            format!("Inside: {}", fmt(vert.inside_length)),
            DimensionType::CutPieceInsideLength,
            v[3], v[2],
        ));

        callouts.push(DimensionCallout::new(
            vert.width,
            format!("Width: {}", fmt(vert.width)),
            DimensionType::CutPieceWidth,
            v[0], v[3],
        ));
    }

    callouts
}
```

### Phase 3: SVG Rendering

**File:** `core/src/visualization/svg.rs`

#### 3.0 Callout Rendering Architecture (Critical Details)

When rendering dimension callouts for cut pieces, follow the existing plan view architecture:

**Arrow Marker Geometry:**
- Arrow markers use `markerUnits="strokeWidth"` (SVG standard)
- `MARKER_WIDTH = 8.0` (from `arrow_geometry` module)
- Actual arrow extension = `MARKER_WIDTH × stroke_width`
- For dimension lines with `stroke-width="1"`, arrows extend **8px**

**Extension Line Positioning:**
```rust
// Extension lines extend from geometry to dimension line with overshoot
const EXTENSION_LINE_OVERSHOOT: f64 = 4.0;  // Subtle overshoot (not 8.0)

// Example for top outside dimension:
// - Trapezoid top edge at y=35
// - Dimension line at y=20
// - Extension line: from y=35 to y=16 (overshoot 4px beyond y=20)

let extension_start = geometry_edge;  // e.g., trapezoid corner
let extension_end = dimension_line_y - EXTENSION_LINE_OVERSHOOT;  // e.g., 20 - 4 = 16
```

**Dimension Line Endpoint Adjustment:**
```rust
// Dimension line endpoints must be adjusted inward so arrow TIPS align with geometry
const ARROW_TIP_EXTENSION: f64 = 8.0;  // MARKER_WIDTH × stroke_width

// Example for horizontal dimension from x=55 to x=745:
// - Extension lines at x=55 and x=745 (from trapezoid corners)
// - Dimension line from x=63 to x=737 (inset by 8px on each end)
// - Arrow-start tip extends left from 63 by 8px → reaches x=55 ✓
// - Arrow-end tip extends right from 737 by 8px → reaches x=745 ✓

let dim_line_start = extent_start_x + ARROW_TIP_EXTENSION;  // 55 + 8 = 63
let dim_line_end = extent_end_x - ARROW_TIP_EXTENSION;      // 745 - 8 = 737
```

**Complete Callout Pattern (Horizontal Dimension Example):**
```rust
// 1. Extension lines (vertical, from trapezoid to beyond dimension line)
svg.push_str(&format!(
    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.8" opacity="0.7"/>"#,
    extent_start_x, trapezoid_top_y,
    extent_start_x, dimension_line_y - 4.0,  // 4px overshoot
    dimension_color
));
svg.push_str(&format!(
    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="0.8" opacity="0.7"/>"#,
    extent_end_x, trapezoid_top_y,
    extent_end_x, dimension_line_y - 4.0,
    dimension_color
));

// 2. Dimension line (adjusted endpoints so arrow tips align with extension lines)
svg.push_str(&format!(
    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="1" marker-start="url(#arrow-start)" marker-end="url(#arrow-end)"/>"#,
    extent_start_x + 8.0, dimension_line_y,  // Inset by 8px
    extent_end_x - 8.0, dimension_line_y,    // Inset by 8px
    dimension_color
));

// 3. Label text (centered on dimension line)
let label_x = (extent_start_x + extent_end_x) / 2.0;
let label_y = dimension_line_y - 4.0;  // Above dimension line
svg.push_str(&format!(
    r#"<text x="{}" y="{}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle">{}</text>"#,
    label_x, label_y, dimension_color, font_family, font_size, label_text
));
```

**Vertical Dimensions (Width):**
Same principle, but extension lines are horizontal and dimension line is vertical:
```rust
// Extension lines extend LEFT from trapezoid edges
let extension_end_x = dimension_line_x - 4.0;  // 4px overshoot

// Dimension line endpoints adjusted so arrow tips align
let dim_line_start_y = extent_start_y + 8.0;  // Top, inset by 8px
let dim_line_end_y = extent_end_y - 8.0;      // Bottom, inset by 8px
```

#### 3.1 Add trapezoid rendering function

```rust
/// Render a trapezoid (cut frame piece) as SVG polygon
fn render_cut_piece_trapezoid(
    geometry: &TrapezoidGeometry,
    style: &DiagramStyle,
) -> String {
    let c = &geometry.corners;
    let points = format!(
        "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
        c[0].x, c[0].y,  // top-left
        c[1].x, c[1].y,  // top-right
        c[2].x, c[2].y,  // bottom-right
        c[3].x, c[3].y,  // bottom-left
    );

    // Use wood color/pattern from material_patterns
    let fill = match &style.material_patterns.frame {
        FillPattern::Solid(color) => color.clone(),
        _ => "#8B6914".to_string(),  // Fallback wood brown
    };

    format!(
        r#"    <polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" opacity="0.7"/>"#,
        points,
        fill,
        style.line_color,
        style.frame_stroke_width,
    )
}
```

#### 3.2 Add quantity badge rendering

```rust
/// Render quantity label (e.g., "x2" or "x4")
fn render_quantity_badge(
    position: Point,
    quantity: usize,
    style: &DiagramStyle,
) -> String {
    format!(
        r#"    <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{}" fill="{}" font-weight="bold">x{}</text>"#,
        position.x,
        position.y,
        style.font_family,
        style.label_font_size * 1.1,  // Slightly larger
        style.dimension_color,
        quantity,
    )
}
```

#### 3.3 Modify combined view layout

In `generate_combined_view()` function:

**Step 1: Apply compression targets**

```rust
// BEFORE:
let gap_between_views = 40.0; // Increased to prevent overlaps

// AFTER:
let gap_between_views = 25.0; // Reduced for cut pieces space
```

**Step 2: Adjust height allocation**

```rust
// BEFORE:
let plan_height = available_height * 0.58;
let section_height = available_height * 0.42;

// AFTER:
let plan_height = available_height * 0.55;
let section_height = available_height * 0.33;
let cut_pieces_height = available_height * 0.12;
```

**Step 3: Generate cut pieces view**

Add after section view generation:

```rust
// Generate cut pieces view
let cut_pieces_options = DiagramOptions {
    view: ViewOption::CutPiecesOnly,  // New enum variant
    canvas_width: options.canvas_width,
    canvas_height: cut_pieces_height,
    ..options.clone()
};

let plan_geometry = PlanViewGeometry::from_design(design, options.canvas_width, plan_height, &plan_style);
let cut_pieces_geometry = CutPieceGeometry::from_design(
    design,
    plan_geometry.scale,  // Use same scale as plan view!
    options.canvas_width,
    cut_pieces_height,
);

let cut_pieces_callouts = generate_cut_piece_callouts(
    design,
    &cut_pieces_geometry,
    options.unit_mm,
    options.use_tape_segments,
);

let cut_pieces_svg = build_cut_pieces_svg(
    design,
    &cut_pieces_geometry,
    &cut_pieces_callouts,
    &cut_pieces_options,
    &plan_style,  // Use plan style, not section style
);
```

**Step 4: Add to combined SVG**

```rust
// Position cut pieces below section view
let cut_pieces_y = title_height + plan_height + gap_between_views + section_height + 10.0;

svg.push_str(&format!(
    r#"  <g id="cut-pieces-view" transform="translate(0, {})">{}</g>"#,
    cut_pieces_y,
    extract_svg_content(&cut_pieces_svg),
));
```

#### 3.4 Create build_cut_pieces_svg function

```rust
/// Build SVG for cut pieces view
fn build_cut_pieces_svg(
    design: &FrameDesign,
    geometry: &CutPieceGeometry,
    callouts: &[DimensionCallout],
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();

    // SVG header
    svg.push_str(&format!(
        r#"<svg viewBox="0 0 {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">"#,
        options.canvas_width, geometry.total_height
    ));
    svg.push('\n');

    svg.push_str("  <g id=\"cut-pieces-geometry\">\n");

    // Render horizontal piece
    svg.push_str(&render_cut_piece_trapezoid(&geometry.horizontal_piece, style));
    svg.push('\n');

    // Quantity badge for horizontal piece
    let h_badge_pos = Point::new(
        geometry.horizontal_piece.center.x + geometry.horizontal_piece.outside_length / 2.0 + 15.0,
        geometry.horizontal_piece.center.y,
    );
    svg.push_str(&render_quantity_badge(h_badge_pos, geometry.horizontal_piece.quantity, style));
    svg.push('\n');

    // Render vertical piece (if not square)
    if !geometry.is_square {
        svg.push_str(&render_cut_piece_trapezoid(&geometry.vertical_piece, style));
        svg.push('\n');

        let v_badge_pos = Point::new(
            geometry.vertical_piece.center.x + geometry.vertical_piece.outside_length / 2.0 + 15.0,
            geometry.vertical_piece.center.y,
        );
        svg.push_str(&render_quantity_badge(v_badge_pos, geometry.vertical_piece.quantity, style));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");

    // TODO: Render dimension callouts (reuse existing callout rendering logic)

    svg.push_str("</svg>");
    svg
}
```

### Phase 4: Compression Optimizations

**File:** `core/src/visualization/geometry.rs`

#### 4.1 Reduce rabbet label extension

Line 394:

```rust
// BEFORE:
let rabbet_label_height = 18.0 + font_size * 2.2; // Leader line + two lines of text

// AFTER:
let rabbet_label_height = 10.0 + font_size * 1.8; // Tighter spacing
```

This saves: `(18 - 10) + (2.2 - 1.8) × 13 = 8 + 5.2 = 13.2 px` (web)

#### 4.2 Reduce legend gap (if needed)

In `svg.rs` lines 1876 and 2345:

```rust
// BEFORE:
let legend_gap = style.label_font_size * 0.4;

// AFTER:
let legend_gap = style.label_font_size * 0.25;
```

This saves: `13 × (0.4 - 0.25) = 1.95 px` (web)

### Phase 5: Type System Updates

**File:** `core/src/visualization/types.rs`

Update `ViewOption` enum:

```rust
pub enum ViewOption {
    PlanOnly,
    SectionOnly,
    CutPiecesOnly,  // NEW
    Both,           // Plan + Section (for PDF)
    All,            // NEW: Plan + Section + Cut Pieces
}
```

Update module exports in `mod.rs`:

```rust
pub use geometry::{PlanViewGeometry, SectionViewGeometry, CutPieceGeometry, TrapezoidGeometry};
pub use callouts::{generate_plan_callouts, generate_section_callouts, generate_cut_piece_callouts};
```

---

## Reference Examples

**Standalone HTML Visualizations:**
- `docs/cut-pieces-example.html` - Rectangular frame (8×12) showing both pieces
- `docs/cut-pieces-example-square.html` - Square frame (12×12) showing single piece with x4

These files demonstrate:
- Correct trapezoid geometry with 45° miters
- Proper callout rendering with extension lines and arrow alignment
- Color scheme matching project palette
- Dimension line endpoint adjustment for arrow tips
- Extension line overshoot (4px beyond dimension line)

Open in browser to verify visual accuracy before implementing in Rust/SVG generation code.

---

## Testing Strategy

### Unit Tests

**File:** `core/src/visualization/geometry.rs` (add to tests module)

```rust
#[test]
fn test_trapezoid_geometry_miters() {
    // Test 45° miter geometry
    let trap = TrapezoidGeometry::new(
        Point::new(100.0, 100.0),
        17.25,  // outside
        15.75,  // inside
        0.75,   // width
        2,
    );

    // Verify miter offset: (outside - inside) / 2 = (17.25 - 15.75) / 2 = 0.75
    let left_offset = trap.corners[0].x - trap.corners[3].x;
    let right_offset = trap.corners[1].x - trap.corners[2].x;
    assert!((left_offset - 0.75).abs() < 0.001);
    assert!((right_offset - 0.75).abs() < 0.001);
}

#[test]
fn test_cut_piece_geometry_square_frame() {
    let mut design = FrameDesign::new(12.0, 12.0);  // Square
    design.frame_material_width = 0.75;

    let geometry = CutPieceGeometry::from_design(&design, 40.0, 800.0, 100.0);

    assert!(geometry.is_square);
    assert_eq!(geometry.horizontal_piece.quantity, 4);
}

#[test]
fn test_cut_piece_geometry_rectangular_frame() {
    let mut design = FrameDesign::new(8.0, 12.0);  // Rectangular
    design.frame_material_width = 0.75;

    let geometry = CutPieceGeometry::from_design(&design, 40.0, 800.0, 100.0);

    assert!(!geometry.is_square);
    assert_eq!(geometry.horizontal_piece.quantity, 2);
    assert_eq!(geometry.vertical_piece.quantity, 2);
}
```

### Visual Tests

Create test fixtures for:
- Small frame (4×6, 0.5" molding)
- Medium frame (8×12, 0.75" molding)
- Large frame (16×20, 1.5" molding)
- Square frame (12×12, 0.75" molding)
- With tape measure format enabled
- PDF export

### Regression Tests

Verify existing views not affected:
- Plan view dimensions unchanged
- Section view dimensions unchanged (except compressed margins)
- Callout positioning correct
- No overlaps or collisions

---

## Rollout Plan

### Step 1: Core geometry (no UI changes)
- Implement `TrapezoidGeometry` and `CutPieceGeometry`
- Add unit tests
- No user-facing changes yet

### Step 2: Callouts and rendering (isolated)
- Implement callout generation
- Implement SVG rendering functions
- Test with standalone `CutPiecesOnly` view option
- Still not integrated into combined view

### Step 3: Integration (feature flag)
- Add to combined view behind a feature flag or option
- Test with various frame sizes
- Gather feedback

### Step 4: Compression optimizations
- Apply margin reductions
- Fine-tune height allocations
- Verify no regressions

### Step 5: Polish and ship
- Final visual tweaks
- Update documentation
- Enable by default

---

## Risk Mitigation

### Risk 1: Callout Collisions
**Mitigation:** Reuse existing callout layout system with collision detection. Start with higher priority for cut piece callouts to ensure they show.

### Risk 2: Section View Too Small
**Mitigation:** Test with various frame sizes. If section view becomes cramped, increase canvas height by 50-100px instead of further compressing section.

### Risk 3: Scale Mismatch
**Mitigation:** Always use `plan_geometry.scale` for cut pieces, never auto-scale independently. This ensures visual consistency.

### Risk 4: Tape Measure Format Overflow
**Mitigation:** Test all dimension types with tape measure format enabled. Cut pieces use same callout system as plan view, so should handle it correctly.

---

## File Modification Summary

| File | Changes | Lines Added (est.) |
|------|---------|-------------------|
| `types.rs` | Add DimensionType variants, update ViewOption | ~15 |
| `geometry.rs` | Add TrapezoidGeometry, CutPieceGeometry, tests | ~200 |
| `callouts.rs` | Add generate_cut_piece_callouts | ~80 |
| `svg.rs` | Add rendering functions, modify combined view | ~150 |
| `mod.rs` | Update exports | ~3 |
| **Total** | - | **~448 lines** |

---

## Success Criteria

- [ ] Cut pieces render with correct 45° miter geometry
- [ ] Dimensions match output of `get_cut_list()`
- [ ] Square frames show single piece with "x4"
- [ ] Rectangular frames show two pieces with "x2" each
- [ ] Same scale as plan view (dimensional consistency)
- [ ] No callout overlaps or collisions
- [ ] Works with tape measure format
- [ ] Fits in combined view without excessive scrolling
- [ ] Section view still readable at 33% height
- [ ] All existing tests pass
- [ ] Visual regression tests pass for plan and section views

---

## Open Questions for Implementation

1. **Callout rendering:** Should we render dimension lines with arrows, or just text labels to save space?
2. **Piece labels:** Add text labels "Horizontal Piece" / "Vertical Piece" or rely on dimensions to distinguish?
3. **Miter angle callout:** Explicitly show "45°" angle annotation, or assume it's understood?
4. **Material grain:** Show wood grain direction on trapezoids (for visual interest), or keep simple solid fill?
5. **Responsive behavior:** What happens if canvas height < 600px? Hide cut pieces? Scroll?

---

## Corner Detail Inset View

**Purpose:** For large frames where nested rectangle lines merge together at normal zoom, provide a zoomed-in corner detail showing frame edge construction clearly.

**Mockup:** `docs/corner-detail-integrated-mockup.html`

### Scope

The corner detail shows **only the frame edge region** (~1" from corner):
- Frame outer edge
- Content area boundary (matboard or artwork edge in the rabbet)
- Frame inner edge (visible opening)

Mat opening, artwork boundary, and overlap dimensions are **excluded** — they stay on the main plan view where they're already readable at any frame size.

### Layout

- **Position:** Bottom-left of plan view, overlaid as an inset with white background + shadow
- **Zoom level:** Fixed at ~120 px/inch (vs plan view's dynamic scale)
- **Mat width callout:** Relocated from bottom-left to bottom-right on main plan view

### Callouts

1. **Frame width** — horizontal, below corner (Blue Slate #577590)
2. **Rabbet depth** — vertical, left side with extension lines (Seaweed #46af8f)
3. **Content edge label** — dogleg connector to right, color/text varies (see below)

### Conditional Color Logic (NEW — does not exist in current codebase)

The overlap zone and content area boundary color/label must vary based on what sits in the rabbet:

```rust
// In corner detail rendering:
let (overlap_color, edge_label) = if design.has_mat() {
    (&style.mat_cut_color, "matboard edge")    // Carrot Orange #f8961e
} else {
    (&style.artwork_color, "artwork edge")     // Willow Green #90be6d
};
```

**Rationale:** The rabbet overlap zone represents whichever content layer sits directly under the frame lip. With matboard, the mat extends into the rabbet. Without matboard, the artwork itself sits in the rabbet.

**Current gap:** `content_boundary_color` in `style.rs` (line 79) is a single value (#8B7355) with no mat/no-mat conditional. The plan view rendering (svg.rs lines 695–740) also draws the content boundary identically regardless of mat presence. Both need updating.

**Required changes:**
1. `style.rs` — Add `artwork_color` field (Willow Green #90be6d) alongside existing `content_boundary_color`
2. `svg.rs` plan view — Use `mat_cut_color` when `has_mat()`, `artwork_color` when not, for:
   - Rabbet overlap zone fill (line 720)
   - Content boundary dashed stroke (line 738)
3. Corner detail renderer (new code) — Same conditional for overlap zone fill, content edge line, and dogleg label

### Zones

- **Rabbet overlap** (between content area and frame inner): filled with the conditional color above
- Frame material area (between outer and inner, outside the overlap): **no fill** (white, matching plan view)

### No-Mat Variant

When matboard is omitted:
- Content area line → Willow Green dashed, labeled "artwork edge"
- Overlap zone → Willow Green fill
- Rabbet callout → unchanged (still teal, still measures frame rabbet depth)

### Red Indicator Box

A small dashed red (#f94144) rectangle on the plan view marks which corner is detailed. Always bottom-left.

### Files to Modify

1. **`style.rs`** — Add `artwork_color` field
2. **`svg.rs`** — New corner detail rendering function; conditional color logic for content boundary
3. **`callouts.rs`** — Relocate mat width callout from bottom-left to bottom-right side

---

**End of Implementation Guide**
