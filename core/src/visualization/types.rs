// Core types for visualization module
//
// These types define the data structures used for generating
// professional frame diagrams with adaptive callout placement.

use serde::{Deserialize, Serialize};

/// A 2D point in diagram coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A rectangle defined by its bounds
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn top(&self) -> f64 {
        self.y
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Smallest rect containing both this rect and `other`
    pub fn union(&self, other: &Rect) -> Self {
        let min_x = self.left().min(other.left());
        let min_y = self.top().min(other.top());
        let max_x = self.right().max(other.right());
        let max_y = self.bottom().max(other.bottom());
        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    /// This rect moved by (dx, dy)
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Check if this rect overlaps with another
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    /// Expand rect by a margin on all sides
    pub fn expand(&self, margin: f64) -> Self {
        Self {
            x: self.x - margin,
            y: self.y - margin,
            width: self.width + 2.0 * margin,
            height: self.height + 2.0 * margin,
        }
    }

    /// Check if this rect overlaps with another, using an extra margin around both
    pub fn overlaps_with_margin(&self, other: &Rect, margin: f64) -> bool {
        self.expand(margin).overlaps(&other.expand(margin))
    }
}

/// Where the thumbnail label text is positioned relative to the thumbnail rect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailLabelPosition {
    /// Label text to the right of thumbnail (landscape, no obstruction)
    Right,
    /// Label text below thumbnail (portrait, or landscape with obstruction)
    Below,
}

/// Bounding boxes for floating annotation elements, used for
/// collision detection and viewBox calculation.
#[derive(Debug, Clone)]
pub struct AnnotationBounds {
    /// Corner detail inset box (always bottom-left when present)
    pub corner_detail_box: Option<Rect>,
    /// Thumbnail rect including label area
    pub thumbnail_box: Option<Rect>,
    /// Where the thumbnail label is placed
    pub thumbnail_label_position: ThumbnailLabelPosition,
    /// Mat cut width label bounding box
    pub mat_cut_width_label: Option<Rect>,
    /// Mat cut height label bounding box
    pub mat_cut_height_label: Option<Rect>,
    /// Pre-computed mat cut width extent (start_point, end_point).
    /// Avoids re-computing side selection in callouts.rs and ensures consistency
    /// with the label bounds reserved during thumbnail placement.
    pub mat_cut_extent: Option<(Point, Point)>,
}

impl AnnotationBounds {
    /// Create empty bounds with no annotations placed yet.
    pub fn empty() -> Self {
        Self {
            corner_detail_box: None,
            thumbnail_box: None,
            thumbnail_label_position: ThumbnailLabelPosition::Below,
            mat_cut_width_label: None,
            mat_cut_height_label: None,
            mat_cut_extent: None,
        }
    }

    /// Get all occupied rects (for collision checking)
    pub fn occupied_rects(&self) -> Vec<Rect> {
        let mut rects = Vec::new();
        if let Some(r) = self.corner_detail_box {
            rects.push(r);
        }
        if let Some(r) = self.thumbnail_box {
            rects.push(r);
        }
        if let Some(r) = self.mat_cut_width_label {
            rects.push(r);
        }
        if let Some(r) = self.mat_cut_height_label {
            rects.push(r);
        }
        rects
    }
}

/// Side of the diagram for placing dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// Check if this is a horizontal side (top/bottom)
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

/// Types of dimensions that can be displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionType {
    // Plan view dimensions
    FrameOutsideWidth,
    FrameOutsideHeight,
    FrameInsideWidthInterior,  // Inside width shown inside the frame opening
    FrameInsideHeightInterior, // Inside height shown inside the frame opening
    MatCutWidth,      // Total mat cut width (visible + rabbet) - shown on bottom
    MatCutHeight,     // Total mat cut height (visible + rabbet) - shown on left when different

    // Section view dimensions
    TotalStackHeight,
}

impl DimensionType {
    /// Get the display priority (1 = highest, must always show)
    /// Lower number = closer to frame (offset_level 0)
    /// Higher number = further from frame (higher offset_level)
    pub fn priority(&self) -> u8 {
        match self {
            // Inside dimensions closest to frame
            DimensionType::FrameInsideWidthInterior => 1,
            DimensionType::FrameInsideHeightInterior => 1,

            // Outside dimensions further from frame
            DimensionType::FrameOutsideWidth => 2,
            DimensionType::FrameOutsideHeight => 2,

            // Section view - always show
            DimensionType::TotalStackHeight => 1,

            // Mat cut dimensions
            DimensionType::MatCutWidth => 2,
            DimensionType::MatCutHeight => 2,
        }
    }

