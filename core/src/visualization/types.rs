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

    /// Calculate distance to another point
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
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

    /// Create from center point and size
    pub fn from_center(center: Point, width: f64, height: f64) -> Self {
        Self {
            x: center.x - width / 2.0,
            y: center.y - height / 2.0,
            width,
            height,
        }
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

    /// Compute the area of overlap between this rect and another
    pub fn overlap_area(&self, other: &Rect) -> f64 {
        let x_overlap = (self.right().min(other.right()) - self.left().max(other.left())).max(0.0);
        let y_overlap = (self.bottom().min(other.bottom()) - self.top().max(other.top())).max(0.0);
        x_overlap * y_overlap
    }

    /// Union this rect with another, returning the bounding rect that contains both
    pub fn union(&self, other: &Rect) -> Self {
        let min_x = self.left().min(other.left());
        let min_y = self.top().min(other.top());
        let max_x = self.right().max(other.right());
        let max_y = self.bottom().max(other.bottom());
        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
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

    /// Check if this is a vertical side (left/right)
    pub fn is_vertical(&self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }

    /// Get the opposite side
    pub fn opposite(&self) -> Self {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
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
    MatVisibleWidth,
    MatVisibleHeight,
    MatCutWidth,      // Total mat cut width (visible + rabbet) - shown on bottom
    MatCutHeight,     // Total mat cut height (visible + rabbet) - shown on left when different
    MatOpeningWidth,
    MatOpeningHeight,
    FrameMaterialWidth,
    RabbetDepth,
    ArtworkWidth,
    ArtworkHeight,

    // Section view dimensions
    FrameDepth,
    GlazingThickness,
    MatboardThickness,
    ArtworkThickness,
    BackingThickness,
    TotalStackHeight,
    Clearance,
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
            DimensionType::FrameDepth => 1,
            DimensionType::TotalStackHeight => 1,

            // Should show if space permits
            DimensionType::MatOpeningWidth => 2,
            DimensionType::MatOpeningHeight => 2,
            DimensionType::FrameMaterialWidth => 3,
            DimensionType::MatVisibleWidth => 3,
            DimensionType::MatVisibleHeight => 3,
            DimensionType::MatCutWidth => 2,  // Same as other mat dimensions
            DimensionType::MatCutHeight => 2,  // Same as other mat dimensions
            DimensionType::RabbetDepth => 3,

            // Nice to have
            DimensionType::ArtworkWidth => 4,
            DimensionType::ArtworkHeight => 4,
            DimensionType::GlazingThickness => 4,
            DimensionType::MatboardThickness => 4,
            DimensionType::ArtworkThickness => 5,
            DimensionType::BackingThickness => 4,
            DimensionType::Clearance => 3,
        }
    }

    /// Get the preferred side for this dimension type
    pub fn preferred_side(&self) -> Side {
        match self {
            // Frame outside dimensions
            DimensionType::FrameOutsideWidth => Side::Top,
            DimensionType::FrameOutsideHeight => Side::Right,

            // Inside dimensions - Interior variants used for display, non-Interior are legacy
            DimensionType::FrameInsideWidthInterior => Side::Top,
            DimensionType::FrameInsideHeightInterior => Side::Right,

            // Mat cut dimensions - width on bottom, height on left (when different)
            DimensionType::MatCutWidth => Side::Bottom,
            DimensionType::MatCutHeight => Side::Left,

            // Other mat dimensions on same side as corresponding frame dims
            DimensionType::MatOpeningWidth
            | DimensionType::MatVisibleWidth
            | DimensionType::ArtworkWidth => Side::Top,

            DimensionType::MatOpeningHeight
            | DimensionType::MatVisibleHeight
            | DimensionType::ArtworkHeight => Side::Right,

            // Small detail dimensions on bottom
            DimensionType::FrameMaterialWidth
            | DimensionType::RabbetDepth => Side::Bottom,

            // Section view dimensions on right
            DimensionType::FrameDepth
            | DimensionType::GlazingThickness
            | DimensionType::MatboardThickness
            | DimensionType::ArtworkThickness
            | DimensionType::BackingThickness
            | DimensionType::TotalStackHeight
            | DimensionType::Clearance => Side::Right,
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

    /// Get the length of the dimension line
    pub fn length(&self) -> f64 {
        self.extent_start.distance_to(&self.extent_end)
    }
}

/// Text anchor for label positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAnchor {
    Start,
    Middle,
    End,
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

    /// Label center position
    pub label_position: Point,

    /// Text anchor for the label
    pub label_anchor: TextAnchor,

    /// Bounding box of the label (for collision detection)
    pub label_bounds: Rect,
}

/// View type for diagram generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewType {
    /// Front-on plan view showing nested rectangles
    Plan,
    /// Cross-section showing material stack
    Section,
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

    /// Frame outer rect center in SVG coordinates — used by the combined view
    /// for frame-centered horizontal alignment instead of viewBox-centered.
    pub frame_center_x: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance_to(&p2) - 5.0).abs() < 0.001);
    }

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
    fn test_side_opposite() {
        assert_eq!(Side::Top.opposite(), Side::Bottom);
        assert_eq!(Side::Left.opposite(), Side::Right);
    }

    #[test]
    fn test_dimension_priority() {
        // Inside dimensions have priority 1 (closest to frame)
        assert_eq!(DimensionType::FrameInsideWidthInterior.priority(), 1);
        // Outside dimensions have priority 2 (further from frame)
        assert_eq!(DimensionType::FrameOutsideWidth.priority(), 2);
        // Nice to have dimensions have lower priority
        assert_eq!(DimensionType::ArtworkThickness.priority(), 5);
    }

    #[test]
    fn test_callout_length() {
        let callout = DimensionCallout::new(
            10.0,
            "10\"".to_string(),
            DimensionType::FrameOutsideWidth,
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        );
        assert!((callout.length() - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_rect_from_center() {
        let center = Point::new(50.0, 30.0);
        let r = Rect::from_center(center, 20.0, 10.0);
        assert!((r.x - 40.0).abs() < 0.001);
        assert!((r.y - 25.0).abs() < 0.001);
        assert!((r.width - 20.0).abs() < 0.001);
        assert!((r.height - 10.0).abs() < 0.001);
        // Center should round-trip
        let c = r.center();
        assert!((c.x - 50.0).abs() < 0.001);
        assert!((c.y - 30.0).abs() < 0.001);
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
    fn test_rect_overlap_area() {
        // Two overlapping rects
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 10.0, 10.0);
        let area = r1.overlap_area(&r2);
        assert!((area - 25.0).abs() < 0.001); // 5x5 overlap

        // Two non-overlapping rects
        let r3 = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert!((r1.overlap_area(&r3) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 20.0, 20.0);
        let u = r1.union(&r2);
        assert!((u.left() - 0.0).abs() < 0.001);
        assert!((u.top() - 0.0).abs() < 0.001);
        assert!((u.right() - 25.0).abs() < 0.001);
        assert!((u.bottom() - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_dimension_type_preferred_side() {
        assert_eq!(DimensionType::FrameOutsideWidth.preferred_side(), Side::Top);
        assert_eq!(DimensionType::FrameOutsideHeight.preferred_side(), Side::Right);
        assert_eq!(DimensionType::MatCutWidth.preferred_side(), Side::Bottom);
        assert_eq!(DimensionType::MatCutHeight.preferred_side(), Side::Left);
    }

    #[test]
    fn test_side_is_horizontal_is_vertical() {
        assert!(Side::Top.is_horizontal());
        assert!(Side::Bottom.is_horizontal());
        assert!(!Side::Left.is_horizontal());
        assert!(!Side::Right.is_horizontal());

        assert!(Side::Left.is_vertical());
        assert!(Side::Right.is_vertical());
        assert!(!Side::Top.is_vertical());
        assert!(!Side::Bottom.is_vertical());
    }
}
