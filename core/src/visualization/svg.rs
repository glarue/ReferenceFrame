// SVG generation for frame diagram
//
// Generates professional, warm-aesthetic SVG diagrams from
// frame designs with adaptive dimension callouts.

use crate::frame::FrameDesign;
use crate::conversions::{format_dimension, Unit};
use super::types::{
    DiagramOptions, DiagramResult, ViewOption, PositionedCallout,
    Rect, Side,
};
use super::style::{DiagramStyle, FillPattern};
use super::geometry::{CornerDetailGeometry, PlanViewGeometry, SectionViewGeometry, estimate_text_width};
use super::callouts::{generate_plan_callouts, generate_section_callouts};
use super::layout::{layout_plan_callouts, LayoutResult};

// ============================================================================
// VISUAL BOUNDARY HELPERS
// ============================================================================
//
// SVG elements have visual extents that differ from their geometric coordinates
// due to stroke widths, marker sizes, and other rendering details. These helpers
// calculate exact visual boundaries for precise alignment.

/// Arrow marker geometry constants
/// These match the inline polygon arrow definitions
mod arrow_geometry {
    /// Arrow marker width in marker units (before stroke-width scaling)
    pub const MARKER_WIDTH: f64 = 8.0;
    /// Arrow marker height in marker units
    #[allow(dead_code)]
    pub const MARKER_HEIGHT: f64 = 5.0;

    /// Leader arrow width (smaller arrow for leaders)
    pub const LEADER_WIDTH: f64 = 6.0;
    /// Leader arrow height
    pub const LEADER_HEIGHT: f64 = 4.0;

    /// Calculate how far an arrow tip extends beyond the line endpoint.
    ///
    /// SVG markers use `markerUnits="strokeWidth"` by default, so marker
    /// dimensions are multiplied by the line's stroke-width.
    ///
    /// For arrow-start (pointing left): refX=8 means tip is at x=0, so tip
    /// extends MARKER_WIDTH units to the left of the line endpoint.
    ///
    /// For arrow-end (pointing right): refX=0 means tip is at x=8, so tip
    /// extends MARKER_WIDTH units to the right of the line endpoint.
    pub fn tip_extension(stroke_width: f64) -> f64 {
        MARKER_WIDTH * stroke_width
    }
}

/// Calculate the line endpoint position needed for an arrow tip to visually
/// align with a target boundary.
/// 
/// # Arguments
/// * `target_x` - The X coordinate where the arrow tip should visually end
/// * `stroke_width` - The stroke width of the line (affects marker scaling)
/// * `is_start_marker` - True for arrow-start (pointing left), false for arrow-end (pointing right)
/// 
/// # Returns
/// The X coordinate to use for the line endpoint so the arrow tip lands at `target_x`
fn arrow_line_endpoint_for_target(target_x: f64, stroke_width: f64, is_start_marker: bool) -> f64 {
    let extension = arrow_geometry::tip_extension(stroke_width);
    if is_start_marker {
        // Arrow-start points left: tip is to the LEFT of line endpoint
        // So line endpoint should be to the RIGHT of target
        target_x + extension
    } else {
        // Arrow-end points right: tip is to the RIGHT of line endpoint
        // So line endpoint should be to the LEFT of target
        target_x - extension
    }
}

/// Calculate the line endpoint position for a vertical arrow to align with a target Y boundary.
fn arrow_line_endpoint_for_target_y(target_y: f64, stroke_width: f64, is_start_marker: bool) -> f64 {
    let extension = arrow_geometry::tip_extension(stroke_width);
    if is_start_marker {
        // Arrow-start points up: tip is ABOVE line endpoint
        // So line endpoint should be BELOW target
        target_y + extension
    } else {
        // Arrow-end points down: tip is BELOW line endpoint
        // So line endpoint should be ABOVE target
        target_y - extension
    }
}

// ============================================================================
// SVG RENDERING CONSTANTS
// ============================================================================
// Cosmetic values that don't belong in DiagramStyle (not user-configurable)
// but should be named for clarity and consistency.

// Dash patterns: "dash,gap" in SVG units
const DASH_BREAK_INDICATOR: &str = "4,3";    // Axis break zigzag lines
const DASH_BOUNDARY: &str = "6,3";           // Content boundary outline
const DASH_ASSEMBLY_MARGIN: &str = "4,2";    // Assembly margin indicator
const DASH_CLEARANCE: &str = "3,2";          // Clearance/interference line

// Opacity values
const OPACITY_CONTENT_BOUNDARY: f64 = 0.5;   // Content boundary outline
const OPACITY_ASSEMBLY_MARGIN: f64 = 0.7;    // Assembly margin rect
const OPACITY_LABEL_BACKGROUND: f64 = 0.75;  // Artwork indicator label bg
const OPACITY_RABBET_BACKGROUND: f64 = 0.5;  // Rabbet indicator bg

// Dimension line break symbols (spark/zigzag on broken dimension lines)
const SPARK_VERTICAL_WIDTH: f64 = 4.0;       // Horizontal extent of vertical spark
const SPARK_VERTICAL_HEIGHT: f64 = 8.0;      // Vertical extent of vertical spark
const SPARK_HORIZONTAL_WIDTH: f64 = 8.0;     // Horizontal extent of horizontal spark
const SPARK_HORIZONTAL_HEIGHT: f64 = 4.0;    // Vertical extent of horizontal spark

// Label layout
const LABEL_MASK_PADDING_X: f64 = 4.0;       // Horizontal padding around label text
const LABEL_MASK_PADDING_Y: f64 = 2.0;       // Vertical padding around label text
const LEADER_LINE_LENGTH: f64 = 10.0;        // Material label leader horizontal segment
const LEADER_STROKE_RATIO: f64 = 0.7;        // Leader line width as fraction of extension_stroke

// Legend
const LEGEND_SWATCH_SIZE: f64 = 12.0;        // Legend color swatch width/height
const LEGEND_SWATCH_STROKE: f64 = 0.5;       // Legend swatch border width
const LEGEND_SWATCH_GAP: f64 = 8.0;          // Gap between swatch and text
const LEGEND_ITEM_GAP: f64 = 16.0;           // Gap between legend items
const LEGEND_CHAR_WIDTH_RATIO: f64 = 0.55;   // Average character width as fraction of font size

// ============================================================================
// AXIS BREAK HELPERS
// ============================================================================

/// Axis break visual constants
const ZIGZAG_AMPLITUDE: f64 = 5.0;
const ZIGZAG_PROUD_AMOUNT: f64 = 8.0;

/// Interpolate y at given x along a line segment between two points
fn y_at_x(p1: (f64, f64), p2: (f64, f64), x: f64) -> f64 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    if (x2 - x1).abs() < 0.001 { return y1; }
    y1 + (x - x1) * (y2 - y1) / (x2 - x1)
}

/// Interpolate x at given y along a line segment between two points
fn x_at_y(p1: (f64, f64), p2: (f64, f64), y: f64) -> f64 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    if (y2 - y1).abs() < 0.001 { return x1; }
    x1 + (y - y1) * (x2 - x1) / (y2 - y1)
}

/// Four control points defining a zigzag break indicator line
struct ZigzagPoints {
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
}

/// Compute horizontal zigzag control points (spans left-to-right at a given y)
fn horizontal_zigzag(center_y: f64, frame_x: f64, frame_w: f64) -> ZigzagPoints {
    ZigzagPoints {
        p0: (frame_x - ZIGZAG_PROUD_AMOUNT, center_y),
        p1: (frame_x + frame_w * 0.15, center_y - ZIGZAG_AMPLITUDE),
        p2: (frame_x + frame_w * 0.85, center_y + ZIGZAG_AMPLITUDE),
        p3: (frame_x + frame_w + ZIGZAG_PROUD_AMOUNT, center_y),
    }
}

/// Compute vertical zigzag control points (spans top-to-bottom at a given x)
fn vertical_zigzag(center_x: f64, frame_y: f64, frame_h: f64) -> ZigzagPoints {
    ZigzagPoints {
        p0: (center_x, frame_y - ZIGZAG_PROUD_AMOUNT),
        p1: (center_x - ZIGZAG_AMPLITUDE, frame_y + frame_h * 0.15),
        p2: (center_x + ZIGZAG_AMPLITUDE, frame_y + frame_h * 0.85),
        p3: (center_x, frame_y + frame_h + ZIGZAG_PROUD_AMOUNT),
    }
}

/// Render a dashed zigzag indicator line
fn render_zigzag_line(svg: &mut String, zz: &ZigzagPoints, line_color: &str, break_line_width: f64) {
    render_zigzag_line_with_opacity(svg, zz, line_color, break_line_width, 1.0);
}

/// Render a dashed zigzag indicator line with custom opacity
fn render_zigzag_line_with_opacity(svg: &mut String, zz: &ZigzagPoints, line_color: &str, break_line_width: f64, opacity: f64) {
    let opacity_attr = if (opacity - 1.0).abs() > 0.001 {
        format!(r#" opacity="{:.2}""#, opacity)
    } else {
        String::new()
    };
    svg.push_str(&format!(
        r#"    <path d="M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="{}" stroke-linecap="round" stroke-linejoin="round"{}/>
"#,
        zz.p0.0, zz.p0.1, zz.p1.0, zz.p1.1,
        zz.p2.0, zz.p2.1, zz.p3.0, zz.p3.1,
        line_color, break_line_width, DASH_BREAK_INDICATOR, opacity_attr
    ));
}

/// Generate SVG polygon element for an arrowhead
///
/// # Arguments
/// * `x1, y1` - Start point of the line
/// * `x2, y2` - End point of the line (where arrow points)
/// * `fill` - Fill color for the arrow
/// * `stroke_width` - Line stroke width (arrows are scaled by this, matching SVG marker behavior)
/// * `is_leader` - True for smaller leader arrows (6x4), false for standard (8x5)
///
/// # Returns
/// SVG polygon element as a string
fn generate_arrow_polygon(x1: f64, y1: f64, x2: f64, y2: f64, fill: &str, stroke_width: f64, is_leader: bool) -> String {
    // Calculate angle from x1,y1 to x2,y2
    let angle = (y2 - y1).atan2(x2 - x1);

    // Base arrow dimensions (unscaled)
    let (base_length, base_width) = if is_leader {
        (arrow_geometry::LEADER_WIDTH, arrow_geometry::LEADER_HEIGHT)
    } else {
        (arrow_geometry::MARKER_WIDTH, arrow_geometry::MARKER_HEIGHT)
    };

    // Scale by stroke width (matching SVG marker markerUnits="strokeWidth" behavior)
    let arrow_length = base_length * stroke_width;
    let arrow_width = base_width * stroke_width;

    // Arrow tip extends beyond the line endpoint (matching SVG marker behavior)
    // With SVG markers, the refX/refY positioned the marker's reference point at the line endpoint,
    // but the arrow tip extended MARKER_WIDTH units beyond it (scaled by stroke_width).
    // We need to replicate this so arrows land exactly at target boundaries.
    let tip_extension = base_length * stroke_width;  // Same as arrow_length
    let tip_x = x2 + tip_extension * angle.cos();
    let tip_y = y2 + tip_extension * angle.sin();

    // Calculate base of arrow (behind the tip)
    let base_x = tip_x - arrow_length * angle.cos();
    let base_y = tip_y - arrow_length * angle.sin();

    // Calculate perpendicular offset for arrow wings
    let perp_x = angle.sin() * (arrow_width / 2.0);
    let perp_y = -angle.cos() * (arrow_width / 2.0);

    // Three points of the triangle
    let p1 = format!("{:.2},{:.2}", tip_x, tip_y);
    let p2 = format!("{:.2},{:.2}", base_x + perp_x, base_y + perp_y);
    let p3 = format!("{:.2},{:.2}", base_x - perp_x, base_y - perp_y);

    // Add data attribute for debugging
    format!(r#"    <polygon points="{} {} {}" fill="{}" data-arrow="true"/>"#, p1, p2, p3, fill)
}

/// Generate SVG line with inline arrow polygons
///
/// Replaces marker-start and marker-end attributes with inline polygon elements.
/// This ensures compatibility with svg2pdf.js which doesn't support SVG markers.
///
/// # Arguments
/// * `x1, y1` - Start point
/// * `x2, y2` - End point
/// * `stroke` - Line color
/// * `stroke_width` - Line width
/// * `arrow_start` - Include arrow at start (pointing from x2 to x1)
/// * `arrow_end` - Include arrow at end (pointing from x1 to x2)
/// * `is_leader` - Use smaller leader arrow dimensions
///
/// # Returns
/// SVG elements as a string (line + polygons)
fn generate_line_with_arrows(
    x1: f64, y1: f64,
    x2: f64, y2: f64,
    stroke: &str,
    stroke_width: f64,
    arrow_start: bool,
    arrow_end: bool,
    is_leader: bool,
) -> String {
    let mut svg = String::new();

    // Add arrow at start if requested (pointing from end to start)
    if arrow_start {
        svg.push_str(&generate_arrow_polygon(x2, y2, x1, y1, stroke, stroke_width, is_leader));
        svg.push('\n');
    }

    // Add the line
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        x1, y1, x2, y2, stroke, stroke_width
    ));
    svg.push('\n');

    // Add arrow at end if requested (pointing from start to end)
    if arrow_end {
        svg.push_str(&generate_arrow_polygon(x1, y1, x2, y2, stroke, stroke_width, is_leader));
        svg.push('\n');
    }

    svg
}

// ============================================================================
// DIMENSION ARROW PRIMITIVE
// ============================================================================
//
// Reusable builder for dimension callouts: extension lines + arrow dimension
// line + label. Encapsulates the geometry and endpoint ordering that was
// previously error-prone when assembled manually.

/// Label content for a DimensionArrow
enum DimensionLabel {
    /// Single line, centered
    Single { text: String, bold: bool },
    /// Two lines (e.g. "Rabbet" + value), right-aligned for vertical dims
    TwoLines { line1: String, line2: String },
}

/// A reusable primitive that renders a complete dimension callout:
/// two extension lines, a dimension line with arrow tips, and a label.
struct DimensionArrow {
    // Measurement boundaries (perpendicular to the dimension line)
    target_a: f64,
    target_b: f64,

    // Where the dimension line sits (in the measurement axis)
    dim_line_pos: f64,

    // Orientation: true = horizontal dim line measuring X distance
    horizontal: bool,

    // Extension lines
    ext_from: f64,        // where extension lines start (geometry edge)
    ext_overshoot: f64,   // how far past dim_line_pos they extend

    // Styling
    stroke_color: String,
    arrow_stroke_width: f64,
    ext_stroke_width: f64,

    // Label
    label: Option<DimensionLabel>,
    font_family: String,
    font_size: f64,
    label_offset: f64,
}

impl DimensionArrow {
    fn new(target_a: f64, target_b: f64, dim_line_pos: f64, horizontal: bool) -> Self {
        Self {
            target_a,
            target_b,
            dim_line_pos,
            horizontal,
            ext_from: 0.0,
            ext_overshoot: 0.0,
            stroke_color: String::new(),
            arrow_stroke_width: 0.5,
            ext_stroke_width: 0.6,
            label: None,
            font_family: String::new(),
            font_size: 10.0,
            label_offset: 15.0,
        }
    }

    fn color(mut self, color: &str) -> Self {
        self.stroke_color = color.to_string();
        self
    }

    fn label(mut self, text: &str, font_family: &str, font_size: f64) -> Self {
        self.label = Some(DimensionLabel::Single { text: text.to_string(), bold: true });
        self.font_family = font_family.to_string();
        self.font_size = font_size;
        self
    }

    fn label_two_lines(mut self, line1: &str, line2: &str, font_family: &str, font_size: f64) -> Self {
        self.label = Some(DimensionLabel::TwoLines {
            line1: line1.to_string(),
            line2: line2.to_string(),
        });
        self.font_family = font_family.to_string();
        self.font_size = font_size;
        self
    }

    fn label_offset(mut self, offset: f64) -> Self {
        self.label_offset = offset;
        self
    }

    fn extension(mut self, from: f64, overshoot: f64) -> Self {
        self.ext_from = from;
        self.ext_overshoot = overshoot;
        self
    }

