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
    FrameInsideWidth,
    FrameInsideHeight,
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
            DimensionType::FrameInsideWidth => 1,
            DimensionType::FrameInsideHeight => 1,
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
            DimensionType::FrameInsideWidth => Side::Bottom,  // Not used in plan view anymore
            DimensionType::FrameInsideHeight => Side::Left,  // Not used in plan view anymore
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

    /// Whether dimensions are in mm (for labels)
    pub unit_mm: bool,
}

impl Default for DiagramOptions {
    fn default() -> Self {
        Self {
            view: ViewOption::PlanOnly,
            canvas_width: 800.0,
            canvas_height: 600.0,
            include_title_block: false,
            unit_mm: false,
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
        assert_eq!(DimensionType::FrameInsideWidth.priority(), 1);
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
}
