# Cut Pieces View - Feature Specification

## Overview

Add a schematic visualization of the unique frame pieces showing their actual cut geometry (trapezoids with 45° miters) and labeled dimensions. This provides a visual "cut list" that complements the existing plan and section views.

## Visual Design

### Geometry

Each frame piece is rendered as a **trapezoid** representing the actual cut profile:
- **Long edge** = outside length (along outer face of frame)
- **Short edge** = inside length (along inner face of frame)
- **45° miters** on both ends
- **Height** = frame material width

For a non-square frame (e.g., 8×12 artwork):
- **Horizontal piece**: wider trapezoid (outside 17.25", inside 15.75", width 0.75")
- **Vertical piece**: narrower trapezoid (outside 13.25", inside 11.75", width 0.75")

### Layout (Selected)

**Vertically stacked** (both pieces in horizontal orientation)
```
╲────────────────────────╱  x2
 ╲   Horizontal Piece   ╱
  ╲────────────────────╱

╲──────────────────╱  x2 (or x4 for square frames)
 ╲ Vertical Piece ╱
  ╲──────────────╱
```

**Rationale:** When scaled to match the plan view, vertical stacking requires only `max(horiz_length, vert_length)` width (fits within plan view width), whereas side-by-side would require `horiz_length + vert_length` (could exceed canvas width).

Note: Trapezoids show actual miter geometry with 45° cuts on each end.

## Dimension Callouts

Each piece shows:

1. **Outside length** - top edge (longest dimension)
2. **Inside length** - bottom edge (shortest dimension)
3. **Width** - height of the piece (frame material width)
4. **Quantity label** - "x2" badge positioned near the piece

### Callout Placement

Reuse existing `DimensionCallout` and `PositionedCallout` system:
- Outside length → top side
- Inside length → bottom side (or interior if space allows)
- Width → left or right side
- Quantity → floating badge (top-right corner of piece)

### New DimensionTypes

Add to `types.rs::DimensionType`:
```rust
// Cut piece dimensions
CutPieceOutsideLength,
CutPieceInsideLength,
CutPieceWidth,
```

## Architecture Changes

### 1. Types (`types.rs`)

```rust
pub enum ViewType {
    Plan,
    Section,
    CutPieces,  // NEW
}

pub enum ViewOption {
    PlanOnly,
    SectionOnly,
    CutPiecesOnly,  // NEW
    Both,  // Plan + Section (existing PDF behavior)
    All,   // NEW: Plan + Section + CutPieces
}
```

### 2. New Geometry Module (`geometry/cut_pieces.rs` or add to `geometry.rs`)

```rust
pub struct CutPieceGeometry {
    /// Horizontal piece trapezoid (outside_length > inside_length)
    pub horizontal_piece: TrapezoidGeometry,

    /// Vertical piece trapezoid
    pub vertical_piece: TrapezoidGeometry,

    /// Canvas/viewport bounds
    pub canvas_width: f64,
    pub canvas_height: f64,

    /// Scale factor (pixels per inch)
    pub scale: f64,
}

pub struct TrapezoidGeometry {
    /// Center position of the piece
    pub center: Point,

    /// Outside edge length (top/long edge)
    pub outside_length: f64,

    /// Inside edge length (bottom/short edge)
    pub inside_length: f64,

    /// Width (height of trapezoid)
    pub width: f64,

    /// Four corner points (for SVG polygon)
    /// Ordered: top-left, top-right, bottom-right, bottom-left
    pub corners: [Point; 4],

    /// Quantity (2 for rectangular frames, 4 for square frames)
    pub quantity: usize,
}

impl TrapezoidGeometry {
    /// Calculate trapezoid corners from center position and dimensions.
    ///
    /// Creates a horizontal trapezoid with 45° miters on each end:
    /// - Left miter slopes \ (down and to the right)
    /// - Right miter slopes / (down and to the left)
    ///
    /// The miter offset is implicit in the relationship: outside = inside + 2×width
    /// No explicit offset calculation is needed - the geometry naturally forms
    /// correct 45° miters when connecting the corners.
    pub fn new(center: Point, outside: f64, inside: f64, width: f64) -> Self {
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
            quantity: 2,
        }
    }

    /// Get bounding box for layout calculations
    pub fn bounds(&self) -> Rect {
        let min_x = self.corners.iter().map(|p| p.x).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_x = self.corners.iter().map(|p| p.x).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let min_y = self.corners.iter().map(|p| p.y).min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        let max_y = self.corners.iter().map(|p| p.y).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

impl CutPieceGeometry {
    /// Create cut piece geometry using the same scale as the plan view.
    ///
    /// This ensures dimensional consistency - users can visually compare
    /// the cut piece lengths to the plan view dimensions.
    pub fn from_design(design: &FrameDesign, scale: f64, available_width: f64, available_height: f64) -> Self {
        let cut_list = design.get_cut_list();

        // Extract dimensions (assuming standard rectangular frame)
        let horiz = &cut_list.horizontal_pieces[0];
        let vert = &cut_list.vertical_pieces[0];

        // Layout: vertically stacked
        // [horizontal piece]
        //   (gap)
        // [vertical piece]
        let gap = 20.0;  // pixels between pieces
        let margin_x = 10.0;  // minimal horizontal margin
        let margin_y = 30.0;  // vertical margin for callouts

        // Scale dimensions
        let horiz_scaled = (horiz.outside_length * scale, horiz.inside_length * scale, horiz.width * scale);
        let vert_scaled = (vert.outside_length * scale, vert.inside_length * scale, vert.width * scale);

        // Total height needed
        let total_height = horiz_scaled.2 + gap + vert_scaled.2 + 2.0 * margin_y;

        // Center horizontally, stack vertically
        let center_x = available_width / 2.0;
        let horiz_y = margin_y + horiz_scaled.2 / 2.0;
        let vert_y = horiz_y + horiz_scaled.2 / 2.0 + gap + vert_scaled.2 / 2.0;

        let horizontal_piece = TrapezoidGeometry::new(
            Point::new(center_x, horiz_y),
            horiz_scaled.0,  // outside
            horiz_scaled.1,  // inside
            horiz_scaled.2,  // width
        );

        let vertical_piece = TrapezoidGeometry::new(
            Point::new(center_x, vert_y),
            vert_scaled.0,
            vert_scaled.1,
            vert_scaled.2,
        );

        Self {
            horizontal_piece,
            vertical_piece,
            canvas_width: available_width,
            canvas_height: total_height,
            scale,
        }
    }
}
```

### 3. Callout Generation (`callouts.rs`)

```rust
/// Generate dimension callouts for cut pieces view
pub fn generate_cut_piece_callouts(
    design: &FrameDesign,
    geometry: &CutPieceGeometry,
    unit_mm: bool,
    use_tape_segments: bool,
    style: &DiagramStyle,
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |value: f64| format_dimension(value, unit, use_tape_segments);

    let cut_list = design.get_cut_list();
    let horiz = &cut_list.horizontal_pieces[0];
    let vert = &cut_list.vertical_pieces[0];

    // Horizontal piece callouts
    let h_corners = &geometry.horizontal_piece.corners;

    // Outside length (top edge)
    callouts.push(DimensionCallout::new(
        horiz.outside_length,
        format!("Outside: {}", fmt(horiz.outside_length)),
        DimensionType::CutPieceOutsideLength,
        h_corners[0],  // top-left
        h_corners[1],  // top-right
    ));

    // Inside length (bottom edge)
    callouts.push(DimensionCallout::new(
        horiz.inside_length,
        format!("Inside: {}", fmt(horiz.inside_length)),
        DimensionType::CutPieceInsideLength,
        h_corners[3],  // bottom-left
        h_corners[2],  // bottom-right
    ));

    // Width (left edge)
    callouts.push(DimensionCallout::new(
        horiz.width,
        format!("Width: {}", fmt(horiz.width)),
        DimensionType::CutPieceWidth,
        h_corners[0],  // top-left
        h_corners[3],  // bottom-left
    ));

    // Repeat for vertical piece...

    callouts
}
```

### 4. SVG Rendering (`svg.rs`)

Add rendering function for trapezoid:

```rust
fn render_cut_piece_trapezoid(
    geometry: &TrapezoidGeometry,
    style: &DiagramStyle,
) -> String {
    let corners = &geometry.corners;
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        corners[0].x, corners[0].y,
        corners[1].x, corners[1].y,
        corners[2].x, corners[2].y,
        corners[3].x, corners[3].y,
    );

    format!(
        r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
        points,
        style.wood_color(),  // Use wood fill pattern
        style.frame_color,
        style.frame_stroke_width,
    )
}

fn render_quantity_badge(
    position: Point,
    quantity: usize,
    style: &DiagramStyle,
) -> String {
    format!(
        r#"<text x="{}" y="{}" font-family="{}" font-size="{}" fill="{}" font-weight="bold">x{}</text>"#,
        position.x,
        position.y,
        style.font_family,
        style.font_size_large,
        style.text_color,
        quantity,
    )
}
```

### 5. Integration into `generate_diagram_with_style()`

Update main rendering function to handle `ViewType::CutPieces`:

```rust
pub fn generate_diagram_with_style(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    // ... existing code ...

    match options.view {
        ViewOption::CutPiecesOnly => {
            let geometry = CutPieceGeometry::from_design(design, canvas_width, canvas_height);
            let callouts = generate_cut_piece_callouts(design, &geometry, options.unit_mm, options.use_tape_segments, style);
            let positioned = layout_cut_piece_callouts(callouts, &geometry);

            // Render SVG
            let mut svg_body = String::new();
            svg_body.push_str(&render_cut_piece_trapezoid(&geometry.horizontal_piece, style));
            svg_body.push_str(&render_cut_piece_trapezoid(&geometry.vertical_piece, style));
            svg_body.push_str(&render_quantity_badge(/* position */, 2, style));
            svg_body.push_str(&render_callouts(&positioned, style));

            // ... wrap in SVG tags ...
        },
        // ... other view options ...
    }
}
```

## UI Integration

### Layout Strategy (Selected)
- **Show alongside existing views** - cut pieces appear in the same canvas as plan/section
- **Compress existing spacing** - reduce margins between section view and its legend to make room
- **Maintain scale** - use same scale as plan view for consistency
- **Horizontal orientation** - both pieces shown horizontally for space efficiency
- **Square frame optimization** - show single piece with "x4" label instead of two identical pieces

### Web Platform
- Cut pieces rendered in same SVG as plan/section views
- Positioned to the right or below based on available space

### iOS Platform
- Integrated into existing diagram view
- Same responsive layout as web

## Future Enhancements

1. **Interactive orientation toggle** - rotate vertical piece 90° for visual comparison
2. **Assembly animation** - show pieces coming together to form frame
3. **Material grain direction** - show wood grain pattern on trapezoids
4. **Export to PDF** - include cut pieces in PDF export alongside plan/section
5. **Miter angle callout** - explicitly label the 45° angle
6. **Cut waste visualization** - show the triangular offcuts from mitering

## Implementation Phases

### Phase 1: Core Geometry (Minimal viable feature)
- [ ] Add `TrapezoidGeometry` struct
- [ ] Add `CutPieceGeometry::from_design()`
- [ ] Calculate 45° miter corners correctly
- [ ] Test with square and rectangular frames

### Phase 2: Callouts & Rendering
- [ ] Add new `DimensionType` variants
- [ ] Implement `generate_cut_piece_callouts()`
- [ ] Render trapezoid polygons with wood fill
- [ ] Render quantity badges ("x2")
- [ ] Layout callouts to avoid overlap

### Phase 3: Integration
- [ ] Update `ViewType` and `ViewOption` enums
- [ ] Wire up to `generate_diagram_with_style()`
- [ ] Add tests for cut piece geometry calculations
- [ ] Add visual regression tests

### Phase 4: UI/UX
- [ ] Web: Add view toggle
- [ ] iOS: Add view selector
- [ ] Polish: colors, spacing, fonts
- [ ] Documentation

## Resolved Design Decisions

1. **View placement:** ✅ Show alongside plan/section in same canvas
2. **Layout:** ✅ Vertically stacked (space-efficient, fits within plan view width)
3. **Orientation:** ✅ Both pieces horizontal (not rotated)
4. **Symmetry handling:** ✅ Square frames show one piece with "x4"
5. **Miter geometry:** ✅ Miters naturally form from corner positions (left slopes \, right slopes /)
6. **Scale consistency:** ✅ Use same scale as plan view (dimensional consistency)

## Remaining Open Questions

1. **Exact positioning:** Where in the canvas? Below section view? To the right of plan view?
2. **Margin compression:** How much space can we reclaim from section view layout?
3. **Label placement:** Above pieces, below, or to the side?
4. **Visual treatment:** Wood grain pattern/texture on trapezoids, or solid fill?

## Related Files

- `core/src/frame.rs` - `get_cut_list()` provides source data
- `core/src/visualization/types.rs` - Core data structures
- `core/src/visualization/geometry.rs` - Add `CutPieceGeometry`
- `core/src/visualization/callouts.rs` - Add callout generation
- `core/src/visualization/svg.rs` - Add rendering logic
- `core/src/visualization/style.rs` - Reuse existing `DiagramStyle`

## Success Criteria

✅ Trapezoids accurately represent the physical cut pieces with correct 45° miters
✅ Dimensions match the output of `get_cut_list()`
✅ Callouts are clearly labeled and don't overlap
✅ Visual style matches existing plan/section views
✅ Works for both square and rectangular frames
✅ Responsive to canvas size (proper scaling)
✅ Test coverage for geometry calculations