    fn stroke(mut self, arrow_width: f64, ext_width: f64) -> Self {
        self.arrow_stroke_width = arrow_width;
        self.ext_stroke_width = ext_width;
        self
    }

    fn render(&self) -> String {
        let mut svg = String::new();
        let color = &self.stroke_color;

        // Sort targets so smaller value is "start" and larger is "end"
        let (t_start, t_end) = if self.target_a <= self.target_b {
            (self.target_a, self.target_b)
        } else {
            (self.target_b, self.target_a)
        };
        let mid = (t_start + t_end) / 2.0;

        // ---- Extension lines ----
        if self.horizontal {
            // Horizontal dim line: extension lines are vertical
            let ext_end = self.dim_line_pos + self.ext_overshoot;
            for &tx in &[t_start, t_end] {
                svg.push_str(&format!(
                    r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" opacity="0.5"/>"#,
                    tx, self.ext_from, tx, ext_end, color, self.ext_stroke_width
                ));
                svg.push('\n');
            }
        } else {
            // Vertical dim line: extension lines are horizontal
            let ext_end = self.dim_line_pos + self.ext_overshoot;
            for &ty in &[t_start, t_end] {
                svg.push_str(&format!(
                    r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" opacity="0.5"/>"#,
                    self.ext_from, ty, ext_end, ty, color, self.ext_stroke_width
                ));
                svg.push('\n');
            }
        }

        // ---- Dimension line with arrows ----
        let tip_ext = arrow_geometry::tip_extension(self.arrow_stroke_width);
        let gap = (t_end - t_start).abs();

        if gap > tip_ext * 2.5 {
            // Normal arrows
            if self.horizontal {
                let x1 = arrow_line_endpoint_for_target(t_start, self.arrow_stroke_width, true);
                let x2 = arrow_line_endpoint_for_target(t_end, self.arrow_stroke_width, false);
                svg.push_str(&generate_line_with_arrows(
                    x1, self.dim_line_pos, x2, self.dim_line_pos,
                    color, self.arrow_stroke_width,
                    true, true, false,
                ));
            } else {
                let y1 = arrow_line_endpoint_for_target_y(t_start, self.arrow_stroke_width, true);
                let y2 = arrow_line_endpoint_for_target_y(t_end, self.arrow_stroke_width, false);
                svg.push_str(&generate_line_with_arrows(
                    self.dim_line_pos, y1, self.dim_line_pos, y2,
                    color, self.arrow_stroke_width,
                    true, true, false,
                ));
            }
        } else {
            // Gap too small for inward arrows — use outward-pointing arrows
            let stub_len = tip_ext * 2.5;
            if self.horizontal {
                // Left arrow: stub pointing inward (right) from outside-left
                let left_start = t_start - stub_len;
                let left_end = arrow_line_endpoint_for_target(t_start, self.arrow_stroke_width, false);
                svg.push_str(&generate_line_with_arrows(
                    left_start, self.dim_line_pos, left_end, self.dim_line_pos,
                    color, self.arrow_stroke_width,
                    false, true, false,
                ));
                // Right arrow: stub pointing inward (left) from outside-right
                let right_start = t_end + stub_len;
                let right_end = arrow_line_endpoint_for_target(t_end, self.arrow_stroke_width, true);
                svg.push_str(&generate_line_with_arrows(
                    right_start, self.dim_line_pos, right_end, self.dim_line_pos,
                    color, self.arrow_stroke_width,
                    false, true, false,
                ));
            } else {
                // Top arrow: stub pointing inward (down) from outside-top
                let top_start = t_start - stub_len;
                let top_end = arrow_line_endpoint_for_target_y(t_start, self.arrow_stroke_width, false);
                svg.push_str(&generate_line_with_arrows(
                    self.dim_line_pos, top_start, self.dim_line_pos, top_end,
                    color, self.arrow_stroke_width,
                    false, true, false,
                ));
                // Bottom arrow: stub pointing inward (up) from outside-bottom
                let bot_start = t_end + stub_len;
                let bot_end = arrow_line_endpoint_for_target_y(t_end, self.arrow_stroke_width, true);
                svg.push_str(&generate_line_with_arrows(
                    self.dim_line_pos, bot_start, self.dim_line_pos, bot_end,
                    color, self.arrow_stroke_width,
                    false, true, false,
                ));
            }
        }

        // ---- Label ----
        if let Some(ref label) = self.label {
            match label {
                DimensionLabel::Single { text, bold } => {
                    let weight = if *bold { r#" font-weight="bold""# } else { "" };
                    if self.horizontal {
                        svg.push_str(&format!(
                            r#"    <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}"{} text-anchor="middle">{}</text>"#,
                            mid, self.dim_line_pos + self.label_offset,
                            self.font_family, self.font_size, color, weight,
                            html_escape(text)
                        ));
                    } else {
                        svg.push_str(&format!(
                            r#"    <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}"{} text-anchor="end">{}</text>"#,
                            self.dim_line_pos - self.label_offset, mid,
                            self.font_family, self.font_size, color, weight,
                            html_escape(text)
                        ));
                    }
                    svg.push('\n');
                }
                DimensionLabel::TwoLines { line1, line2 } => {
                    // Two lines, right-aligned to left of vertical dimension line
                    let label_x = self.dim_line_pos - self.label_offset;
                    svg.push_str(&format!(
                        r#"    <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="end" font-weight="bold">{}</text>"#,
                        label_x, mid - 1.0,
                        self.font_family, self.font_size, color,
                        html_escape(line1)
                    ));
                    svg.push('\n');
                    svg.push_str(&format!(
                        r#"    <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="end" font-weight="bold">{}</text>"#,
                        label_x, mid + self.font_size + 1.0,
                        self.font_family, self.font_size, color,
                        html_escape(line2)
                    ));
                    svg.push('\n');
                }
            }
        }

        svg
    }
}

// ============================================================================
// VISUAL GRAMMAR CONSTANTS
// ============================================================================

/// LABEL_BUFFER: Minimum gap between label text and dimension line
/// Reduced for tighter visual association between labels and lines
const LABEL_BUFFER: f64 = 2.0;

/// LABEL_FONT_OFFSET: Multiplier for font size to account for text height/baseline
/// Using consistent value for both horizontal and vertical dimensions
const LABEL_FONT_OFFSET: f64 = 0.4;

/// Main entry point for diagram generation
pub fn generate_diagram(
    design: &FrameDesign,
    options: &DiagramOptions,
) -> DiagramResult {
    let style = DiagramStyle::default();
    generate_diagram_with_style(design, options, &style)
}

/// Generate diagram with custom style (allows PDF-specific styling)
pub fn generate_diagram_with_style(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    match options.view {
        ViewOption::PlanOnly => generate_plan_view(design, options, style),
        ViewOption::SectionOnly => generate_section_view(design, options, style),
        ViewOption::Both => generate_combined_view(design, options, style),
    }
}

/// Generate plan view SVG
fn generate_plan_view(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    // Use preview geometry (scales to artwork) when callouts disabled for stable sizing
    let geometry = if options.show_callouts {
        PlanViewGeometry::from_design_with_mode(
            design,
            options.canvas_width,
            options.canvas_height,
            style,
            options.detail_mode,
        )
    } else {
        PlanViewGeometry::from_design_preview(
            design,
            options.canvas_width,
            options.canvas_height,
            style,
        )
    };

    // Only generate callouts if requested (default true)
    let (callouts, layout) = if options.show_callouts {
        let callouts = generate_plan_callouts(design, &geometry, options.unit_mm, options.use_tape_segments, options.use_decimal_display, style);
        let layout = layout_plan_callouts(&callouts, &geometry, style);
        (callouts, layout)
    } else {
        // Empty callouts for minimal preview
        (Vec::new(), LayoutResult {
            positioned_callouts: Vec::new(),
            warnings: Vec::new(),
        })
    };

    let svg = build_plan_svg(design, &geometry, &callouts, &layout, options, style);

    DiagramResult {
        svg,
        warnings: layout.warnings,
    }
}

/// Generate section view SVG
fn generate_section_view(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    let geometry = SectionViewGeometry::from_design(
        design,
        options.canvas_width,
        options.canvas_height,
        style,
    );

    // Only generate callouts if requested (default true)
    let callouts = if options.show_callouts {
        generate_section_callouts(design, options.unit_mm, options.use_tape_segments, options.use_decimal_display)
    } else {
        Vec::new()
    };
    let svg = build_section_svg(design, &geometry, &callouts, options, style);

    DiagramResult {
        svg,
        warnings: Vec::new(),
    }
}

/// Generate combined view for PDF export
fn generate_combined_view(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    // Vertical stacking: plan view (top), section view (bottom)
    // Add gap between views for breathing room
    // Extra space needed for mat cut label offset (41px downward)
    let gap_between_views = 40.0; // Increased to prevent overlaps
    
    // Account for title block height if present (prevents overlap at top)
    // Title at y=30, subtitle at y=70, then diagram content starts at y=95
    let title_height = if options.include_title_block { 95.0 } else { 0.0 };
    
    // Calculate available height for diagrams
    let available_height = options.canvas_height - gap_between_views - title_height;
    
    // Distribute available height: 58% plan, 42% section
    let plan_height = available_height * 0.58;
    let section_height = available_height * 0.42;

    // Use full PDF font sizes without scaling - dynamic viewBox handles fitting
    // Previously scaled by 0.8× but this made fonts unnecessarily small (17.6pt instead of 22pt)
    let plan_style = style.clone();

    // Section view typically has narrower content, causing larger viewBox scaling
    // Compensate by reducing font sizes proportionally (~24% reduction empirically)
    // This makes rendered font sizes match between plan and section views
    let mut section_style = style.clone();
    section_style.label_font_size = (style.label_font_size * 0.76).round();
    section_style.dimension_offset_base = style.dimension_offset_base * 0.9;
    section_style.dimension_offset_step = style.dimension_offset_step * 0.9;

    let plan_options = DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_height: plan_height,
        ..options.clone()
    };

    let section_options = DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_height: section_height,
        ..options.clone()
    };

    let plan_result = generate_plan_view(design, &plan_options, &plan_style);
    let section_result = generate_section_view(design, &section_options, &section_style);

    // Extract viewBoxes from both views to preserve their coordinate systems
    let plan_viewbox = extract_viewbox(&plan_result.svg);
    let section_viewbox = extract_viewbox(&section_result.svg);

    // Combine into single SVG
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
        options.canvas_width, options.canvas_height
    ));
    svg.push('\n');

    // Title block if requested
    if options.include_title_block {
        svg.push_str(&generate_title_block(design, options, style));
    }

    // Use simple <g> transforms instead of nested <svg> elements
    // This approach is more compatible with flutter_svg and other SVG renderers
    // We manually calculate the transform that emulates the viewBox "meet" behavior
    
    // Plan View Transform (starts after title block)
    let plan_content = extract_svg_content(&plan_result.svg);
    if let Some((vx, vy, vw, vh)) = plan_viewbox {
        let (tx, ty, scale) = calculate_fit_transform(
            vx, vy, vw, vh,
            0.0, title_height, options.canvas_width, plan_height,
            true // Align top (YMin)
        );
        svg.push_str(&format!(
            r#"  <g id="plan-view" transform="translate({:.2}, {:.2}) scale({:.4})">{}</g>"#,
            tx, ty, scale, plan_content
        ));
    } else {
        svg.push_str(&format!(
            r#"  <g id="plan-view">{}</g>"#,
            plan_content
        ));
    }
    svg.push('\n');

    // Section View Transform (starts after plan view + gap)
    let section_content = extract_svg_content(&section_result.svg);
    if let Some((vx, vy, vw, vh)) = section_viewbox {
        let (tx, ty, scale) = calculate_fit_transform(
            vx, vy, vw, vh,
            0.0, title_height + plan_height + gap_between_views, options.canvas_width, section_height,
            true // Align top (YMin)
        );
        svg.push_str(&format!(
            r#"  <g id="section-view" transform="translate({:.2}, {:.2}) scale({:.4})">{}</g>"#,
            tx, ty, scale, section_content
        ));
    } else {
        svg.push_str(&format!(
            r#"  <g id="section-view" transform="translate(0, {})">{}</g>"#,
            title_height + plan_height + gap_between_views,
            section_content
        ));
    }
    svg.push('\n');

    svg.push_str("</svg>");

    let mut warnings = plan_result.warnings;
    warnings.extend(section_result.warnings);

    DiagramResult { svg, warnings }
}

/// Calculate transform (tx, ty, scale) to fit a source rect into a target rect
/// Preserves aspect ratio (meet)
/// align_top: if true, aligns to top of target (YMin), else centers vertically (YMid)
fn calculate_fit_transform(
    src_x: f64, src_y: f64, src_w: f64, src_h: f64,
    dest_x: f64, dest_y: f64, dest_w: f64, dest_h: f64,
    align_top: bool,
) -> (f64, f64, f64) {
    if src_w <= 0.0 || src_h <= 0.0 || dest_w <= 0.0 || dest_h <= 0.0 {
        return (dest_x, dest_y, 1.0);
    }
    
    // Safety check for non-finite values (Infinity/NaN)
    if !src_w.is_finite() || !src_h.is_finite() || !dest_w.is_finite() || !dest_h.is_finite() {
        return (dest_x, dest_y, 1.0);
    }

    // Calculate scale to fit (meet)
    let scale_x = dest_w / src_w;
    let scale_y = dest_h / src_h;
    let scale = scale_x.min(scale_y);

    // Calculate centering offsets
    let new_w = src_w * scale;
    let new_h = src_h * scale;

    let offset_x = (dest_w - new_w) / 2.0;
    
    let offset_y = if align_top {
        0.0 // YMin
    } else {
        (dest_h - new_h) / 2.0 // YMid
    };

    // Calculate translation
    // transform = translate(tx, ty) scale(s)
    
    let tx = dest_x + offset_x - scale * src_x;
    let ty = dest_y + offset_y - scale * src_y;

    // Final safety check
    if !tx.is_finite() || !ty.is_finite() || !scale.is_finite() {
        return (dest_x, dest_y, 1.0);
    }

    (tx, ty, scale)
}