    /// Get the preferred side for this dimension type
    pub fn preferred_side(&self) -> Side {
        match self {
            // Frame outside dimensions
            DimensionType::FrameOutsideWidth => Side::Top,
            DimensionType::FrameOutsideHeight => Side::Right,

            // Inside dimensions (shown inside the frame opening)
            DimensionType::FrameInsideWidthInterior => Side::Top,
            DimensionType::FrameInsideHeightInterior => Side::Right,

            // Mat cut dimensions - width on bottom, height on left (when different)
            DimensionType::MatCutWidth => Side::Bottom,
            DimensionType::MatCutHeight => Side::Left,

            // Section view dimensions on right
            DimensionType::TotalStackHeight => Side::Right,
        }
    }
}

/// A dimension callout to be displayed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionCallout {
    /// The measurement value (in inches, internal)
    pub value: f64,

    /// Formatted label (e.g., "2 3/4\"")
    pub label: String,

    /// What this dimension measures
    pub dimension_type: DimensionType,

    /// Display priority (1 = highest, must show)
    pub priority: u8,

    /// Preferred placement side
    pub preferred_side: Side,

    /// Start and end points in geometry coordinates
    pub extent_start: Point,
    pub extent_end: Point,
}

impl DimensionCallout {
    pub fn new(
        value: f64,
        label: String,
        dimension_type: DimensionType,
        extent_start: Point,
        extent_end: Point,
    ) -> Self {
        Self {
            value,
            label,
            priority: dimension_type.priority(),
            preferred_side: dimension_type.preferred_side(),
            dimension_type,
            extent_start,
            extent_end,
        }
    }
}

/// Text anchor for label positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
}

/// One rendered line of a callout label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelLine {
    pub text: String,
    /// Horizontal labels: (anchor x, visual center y).
    /// Rotated (vertical-side) labels: (visual center x, anchor y), with the
    /// text reading top-to-bottom along +y after `rotate(90)`.
    pub pos: Point,
}

/// Where a callout's label is drawn — computed once by layout and consumed
/// as-is by the renderer (`plan_svg::svg_dimension`), so collision
/// resolution, viewBox bounds, and the SVG all use the same geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelPlacement {
    /// 1 line, or 2 for "Prefix: value" labels (prefix first)
    pub lines: Vec<LabelLine>,
    /// Text anchor along the reading direction
    pub anchor: TextAnchor,
    /// Vertical-side labels are rotated 90° (read top-to-bottom)
    pub rotated: bool,
    /// Background mask that breaks the dimension line under the label
    pub mask: Rect,
}

/// A positioned callout with computed layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedCallout {
    pub callout: DimensionCallout,

    /// Distance level from geometry (for stacking)
    pub offset_level: u8,

    /// Actual side it's placed on (may differ from preferred)
    pub actual_side: Side,

    /// Position of the dimension line
    pub dimension_line_position: f64,

    /// Exactly where the label text and its mask are drawn
    pub label: LabelPlacement,

    /// Visual bounds of the label (text lines ∪ mask), used for collision
    /// detection and viewBox fitting
    pub label_bounds: Rect,
}

impl PositionedCallout {
    /// Move the label (and, along the side's normal, the dimension line) by
    /// (dx, dy). Used to apply collision-pass shifts: horizontal sides move in
    /// Y, vertical sides in X.
    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.dimension_line_position += if self.actual_side.is_horizontal() { dy } else { dx };
        for line in &mut self.label.lines {
            line.pos.x += dx;
            line.pos.y += dy;
        }
        self.label.mask = self.label.mask.translated(dx, dy);
        self.label_bounds = self.label_bounds.translated(dx, dy);
    }
}

/// Options for diagram generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramOptions {
    /// Which view(s) to generate
    pub view: ViewOption,

    /// Canvas width in pixels/points
    pub canvas_width: f64,

    /// Canvas height in pixels/points
    pub canvas_height: f64,

    /// Whether to include title block
    pub include_title_block: bool,

    /// Custom title text (if None, uses "Frame Design")
    pub title_text: Option<String>,

    /// Whether dimensions are in mm (for labels)
    pub unit_mm: bool,

    /// Use tape measure segmented format (e.g., "3/4 - 1/32" instead of "23/32")
    /// Only applies when unit_mm is false (inches mode)
    pub use_tape_segments: bool,

    /// Whether to show dimension callouts (default true)
    /// Set to false for minimal preview diagrams
    pub show_callouts: bool,

    /// Use decimal display for inches (e.g., "4.75" instead of "4 3/4")
    /// Only applies when unit_mm is false (inches mode)
    pub use_decimal_display: bool,

    /// How to handle thin frame layers in plan view
    #[serde(default)]
    pub detail_mode: DetailMode,

    /// Enable corner detail inset in Auto mode (default true)
    #[serde(default = "default_true")]
    pub corner_detail_enabled: bool,

    /// Enable axis break compression in Auto mode (default true)
    #[serde(default = "default_true")]
    pub axis_breaks_enabled: bool,

    /// Show spline (corner key) slot placement overlay (default false)
    #[serde(default)]
    pub show_spline: bool,

    /// Show hanging hardware (D-rings, wire, hook) overlay (default false)
    #[serde(default)]
    pub show_hanging: bool,

    /// Spline slot parameter overrides (None = presets defaults)
    #[serde(default)]
    pub spline_params: Option<crate::joinery::SplineParams>,

    /// Hanging hardware parameter overrides (None = presets defaults)
    #[serde(default)]
    pub hanging_params: Option<crate::hanging::HangingParams>,

    /// Draw the frame face as procedural wood grain (None = outline only, as before)
    #[serde(default)]
    pub wood: Option<super::wood::WoodRender>,
}

