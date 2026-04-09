//! Shared SVG utilities, constants, and primitives.
//!
//! Extracted from `svg.rs` to keep it focused on high-level rendering.
//! Contains: arrow geometry helpers, axis-break zigzag helpers, dimension
//! arrow primitive, SVG element builders, and rendering constants.

use super::types::Rect;
use super::style::{DiagramStyle, FillPattern};

// ============================================================================
// VISUAL BOUNDARY HELPERS
// ============================================================================
//
// SVG elements have visual extents that differ from their geometric coordinates
// due to stroke widths, marker sizes, and other rendering details. These helpers
// calculate exact visual boundaries for precise alignment.

/// Arrow marker geometry constants
/// These match the inline polygon arrow definitions
pub(crate) mod arrow_geometry {
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
pub(crate) fn arrow_line_endpoint_for_target(target_x: f64, stroke_width: f64, is_start_marker: bool) -> f64 {
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
pub(crate) fn arrow_line_endpoint_for_target_y(target_y: f64, stroke_width: f64, is_start_marker: bool) -> f64 {
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

// === Dash Patterns ===
// "dash,gap" values in SVG user units.
pub(crate) const DASH_BREAK_INDICATOR: &str = "4,3";    // Axis break zigzag lines
pub(crate) const DASH_BOUNDARY: &str = "6,3";           // Content boundary outline
pub(crate) const DASH_ASSEMBLY_MARGIN: &str = "4,2";    // Assembly margin indicator
pub(crate) const DASH_CLEARANCE: &str = "3,2";          // Clearance/interference line

// === Opacity Values ===
pub(crate) const OPACITY_CONTENT_BOUNDARY: f64 = 0.5;   // Content boundary outline
pub(crate) const OPACITY_ASSEMBLY_MARGIN: f64 = 0.7;    // Assembly margin rect
pub(crate) const OPACITY_LABEL_BACKGROUND: f64 = 0.75;  // Artwork indicator label bg
pub(crate) const OPACITY_RABBET_BACKGROUND: f64 = 0.5;  // Rabbet indicator bg

// === Axis Break Spark Symbols ===
// Dimensions of the small zigzag "spark" drawn on broken dimension lines.
pub(crate) const SPARK_VERTICAL_WIDTH: f64 = 4.0;       // Horizontal extent of vertical spark
pub(crate) const SPARK_VERTICAL_HEIGHT: f64 = 8.0;      // Vertical extent of vertical spark
pub(crate) const SPARK_HORIZONTAL_WIDTH: f64 = 8.0;     // Horizontal extent of horizontal spark
pub(crate) const SPARK_HORIZONTAL_HEIGHT: f64 = 4.0;    // Vertical extent of horizontal spark

// === Leader Lines and Labels ===
// (LABEL_MASK_PADDING_X/Y imported from style.rs)
pub(crate) const LEADER_LINE_LENGTH: f64 = 10.0;        // Material label leader horizontal segment
pub(crate) const LEADER_STROKE_RATIO: f64 = 0.7;        // Leader line width as fraction of extension_stroke

// === Legend Layout ===
pub(crate) const LEGEND_SWATCH_SIZE: f64 = 12.0;        // Legend color swatch width/height
pub(crate) const LEGEND_SWATCH_STROKE: f64 = 0.5;       // Legend swatch border width
pub(crate) const LEGEND_SWATCH_GAP: f64 = 8.0;          // Gap between swatch and text
pub(crate) const LEGEND_ITEM_GAP: f64 = 16.0;           // Gap between legend items
pub(crate) const LEGEND_CHAR_WIDTH_RATIO: f64 = 0.55;   // Average character width as fraction of font size

// === Combined (PDF) View Layout ===
pub(crate) const TITLE_BLOCK_HEIGHT: f64 = 95.0;       // Height reserved for title block
pub(crate) const PLAN_HEIGHT_RATIO: f64 = 0.58;        // Plan view share of available height
pub(crate) const SECTION_HEIGHT_RATIO: f64 = 0.42;     // Section view share of available height
pub(crate) const SECTION_FONT_SCALE: f64 = 0.76;       // Section font size relative to plan
pub(crate) const SECTION_DIM_OFFSET_SCALE: f64 = 0.9;  // Section dimension offsets relative to plan

// === Text Rendering ===
pub(crate) const BASELINE_SHIFT_RATIO: f64 = 0.35;     // Vertical centering shift for SVG text

// === Tight-Space Dimension Arrows ===
pub(crate) const TIGHT_SPACE_MULTIPLIER: f64 = 3.0;    // Arrow placed outside when span < multiplier * stroke

// ============================================================================
// AXIS BREAK HELPERS
// ============================================================================

/// Axis break visual constants
pub(crate) const ZIGZAG_AMPLITUDE: f64 = 3.5;
pub(crate) const ZIGZAG_PROUD_AMOUNT: f64 = 8.0;

/// Interpolate y at given x along a line segment between two points
pub(crate) fn y_at_x(p1: (f64, f64), p2: (f64, f64), x: f64) -> f64 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    if (x2 - x1).abs() < 0.001 { return y1; }
    y1 + (x - x1) * (y2 - y1) / (x2 - x1)
}

/// Interpolate x at given y along a line segment between two points
pub(crate) fn x_at_y(p1: (f64, f64), p2: (f64, f64), y: f64) -> f64 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    if (y2 - y1).abs() < 0.001 { return x1; }
    x1 + (y - y1) * (x2 - x1) / (y2 - y1)
}

/// Four control points defining a zigzag break indicator line
pub(crate) struct ZigzagPoints {
    pub p0: (f64, f64),
    pub p1: (f64, f64),
    pub p2: (f64, f64),
    pub p3: (f64, f64),
}

/// Compute horizontal zigzag control points (spans left-to-right at a given y)
pub(crate) fn horizontal_zigzag(center_y: f64, frame_x: f64, frame_w: f64) -> ZigzagPoints {
    ZigzagPoints {
        p0: (frame_x - ZIGZAG_PROUD_AMOUNT, center_y),
        p1: (frame_x + frame_w * 0.15, center_y - ZIGZAG_AMPLITUDE),
        p2: (frame_x + frame_w * 0.85, center_y + ZIGZAG_AMPLITUDE),
        p3: (frame_x + frame_w + ZIGZAG_PROUD_AMOUNT, center_y),
    }
}

/// Compute vertical zigzag control points (spans top-to-bottom at a given x)
pub(crate) fn vertical_zigzag(center_x: f64, frame_y: f64, frame_h: f64) -> ZigzagPoints {
    ZigzagPoints {
        p0: (center_x, frame_y - ZIGZAG_PROUD_AMOUNT),
        p1: (center_x - ZIGZAG_AMPLITUDE, frame_y + frame_h * 0.15),
        p2: (center_x + ZIGZAG_AMPLITUDE, frame_y + frame_h * 0.85),
        p3: (center_x, frame_y + frame_h + ZIGZAG_PROUD_AMOUNT),
    }
}

/// Render a dashed zigzag indicator line
pub(crate) fn render_zigzag_line(svg: &mut String, zz: &ZigzagPoints, line_color: &str, break_line_width: f64) {
    render_zigzag_line_with_opacity(svg, zz, line_color, break_line_width, 1.0);
}

/// Render a dashed zigzag indicator line with custom opacity
pub(crate) fn render_zigzag_line_with_opacity(svg: &mut String, zz: &ZigzagPoints, line_color: &str, break_line_width: f64, opacity: f64) {
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

/// Render a spark/zigzag break symbol on a dimension line.
///
/// Two orientations supported:
/// - Horizontal: zigzag perpendicular to a horizontal line (varies in Y)
/// - Vertical: zigzag perpendicular to a vertical line (varies in X)
///
/// `inner_fraction` controls the width of the inner zigzag points relative to the
/// full spark extent (e.g., 0.25 for plan view, ~0.167 for section horizontal).
pub(crate) fn render_spark_symbol(
    svg: &mut String,
    center_x: f64,
    center_y: f64,
    horizontal: bool,
    color: &str,
    stroke_width: f64,
) {
    if horizontal {
        let sw = SPARK_HORIZONTAL_WIDTH;
        let sh = SPARK_HORIZONTAL_HEIGHT;
        svg.push_str(&format!(
            r#"    <path d="M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            center_x - sw / 2.0, center_y,
            center_x - sw / 4.0, center_y - sh,
            center_x + sw / 4.0, center_y + sh,
            center_x + sw / 2.0, center_y,
            color, stroke_width
        ));
    } else {
        let sw = SPARK_VERTICAL_WIDTH;
        let sh = SPARK_VERTICAL_HEIGHT;
        svg.push_str(&format!(
            r#"    <path d="M {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2} L {:.2},{:.2}" stroke="{}" stroke-width="{}" fill="none"/>"#,
            center_x, center_y - sh / 2.0,
            center_x + sw, center_y - sh / 4.0,
            center_x - sw, center_y + sh / 4.0,
            center_x, center_y + sh / 2.0,
            color, stroke_width
        ));
    }
    svg.push('\n');
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
pub(crate) fn generate_arrow_polygon(x1: f64, y1: f64, x2: f64, y2: f64, fill: &str, stroke_width: f64, is_leader: bool) -> String {
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
pub(crate) fn generate_line_with_arrows(
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
pub(crate) enum DimensionLabel {
    /// Single line, centered
    Single { text: String, bold: bool },
    /// Two lines (e.g. "Rabbet" + value), right-aligned for vertical dims
    #[allow(dead_code)] // Used in tests
    TwoLines { line1: String, line2: String },
}

/// A reusable primitive that renders a complete dimension callout:
/// two extension lines, a dimension line with arrow tips, and a label.
pub(crate) struct DimensionArrow {
    // Measurement boundaries (perpendicular to the dimension line)
    pub target_a: f64,
    pub target_b: f64,

    // Where the dimension line sits (in the measurement axis)
    pub dim_line_pos: f64,

    // Orientation: true = horizontal dim line measuring X distance
    pub horizontal: bool,

    // Extension lines
    pub ext_from: f64,        // where extension lines start (geometry edge)
    pub ext_overshoot: f64,   // how far past dim_line_pos they extend

    // Styling
    pub stroke_color: String,
    pub arrow_stroke_width: f64,
    pub ext_stroke_width: f64,

    // Label
    pub label: Option<DimensionLabel>,
    pub font_family: String,
    pub font_size: f64,
    pub label_offset: f64,
}

impl DimensionArrow {
    pub(crate) fn new(target_a: f64, target_b: f64, dim_line_pos: f64, horizontal: bool) -> Self {
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

    pub(crate) fn color(mut self, color: &str) -> Self {
        self.stroke_color = color.to_string();
        self
    }

    pub(crate) fn label(mut self, text: &str, font_family: &str, font_size: f64) -> Self {
        self.label = Some(DimensionLabel::Single { text: text.to_string(), bold: true });
        self.font_family = font_family.to_string();
        self.font_size = font_size;
        self
    }

    #[allow(dead_code)] // Used in tests
    pub(crate) fn label_two_lines(mut self, line1: &str, line2: &str, font_family: &str, font_size: f64) -> Self {
        self.label = Some(DimensionLabel::TwoLines {
            line1: line1.to_string(),
            line2: line2.to_string(),
        });
        self.font_family = font_family.to_string();
        self.font_size = font_size;
        self
    }

    pub(crate) fn label_offset(mut self, offset: f64) -> Self {
        self.label_offset = offset;
        self
    }

    pub(crate) fn extension(mut self, from: f64, overshoot: f64) -> Self {
        self.ext_from = from;
        self.ext_overshoot = overshoot;
        self
    }

    pub(crate) fn stroke(mut self, arrow_width: f64, ext_width: f64) -> Self {
        self.arrow_stroke_width = arrow_width;
        self.ext_stroke_width = ext_width;
        self
    }

    pub(crate) fn render(&self) -> String {
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
                            r#"    <text transform="translate({:.2}, {:.2})" font-family="{}" font-size="{:.1}px" fill="{}"{} text-anchor="middle">{}</text>"#,
                            mid, self.dim_line_pos + self.label_offset,
                            self.font_family, self.font_size, color, weight,
                            html_escape(text)
                        ));
                    } else {
                        svg.push_str(&format!(
                            r#"    <text transform="translate({:.2}, {:.2})" font-family="{}" font-size="{:.1}px" fill="{}"{} text-anchor="end">{}</text>"#,
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
                        r#"    <text transform="translate({:.2}, {:.2})" font-family="{}" font-size="{:.1}px" fill="{}" text-anchor="end" font-weight="bold">{}</text>"#,
                        label_x, mid - 1.0,
                        self.font_family, self.font_size, color,
                        html_escape(line1)
                    ));
                    svg.push('\n');
                    svg.push_str(&format!(
                        r#"    <text transform="translate({:.2}, {:.2})" font-family="{}" font-size="{:.1}px" fill="{}" text-anchor="end" font-weight="bold">{}</text>"#,
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
// SVG ELEMENT BUILDERS
// ============================================================================

/// Generate SVG defs (patterns, markers, etc.)
pub(crate) fn generate_defs(_style: &DiagramStyle) -> String {
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
pub(crate) fn svg_rect(rect: &Rect, stroke: &str, stroke_width: f64, fill: Option<&str>) -> String {
    let fill_str = fill.unwrap_or("none");
    format!(
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" stroke="{}" stroke-width="{}" fill="{}"/>"#,
        rect.x, rect.y, rect.width, rect.height, stroke, stroke_width, fill_str
    ) + "\n"
}

/// Get fill color for a pattern
pub(crate) fn get_fill_for_pattern(pattern: &FillPattern) -> String {
    match pattern {
        FillPattern::Solid(color) => color.clone(),
        FillPattern::Hatched { color, .. } => color.clone(),
        FillPattern::CrossHatched { color, .. } => color.clone(),
    }
}

/// Extract viewBox dimensions from SVG string
/// Returns (x, y, width, height) if found
pub(crate) fn extract_viewbox(svg: &str) -> Option<(f64, f64, f64, f64)> {
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
pub(crate) fn extract_svg_content(svg: &str) -> String {
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
pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