/// Render the corner detail inset overlay for plan view.
/// Shows a zoomed bottom-left corner with frame outer, frame inner,
/// content area (matboard/artwork edge), and rabbet overlap zone.
/// Layout matches the HTML mockup: corner origin near bottom-left,
/// L-shape extends RIGHT and UP.
fn render_corner_detail(
    design: &FrameDesign,
    cd: &CornerDetailGeometry,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();
    let s = cd.detail_scale;
    // Corner origin (outside corner of frame) — near bottom-left of box
    let cx = cd.corner_origin.x;
    let cy = cd.corner_origin.y;
    let bx = cd.box_rect.x;
    let by = cd.box_rect.y;
    let bw = cd.box_rect.width;
    let bh = cd.box_rect.height;

    let frame_w = design.frame_material_width * s;
    let rabbet_w = design.rabbet_width * s;
    // Content area inset from outer edge (frame_w - rabbet_w from outer)
    let content_inset = frame_w - rabbet_w;

    // L-shape extends RIGHT from cx, and UP from cy
    let arm_right = bx + bw - cx - 16.0; // right padding for matboard label
    let arm_up = cy - by - 30.0;        // top padding for title + breathing room

    svg.push_str("  <g id=\"corner-detail\">\n");

    // Clip path for zoomed content (inset from box edges for breathing room)
    let clip_inset_left = 4.0;
    let clip_inset_top = 24.0;   // room for title text
    let clip_inset_right = 4.0;
    let clip_inset_bottom = 4.0;
    let clip_id = "corner-detail-clip";
    svg.push_str(&format!(
        "    <defs><clipPath id=\"{}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath></defs>\n",
        clip_id, bx + clip_inset_left, by + clip_inset_top, bw - clip_inset_left - clip_inset_right, bh - clip_inset_top - clip_inset_bottom
    ));

    // Background box
    svg.push_str(&format!(
        "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"#999\" stroke-width=\"0.75\" rx=\"4\"/>\n",
        bx, by, bw, bh, style.background_color
    ));

    // Title — scale with box height
    let title_font = (bh * 0.08).min(style.dimension_font_size * 0.9);
    let title_y = by + title_font + 5.0;
    svg.push_str(&format!(
        "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"#555\" font-family=\"{}\" font-size=\"{:.1}\" font-weight=\"bold\" text-anchor=\"middle\">Corner Detail</text>\n",
        bx + bw / 2.0, title_y, style.font_family, title_font
    ));

    // Clipped group for zoomed geometry
    svg.push_str(&format!("    <g clip-path=\"url(#{})\">\n", clip_id));

    // Content area / matboard edge color
    let content_color = if design.has_mat() {
        &style.artwork_dimension_color  // Carrot Orange for matboard
    } else {
        &style.artwork_color            // Willow Green for artwork
    };

    // Rabbet overlap zone: L-shaped fill using a single path (no overlap doubling)
    let ci_x = cx + content_inset;  // content inset x
    let ci_y = cy - content_inset;  // content inset y
    let fi_x = cx + frame_w;       // frame inner x
    let fi_y = cy - frame_w;       // frame inner y
    let top_y = cy - arm_up;       // top of visible area
    let right_x = cx + arm_right;  // right of visible area
    // Single L-shaped path: no overlapping rectangles
    svg.push_str(&format!(
        "    <path d=\"M{:.2},{:.2} V{:.2} H{:.2} V{:.2} H{:.2} V{:.2} H{:.2} Z\" fill=\"{}\" fill-opacity=\"0.10\" stroke=\"none\"/>\n",
        ci_x, top_y,     // top-left of vertical strip
        ci_y,            // down to content corner Y
        right_x,         // right along content line to right edge
        fi_y,            // up to frame inner Y
        fi_x,            // left to frame inner X
        top_y,           // up to top
        ci_x,            // back to start X (close)
        content_color
    ));

    // Content area / matboard edge: dashed line
    // Horizontal (goes right from content_inset above outer line)
    svg.push_str(&format!(
        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6,3\" opacity=\"0.7\"/>\n",
        ci_x, ci_y, cx + arm_right, ci_y, content_color
    ));
    // Vertical (goes up from content_inset right of outer line)
    svg.push_str(&format!(
        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6,3\" opacity=\"0.7\"/>\n",
        ci_x, ci_y, ci_x, top_y, content_color
    ));

    // Frame outer L-shape (thick dark) — polyline for clean corner join
    let outer_sw = style.frame_stroke_width;
    svg.push_str(&format!(
        "    <polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linejoin=\"miter\"/>\n",
        cx, top_y, cx, cy, cx + arm_right, cy, style.line_color, outer_sw
    ));

    // Frame inner L-shape (thick dark) — polyline for clean corner join
    svg.push_str(&format!(
        "    <polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linejoin=\"miter\"/>\n",
        fi_x, top_y, fi_x, fi_y, cx + arm_right, fi_y, style.line_color, outer_sw
    ));

    svg.push_str("    </g>\n"); // end clipped group (geometry only)

    // ============ DIMENSION CALLOUTS (outside clip, inside box clip) ============
    // Annotations are clipped to the full box rect so labels near edges stay visible
    // but don't escape the detail box boundary.
    let annot_clip_id = "corner-detail-annot-clip";
    let annot_pad = 2.0;
    svg.push_str(&format!(
        "    <defs><clipPath id=\"{}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath></defs>\n",
        annot_clip_id, bx + annot_pad, by + annot_pad, bw - 2.0 * annot_pad, bh - 2.0 * annot_pad
    ));
    svg.push_str(&format!("    <g clip-path=\"url(#{})\">\n", annot_clip_id));

    let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);
    let label_font = (bh * 0.065).min(style.dimension_font_size * 0.75);

    // 1. Frame width: horizontal dimension between outer and inner, below the corner
    let frame_label = format!("Frame: {}", fmt(design.frame_material_width));
    let fw_dim_y = cy + 12.0; // dimension line position below corner
    let fw_arrow = DimensionArrow::new(cx, cx + frame_w, fw_dim_y, true)
        .color(&style.outside_dimension_color)
        .extension(cy, 2.0)
        .stroke(0.75, 0.5)
        .label(&frame_label, &style.font_family, label_font)
        .label_offset(label_font + 2.0);
    svg.push_str(&fw_arrow.render());

    // 2. Rabbet: vertical dimension between content area and inner, left side
    let rb_dim_x = cx - 6.0; // dimension line position left of corner
    let rb_arrow = DimensionArrow::new(ci_y, fi_y, rb_dim_x, false)
        .color(&style.inside_dimension_color)
        .extension(ci_x, -2.0) // extension lines go leftward from geometry
        .stroke(0.75, 0.5);
    svg.push_str(&rb_arrow.render());
    // Rabbet label: "Rabbet" + value, right-aligned just left of dimension line
    let rabbet_mid_y = (ci_y + fi_y) / 2.0;
    let rb_label_x = rb_dim_x - 4.0;
    svg.push_str(&format!(
        "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" text-anchor=\"end\" font-weight=\"bold\">Rabbet</text>\n",
        rb_label_x, rabbet_mid_y - 1.0,
        style.inside_dimension_color, style.font_family, label_font
    ));
    svg.push_str(&format!(
        "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" text-anchor=\"end\" font-weight=\"bold\">{}</text>\n",
        rb_label_x, rabbet_mid_y + label_font,
        style.inside_dimension_color, style.font_family, label_font,
        html_escape(&fmt(design.rabbet_width))
    ));

    // 3. Content label ("matboard" or "artwork") — dog-leg leader with white background
    // Clamp so label + background stays within box boundary
    let content_label = if design.has_mat() { "matboard" } else { "artwork" };
    let cl_font = label_font * 0.9;
    let cl_text_w = estimate_text_width(content_label, cl_font);
    let box_right = bx + bw - annot_pad;
    let box_top = by + annot_pad;

    let leader_start_x = cx + arm_right * 0.5;
    let leader_start_y = ci_y;
    // Compute ideal end position, then clamp rightward extent to box boundary
    let ideal_end_x = leader_start_x + 6.0 + 4.0;
    let cl_bg_pad = 2.0;
    let max_end_x = box_right - cl_text_w - cl_bg_pad * 2.0 - 2.0; // background padding + safety margin
    let leader_end_x = ideal_end_x.min(max_end_x);
    let leader_bend_x = leader_end_x - 4.0;
    let leader_bend_y = (ci_y - 8.0).max(box_top + cl_font);
    let leader_end_y = leader_bend_y;
    // Dog-leg polyline
    svg.push_str(&format!(
        "    <polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\" opacity=\"0.5\"/>\n",
        leader_start_x, leader_start_y,
        leader_bend_x, leader_bend_y,
        leader_end_x, leader_end_y,
        content_color
    ));
    // White background behind label for contrast
    // Text y is the baseline; cap-height ~0.7em above, descender ~0.25em below.
    // Center the background on the text's visual midpoint (baseline - 0.225em).
    let cl_bg_h = cl_font * 1.2;
    let cl_bg_x = leader_end_x;
    let cl_bg_y = leader_end_y - cl_font * 0.225 - cl_bg_h * 0.5;
    svg.push_str(&format!(
        "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\" rx=\"1\"/>\n",
        cl_bg_x, cl_bg_y, cl_text_w + cl_bg_pad * 2.0, cl_bg_h, style.background_color
    ));
    svg.push_str(&format!(
        "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" font-weight=\"bold\" opacity=\"0.8\">{}</text>\n",
        leader_end_x + cl_bg_pad, leader_end_y,
        content_color, style.font_family, cl_font,
        content_label
    ));

    svg.push_str("    </g>\n"); // end annotation clip group

    svg.push_str("  </g>\n");
    svg
}