fn default_true() -> bool { true }

impl Default for DiagramOptions {
    fn default() -> Self {
        Self {
            view: ViewOption::PlanOnly,
            canvas_width: 800.0,
            canvas_height: 600.0,
            include_title_block: false,
            title_text: None,
            unit_mm: false,
            use_tape_segments: false, // Default off to avoid breaking existing behavior
            use_decimal_display: false,
            show_callouts: true, // Default on for normal diagrams
            detail_mode: DetailMode::Auto,
            corner_detail_enabled: true,
            axis_breaks_enabled: true,
            show_spline: false,
            show_hanging: false,
            spline_params: None,
            hanging_params: None,
            wood: None,
        }
    }
}

/// View selection for diagram generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewOption {
    PlanOnly,
    SectionOnly,
    Both, // For PDF export
}

/// How to handle thin frame layers in plan view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailMode {
    /// Automatic: use corner detail / axis breaks when conditions are met
    Auto,
    /// No detail enhancements (no corner detail, no axis breaks)
    None,
}

impl Default for DetailMode {
    fn default() -> Self {
        DetailMode::Auto
    }
}

/// Result of diagram generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    /// Generated SVG content
    pub svg: String,

    /// Any warnings (e.g., "Mat width dimension omitted due to space")
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_overlap() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 10.0, 10.0);
        let r3 = Rect::new(20.0, 20.0, 10.0, 10.0);

        assert!(r1.overlaps(&r2));
        assert!(!r1.overlaps(&r3));
    }

    #[test]
    fn test_rect_center() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        let c = r.center();
        assert!((c.x - 60.0).abs() < 0.001);
        assert!((c.y - 45.0).abs() < 0.001);
    }

    #[test]
    fn test_dimension_priority() {
        // Inside dimensions have priority 1 (closest to frame)
        assert_eq!(DimensionType::FrameInsideWidthInterior.priority(), 1);
        // Outside dimensions have priority 2 (further from frame)
        assert_eq!(DimensionType::FrameOutsideWidth.priority(), 2);
    }

    #[test]
    fn test_rect_expand() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        let expanded = r.expand(5.0);
        assert!((expanded.x - 5.0).abs() < 0.001);
        assert!((expanded.y - 15.0).abs() < 0.001);
        assert!((expanded.width - 40.0).abs() < 0.001);
        assert!((expanded.height - 50.0).abs() < 0.001);

        // Negative margin (shrink)
        let shrunk = r.expand(-2.0);
        assert!((shrunk.x - 12.0).abs() < 0.001);
        assert!((shrunk.y - 22.0).abs() < 0.001);
        assert!((shrunk.width - 26.0).abs() < 0.001);
        assert!((shrunk.height - 36.0).abs() < 0.001);
    }

    #[test]
    fn test_rect_overlaps_with_margin() {
        // Two touching rects (no gap, no overlap)
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(10.0, 0.0, 10.0, 10.0);

        // Without margin: touching rects do NOT overlap (strict inequality)
        assert!(!r1.overlaps(&r2));
        // With margin: the expanded rects DO overlap
        assert!(r1.overlaps_with_margin(&r2, 1.0));
    }

    #[test]
    fn test_dimension_type_preferred_side() {
        assert_eq!(DimensionType::FrameOutsideWidth.preferred_side(), Side::Top);
        assert_eq!(DimensionType::FrameOutsideHeight.preferred_side(), Side::Right);
        assert_eq!(DimensionType::MatCutWidth.preferred_side(), Side::Bottom);
        assert_eq!(DimensionType::MatCutHeight.preferred_side(), Side::Left);
    }

    #[test]
    fn test_side_is_horizontal() {
        assert!(Side::Top.is_horizontal());
        assert!(Side::Bottom.is_horizontal());
        assert!(!Side::Left.is_horizontal());
        assert!(!Side::Right.is_horizontal());
    }
}