/// Build SVG string for plan view
fn build_plan_svg(
    design: &FrameDesign,
    geometry: &PlanViewGeometry,
    _callouts: &[super::types::DimensionCallout],
    layout: &LayoutResult,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    // Calculate viewBox dimensions
    let (min_x, min_y, viewbox_width, viewbox_height) = if options.show_callouts {
        // With callouts: calculate bounds from actual geometry and layout data
        let mut min_x = geometry.frame_outer.left() - style.frame_stroke_width / 2.0;
        let mut max_x = geometry.frame_outer.right() + style.frame_stroke_width / 2.0;
        let mut min_y = geometry.frame_outer.top() - style.frame_stroke_width / 2.0;
        let mut max_y = geometry.frame_outer.bottom() + style.frame_stroke_width / 2.0;

        // Include dimension callouts in bounds
        for callout in &layout.positioned_callouts {
            use super::types::DimensionType;

            // Dimension lines extend beyond geometry
            let dim_line_pos = callout.dimension_line_position;
            let extent_start = &callout.callout.extent_start;
            let extent_end = &callout.callout.extent_end;

            // Track dimension line and extension line bounds
            min_x = min_x.min(extent_start.x.min(extent_end.x) - style.dimension_offset_step);
            max_x = max_x.max(extent_start.x.max(extent_end.x) + style.dimension_offset_step);
            min_y = min_y.min(extent_start.y.min(extent_end.y) - style.dimension_offset_step);
            max_y = max_y.max(extent_start.y.max(extent_end.y) + style.dimension_offset_step);

            // Account for dimension labels using actual label text length
            let label_text_width = estimate_text_width(&callout.callout.label, style.label_font_size);
            let label_height = style.label_font_size * 1.2;

            // Mat cut dimensions get extra offset - calculate it here
            let mat_cut_offset = style.extension_line_overshoot + style.label_font_size / 2.0 + style.dimension_offset_base;

            // Only extend bounds in the direction perpendicular to the dimension line
            // Horizontal dimensions: label centered above/below line
            if (extent_start.y - extent_end.y).abs() < 1.0 {
                // Check if this is a mat cut width dimension (needs extra downward offset)
                let extra_offset = if callout.callout.dimension_type == DimensionType::MatCutWidth {
                    mat_cut_offset
                } else {
                    0.0
                };
                min_y = min_y.min(dim_line_pos - label_height);
                max_y = max_y.max(dim_line_pos + label_height + extra_offset);
                // Horizontal extent of centered text
                let mid_x = (extent_start.x + extent_end.x) / 2.0;
                min_x = min_x.min(mid_x - label_text_width / 2.0);
                max_x = max_x.max(mid_x + label_text_width / 2.0);
            } else {
                // Vertical dimensions: rotated label (text width becomes vertical extent)
                // Check if this is a mat cut height dimension (needs extra leftward offset)
                let extra_offset = if callout.callout.dimension_type == DimensionType::MatCutHeight {
                    mat_cut_offset
                } else {
                    0.0
                };
                min_x = min_x.min(dim_line_pos - label_height - extra_offset);
                max_x = max_x.max(dim_line_pos + label_height);
                let mid_y = (extent_start.y + extent_end.y) / 2.0;
                min_y = min_y.min(mid_y - label_text_width / 2.0);
                max_y = max_y.max(mid_y + label_text_width / 2.0);
            }
        }

        // Include proportional thumbnail in bounds
        if let Some(thumb) = &geometry.thumbnail {
            min_x = min_x.min(thumb.left());
            max_x = max_x.max(thumb.right());
            min_y = min_y.min(thumb.top());
            // Two label lines: 10px gap + 8px line + 10px gap + 8px line
            max_y = max_y.max(thumb.bottom() + 10.0 + 10.0 + 8.0);
        }

        // Include corner detail inset in bounds
        if let Some(cd) = &geometry.corner_detail {
            min_x = min_x.min(cd.box_rect.left());
            max_x = max_x.max(cd.box_rect.right());
            min_y = min_y.min(cd.box_rect.top());
            max_y = max_y.max(cd.box_rect.bottom());
        }

        // Add padding for visual comfort
        let padding = style.margin;
        (min_x - padding, min_y - padding, max_x - min_x + 2.0 * padding, max_y - min_y + 2.0 * padding)
    } else {
        // Without callouts (preview mode): use fixed viewBox matching canvas dimensions
        // This ensures the diagram size stays constant regardless of frame dimensions
        (0.0, 0.0, options.canvas_width, options.canvas_height)
    };

    // Build SVG with dynamic viewBox
    let mut svg = String::new();

    // SVG header with calculated viewBox
    svg.push_str(&format!(
        r#"<svg viewBox="{:.2} {:.2} {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">"#,
        min_x, min_y, viewbox_width, viewbox_height
    ));
    svg.push('\n');

    // Defs for patterns
    svg.push_str(&generate_defs(style));

    let has_breaks = geometry.use_axis_break_x || geometry.use_axis_break_y;

    // Conditional color: what sits in the rabbet determines the content edge color
    let content_edge_color = if design.has_mat() {
        &style.artwork_dimension_color  // Carrot Orange #f8961e (matboard edge)
    } else {
        &style.artwork_color            // Willow Green #90be6d (artwork edge)
    };

    // When breaks are NOT active, draw full rect strokes as before.
    // When breaks ARE active, skip rect strokes here — they'll be drawn as
    // corner segments after the zigzag ribbons mask the fills.
    if !has_breaks {
        // Geometry group — full rect strokes (no breaks)
        svg.push_str("  <g id=\"geometry\">\n");
        svg.push_str(&svg_rect(&geometry.frame_outer, &style.line_color, style.frame_stroke_width, None));
        svg.push_str(&svg_rect(&geometry.frame_inner, &style.line_color, style.frame_stroke_width, None));
        if let Some(mat_opening) = &geometry.mat_opening {
            svg.push_str(&svg_rect(mat_opening, &style.line_color, style.mat_stroke_width, None));
            svg.push_str(&format!(
                "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"4,2\" fill=\"none\" opacity=\"0.6\"/>\n",
                geometry.artwork.x, geometry.artwork.y,
                geometry.artwork.width, geometry.artwork.height,
                style.artwork_color, style.extension_stroke_width
            ));
        }
        svg.push_str("  </g>\n");
    }

    // Frame/mat overlap visualization - semi-transparent fill showing rabbet overlap area
    let rabbet_scaled = design.rabbet_width * geometry.scale;
    if rabbet_scaled > 0.5 {
        svg.push_str("  <g id=\"rabbet-overlap\">\n");
        let ox = geometry.content_area.x;
        let oy = geometry.content_area.y;
        let ow = geometry.content_area.width;
        let oh = geometry.content_area.height;
        let ix = geometry.frame_inner.x;
        let iy = geometry.frame_inner.y;
        let iw = geometry.frame_inner.width;
        let ih = geometry.frame_inner.height;
        let path_d = format!(
            "M{:.2},{:.2} h{:.2} v{:.2} h{:.2} Z M{:.2},{:.2} v{:.2} h{:.2} v{:.2} Z",
            ox, oy, ow, oh, -ow,
            ix, iy, ih, iw, -ih
        );
        svg.push_str(&format!(
            "    <path d=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
            path_d, content_edge_color
        ));
        if !has_breaks {
            svg.push_str(&format!(
                "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"{}\" stroke-opacity=\"{}\"/>\n",
                ox, oy, ow, oh, content_edge_color, style.extension_stroke_width * 0.8, DASH_ASSEMBLY_MARGIN, OPACITY_CONTENT_BOUNDARY
            ));
        }
        svg.push_str("  </g>\n");
    }

    // Content/matboard boundary
    if !has_breaks {
        svg.push_str("  <g id=\"content-boundary\">\n");
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"{}\" fill=\"none\" opacity=\"{}\"/>\n",
            geometry.content_area.x, geometry.content_area.y,
            geometry.content_area.width, geometry.content_area.height,
            content_edge_color, style.extension_stroke_width, DASH_BOUNDARY, OPACITY_CONTENT_BOUNDARY
        ));
        svg.push_str("  </g>\n");
    }

    // Mat/artwork overlap visualization
    if let Some(mat_opening) = &geometry.mat_opening {
        let mat_overlap_scaled = design.mat_overlap * geometry.scale;
        if mat_overlap_scaled > 0.5 && design.has_mat() {
            svg.push_str("  <g id=\"mat-overlap\">\n");
            let ox = geometry.artwork.x;
            let oy = geometry.artwork.y;
            let ow = geometry.artwork.width;
            let oh = geometry.artwork.height;
            let ix = mat_opening.x;
            let iy = mat_opening.y;
            let iw = mat_opening.width;
            let ih = mat_opening.height;
            let path_d = format!(
                "M{:.2},{:.2} h{:.2} v{:.2} h{:.2} Z M{:.2},{:.2} v{:.2} h{:.2} v{:.2} Z",
                ox, oy, ow, oh, -ow,
                ix, iy, ih, iw, -ih
            );
            svg.push_str(&format!(
                "    <path d=\"{}\" fill=\"#888888\" fill-opacity=\"0.12\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
                path_d
            ));
            if !has_breaks {
                svg.push_str(&format!(
                    "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#888888\" stroke-width=\"{:.2}\" stroke-dasharray=\"3,2\" stroke-opacity=\"0.4\"/>\n",
                    ox, oy, ow, oh, style.extension_stroke_width * 0.8
                ));
            }
            svg.push_str("  </g>\n");
        }
    }

    // Plan view axis break: zigzag ribbon masking + corner strokes
    if has_breaks {
        let break_line_width = style.frame_stroke_width * 0.5;
        svg.push_str("  <g id=\"plan-axis-breaks\">\n");

        // Compute zigzag control points for each active break
        let x_zigzags = if geometry.use_axis_break_x {
            let left_zz = vertical_zigzag(geometry.break_x_start, geometry.frame_outer.top(), geometry.frame_outer.height);
            let right_zz = vertical_zigzag(geometry.break_x_end, geometry.frame_outer.top(), geometry.frame_outer.height);
            Some((left_zz, right_zz))
        } else {
            None
        };

        let y_zigzags = if geometry.use_axis_break_y {
            let top_zz = horizontal_zigzag(geometry.break_y_start, geometry.frame_outer.left(), geometry.frame_outer.width);
            let bot_zz = horizontal_zigzag(geometry.break_y_end, geometry.frame_outer.left(), geometry.frame_outer.width);
            Some((top_zz, bot_zz))
        } else {
            None
        };

        // STEP 1: Full rect strokes (ribbon masks in step 2 will clip break zones)
        // Drawing full <rect> elements gives clean mitered corners without join artifacts.
        struct RectStroke<'a> {
            rect: &'a Rect,
            color: &'a str,
            width: f64,
            dasharray: Option<&'a str>,
            opacity: f64,
        }
        let mut rect_strokes: Vec<RectStroke> = vec![
            RectStroke { rect: &geometry.frame_outer, color: &style.line_color, width: style.frame_stroke_width, dasharray: None, opacity: 1.0 },
            RectStroke { rect: &geometry.frame_inner, color: &style.line_color, width: style.frame_stroke_width, dasharray: None, opacity: 1.0 },
            RectStroke { rect: &geometry.content_area, color: content_edge_color, width: style.extension_stroke_width, dasharray: Some(DASH_BOUNDARY), opacity: OPACITY_CONTENT_BOUNDARY },
        ];
        if let Some(ref mat_opening) = geometry.mat_opening {
            rect_strokes.push(RectStroke { rect: mat_opening, color: &style.line_color, width: style.mat_stroke_width, dasharray: None, opacity: 1.0 });
            rect_strokes.push(RectStroke { rect: &geometry.artwork, color: &style.artwork_color, width: style.extension_stroke_width, dasharray: Some("4,2"), opacity: 0.6 });
        }
        if rabbet_scaled > 0.5 {
            rect_strokes.push(RectStroke { rect: &geometry.content_area, color: content_edge_color, width: style.extension_stroke_width * 0.8, dasharray: Some(DASH_ASSEMBLY_MARGIN), opacity: OPACITY_CONTENT_BOUNDARY });
        }
        if let Some(ref mat_opening) = geometry.mat_opening {
            let mat_overlap_scaled = design.mat_overlap * geometry.scale;
            if mat_overlap_scaled > 0.5 && design.has_mat() {
                rect_strokes.push(RectStroke { rect: &geometry.artwork, color: "#888888", width: style.extension_stroke_width * 0.8, dasharray: Some("3,2"), opacity: 0.4 });
                let _ = mat_opening;
            }
        }

        for rs in &rect_strokes {
            let dash_attr = if let Some(da) = rs.dasharray {
                format!(r#" stroke-dasharray="{}""#, da)
            } else {
                String::new()
            };
            let opacity_attr = if (rs.opacity - 1.0).abs() > 0.001 {
                format!(r#" opacity="{:.2}""#, rs.opacity)
            } else {
                String::new()
            };
            svg.push_str(&format!(
                r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="{}"{}{}/>"#,
                rs.rect.x, rs.rect.y, rs.rect.width, rs.rect.height,
                rs.color, rs.width, dash_attr, opacity_attr
            ));
            svg.push('\n');
        }

        // STEP 2: Zigzag ribbon masks (white-filled closed paths that hide break zones)
        if let Some((ref left_zz, ref right_zz)) = x_zigzags {
            let ribbon = format!(
                "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
                left_zz.p0.0, left_zz.p0.1, left_zz.p1.0, left_zz.p1.1,
                left_zz.p2.0, left_zz.p2.1, left_zz.p3.0, left_zz.p3.1,
                right_zz.p3.0, right_zz.p3.1, right_zz.p2.0, right_zz.p2.1,
                right_zz.p1.0, right_zz.p1.1, right_zz.p0.0, right_zz.p0.1,
            );
            svg.push_str(&format!(
                r#"    <path d="{}" fill="{}" stroke="none"/>"#,
                ribbon, style.background_color
            ));
            svg.push('\n');
        }

        if let Some((ref top_zz, ref bot_zz)) = y_zigzags {
            let ribbon = format!(
                "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
                top_zz.p0.0, top_zz.p0.1, top_zz.p1.0, top_zz.p1.1,
                top_zz.p2.0, top_zz.p2.1, top_zz.p3.0, top_zz.p3.1,
                bot_zz.p3.0, bot_zz.p3.1, bot_zz.p2.0, bot_zz.p2.1,
                bot_zz.p1.0, bot_zz.p1.1, bot_zz.p0.0, bot_zz.p0.1,
            );
            svg.push_str(&format!(
                r#"    <path d="{}" fill="{}" stroke="none"/>"#,
                ribbon, style.background_color
            ));
            svg.push('\n');
        }

        // STEP 3: Zigzag indicator lines (dashed, reduced opacity so artwork indicators stay legible)
        let zz_opacity = 0.45;
        if let Some((ref left_zz, ref right_zz)) = x_zigzags {
            render_zigzag_line_with_opacity(&mut svg, left_zz, &style.line_color, break_line_width, zz_opacity);
            render_zigzag_line_with_opacity(&mut svg, right_zz, &style.line_color, break_line_width, zz_opacity);
        }
        if let Some((ref top_zz, ref bot_zz)) = y_zigzags {
            render_zigzag_line_with_opacity(&mut svg, top_zz, &style.line_color, break_line_width, zz_opacity);
            render_zigzag_line_with_opacity(&mut svg, bot_zz, &style.line_color, break_line_width, zz_opacity);
        }

        svg.push_str("  </g>\n");
    }

    // Dimensions group (only if callouts are enabled)
    if options.show_callouts {
        svg.push_str("  <g id=\"dimensions\">\n");
        for callout in &layout.positioned_callouts {
            svg.push_str(&svg_dimension(callout, style, geometry));
        }
        svg.push_str("  </g>\n");
    }

    // Artwork dimensions indicator - arrows extending to artwork boundary
    // Only show if callouts are enabled
    if options.show_callouts {
        // The artwork boundary is shown as a dashed line with stroke width extension_stroke_width * 0.8
        // We want arrow tips to land exactly at the INNER edge of that dashed stroke
        let artwork_center = geometry.artwork.center();
        let artwork_center_y = artwork_center.y;
        let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };

        // Always use artwork color — these arrows indicate artwork size
        let artwork_indicator_color = &style.artwork_color;

        svg.push_str(&format!(
            r#"  <g id="artwork-indicator">"#
        ));
        svg.push('\n');

        // Calculate arrow stroke width (used for marker scaling)
        let arrow_stroke_width = style.dimension_stroke_width * 0.7;

        // The artwork boundary dashed line has this stroke width
        let artwork_boundary_stroke = style.extension_stroke_width * 0.8;

        // Arrow tips should land at the inner edge of the artwork boundary stroke
        // Inner edge = geometric boundary + half the boundary stroke width
        let target_left = geometry.artwork.left() + artwork_boundary_stroke / 2.0;
        let target_right = geometry.artwork.right() - artwork_boundary_stroke / 2.0;
        let target_top = geometry.artwork.top() + artwork_boundary_stroke / 2.0;
        let target_bottom = geometry.artwork.bottom() - artwork_boundary_stroke / 2.0;

        // Calculate line endpoints so arrow tips land at targets
        let h_line_x1 = arrow_line_endpoint_for_target(target_left, arrow_stroke_width, true);
        let h_line_x2 = arrow_line_endpoint_for_target(target_right, arrow_stroke_width, false);
        let v_line_y1 = arrow_line_endpoint_for_target_y(target_top, arrow_stroke_width, true);
        let v_line_y2 = arrow_line_endpoint_for_target_y(target_bottom, arrow_stroke_width, false);

        // Artwork dimension label (compute size first so arrow lines can stop at label edges)
        let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);
        let artwork_label = format!(
            "{} × {}",
            fmt(design.artwork_height),
            fmt(design.artwork_width)
        );
        let mask_margin = LABEL_MASK_PADDING_X;
        let text_bg_w = estimate_text_width(&artwork_label, style.label_font_size) + mask_margin * 2.0;
        let text_bg_h = style.label_font_size * 1.3 + mask_margin * 2.0;

        // Label background edges — arrow lines stop here instead of passing through
        let label_left = artwork_center.x - text_bg_w / 2.0;
        let label_right = artwork_center.x + text_bg_w / 2.0;
        let label_top = artwork_center_y - text_bg_h / 2.0;
        let label_bottom = artwork_center_y + text_bg_h / 2.0;

        // Horizontal line with arrows (with spark symbol if X break active)
        // Split into left and right segments that stop at label edges
        if geometry.use_axis_break_x {
            let break_center_x = (geometry.break_x_start + geometry.break_x_end) / 2.0;
            let sw = SPARK_HORIZONTAL_WIDTH;
            let sh = SPARK_HORIZONTAL_HEIGHT;

            // Left segment: arrow to spark
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                break_center_x - sw / 2.0, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            // Spark symbol
            svg.push_str(&format!(
                r#"    <path d="M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none"/>"#,
                break_center_x - sw / 2.0, artwork_center_y,
                break_center_x - sw / 4.0, artwork_center_y - sh,
                break_center_x + sw / 4.0, artwork_center_y + sh,
                break_center_x + sw / 2.0, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width
            ));
            svg.push('\n');
            // Right segment: spark to arrow
            svg.push_str(&generate_line_with_arrows(
                break_center_x + sw / 2.0, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else if label_left > h_line_x1 && label_right < h_line_x2 {
            // Label fits inside artwork area — split into two segments at label edges
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                label_left, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            svg.push_str(&generate_line_with_arrows(
                label_right, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else {
            // Label wider than artwork area — draw full line (label bg will mask center)
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, true, false,
            ));
        }

        // Vertical line with arrows (with spark symbol if Y break active)
        // Split into top and bottom segments that stop at label edges
        if geometry.use_axis_break_y {
            let break_center_y = (geometry.break_y_start + geometry.break_y_end) / 2.0;
            let sw = SPARK_VERTICAL_WIDTH;
            let sh = SPARK_VERTICAL_HEIGHT;

            // Top segment: arrow to spark
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, break_center_y - sh / 2.0,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            // Spark symbol (vertical orientation)
            svg.push_str(&format!(
                r#"    <path d="M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none"/>"#,
                artwork_center.x, break_center_y - sh / 2.0,
                artwork_center.x + sw, break_center_y - sh / 4.0,
                artwork_center.x - sw, break_center_y + sh / 4.0,
                artwork_center.x, break_center_y + sh / 2.0,
                artwork_indicator_color, arrow_stroke_width
            ));
            svg.push('\n');
            // Bottom segment: spark to arrow
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, break_center_y + sh / 2.0,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else if label_top > v_line_y1 && label_bottom < v_line_y2 {
            // Label fits inside artwork area — split into two segments at label edges
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, label_top,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, label_bottom,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else {
            // Label taller than artwork area — draw full line (label bg will mask center)
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                true, true, false,
            ));
        }

        // Draw background rectangle FIRST (so it appears behind the text)
        // Centered on artwork_center
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="{}" stroke="none" rx="2"/>"#,
            artwork_center.x - text_bg_w / 2.0,
            artwork_center_y - text_bg_h / 2.0,
            text_bg_w,
            text_bg_h,
            style.background_color,
            OPACITY_LABEL_BACKGROUND
        ));
        svg.push('\n');

        // Draw text SECOND (so it appears on top)
        // text-anchor="middle" ensures horizontal centering at artwork_center.x
        // dominant-baseline="middle" ensures vertical centering at artwork_center.y
        //
        // KNOWN ISSUE: svg2pdf.js ignores dominant-baseline, causing text to sit above the line
        // instead of being bisected by it in PDF exports. The proper fix would be to offset the
        // y-coordinate by ~0.35em (half the text height), but this requires either:
        //   1. Font metrics library (ttf-parser) - adds dependencies, requires bundling fonts
        //   2. JavaScript post-processing - adds complexity, fragile
        //   3. Hardcoded font-specific metrics - breaks with font fallbacks
        //
        // DECISION: Accept imperfect PDF rendering rather than engineering complexity.
        // The browser rendering is correct, and the PDF issue is a minor aesthetic imperfection.
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{:.2}" text-anchor="middle" dominant-baseline="middle">{}</text>"#,
            artwork_center.x, artwork_center_y,
            artwork_indicator_color, style.font_family, style.label_font_size,
            html_escape(&artwork_label)
        ));
        svg.push('\n');
        svg.push_str("  </g>\n");
    }

    // Corner detail inset overlay (only when breaks active)
    if let Some(cd) = &geometry.corner_detail {
        svg.push_str(&render_corner_detail(design, cd, options, style));
    }

    // Proportional thumbnail — true aspect ratio silhouette (only when breaks active)
    if let Some(thumb) = &geometry.thumbnail {
        svg.push_str("  <g id=\"thumbnail\">\n");
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"#999\" stroke-width=\"0.75\" fill=\"none\"/>\n",
            thumb.x, thumb.y, thumb.width, thumb.height
        ));
        let label_x = thumb.x + thumb.width / 2.0;
        let label_y = thumb.bottom() + 10.0;
        svg.push_str(&format!(
            "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"#999\" font-family=\"{}\" font-size=\"8\" text-anchor=\"middle\">Actual</text>\n",
            label_x, label_y, style.font_family
        ));
        svg.push_str(&format!(
            "    <text x=\"{:.2}\" y=\"{:.2}\" fill=\"#999\" font-family=\"{}\" font-size=\"8\" text-anchor=\"middle\">proportions</text>\n",
            label_x, label_y + 10.0, style.font_family
        ));
        svg.push_str("  </g>\n");
    }

    svg.push_str("</svg>");
    svg
}

/// Build SVG string for section view
///
/// Shows frame L-shape profile with materials stacked vertically.
/// Layout: Frame on left, materials stack from top to bottom in rabbet area,
/// dog-leg labels to the right for clear text positioning.
///
/// This function uses a self-centering approach: content is rendered at its
/// natural coordinates, then the actual horizontal bounds are calculated and
/// a centering transform is applied to horizontally center the content.
fn build_section_svg(
    design: &FrameDesign,
    geometry: &SectionViewGeometry,
    callouts: &[super::types::DimensionCallout],
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);

    // Section view uses black for all dimension lines/text (not the colored scheme from plan view)
    let dim_color = &style.line_color;

    // Track content bounds for dynamic viewBox
    // These will be updated as we render content
    let mut content_min_x = f64::MAX;
    let mut content_max_x = f64::MIN;
    let mut content_min_y = f64::MAX;
    let mut content_max_y = f64::MIN;

    // Helper macros to track bounds
    macro_rules! track_x {
        ($x:expr) => {
            {
                let x = $x;
                if x < content_min_x { content_min_x = x; }
                if x > content_max_x { content_max_x = x; }
            }
        };
        ($x1:expr, $x2:expr) => {
            {
                track_x!($x1);
                track_x!($x2);
            }
        };
    }

    macro_rules! track_y {
        ($y:expr) => {
            {
                let y = $y;
                if y < content_min_y { content_min_y = y; }
                if y > content_max_y { content_max_y = y; }
            }
        };
        ($y1:expr, $y2:expr) => {
            {
                track_y!($y1);
                track_y!($y2);
            }
        };
    }

    let mut svg = String::new();

    // SVG header
    svg.push_str(&format!(
        r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
        options.canvas_width, options.canvas_height
    ));
    svg.push('\n');

    // Defs
    svg.push_str(&generate_defs(style));

    // Section geometry
    svg.push_str("  <g id=\"section-geometry\">\n");

    // Frame dimensions
    let frame_x = geometry.frame_profile.x;
    let frame_y = geometry.frame_profile.y;
    let frame_w = geometry.frame_profile.width;
    let frame_h = geometry.frame_profile.height;
    let rabbet_w = geometry.rabbet_area.width;
    let rabbet_h = geometry.rabbet_area.height;

    // Track frame bounds
    track_x!(frame_x);
    track_x!(frame_x + frame_w);
    track_y!(frame_y);
    track_y!(frame_y + frame_h);

    // Track material bounds
    track_x!(geometry.glazing.left(), geometry.glazing.right());
    track_y!(geometry.glazing.top(), geometry.glazing.bottom());

    if let Some(matboard) = &geometry.matboard {
        track_x!(matboard.left(), matboard.right());
        track_y!(matboard.top(), matboard.bottom());
    }

    track_x!(geometry.artwork.left(), geometry.artwork.right());
    track_y!(geometry.artwork.top(), geometry.artwork.bottom());

    track_x!(geometry.backing.left(), geometry.backing.right());
    track_y!(geometry.backing.top(), geometry.backing.bottom());

    if geometry.assembly_margin.height > 0.5 {
        track_x!(geometry.assembly_margin.left(), geometry.assembly_margin.right());
        track_y!(geometry.assembly_margin.top(), geometry.assembly_margin.bottom());
    }

    // Draw materials FIRST (so frame overlaps them at rabbet)
    // Materials are stacked vertically (glazing at top, backing at bottom)
    //
    // TECHNICAL DRAWING CONVENTION: Lines represent edges, not physical objects.
    // The stroke width is purely for visual hierarchy/legibility, not physical dimension.
    // When two surfaces meet (e.g., glazing top meets rabbet lip), they share the same
    // geometric edge line. The frame is drawn LAST so its stroke cleanly covers the
    // material edges at the rabbet boundary - this is standard practice.
    //
    // We draw materials at their TRUE geometric positions (no stroke offsets).
    svg.push_str(&svg_rect(
        &geometry.glazing,
        &style.line_color,
        style.extension_stroke_width,
        Some(&get_fill_for_pattern(&style.material_patterns.glazing)),
    ));

    if let Some(matboard) = &geometry.matboard {
        svg.push_str(&svg_rect(
            matboard,
            &style.line_color,
            style.extension_stroke_width,
            Some(&get_fill_for_pattern(&style.material_patterns.matboard)),
        ));
    }

    svg.push_str(&svg_rect(
        &geometry.artwork,
        &style.line_color,
        style.extension_stroke_width,
        Some(&get_fill_for_pattern(&style.material_patterns.artwork)),
    ));

    svg.push_str(&svg_rect(
        &geometry.backing,
        &style.line_color,
        style.extension_stroke_width,
        Some(&get_fill_for_pattern(&style.material_patterns.backing)),
    ));

    // Assembly margin - shown as unfilled dashed rectangle
    // This represents the tolerance/clearance allowed for assembly
    if geometry.assembly_margin.height > 0.5 { // Only show if visible
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" stroke="{}" stroke-width="{}" stroke-dasharray="{}" fill="none" opacity="{}"/>"#,
            geometry.assembly_margin.x, geometry.assembly_margin.y,
            geometry.assembly_margin.width, geometry.assembly_margin.height,
            style.dimension_color, style.extension_stroke_width,
            DASH_ASSEMBLY_MARGIN, OPACITY_ASSEMBLY_MARGIN
        ));
        svg.push('\n');
    }

    // Draw frame as L-shape polygon with rabbet cutout at bottom-right
    // TOP = front of frame, BOTTOM = back of frame
    // The rabbet is a step cut from the back, materials sit in it pressed against the lip
    //
    // If using horizontal axis break, the frame is drawn in two portions with a break indicator between:
    // - Left portion: outer edge of frame
    // - Right portion: L-shape with rabbet area
    //
    // If using vertical axis break, the frame is drawn in two portions:
    // - Top portion: front face (simple rectangle)
    // - Bottom portion: L-shape with rabbet area
    //
    // Both breaks can be active simultaneously
    
    if geometry.use_axis_break_y && !geometry.use_axis_break {
        // Vertical break only (no horizontal break)
        let break_line_width = style.frame_stroke_width * 0.5;

        let top_zz = horizontal_zigzag(geometry.axis_break_start_y, frame_x, frame_w);
        let bottom_zz = horizontal_zigzag(geometry.axis_break_end_y, frame_x, frame_w);

        // Calculate where zigzag crosses frame boundaries
        let top_y_at_left = y_at_x(top_zz.p0, top_zz.p1, frame_x);
        let top_y_at_right = y_at_x(top_zz.p2, top_zz.p3, frame_x + frame_w);
        let bottom_y_at_left = y_at_x(bottom_zz.p0, bottom_zz.p1, frame_x);
        let bottom_y_at_right = y_at_x(bottom_zz.p2, bottom_zz.p3, frame_x + frame_w);

        // Top portion: simple rectangle (front face)
        let top_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, frame_y,
            frame_x + frame_w, frame_y,
            frame_x + frame_w, top_y_at_right,
            top_zz.p2.0, top_zz.p2.1,
            top_zz.p1.0, top_zz.p1.1,
            frame_x, top_y_at_left,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            top_fill_path, get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');

        // Stroke the non-break edges of top portion (top edge and left/right edges down to zigzag)
        let top_stroke_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            frame_x, top_y_at_left,      // Start at left edge where zigzag meets frame
            frame_x, frame_y,             // Up to top-left corner
            frame_x + frame_w, frame_y,   // Across to top-right corner
            frame_x + frame_w, top_y_at_right,  // Down to where zigzag meets right edge
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            top_stroke_path, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Bottom portion: L-shape with rabbet
        let bottom_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, bottom_y_at_left,
            bottom_zz.p1.0, bottom_zz.p1.1,
            bottom_zz.p2.0, bottom_zz.p2.1,
            frame_x + frame_w, bottom_y_at_right,
            frame_x + frame_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h,
            frame_x, frame_y + frame_h,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            bottom_fill_path, get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');

        let bottom_stroke_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            frame_x + frame_w, bottom_y_at_right,
            frame_x + frame_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h,
            frame_x, frame_y + frame_h,
            frame_x, bottom_y_at_left,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            bottom_stroke_path, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Break indicator: dashed zigzag lines
        render_zigzag_line(&mut svg, &top_zz, &style.line_color, break_line_width);
        render_zigzag_line(&mut svg, &bottom_zz, &style.line_color, break_line_width);
    } else if geometry.use_axis_break && geometry.use_axis_break_y {
        // Both horizontal and vertical breaks active
        // OVERLAY APPROACH: Draw full L-shape, then overlay white zigzag-shaped gap bands
        let break_line_width = style.frame_stroke_width * 0.5;

        let top_zz = horizontal_zigzag(geometry.axis_break_start_y, frame_x, frame_w);
        let bot_zz = horizontal_zigzag(geometry.axis_break_end_y, frame_x, frame_w);
        let left_zz = vertical_zigzag(geometry.axis_break_start_x, frame_y, frame_h);
        let right_zz = vertical_zigzag(geometry.axis_break_end_x, frame_y, frame_h);

        // Calculate intersections for strokes
        let top_y_at_left = y_at_x(top_zz.p0, top_zz.p1, frame_x);
        let top_y_at_right = y_at_x(top_zz.p2, top_zz.p3, frame_x + frame_w);
        let bot_y_at_left = y_at_x(bot_zz.p0, bot_zz.p1, frame_x);
        let bot_y_at_right = y_at_x(bot_zz.p2, bot_zz.p3, frame_x + frame_w);
        let left_x_at_top = x_at_y(left_zz.p0, left_zz.p1, frame_y);
        let left_x_at_bottom = x_at_y(left_zz.p2, left_zz.p3, frame_y + frame_h);
        let right_x_at_top = x_at_y(right_zz.p0, right_zz.p1, frame_y);
        let right_x_at_bottom = x_at_y(right_zz.p2, right_zz.p3, frame_y + frame_h);

        // STEP 1: Draw full L-shape frame fill (same as no-break case)
        let frame_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, frame_y,
            frame_x + frame_w, frame_y,
            frame_x + frame_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h,
            frame_x, frame_y + frame_h,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            frame_fill_path, get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');

        // STEP 2: Draw horizontal gap band (white zigzag-shaped ribbon)
        // Traces: top zigzag left-to-right, then bottom zigzag right-to-left
        let h_gap_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            top_zz.p0.0, top_zz.p0.1,
            top_zz.p1.0, top_zz.p1.1,
            top_zz.p2.0, top_zz.p2.1,
            top_zz.p3.0, top_zz.p3.1,
            bot_zz.p3.0, bot_zz.p3.1,
            bot_zz.p2.0, bot_zz.p2.1,
            bot_zz.p1.0, bot_zz.p1.1,
            bot_zz.p0.0, bot_zz.p0.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            h_gap_path, style.background_color
        ));
        svg.push('\n');

        // STEP 3: Draw vertical gap band (white zigzag-shaped ribbon)
        // Traces: left zigzag top-to-bottom, then right zigzag bottom-to-top
        let v_gap_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            left_zz.p0.0, left_zz.p0.1,
            left_zz.p1.0, left_zz.p1.1,
            left_zz.p2.0, left_zz.p2.1,
            left_zz.p3.0, left_zz.p3.1,
            right_zz.p3.0, right_zz.p3.1,
            right_zz.p2.0, right_zz.p2.1,
            right_zz.p1.0, right_zz.p1.1,
            right_zz.p0.0, right_zz.p0.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            v_gap_path, style.background_color
        ));
        svg.push('\n');

        // STEP 4: Draw frame strokes for the 4 visible corner portions
        // Top-left corner stroke
        let tl_stroke = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            frame_x, top_y_at_left,
            frame_x, frame_y,
            left_x_at_top, frame_y,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            tl_stroke, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Top-right corner stroke
        let tr_stroke = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            right_x_at_top, frame_y,
            frame_x + frame_w, frame_y,
            frame_x + frame_w, top_y_at_right,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            tr_stroke, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Bottom-left corner stroke
        let bl_stroke = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            frame_x, bot_y_at_left,
            frame_x, frame_y + frame_h,
            left_x_at_bottom, frame_y + frame_h,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            bl_stroke, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Bottom-right corner stroke (L-shape with rabbet)
        let br_stroke = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            frame_x + frame_w, bot_y_at_right,
            frame_x + frame_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h,
            frame_x + frame_w - rabbet_w, frame_y + frame_h,
            right_x_at_bottom, frame_y + frame_h,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            br_stroke, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // STEP 5: Draw zigzag indicator lines (dashed)
        render_zigzag_line(&mut svg, &top_zz, &style.line_color, break_line_width);
        render_zigzag_line(&mut svg, &bot_zz, &style.line_color, break_line_width);
        render_zigzag_line(&mut svg, &left_zz, &style.line_color, break_line_width);
        render_zigzag_line(&mut svg, &right_zz, &style.line_color, break_line_width);
    } else if geometry.use_axis_break && !geometry.use_axis_break_y {
        // Horizontal break only (no vertical break)
        let break_line_width = style.frame_stroke_width * 0.5;

        let left_zz = vertical_zigzag(geometry.axis_break_start_x, frame_y, frame_h);
        let right_zz = vertical_zigzag(geometry.axis_break_end_x, frame_y, frame_h);

        // Calculate where zigzag crosses frame boundaries
        let left_x_at_top = x_at_y(left_zz.p0, left_zz.p1, frame_y);
        let right_x_at_top = x_at_y(right_zz.p0, right_zz.p1, frame_y);
        let left_x_at_bottom = x_at_y(left_zz.p2, left_zz.p3, frame_y + frame_h);
        let right_x_at_bottom = x_at_y(right_zz.p2, right_zz.p3, frame_y + frame_h);

        // Left portion: fill edge follows zigzag exactly
        let left_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, frame_y,                      // Top-left corner
            left_x_at_top, frame_y,                // Where zigzag crosses top edge
            left_zz.p1.0, left_zz.p1.1,           // First peak
            left_zz.p2.0, left_zz.p2.1,           // Second peak
            left_x_at_bottom, frame_y + frame_h,   // Where zigzag crosses bottom edge
            frame_x, frame_y + frame_h,            // Bottom-left corner
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            left_fill_path, get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');
        // Stroke the 3 non-break edges, connecting to zigzag intersection points
        let left_stroke_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            left_x_at_top, frame_y,                // Start at zigzag/top intersection
            frame_x, frame_y,                      // Top-left corner
            frame_x, frame_y + frame_h,            // Bottom-left corner
            left_x_at_bottom, frame_y + frame_h,   // End at zigzag/bottom intersection
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            left_stroke_path, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Right portion: L-shape with left edge following zigzag exactly
        let right_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            right_x_at_top, frame_y,                            // Where zigzag crosses top edge
            frame_x + frame_w, frame_y,                         // Top-right
            frame_x + frame_w, frame_y + frame_h - rabbet_h,    // Down to lip
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h, // Step left
            frame_x + frame_w - rabbet_w, frame_y + frame_h,    // Down to bottom
            right_x_at_bottom, frame_y + frame_h,               // Where zigzag crosses bottom edge
            right_zz.p2.0, right_zz.p2.1,                      // Second peak - going up
            right_zz.p1.0, right_zz.p1.1,                      // First peak
        );
        svg.push_str(&format!(
            r#"    <path d="{}" fill="{}" stroke="none"/>"#,
            right_fill_path, get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');
        // Stroke the 5 non-break edges
        let right_stroke_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            right_x_at_top, frame_y,                            // Start at zigzag/top intersection
            frame_x + frame_w, frame_y,                         // Top-right
            frame_x + frame_w, frame_y + frame_h - rabbet_h,    // Down to lip
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h, // Step left
            frame_x + frame_w - rabbet_w, frame_y + frame_h,    // Down to bottom
            right_x_at_bottom, frame_y + frame_h,               // End at zigzag/bottom intersection
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            right_stroke_path, style.line_color, style.frame_stroke_width
        ));
        svg.push('\n');

        // Break indicator: dashed zigzag lines
        render_zigzag_line(&mut svg, &left_zz, &style.line_color, break_line_width);
        render_zigzag_line(&mut svg, &right_zz, &style.line_color, break_line_width);
    } else {
        // No axis break - draw full L-shape
        let l_shape_points = format!(
            "{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
            frame_x, frame_y,                                   // Top-left (front of frame)
            frame_x + frame_w, frame_y,                         // Top-right
            frame_x + frame_w, frame_y + frame_h - rabbet_h,    // Down to lip (top of rabbet)
            frame_x + frame_w - rabbet_w, frame_y + frame_h - rabbet_h, // Step left (inner corner)
            frame_x + frame_w - rabbet_w, frame_y + frame_h,    // Down to bottom (back of frame)
            frame_x, frame_y + frame_h,                         // Bottom-left
        );
        svg.push_str(&format!(
            r#"    <polygon points="{}" stroke="{}" stroke-width="{}" fill="{}"/>"#,
            l_shape_points, style.line_color, style.frame_stroke_width,
            get_fill_for_pattern(&style.material_patterns.frame)
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");

    // Dimension callouts for section view
    svg.push_str("  <g id=\"section-dimensions\">\n");

    // Frame depth dimension (left side, vertical)
    let dim_x = frame_x - 30.0;
    let dim_y1 = frame_y;
    let dim_y2 = frame_y + frame_h;

    // Track left extent (extension lines extend to dim_x - EXTENSION_OVERSHOOT)
    track_x!(dim_x - style.extension_line_overshoot);

    // Extension lines
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        frame_x - style.extension_line_gap, dim_y1,
        dim_x - style.extension_line_overshoot, dim_y1,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        frame_x - style.extension_line_gap, dim_y2,
        dim_x - style.extension_line_overshoot, dim_y2,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    // Dimension line (with break symbol if vertical axis break is active)
    // Arrow tips land exactly at geometric boundaries (dim_y1, dim_y2)
    let depth_line_y1 = arrow_line_endpoint_for_target_y(dim_y1, style.dimension_stroke_width, true);
    let depth_line_y2 = arrow_line_endpoint_for_target_y(dim_y2, style.dimension_stroke_width, false);
    
    if geometry.use_axis_break_y {
        // Draw dimension line with break symbol in the middle
        let axis_break_start_y = geometry.axis_break_start_y;
        let axis_break_end_y = geometry.axis_break_end_y;
        let break_center_y = (axis_break_start_y + axis_break_end_y) / 2.0;
        let spark_width = SPARK_VERTICAL_WIDTH;
        let spark_height = SPARK_VERTICAL_HEIGHT;
        
        // Line from top arrow to break
        svg.push_str(&generate_line_with_arrows(
            dim_x, depth_line_y1, dim_x, break_center_y - spark_height / 2.0,
            dim_color, style.dimension_stroke_width,
            true, false, false, // arrow_start only
        ));

        // Spark/zigzag break symbol (vertical orientation)
        svg.push_str(&format!(
            r#"    <path d="M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            dim_x, break_center_y - spark_height / 2.0,
            dim_x + spark_width, break_center_y - spark_height / 4.0,
            dim_x - spark_width, break_center_y + spark_height / 4.0,
            dim_x, break_center_y + spark_height / 2.0,
            dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');

        // Line from break to bottom arrow
        svg.push_str(&generate_line_with_arrows(
            dim_x, break_center_y + spark_height / 2.0, dim_x, depth_line_y2,
            dim_color, style.dimension_stroke_width,
            false, true, false, // arrow_end only
        ));
    } else {
        // Normal continuous dimension line
        svg.push_str(&generate_line_with_arrows(
            dim_x, depth_line_y1, dim_x, depth_line_y2,
            dim_color, style.dimension_stroke_width,
            true, true, false, // both arrows
        ));
    }
    
    // Label - extra offset to avoid crowding arrows
    // When axis break is used, show actual frame depth (not display depth)
    let label_offset = LABEL_BUFFER + style.label_font_size * LABEL_FONT_OFFSET + style.extension_line_gap;
    let depth_label_x = dim_x - label_offset;
    let depth_label_y = (dim_y1 + dim_y2) / 2.0;
    
    // Track left extent - the rotated label extends half its height to the left of its x position
    // (rotated -90 degrees means the text height becomes width, text is anchored at middle)
    track_x!(depth_label_x - style.label_font_size / 2.0);
    
    let depth_value = if geometry.use_axis_break_y {
        geometry.actual_frame_depth
    } else {
        design.frame_material_depth
    };
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">Depth: {}</text>"#,
        depth_label_x, depth_label_y,
        dim_color, style.font_family, style.label_font_size,
        depth_label_x, depth_label_y,
        fmt(depth_value)
    ));
    svg.push('\n');

    // Frame width dimension (horizontal, at top)
    // Always spans from left edge to right edge (full display width)
    // Use same offset as calculated in geometry.rs for consistency
    let fw_y = frame_y - 32.0; // Matches width_line_offset in geometry calculation
    let fw_x1 = frame_x;
    let fw_x2 = frame_x + frame_w;

    // Extension lines
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        fw_x1, frame_y - style.extension_line_gap,
        fw_x1, fw_y - style.extension_line_overshoot,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        fw_x2, frame_y - style.extension_line_gap,
        fw_x2, fw_y - style.extension_line_overshoot,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');

    // Dimension line - with zigzag break symbol if axis break is used
    // Arrow tips land exactly at geometric boundaries (fw_x1, fw_x2)
    let width_line_x1 = arrow_line_endpoint_for_target(fw_x1, style.dimension_stroke_width, true);
    let width_line_x2 = arrow_line_endpoint_for_target(fw_x2, style.dimension_stroke_width, false);
    
    if geometry.use_axis_break {
        // Break symbol parameters
        let break_center = (geometry.axis_break_start_x + geometry.axis_break_end_x) / 2.0;
        let spark_width = SPARK_HORIZONTAL_WIDTH;
        let spark_height = SPARK_HORIZONTAL_HEIGHT;

        // Line from left arrow to break
        svg.push_str(&generate_line_with_arrows(
            width_line_x1, fw_y, break_center - spark_width / 2.0, fw_y,
            dim_color, style.dimension_stroke_width,
            true, false, false, // arrow_start only
        ));

        // Spark/zigzag symbol (same shape as frame break, just smaller)
        let spark_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            break_center - spark_width / 2.0, fw_y,
            break_center - spark_width / 6.0, fw_y - spark_height,
            break_center + spark_width / 6.0, fw_y + spark_height,
            break_center + spark_width / 2.0, fw_y,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-linejoin="round"/>"#,
            spark_path, dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');

        // Line from break to right arrow
        svg.push_str(&generate_line_with_arrows(
            break_center + spark_width / 2.0, fw_y, width_line_x2, fw_y,
            dim_color, style.dimension_stroke_width,
            false, true, false, // arrow_end only
        ));
    } else {
        // No break - single continuous line
        svg.push_str(&generate_line_with_arrows(
            width_line_x1, fw_y, width_line_x2, fw_y,
            dim_color, style.dimension_stroke_width,
            true, true, false, // both arrows
        ));
    }

    let fw_label_y = fw_y - label_offset;

    // Track width label Y bounds (text baseline is at fw_label_y, extends above and below)
    track_y!(fw_label_y - style.label_font_size * 0.8); // Above baseline (most of glyph height)
    track_y!(fw_label_y + style.label_font_size * 0.2); // Below baseline (descenders)

    // Show actual frame width (not display width) in label
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle">Width: {}</text>"#,
        (fw_x1 + fw_x2) / 2.0, fw_label_y,
        dim_color, style.font_family, style.label_font_size,
        fmt(geometry.actual_frame_width)
    ));
    svg.push('\n');

    // Rabbet dimension indicator - crosshair showing rabbet width and depth
    // Uses TRUE GEOMETRIC boundaries (following technical drawing convention where
    // lines represent edges, not physical objects with stroke thickness)
    //
    // The rabbet_area represents the actual rabbet cutout area
    let rabbet_center_x = geometry.rabbet_area.x + rabbet_w / 2.0;
    let rabbet_center_y = geometry.rabbet_area.y + rabbet_h / 2.0;

    // Add semi-transparent background behind rabbet indicator for legibility
    svg.push_str(&format!(
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="{}" stroke="none" rx="1"/>"#,
        geometry.rabbet_area.x,
        geometry.rabbet_area.y,
        geometry.rabbet_area.width,
        geometry.rabbet_area.height,
        style.background_color,
        OPACITY_RABBET_BACKGROUND
    ));
    svg.push('\n');
    
    // Arrow stroke width for calculating tip positions
    let rabbet_arrow_stroke = style.dimension_stroke_width * 0.7;
    
    // Horizontal arrow: spans the rabbet width (true geometric boundaries)
    let h_target_left = geometry.rabbet_area.x;
    let h_target_right = geometry.rabbet_area.x + geometry.rabbet_area.width;
    let h_line_x1 = arrow_line_endpoint_for_target(h_target_left, rabbet_arrow_stroke, true);
    let h_line_x2 = arrow_line_endpoint_for_target(h_target_right, rabbet_arrow_stroke, false);
    
    svg.push_str(&generate_line_with_arrows(
        h_line_x1, rabbet_center_y,
        h_line_x2, rabbet_center_y,
        dim_color, rabbet_arrow_stroke,
        true, true, false, // both arrows
    ));

    // Vertical arrow: spans the rabbet depth (true geometric boundaries)
    let v_target_top = geometry.rabbet_area.y;
    let v_target_bottom = geometry.rabbet_area.y + geometry.rabbet_area.height;
    let v_line_y1 = arrow_line_endpoint_for_target_y(v_target_top, rabbet_arrow_stroke, true);
    let v_line_y2 = arrow_line_endpoint_for_target_y(v_target_bottom, rabbet_arrow_stroke, false);

    svg.push_str(&generate_line_with_arrows(
        rabbet_center_x, v_line_y1,
        rabbet_center_x, v_line_y2,
        dim_color, rabbet_arrow_stroke,
        true, true, false, // both arrows
    ));

    // Material thickness labels with dog-leg leader lines
    // Labels positioned to the right in a vertical column
    // Minimal spacing for maximum diagram space
    let base_offset = 18.0_f64.min(geometry.scale * 0.4 + 12.0); // Reduced from 35.0
    let label_base_x = geometry.glazing.right() + base_offset;
    let label_spacing = style.label_font_size * 1.6; // Scale with font size (screen: ~21px, PDF: ~38px)

    // Materials are drawn at true geometric positions - labels point to actual centers

    struct MaterialLabel<'a> {
        name: &'a str,
        center_y: f64,
        right_edge: f64,
        thickness: f64,
    }

    let mut materials: Vec<MaterialLabel> = Vec::new();

    materials.push(MaterialLabel {
        name: "Glazing",
        center_y: geometry.glazing.y + geometry.glazing.height / 2.0,
        right_edge: geometry.glazing.right(),
        thickness: design.glazing_thickness,
    });

    if let Some(mat) = &geometry.matboard {
        materials.push(MaterialLabel {
            name: "Mat",
            center_y: mat.y + mat.height / 2.0,
            right_edge: mat.right(),
            thickness: design.matboard_thickness,
        });
    }

    materials.push(MaterialLabel {
        name: "Artwork",
        center_y: geometry.artwork.y + geometry.artwork.height / 2.0,
        right_edge: geometry.artwork.right(),
        thickness: design.artwork_thickness,
    });

    materials.push(MaterialLabel {
        name: "Backing",
        center_y: geometry.backing.y + geometry.backing.height / 2.0,
        right_edge: geometry.backing.right(),
        thickness: design.backing_thickness,
    });

    // Only add assembly margin label if it has meaningful height
    if geometry.assembly_margin.height > 0.5 {
        materials.push(MaterialLabel {
            name: "Margin",
            center_y: geometry.assembly_margin.y + geometry.assembly_margin.height / 2.0,
            right_edge: geometry.assembly_margin.right(),
            thickness: design.assembly_margin,
        });
    }

    // Calculate evenly spaced label Y positions
    // Center the label group at the midpoint of the entire material stack
    let stack_top = geometry.glazing.y;
    let stack_bottom = if geometry.assembly_margin.height > 0.5 {
        geometry.assembly_margin.y + geometry.assembly_margin.height
    } else {
        geometry.backing.y + geometry.backing.height
    };
    let stack_center = stack_top + (stack_bottom - stack_top) / 2.0;

    let total_label_height = (materials.len() - 1) as f64 * label_spacing;
    let first_label_y = stack_center - total_label_height / 2.0;

    for (i, mat) in materials.iter().enumerate() {
        let label_y = first_label_y + i as f64 * label_spacing;

        // Dog-leg leader line:
        // 1. Horizontal from material edge (shorter for compact layout)
        let horiz_length = LEADER_LINE_LENGTH;
        let horiz_end_x = mat.right_edge + horiz_length;

        svg.push_str(&generate_line_with_arrows(
            mat.right_edge + 3.0, mat.center_y,
            horiz_end_x, mat.center_y,
            dim_color, style.extension_stroke_width * LEADER_STROKE_RATIO,
            true, false, true, // arrow_start only, is_leader
        ));

        // 2. Angled segment to label position
        // Use label_y directly - dominant-baseline="central" centers text at this position
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            horiz_end_x, mat.center_y,
            label_base_x - 5.0, label_y,
            dim_color, style.extension_stroke_width * LEADER_STROKE_RATIO
        ));
        svg.push('\n');

        // Label text - use label_font_size for material identification labels
        // Position text so baseline is slightly below label_y (visual center)
        // This makes dog-leg line hit visual center regardless of baseline rendering
        let text_y = label_y + style.label_font_size * 0.35;
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}">{}: {}</text>"#,
            label_base_x, text_y,
            dim_color, style.font_family, style.label_font_size,
            mat.name, fmt(mat.thickness)
        ));
        svg.push('\n');
    }

    // Total stack height dimension - vertical, positioned well to the right of labels
    let stack_top = geometry.glazing.y;
    let stack_bottom = geometry.backing.y + geometry.backing.height;

    // Estimate max label width by checking each label
    let max_label_width = materials.iter()
        .map(|m| {
            let label = format!("{}: {}", m.name, fmt(m.thickness));
            estimate_text_width(&label, style.label_font_size * 0.85)
        })
        .fold(0.0_f64, |a, b| a.max(b));

    // Position stack dimension with clearance from labels (reduced for compact layout)
    let stack_dim_x = label_base_x + max_label_width + 20.0;

    // Find total stack callout
    let total_stack = callouts.iter().find(|c| c.dimension_type == super::types::DimensionType::TotalStackHeight);
    if let Some(callout) = total_stack {
        // Extension lines - start from after label area
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            label_base_x + max_label_width + 10.0, stack_top,
            stack_dim_x + style.extension_line_overshoot, stack_top,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            label_base_x + max_label_width + 10.0, stack_bottom,
            stack_dim_x + style.extension_line_overshoot, stack_bottom,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        // Dimension line - arrow tips land exactly at stack boundaries
        let stack_line_y1 = arrow_line_endpoint_for_target_y(stack_top, style.dimension_stroke_width, true);
        let stack_line_y2 = arrow_line_endpoint_for_target_y(stack_bottom, style.dimension_stroke_width, false);
        svg.push_str(&generate_line_with_arrows(
            stack_dim_x, stack_line_y1, stack_dim_x, stack_line_y2,
            dim_color, style.dimension_stroke_width,
            true, true, false, // both arrows
        ));
        // Label - rotated vertically with more offset
        let stack_label_x = stack_dim_x + label_offset + 4.0;
        let stack_label_y = (stack_top + stack_bottom) / 2.0;
        
        // Track right extent - the rotated label extends half its height to the right of its x position
        track_x!(stack_label_x + style.label_font_size / 2.0);

        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">{}</text>"#,
            stack_label_x, stack_label_y,
            dim_color, style.font_family, style.label_font_size,
            stack_label_x, stack_label_y,
            callout.label.clone()
        ));
        svg.push('\n');
    }

    // Rabbet label - below the frame with leader from rabbet area
    let rabbet_label_y = frame_y + frame_h + 18.0;
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-dasharray="{}"/>"#,
        rabbet_center_x, geometry.rabbet_area.y + rabbet_h + 2.0,
        rabbet_center_x, rabbet_label_y - 6.0,
        dim_color, style.extension_stroke_width, DASH_CLEARANCE
    ));
    svg.push('\n');

    // Clearance/interference indicator
    let indicator_color = if geometry.has_interference() {
        &style.warning_color
    } else {
        &style.success_color
    };

    // Format rabbet dimensions - show both if different (non-square rabbet)
    let rabbet_label = if (design.rabbet_width - design.rabbet_depth).abs() < 0.001 {
        // Square rabbet - just show one value
        format!("Rabbet: {}", fmt(design.rabbet_depth))
    } else {
        // Non-square rabbet - show width × depth
        format!("Rabbet: {} × {}", fmt(design.rabbet_width), fmt(design.rabbet_depth))
    };

    // Clearance/interference text on separate line to avoid overlap with material labels
    let clearance_line = if geometry.has_interference() {
        format!("(INTERFERENCE: {})", fmt(-geometry.clearance))
    } else {
        format!("(clearance: {})", fmt(geometry.clearance))
    };

    // Estimate text width of rabbet label to prevent clipping at left edge
    let estimated_text_width = estimate_text_width(&rabbet_label, style.label_font_size * 0.85);
    let min_x_for_centering = estimated_text_width / 2.0 + 5.0; // 5px margin from edge

    let (text_x, text_anchor) = if rabbet_center_x >= min_x_for_centering {
        (rabbet_center_x, "middle")
    } else {
        (5.0, "start") // Left-align with small margin if centering would clip
    };

    // Line spacing for two-line label
    let line_height = style.label_font_size * 1.2;

    // Rabbet dimensions on first line
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="{}">{}</text>"#,
        text_x, rabbet_label_y,
        indicator_color, style.font_family, style.label_font_size,
        text_anchor, rabbet_label
    ));
    svg.push('\n');

    // Clearance/interference on second line
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="{}">{}</text>"#,
        text_x, rabbet_label_y + line_height,
        indicator_color, style.font_family, style.label_font_size,
        text_anchor, clearance_line
    ));
    svg.push('\n');

    svg.push_str("  </g>\n");

    // =================================================================
    // DYNAMIC VIEWBOX: Calculate legend bounds
    // =================================================================
    // Calculate legend width based on actual text lengths (same logic as generate_section_legend)
    let material_names = if design.has_mat() {
        vec!["Frame", "Glazing", "Matboard", "Artwork", "Backing"]
    } else {
        vec!["Frame", "Glazing", "Artwork", "Backing"]
    };

    let mut item_widths: Vec<f64> = material_names.iter().map(|name| {
        let text_width = name.len() as f64 * style.label_font_size * LEGEND_CHAR_WIDTH_RATIO;
        LEGEND_SWATCH_SIZE + LEGEND_SWATCH_GAP + text_width + LEGEND_ITEM_GAP
    }).collect();

    if let Some(last_width) = item_widths.last_mut() {
        *last_width -= LEGEND_ITEM_GAP;
    }

    let total_width: f64 = item_widths.iter().sum();
    let legend_start_x = (options.canvas_width - total_width) / 2.0;
    let legend_end_x = legend_start_x + total_width;

    let content_bottom = geometry.bounds.bottom();
    let legend_gap = style.label_font_size * 0.4;  // Reduced to recover space for title
    let legend_y = content_bottom + legend_gap;
    let legend_bottom = legend_y + style.label_font_size * 1.2;

    // =================================================================
    // SELF-CENTERING: Calculate horizontal centering
    // =================================================================
    // Calculate the horizontal offset needed to center the actual content
    // within the canvas. This dynamically adapts to content variations.
    let content_center_x = (content_min_x + content_max_x) / 2.0;
    let canvas_center_x = options.canvas_width / 2.0;
    let center_offset_x = canvas_center_x - content_center_x;

    // Calculate final actual bounds of the content after transform
    let shifted_content_min_x = content_min_x + center_offset_x;
    let shifted_content_max_x = content_max_x + center_offset_x;

    // Calculate final bounds including legend
    let mut min_x = shifted_content_min_x.min(legend_start_x);
    let mut max_x = shifted_content_max_x.max(legend_end_x);
    let mut min_y = content_min_y;
    let mut max_y = content_max_y.max(legend_bottom);

    // Add padding
    let padding = style.margin;
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    // Calculate viewBox dimensions
    let viewbox_width = max_x - min_x;
    let viewbox_height = max_y - min_y;

    // Replace fixed viewBox with dynamic one
    if let Some(viewbox_start) = svg.find("viewBox=\"") {
        if let Some(quote_offset) = svg[viewbox_start..].find('"') {
            let viewbox_end = viewbox_start + quote_offset + 1;

            // Find closing quote of the attribute
            if let Some(closing_quote_offset) = svg[viewbox_end..].find('"') {
                let after_viewbox = viewbox_end + closing_quote_offset;

                // Build new SVG with dynamic viewBox
                let mut svg_with_dynamic_viewbox = String::new();
                svg_with_dynamic_viewbox.push_str(&svg[..viewbox_start]);
                svg_with_dynamic_viewbox.push_str(&format!(
                    "viewBox=\"{:.2} {:.2} {:.2} {:.2}\"",
                    min_x, min_y, viewbox_width, viewbox_height
                ));

                // Skip the original closing quote since we added our own
                if after_viewbox + 1 < svg.len() {
                    svg_with_dynamic_viewbox.push_str(&svg[after_viewbox + 1..]);
                }
                svg = svg_with_dynamic_viewbox;
            }
        }
    }

    // Build final SVG with centering transform wrapper
    let mut final_svg = String::new();
    
    // Copy everything up to the start of the geometry group
    // We use "section-geometry" as the reliable anchor point since background rect is not present
    if let Some(geom_start) = svg.find("<g id=\"section-geometry\">") {
        final_svg.push_str(&svg[..geom_start]);
        
        // Add centering transform wrapper around the content
        final_svg.push_str(&format!(
            "  <g id=\"section-content\" transform=\"translate({:.2}, 0)\">\n",
            center_offset_x
        ));
        
        // Add the rest of the content (geometry and dimensions groups)
        final_svg.push_str(&svg[geom_start..]);
        
        // Close the centering wrapper
        final_svg.push_str("  </g>\n");
    } else {
        // Fallback: use original SVG if we can't find the expected structure
        final_svg = svg;
    }

    // Compact legend (horizontal at very bottom of canvas)
    // Pass content bounds for dynamic viewBox centering
    final_svg.push_str(&generate_section_legend(
        design,
        geometry,
        style,
        options.canvas_width,
        options.canvas_height,
        Some((shifted_content_min_x, shifted_content_max_x)), // Use shifted bounds for legend centering
    ));

    final_svg.push_str("</svg>");
    
    final_svg
}


/// Generate SVG defs (patterns, markers, etc.)
fn generate_defs(_style: &DiagramStyle) -> String {
    let mut defs = String::new();
    defs.push_str("  <defs>\n");

    // Hatching pattern for artwork (single diagonal)
    defs.push_str("    <pattern id=\"hatch\" patternUnits=\"userSpaceOnUse\" width=\"4\" height=\"4\">");
    defs.push_str("<path d=\"M-1,1 l2,-2 M0,4 l4,-4 M3,5 l2,-2\" stroke=\"#CCCCCC\" stroke-width=\"0.5\"/>");
    defs.push_str("</pattern>\n");

    // Note: Arrow markers removed - now using inline polygon elements for arrowheads
    // This ensures compatibility with svg2pdf.js which doesn't support SVG markers.
    // See generate_arrow_polygon() and generate_line_with_arrows() functions above.

    defs.push_str("  </defs>\n");
    defs
}
/// Generate SVG for a rectangle
fn svg_rect(rect: &Rect, stroke: &str, stroke_width: f64, fill: Option<&str>) -> String {
    let fill_str = fill.unwrap_or("none");
    format!(
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" stroke="{}" stroke-width="{}" fill="{}"/>"#,
        rect.x, rect.y, rect.width, rect.height, stroke, stroke_width, fill_str
    ) + "\n"
}

/// Generate SVG for a dimension callout
/// Visual grammar:
/// - Extension lines start from geometry with small gap, extend past dimension line
/// - Dimension line shortened so arrow tips end before extension lines
/// - Labels positioned with buffer from dimension line
fn svg_dimension(callout: &PositionedCallout, style: &DiagramStyle, geometry: &PlanViewGeometry) -> String {
    let mut svg = String::new();

    // Determine color based on dimension type
    use super::types::DimensionType;
    let dim_color = match callout.callout.dimension_type {
        DimensionType::FrameInsideWidthInterior
        | DimensionType::FrameInsideHeightInterior => &style.inside_dimension_color,
        DimensionType::FrameOutsideWidth
        | DimensionType::FrameOutsideHeight => &style.outside_dimension_color,
        DimensionType::MatCutWidth
        | DimensionType::MatCutHeight
        | DimensionType::MatOpeningWidth
        | DimensionType::MatOpeningHeight
        | DimensionType::MatVisibleWidth
        | DimensionType::MatVisibleHeight => &style.mat_dimension_color,
        DimensionType::ArtworkWidth
        | DimensionType::ArtworkHeight => &style.artwork_dimension_color,
        _ => &style.dimension_color,
    };

    svg.push_str(&format!(r#"    <g class="dimension">"#));
    svg.push('\n');

    // Determine if horizontal or vertical
    let is_horizontal = callout.actual_side.is_horizontal();

    if is_horizontal {
        // For horizontal dimensions (Top/Bottom sides):
        // - Extension lines are vertical
        // - Dimension line is horizontal between them

        let geom_y = callout.callout.extent_start.y; // Y position at geometry
        let dim_y = callout.dimension_line_position; // Y position of dimension line

        // Determine direction (Top = lines go up, Bottom = lines go down)
        let going_up = callout.actual_side == Side::Top;

        // Extension line endpoints:
        // Start: small gap from geometry
        // End: past the dimension line by EXTENSION_OVERSHOOT
        let ext_start_y = if going_up { geom_y - style.extension_line_gap } else { geom_y + style.extension_line_gap };
        let ext_end_y = if going_up { dim_y - style.extension_line_overshoot } else { dim_y + style.extension_line_overshoot };

        // Extension lines - special case for MatCutWidth: both lines extend to same y-value
        // at the mat opening's bottom edge (with small offset)
        let (mat_cut_ext_start_y, mat_cut_ext_end_y) = if callout.callout.dimension_type == crate::visualization::DimensionType::MatCutWidth {
            // Use the actual mat opening bottom coordinate
            if let Some(mat_opening) = &geometry.mat_opening {
                let target_y = mat_opening.bottom() + 3.0; // Small offset below mat opening bottom
                (target_y, ext_end_y)
            } else {
                (ext_start_y, ext_end_y)
            }
        } else {
            (ext_start_y, ext_end_y)
        };

        // Left extension line
        svg.push_str(&format!(
            r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            callout.callout.extent_start.x, mat_cut_ext_start_y,
            callout.callout.extent_start.x, mat_cut_ext_end_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Right extension line (uses same y-values as left for MatCutWidth)
        svg.push_str(&format!(
            r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            callout.callout.extent_end.x, mat_cut_ext_start_y,
            callout.callout.extent_end.x, mat_cut_ext_end_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dimension line with arrows - tips land exactly at extent boundaries
        // When space is too tight for inward-pointing arrows, flip to outward-pointing
        let extent_span = (callout.callout.extent_end.x - callout.callout.extent_start.x).abs();
        let arrow_tip_size = arrow_geometry::tip_extension(style.dimension_stroke_width);
        let tight_space = extent_span < arrow_tip_size * 3.0;

        let line_x1 = arrow_line_endpoint_for_target(callout.callout.extent_start.x, style.dimension_stroke_width, true);
        let line_x2 = arrow_line_endpoint_for_target(callout.callout.extent_end.x, style.dimension_stroke_width, false);
        if style.use_tick_marks {
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                line_x1, dim_y,
                line_x2, dim_y,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        } else if tight_space {
            // Outward-pointing arrows: short stubs extending outward from extension lines
            let stub_len = arrow_tip_size * 2.5;
            // Left arrow: points inward (right) from outside-left
            let left_stub_start = callout.callout.extent_start.x - stub_len;
            let left_stub_end = arrow_line_endpoint_for_target(callout.callout.extent_start.x, style.dimension_stroke_width, false);
            let arrow_svg = generate_line_with_arrows(
                left_stub_start, dim_y, left_stub_end, dim_y,
                dim_color, style.dimension_stroke_width,
                false, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
            // Right arrow: points inward (left) from outside-right
            let right_stub_start = callout.callout.extent_end.x + stub_len;
            let right_stub_end = arrow_line_endpoint_for_target(callout.callout.extent_end.x, style.dimension_stroke_width, true);
            let arrow_svg = generate_line_with_arrows(
                right_stub_start, dim_y, right_stub_end, dim_y,
                dim_color, style.dimension_stroke_width,
                false, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
        } else {
            // Normal inward-pointing arrows
            let arrow_svg = generate_line_with_arrows(
                line_x1, dim_y,
                line_x2, dim_y,
                dim_color, style.dimension_stroke_width,
                true, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
        }

        // Tick marks (only if not using arrows)
        if style.use_tick_marks {
            let tick_half = style.tick_size / 2.0;
            // Left tick (angled)
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                callout.callout.extent_start.x - tick_half, dim_y - tick_half,
                callout.callout.extent_start.x + tick_half, dim_y + tick_half,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
            // Right tick
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                callout.callout.extent_end.x - tick_half, dim_y - tick_half,
                callout.callout.extent_end.x + tick_half, dim_y + tick_half,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        }
    } else {
        // For vertical dimensions (Left/Right sides):
        // - Extension lines are horizontal
        // - Dimension line is vertical between them

        let geom_x = callout.callout.extent_start.x; // X position at geometry
        let dim_x = callout.dimension_line_position; // X position of dimension line

        // Determine direction (Right = lines go right, Left = lines go left)
        let going_right = callout.actual_side == Side::Right;

        // Extension line endpoints:
        // Start: small gap from geometry
        // End: past the dimension line by EXTENSION_OVERSHOOT
        let ext_start_x = if going_right { geom_x + style.extension_line_gap } else { geom_x - style.extension_line_gap };
        let ext_end_x = if going_right { dim_x + style.extension_line_overshoot } else { dim_x - style.extension_line_overshoot };

        // Top extension line
        svg.push_str(&format!(
            r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_start_x, callout.callout.extent_start.y,
            ext_end_x, callout.callout.extent_start.y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Bottom extension line
        svg.push_str(&format!(
            r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_start_x, callout.callout.extent_end.y,
            ext_end_x, callout.callout.extent_end.y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dimension line with arrows - tips land exactly at extent boundaries
        // When space is too tight for inward-pointing arrows, flip to outward-pointing
        let extent_span_v = (callout.callout.extent_end.y - callout.callout.extent_start.y).abs();
        let arrow_tip_size_v = arrow_geometry::tip_extension(style.dimension_stroke_width);
        let tight_space_v = extent_span_v < arrow_tip_size_v * 3.0;

        let line_y1 = arrow_line_endpoint_for_target_y(callout.callout.extent_start.y, style.dimension_stroke_width, true);
        let line_y2 = arrow_line_endpoint_for_target_y(callout.callout.extent_end.y, style.dimension_stroke_width, false);
        if style.use_tick_marks {
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                dim_x, line_y1,
                dim_x, line_y2,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        } else if tight_space_v {
            // Outward-pointing arrows: short stubs extending outward from extension lines
            let stub_len = arrow_tip_size_v * 2.5;
            // Top arrow: points inward (down) from outside-top
            let top_stub_start = callout.callout.extent_start.y - stub_len;
            let top_stub_end = arrow_line_endpoint_for_target_y(callout.callout.extent_start.y, style.dimension_stroke_width, false);
            let arrow_svg = generate_line_with_arrows(
                dim_x, top_stub_start, dim_x, top_stub_end,
                dim_color, style.dimension_stroke_width,
                false, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
            // Bottom arrow: points inward (up) from outside-bottom
            let bot_stub_start = callout.callout.extent_end.y + stub_len;
            let bot_stub_end = arrow_line_endpoint_for_target_y(callout.callout.extent_end.y, style.dimension_stroke_width, true);
            let arrow_svg = generate_line_with_arrows(
                dim_x, bot_stub_start, dim_x, bot_stub_end,
                dim_color, style.dimension_stroke_width,
                false, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
        } else {
            // Normal inward-pointing arrows
            let arrow_svg = generate_line_with_arrows(
                dim_x, line_y1,
                dim_x, line_y2,
                dim_color, style.dimension_stroke_width,
                true, true, false,
            );
            for line in arrow_svg.lines() {
                svg.push_str("    ");
                svg.push_str(line);
                svg.push('\n');
            }
        }

        // Tick marks (only if not using arrows)
        if style.use_tick_marks {
            let tick_half = style.tick_size / 2.0;
            // Top tick
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                dim_x - tick_half, callout.callout.extent_start.y - tick_half,
                dim_x + tick_half, callout.callout.extent_start.y + tick_half,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
            // Bottom tick
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                dim_x - tick_half, callout.callout.extent_end.y - tick_half,
                dim_x + tick_half, callout.callout.extent_end.y + tick_half,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        }
    }

    // Label - centered directly ON the dimension line with masking
    // This creates a compact layout: |<--- Label --->|

    // Estimate label dimensions for masking
    let label_text_width = estimate_text_width(&callout.callout.label, style.label_font_size);
    let mask_padding_x = LABEL_MASK_PADDING_X;
    let mask_padding_y = LABEL_MASK_PADDING_Y;
    let mask_width = label_text_width + mask_padding_x * 2.0;
    let mask_height = style.label_font_size + mask_padding_y * 2.0;

    let (label_x, label_y, offset_applied) = if is_horizontal {
        // Horizontal dimension: label centered on the dimension line
        let mid_x = (callout.callout.extent_start.x + callout.callout.extent_end.x) / 2.0;
        let base_y = callout.dimension_line_position;

        // Mat cut width labels need extra padding from extension lines
        // Calculate offset based on scaled properties (automatically adapts to combined vs inline view)
        let mat_cut_offset = style.extension_line_overshoot + style.label_font_size / 2.0 + style.dimension_offset_base;
        let (label_y, offset) = if callout.callout.dimension_type == crate::visualization::DimensionType::MatCutWidth {
            (base_y + mat_cut_offset, true)
        } else {
            (base_y, false)
        };

        (mid_x, label_y, offset)
    } else {
        // Vertical dimension: label centered on the dimension line (will be rotated)
        let mid_y = (callout.callout.extent_start.y + callout.callout.extent_end.y) / 2.0;
        let base_x = callout.dimension_line_position;

        // Mat cut height labels need extra padding from extension lines
        // Calculate offset based on scaled properties (automatically adapts to combined vs inline view)
        let mat_cut_offset = style.extension_line_overshoot + style.label_font_size / 2.0 + style.dimension_offset_base;
        let (label_x, offset) = if callout.callout.dimension_type == super::types::DimensionType::MatCutHeight {
            (base_x - mat_cut_offset, true)
        } else {
            (base_x, false)
        };

        (label_x, mid_y, offset)
    };

    // Debug: Add SVG comment showing dimension type and whether offset was applied
    svg.push_str(&format!(
        "      <!-- Dimension type: {:?}, offset applied: {} -->\n",
        callout.callout.dimension_type, offset_applied
    ));

    // For vertical dimensions, rotate text 90° (reads bottom-to-top)
    let transform = if !is_horizontal {
        format!(r#" transform="rotate(90 {:.2} {:.2})""#, label_x, label_y)
    } else {
        String::new()
    };

    // Mask rectangle - draw before text to create visual break in the dimension line
    // For vertical dimensions, swap width/height since the mask rotates with the text
    let (mask_w, mask_h) = if is_horizontal {
        (mask_width, mask_height)
    } else {
        (mask_height, mask_width)  // Swapped for vertical orientation
    };
    svg.push_str(&format!(
        r#"      <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
        label_x - mask_w / 2.0, label_y - mask_h / 2.0,
        mask_w, mask_h,
        style.background_color
    ));
    svg.push('\n');

    // Text label centered on the dimension line
    // Use dominant-baseline for better vertical centering
    svg.push_str(&format!(
        r#"      <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" dominant-baseline="central"{}>{}</text>"#,
        label_x, label_y,
        dim_color, style.font_family, style.label_font_size,
        transform,
        html_escape(&callout.callout.label)
    ));
    svg.push('\n');

    svg.push_str("    </g>\n");
    svg
}

/// Generate section view legend (horizontal layout positioned below content)
fn generate_section_legend(
    design: &FrameDesign,
    geometry: &SectionViewGeometry,
    style: &DiagramStyle,
    canvas_width: f64,
    _canvas_height: f64,
    content_bounds_x: Option<(f64, f64)>, // (min_x, max_x) for dynamic viewBox centering
) -> String {
    let mut svg = String::new();
    svg.push_str("  <g id=\"legend\">\n");

    let materials: Vec<(&str, &FillPattern)> = vec![
        ("Frame", &style.material_patterns.frame),
        ("Glazing", &style.material_patterns.glazing),
        ("Matboard", &style.material_patterns.matboard),
        ("Artwork", &style.material_patterns.artwork),
        ("Backing", &style.material_patterns.backing),
    ].into_iter()
        .filter(|(name, _)| *name != "Matboard" || design.has_mat())
        .collect();

    let mut item_widths: Vec<f64> = materials.iter().map(|(name, _)| {
        let text_width = name.len() as f64 * style.label_font_size * LEGEND_CHAR_WIDTH_RATIO;
        LEGEND_SWATCH_SIZE + LEGEND_SWATCH_GAP + text_width + LEGEND_ITEM_GAP
    }).collect();

    // Don't add inter-item gap after the last item
    if let Some(last_width) = item_widths.last_mut() {
        *last_width -= LEGEND_ITEM_GAP;
    }

    let total_width: f64 = item_widths.iter().sum();

    // Center legend relative to content bounds (for dynamic viewBox) or canvas (for fixed viewBox)
    let start_x = if let Some((min_x, max_x)) = content_bounds_x {
        let content_center = (min_x + max_x) / 2.0;
        content_center - total_width / 2.0
    } else {
        (canvas_width - total_width) / 2.0
    };

    // Position legend tightly below the content bounds
    let content_bottom = geometry.bounds.bottom();
    let legend_gap = style.label_font_size * 0.4;  // Reduced to recover space for title  // Scale with label font size
    let legend_y = content_bottom + legend_gap;

    let mut current_x = start_x;
    for ((name, pattern), item_width) in materials.iter().zip(item_widths.iter()) {
        let fill = get_fill_for_pattern(pattern);
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
            current_x, legend_y - 10.0, LEGEND_SWATCH_SIZE, LEGEND_SWATCH_SIZE, fill, style.line_color, LEGEND_SWATCH_STROKE
        ));
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}">{}</text>"#,
            current_x + LEGEND_SWATCH_SIZE + LEGEND_SWATCH_GAP, legend_y, style.dimension_color, style.font_family, style.label_font_size, name
        ));
        svg.push('\n');
        current_x += item_width;
    }

    svg.push_str("  </g>\n");
    svg
}

/// Generate title block
fn generate_title_block(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();
    svg.push_str("  <g id=\"title-block\">\n");

    let title = options.title_text
        .as_ref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| html_escape(t.trim()))
        .unwrap_or_else(|| "Frame Design".to_string());

    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="30" fill="{}" font-family="{}" font-size="{}" font-weight="bold" text-anchor="middle">{}</text>"#,
        options.canvas_width / 2.0, style.line_color, style.font_family, style.title_font_size, title
    ));
    svg.push('\n');

    // Only show subtitle (dimensions) when using default title
    let has_custom_title = options.title_text
        .as_ref()
        .map_or(false, |t| !t.trim().is_empty());
    if !has_custom_title {
        let (outside_h, outside_w) = design.get_frame_outside_dimensions();
        let subtitle = format!(
            "{:.2}\" × {:.2}\" outside",
            outside_h, outside_w
        );
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="70" fill="{}" font-family="{}" font-size="{}" text-anchor="middle">{}</text>"#,
            options.canvas_width / 2.0, style.dimension_color, style.font_family, style.label_font_size, subtitle
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
    svg
}

/// Get fill color for a pattern
fn get_fill_for_pattern(pattern: &FillPattern) -> String {
    match pattern {
        FillPattern::Solid(color) => color.clone(),
        FillPattern::Hatched { color, .. } => color.clone(),
        FillPattern::CrossHatched { color, .. } => color.clone(),
    }
}

/// Extract viewBox dimensions from SVG string
/// Returns (x, y, width, height) if found
fn extract_viewbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
    if let Some(start) = svg.find("viewBox=\"") {
        let values_start = start + 9; // Length of "viewBox=\""
        if let Some(end) = svg[values_start..].find('"') {
            let viewbox_str = &svg[values_start..values_start + end];
            let parts: Vec<&str> = viewbox_str.split_whitespace().collect();
            if parts.len() == 4 {
                if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    return Some((x, y, w, h));
                }
            }
        }
    }
    None
}

/// Extract content from SVG (between opening and closing tags), preserving defs
fn extract_svg_content(svg: &str) -> String {
    if let Some(start) = svg.find('>') {
        if let Some(end) = svg.rfind("</svg>") {
            let content = &svg[start + 1..end];
            // Remove background rectangle to avoid overlay issues in combined view
            // Background rect pattern: <rect fill="..." width="100%" height="100%"/>
            // IMPORTANT: Preserve <defs> section which contains arrow markers
            if let Some(bg_start) = content.find("<rect fill=") {
                if let Some(bg_end) = content[bg_start..].find("/>") {
                    // Extract parts: before rect (includes defs), and after rect
                    let before_rect = &content[..bg_start];
                    let after_rect = &content[bg_start + bg_end + 2..].trim_start();
                    return format!("{}{}", before_rect, after_rect);
                }
            }
            return content.to_string();
        }
    }
    svg.to_string()
}

/// HTML-escape special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_design() -> FrameDesign {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 1.0;
        design
    }

    #[test]
    fn test_generate_plan_view() {
        let design = test_design();
        let options = DiagramOptions::default();

        let result = generate_diagram(&design, &options);

        assert!(result.svg.contains("<svg"));
        assert!(result.svg.contains("</svg>"));
        assert!(result.svg.contains("geometry"));
        assert!(result.svg.contains("dimensions"));
    }

    #[test]
    fn test_generate_section_view() {
        let design = test_design();
        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);

        assert!(result.svg.contains("<svg"));
        assert!(result.svg.contains("section-geometry"));
    }

    #[test]
    fn test_generate_combined_view() {
        let design = test_design();
        let options = DiagramOptions {
            view: ViewOption::Both,
            include_title_block: true,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);

        assert!(result.svg.contains("plan-view"));
        assert!(result.svg.contains("section-view"));
        assert!(result.svg.contains("title-block"));
    }

    #[test]
    fn test_svg_contains_dimensions() {
        let design = test_design();
        let options = DiagramOptions::default();

        let result = generate_diagram(&design, &options);

        // Should contain dimension labels
        assert!(result.svg.contains("<text"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("12 3/4\""), "12 3/4&quot;");
        assert_eq!(html_escape("<test>"), "&lt;test&gt;");
    }

    #[test]
    fn test_no_mat_svg() {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let options = DiagramOptions::default();
        let result = generate_diagram(&design, &options);

        // Should still generate valid SVG
        assert!(result.svg.contains("<svg"));
        assert!(result.svg.contains("</svg>"));
    }

    #[test]
    fn test_design_system_colors() {
        let design = test_design();
        let options = DiagramOptions::default();

        let result = generate_diagram(&design, &options);

        // Should use white background
        assert!(result.svg.contains("#FFFFFF"));
    }

    #[test]
    fn test_section_view_clearance_indicator() {
        let mut design = test_design();
        design.frame_material_depth = 1.0; // Deep frame
        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);

        // Should have clearance indicator (lowercase "clearance" or uppercase "INTERFERENCE")
        assert!(result.svg.contains("clearance") || result.svg.contains("INTERFERENCE"));
    }


    #[test]
    fn test_section_svg_output() {
        let design = test_design();
        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);
        assert!(result.svg.contains("<svg"));
        assert!(result.svg.contains("section-geometry"));
        assert!(result.svg.contains("section-dimensions"));
        assert!(result.warnings.is_empty());
        println!("SECTION SVG:\n{}", result.svg);
    }

    #[test]
    fn test_plan_svg_output() {
        let design = test_design();
        let options = DiagramOptions::default();

        let result = generate_diagram(&design, &options);
        assert!(result.svg.contains("<svg"));
        assert!(result.svg.contains("geometry"));
        assert!(result.svg.contains("dimensions"));
        assert!(result.warnings.is_empty());
        println!("PLAN SVG:\n{}", result.svg);
    }

    #[test]
    fn test_vertical_axis_break() {
        let mut design = test_design();
        design.frame_material_depth = 5.0; // Deep frame > 4" threshold
        design.frame_material_width = 2.0; // Normal width, no horizontal break
        
        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);
        
        // Should have axis break indicators (dashed zigzag)
        assert!(result.svg.contains("stroke-dasharray"));
        println!("VERTICAL AXIS BREAK SVG:\n{}", result.svg);
    }

    #[test]
    fn test_horizontal_axis_break() {
        let mut design = test_design();
        design.frame_material_width = 5.0; // Wide frame > 4" threshold
        design.frame_material_depth = 1.0; // Normal depth, no vertical break
        
        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);
        
        // Should have axis break indicators
        assert!(result.svg.contains("stroke-dasharray"));
        println!("HORIZONTAL AXIS BREAK SVG:\n{}", result.svg);
    }

    #[test]
    fn test_both_axis_breaks() {
        let mut design = test_design();
        design.frame_material_width = 5.0; // Wide frame > 4" threshold
        design.frame_material_depth = 5.0; // Deep frame > 4" threshold

        let options = DiagramOptions {
            view: ViewOption::SectionOnly,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);

        // Should have axis break indicators
        assert!(result.svg.contains("stroke-dasharray"));
        println!("BOTH AXIS BREAKS SVG:\n{}", result.svg);
    }

    #[test]
    fn test_dimension_arrow_horizontal() {
        let da = DimensionArrow::new(10.0, 50.0, 70.0, true)
            .color("#577590")
            .label("Frame: 1\"", "Arial", 11.0)
            .label_offset(15.0)
            .extension(60.0, 5.0)
            .stroke(0.5, 0.6);
        let svg = da.render();
        // Extension lines present (vertical, at x=10 and x=50)
        assert!(svg.contains("x1=\"10.00\""));
        assert!(svg.contains("x1=\"50.00\""));
        // Arrow line present
        assert!(svg.contains("data-arrow=\"true\""));
        // Label present
        assert!(svg.contains("Frame: 1&quot;"));
        assert!(svg.contains("text-anchor=\"middle\""));
    }

    #[test]
    fn test_dimension_arrow_vertical() {
        let da = DimensionArrow::new(20.0, 60.0, 5.0, false)
            .color("#46af8f")
            .label_two_lines("Rabbet", "3/8\"", "Arial", 10.0)
            .label_offset(5.0)
            .extension(15.0, -5.0)
            .stroke(0.5, 0.6);
        let svg = da.render();
        // Extension lines present (horizontal, at y=20 and y=60)
        assert!(svg.contains("y1=\"20.00\""));
        assert!(svg.contains("y1=\"60.00\""));
        // Two-line label present
        assert!(svg.contains("Rabbet"));
        assert!(svg.contains("3/8&quot;"));
        assert!(svg.contains("text-anchor=\"end\""));
    }

    #[test]
    fn test_dimension_arrow_target_ordering() {
        // Targets passed in reverse order should produce identical output
        let da_forward = DimensionArrow::new(10.0, 50.0, 70.0, true)
            .color("#577590")
            .stroke(0.5, 0.6)
            .extension(60.0, 5.0);
        let da_reverse = DimensionArrow::new(50.0, 10.0, 70.0, true)
            .color("#577590")
            .stroke(0.5, 0.6)
            .extension(60.0, 5.0);
        assert_eq!(da_forward.render(), da_reverse.render());
    }

    #[test]
    #[ignore] // Run manually: cargo test --lib test_dump_plan_svg -- --ignored --nocapture
    fn test_dump_plan_svg() {
        // Standard reference: 8×12 artwork, 2" mat, 3/4" frame, 3/8" rabbet
        let mut design = FrameDesign::new(8.0, 12.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.rabbet_width = 0.375;

        // Use mobile-like canvas dimensions (iPhone ~375pt - 32 padding)
        let options = DiagramOptions {
            view: ViewOption::PlanOnly,
            canvas_width: 343.0,
            canvas_height: 500.0,
            show_callouts: true,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);
        std::fs::write("/tmp/plan_view_test.svg", &result.svg).unwrap();
        eprintln!("SVG written to /tmp/plan_view_test.svg ({} bytes)", result.svg.len());
    }
}
