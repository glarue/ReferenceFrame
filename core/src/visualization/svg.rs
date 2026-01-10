// SVG generation for frame diagrams
//
// Generates professional, warm-aesthetic SVG diagrams from
// frame designs with adaptive dimension callouts.

use crate::frame::FrameDesign;
use crate::conversions::{format_value, Unit};
use super::types::{
    DiagramOptions, DiagramResult, ViewOption, PositionedCallout,
    Rect, Side,
};
use super::style::{DiagramStyle, FillPattern};
use super::geometry::{PlanViewGeometry, SectionViewGeometry};
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

/// Calculate the visual extent of a stroked rectangle.
/// SVG strokes are centered on the path, so the visual boundary extends
/// stroke_width/2 beyond the geometric coordinates.
#[allow(dead_code)]
fn stroked_rect_visual_extent(rect: &Rect, stroke_width: f64) -> Rect {
    let half_stroke = stroke_width / 2.0;
    Rect::new(
        rect.x - half_stroke,
        rect.y - half_stroke,
        rect.width + stroke_width,
        rect.height + stroke_width,
    )
}

/// Calculate the inner visual boundary of a stroked rectangle.
/// This is the area inside the stroke (where the fill appears).
#[allow(dead_code)]
fn stroked_rect_inner_boundary(rect: &Rect, stroke_width: f64) -> Rect {
    let half_stroke = stroke_width / 2.0;
    Rect::new(
        rect.x + half_stroke,
        rect.y + half_stroke,
        rect.width - stroke_width,
        rect.height - stroke_width,
    )
}

// ============================================================================
// VISUAL GRAMMAR CONSTANTS
// ============================================================================

/// Visual grammar constants for professional dimension callouts
///
/// ARROW_GAP: Legacy constant, kept for reference
/// Arrow tips now land exactly at target geometry using helper functions
#[allow(dead_code)]
const ARROW_GAP: f64 = 6.0;

/// EXTENSION_OVERSHOOT: How far extension lines extend past the dimension line
/// - Creates the classic drafting look where witness lines extend beyond arrows
const EXTENSION_OVERSHOOT: f64 = 8.0;

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
    let geometry = PlanViewGeometry::from_design(
        design,
        options.canvas_width,
        options.canvas_height,
        style,
    );

    let callouts = generate_plan_callouts(design, &geometry, options.unit_mm, style);
    let layout = layout_plan_callouts(&callouts, &geometry, style);

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

    let callouts = generate_section_callouts(design, options.unit_mm);
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
    let gap_between_views = 30.0;
    let plan_height = (options.canvas_height - gap_between_views) * 0.58;
    let section_height = (options.canvas_height - gap_between_views) * 0.42;

    // Scale factor for combined view
    let scale_factor = 0.80;

    // Create compact style for plan view
    let mut plan_style = style.clone();
    plan_style.margin = style.margin * scale_factor;
    plan_style.dimension_offset_base = style.dimension_offset_base * scale_factor;
    plan_style.dimension_offset_step = style.dimension_offset_step * scale_factor;
    plan_style.dimension_font_size = style.dimension_font_size * scale_factor;
    plan_style.label_font_size = style.label_font_size * scale_factor;
    plan_style.title_font_size = style.title_font_size * scale_factor;

    // Create compact style for section view
    let mut section_style = style.clone();
    section_style.margin = style.margin * scale_factor;
    section_style.dimension_font_size = style.dimension_font_size * scale_factor;
    section_style.label_font_size = style.label_font_size * scale_factor;
    section_style.title_font_size = style.title_font_size * scale_factor;

    let plan_options = DiagramOptions {
        canvas_height: plan_height,
        ..options.clone()
    };

    let section_options = DiagramOptions {
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

    // Plan view on top - embed as nested SVG with preserved viewBox
    if let Some((vb_x, vb_y, vb_w, vb_h)) = plan_viewbox {
        svg.push_str(&format!(
            r#"  <svg id="plan-view" x="0" y="0" width="{}" height="{}" viewBox="{} {} {} {}" preserveAspectRatio="xMidYMin meet">{}</svg>"#,
            options.canvas_width,
            plan_height,
            vb_x, vb_y, vb_w, vb_h,
            extract_svg_content(&plan_result.svg)
        ));
    } else {
        // Fallback
        let plan_content = extract_svg_content(&plan_result.svg);
        svg.push_str(&format!(
            r#"  <g id="plan-view" transform="translate(0, 0)">{}</g>"#,
            plan_content
        ));
    }
    svg.push('\n');

    // Section view below - embed as nested SVG with preserved viewBox
    // This ensures the section view (including legend) scales to fit within section_height
    if let Some((vb_x, vb_y, vb_w, vb_h)) = section_viewbox {
        svg.push_str(&format!(
            r#"  <svg id="section-view" x="0" y="{}" width="{}" height="{}" viewBox="{} {} {} {}" preserveAspectRatio="xMidYMin meet">{}</svg>"#,
            plan_height + gap_between_views,
            options.canvas_width,
            section_height,
            vb_x, vb_y, vb_w, vb_h,
            extract_svg_content(&section_result.svg)
        ));
    } else {
        // Fallback
        let section_content = extract_svg_content(&section_result.svg);
        svg.push_str(&format!(
            r#"  <g id="section-view" transform="translate(0, {})">{}</g>"#,
            plan_height + gap_between_views,
            section_content
        ));
    }
    svg.push('\n');

    svg.push_str("</svg>");

    let mut warnings = plan_result.warnings;
    warnings.extend(section_result.warnings);

    DiagramResult { svg, warnings }
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
    // Calculate bounds from actual geometry and layout data
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
        let label_text_width = callout.callout.label.len() as f64 * style.dimension_font_size * 0.6;
        let label_height = style.dimension_font_size * 1.2;

        // Mat cut dimensions get extra offset - calculate it here
        let mat_cut_offset = EXTENSION_OVERSHOOT + style.dimension_font_size / 2.0 + style.dimension_offset_base;

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

    // Add padding for visual comfort
    let padding = style.margin;
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    // Calculate viewBox dimensions
    let viewbox_width = max_x - min_x;
    let viewbox_height = max_y - min_y;

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

    // Geometry group
    svg.push_str("  <g id=\"geometry\">\n");

    // Frame outer (thickest line)
    svg.push_str(&svg_rect(
        &geometry.frame_outer,
        &style.line_color,
        style.frame_stroke_width,
        None,
    ));

    // Frame inner / visible opening (same weight as outer for visual consistency)
    svg.push_str(&svg_rect(
        &geometry.frame_inner,
        &style.line_color,
        style.frame_stroke_width,
        None,
    ));

    // Mat opening (if present) - thinner than frame lines
    if let Some(mat_opening) = &geometry.mat_opening {
        svg.push_str(&svg_rect(
            mat_opening,
            &style.line_color,
            style.mat_stroke_width,
            None,
        ));

        // Artwork rectangle (dashed line, shows actual artwork boundary)
        // This is larger than mat_opening when there's mat overlap
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"4,2\" fill=\"none\" opacity=\"0.6\"/>\n",
            geometry.artwork.x, geometry.artwork.y,
            geometry.artwork.width, geometry.artwork.height,
            style.dimension_color, style.extension_stroke_width
        ));
    }

    svg.push_str("  </g>\n");

    // Frame/mat overlap visualization - semi-transparent fill showing rabbet overlap area
    // This is the area between frame_inner (visible opening) and content_area (matboard edge)
    // The frame lip covers this area - use rabbet_width (horizontal lip overlap)
    let rabbet_scaled = design.rabbet_width * geometry.scale;
    if rabbet_scaled > 0.5 {
        svg.push_str("  <g id=\"rabbet-overlap\">\n");

        // Draw as path with hole: outer = content_area, inner = frame_inner
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
            ox, oy, ow, oh, -ow,  // Outer: clockwise (content_area)
            ix, iy, ih, iw, -ih   // Inner: counter-clockwise (frame_inner)
        );
        // Semi-transparent fill (ring shape)
        svg.push_str(&format!(
            "    <path d=\"{}\" fill=\"#8B7355\" fill-opacity=\"0.15\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
            path_d
        ));
        // Dashed stroke on outer edge only
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#8B7355\" stroke-width=\"{:.2}\" stroke-dasharray=\"4,2\" stroke-opacity=\"0.5\"/>\n",
            ox, oy, ow, oh, style.extension_stroke_width * 0.8
        ));

        svg.push_str("  </g>\n");
    }

    // Content/matboard boundary - dashed line showing where content sits under frame lip
    // This is the content_area boundary (matboard outer edge that sits in the rabbet)
    svg.push_str("  <g id=\"content-boundary\">\n");
    svg.push_str(&format!(
        "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"6,3\" fill=\"none\" opacity=\"0.5\"/>\n",
        geometry.content_area.x, geometry.content_area.y,
        geometry.content_area.width, geometry.content_area.height,
        "#8B7355", style.extension_stroke_width
    ));
    svg.push_str("  </g>\n");

    // Mat/artwork overlap visualization - shows where mat covers artwork edges
    // This is the area BETWEEN the artwork boundary (outer) and mat opening (inner)
    // The artwork is larger than the mat opening; the mat covers the artwork edges
    if let Some(mat_opening) = &geometry.mat_opening {
        let mat_overlap_scaled = design.mat_overlap * geometry.scale;

        if mat_overlap_scaled > 0.5 && design.has_mat() {
            svg.push_str("  <g id=\"mat-overlap\">\n");

            // Draw as single path with hole to avoid seams
            // Outer rect = artwork boundary (larger)
            let ox = geometry.artwork.x;
            let oy = geometry.artwork.y;
            let ow = geometry.artwork.width;
            let oh = geometry.artwork.height;
            // Inner rect = mat_opening (the visible window, smaller)
            let ix = mat_opening.x;
            let iy = mat_opening.y;
            let iw = mat_opening.width;
            let ih = mat_opening.height;

            // Path: outer rect clockwise, inner rect counter-clockwise
            let path_d = format!(
                "M{:.2},{:.2} h{:.2} v{:.2} h{:.2} Z M{:.2},{:.2} v{:.2} h{:.2} v{:.2} Z",
                ox, oy, ow, oh, -ow,  // Outer: clockwise (artwork)
                ix, iy, ih, iw, -ih   // Inner: counter-clockwise (mat opening)
            );
            // Semi-transparent fill (ring shape)
            svg.push_str(&format!(
                "    <path d=\"{}\" fill=\"#888888\" fill-opacity=\"0.12\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
                path_d
            ));
            // Dashed stroke on outer edge only
            svg.push_str(&format!(
                "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"#888888\" stroke-width=\"{:.2}\" stroke-dasharray=\"3,2\" stroke-opacity=\"0.4\"/>\n",
                ox, oy, ow, oh, style.extension_stroke_width * 0.8
            ));

            svg.push_str("  </g>\n");
        }
    }

    // Dimensions group
    svg.push_str("  <g id=\"dimensions\">\n");
    for callout in &layout.positioned_callouts {
        svg.push_str(&svg_dimension(callout, style, geometry));
    }
    svg.push_str("  </g>\n");

    // Artwork dimensions indicator - arrows extending to artwork boundary
    // The artwork boundary is shown as a dashed line with stroke width extension_stroke_width * 0.8
    // We want arrow tips to land exactly at the INNER edge of that dashed stroke
    let artwork_center = geometry.artwork.center();
    let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };

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

    // Horizontal line with arrows
    svg.push_str(&generate_line_with_arrows(
        h_line_x1, artwork_center.y,
        h_line_x2, artwork_center.y,
        &style.artwork_dimension_color,
        arrow_stroke_width,
        true,  // arrow_start
        true,  // arrow_end
        false, // is_leader
    ));

    // Vertical line with arrows
    svg.push_str(&generate_line_with_arrows(
        artwork_center.x, v_line_y1,
        artwork_center.x, v_line_y2,
        &style.artwork_dimension_color,
        arrow_stroke_width,
        true,  // arrow_start
        true,  // arrow_end
        false, // is_leader
    ));

    // Artwork dimension label (height × width)
    let artwork_label = format!(
        "{} × {}",
        format_value(design.artwork_height, unit),
        format_value(design.artwork_width, unit)
    );

    // Calculate background rectangle dimensions
    // Use better text width estimation: 0.6 * font_size per character for typical fonts
    let mask_margin = 4.0;
    let estimated_char_width = style.dimension_font_size * 0.6;
    let text_bg_w = artwork_label.len() as f64 * estimated_char_width + mask_margin * 2.0;
    let text_bg_h = style.dimension_font_size * 1.3 + mask_margin * 2.0;

    // Draw background rectangle FIRST (so it appears behind the text)
    // Centered on artwork_center
    svg.push_str(&format!(
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.85" stroke="none" rx="2"/>"#,
        artwork_center.x - text_bg_w / 2.0,
        artwork_center.y - text_bg_h / 2.0,
        text_bg_w,
        text_bg_h,
        style.background_color
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
        artwork_center.x, artwork_center.y,
        style.artwork_dimension_color, style.font_family, style.dimension_font_size,
        html_escape(&artwork_label)
    ));
    svg.push('\n');
    svg.push_str("  </g>\n");

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
    use crate::conversions::{format_value, Unit};
    let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };

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
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" stroke="{}" stroke-width="{}" stroke-dasharray="4,2" fill="none" opacity="0.7"/>"#,
            geometry.assembly_margin.x, geometry.assembly_margin.y,
            geometry.assembly_margin.width, geometry.assembly_margin.height,
            style.dimension_color, style.extension_stroke_width
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
        let break_start_y = geometry.axis_break_start_y;
        let break_end_y = geometry.axis_break_end_y;
        let _outer_edge_depth_s = geometry.outer_edge_depth * geometry.scale;

        // Break indicator geometry for vertical breaks
        let zigzag_amplitude = 5.0;
        let break_line_width = style.frame_stroke_width * 0.5;
        let proud_amount = 8.0;

        // Define the 4 control points of the zigzag (left to right)
        let x_left = frame_x - proud_amount;
        let x_right = frame_x + frame_w + proud_amount;
        let peak1_x = frame_x + frame_w * 0.15;
        let peak2_x = frame_x + frame_w * 0.85;

        // Top zigzag control points
        let top_p0 = (x_left, break_start_y);
        let top_p1 = (peak1_x, break_start_y - zigzag_amplitude);
        let top_p2 = (peak2_x, break_start_y + zigzag_amplitude);
        let top_p3 = (x_right, break_start_y);

        // Bottom zigzag control points
        let bottom_p0 = (x_left, break_end_y);
        let bottom_p1 = (peak1_x, break_end_y - zigzag_amplitude);
        let bottom_p2 = (peak2_x, break_end_y + zigzag_amplitude);
        let bottom_p3 = (x_right, break_end_y);

        // Helper: interpolate y at given x along a line segment
        fn y_at_x(p1: (f64, f64), p2: (f64, f64), x: f64) -> f64 {
            let (x1, y1) = p1;
            let (x2, y2) = p2;
            if (x2 - x1).abs() < 0.001 { return y1; }
            y1 + (x - x1) * (y2 - y1) / (x2 - x1)
        }

        // Calculate where zigzag crosses frame boundaries
        let top_y_at_left = y_at_x(top_p0, top_p1, frame_x);
        let top_y_at_right = y_at_x(top_p2, top_p3, frame_x + frame_w);
        let bottom_y_at_left = y_at_x(bottom_p0, bottom_p1, frame_x);
        let bottom_y_at_right = y_at_x(bottom_p2, bottom_p3, frame_x + frame_w);

        // Top portion: simple rectangle (front face)
        let top_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, frame_y,
            frame_x + frame_w, frame_y,
            frame_x + frame_w, top_y_at_right,
            top_p2.0, top_p2.1,
            top_p1.0, top_p1.1,
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
            bottom_p1.0, bottom_p1.1,
            bottom_p2.0, bottom_p2.1,
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
        let top_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            top_p0.0, top_p0.1,
            top_p1.0, top_p1.1,
            top_p2.0, top_p2.1,
            top_p3.0, top_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            top_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');

        let bottom_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            bottom_p0.0, bottom_p0.1,
            bottom_p1.0, bottom_p1.1,
            bottom_p2.0, bottom_p2.1,
            bottom_p3.0, bottom_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            bottom_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');
    } else if geometry.use_axis_break && geometry.use_axis_break_y {
        // Both horizontal and vertical breaks active
        // OVERLAY APPROACH: Draw full L-shape, then overlay white zigzag-shaped gap bands
        let h_break_start = geometry.axis_break_start_x;
        let h_break_end = geometry.axis_break_end_x;
        let v_break_start = geometry.axis_break_start_y;
        let v_break_end = geometry.axis_break_end_y;

        let zigzag_amplitude = 5.0;
        let break_line_width = style.frame_stroke_width * 0.5;
        let proud_amount = 8.0;

        // Horizontal zigzag control points (for vertical break - top/bottom)
        let h_x_left = frame_x - proud_amount;
        let h_x_right = frame_x + frame_w + proud_amount;
        let h_peak1_x = frame_x + frame_w * 0.15;
        let h_peak2_x = frame_x + frame_w * 0.85;

        // Top zigzag (horizontal)
        let top_zz_p0 = (h_x_left, v_break_start);
        let top_zz_p1 = (h_peak1_x, v_break_start - zigzag_amplitude);
        let top_zz_p2 = (h_peak2_x, v_break_start + zigzag_amplitude);
        let top_zz_p3 = (h_x_right, v_break_start);

        // Bottom zigzag (horizontal)
        let bot_zz_p0 = (h_x_left, v_break_end);
        let bot_zz_p1 = (h_peak1_x, v_break_end - zigzag_amplitude);
        let bot_zz_p2 = (h_peak2_x, v_break_end + zigzag_amplitude);
        let bot_zz_p3 = (h_x_right, v_break_end);

        // Vertical zigzag control points (for horizontal break - left/right)
        let v_y_top = frame_y - proud_amount;
        let v_y_bottom = frame_y + frame_h + proud_amount;
        let v_peak1_y = frame_y + frame_h * 0.15;
        let v_peak2_y = frame_y + frame_h * 0.85;

        // Left zigzag (vertical)
        let left_zz_p0 = (h_break_start, v_y_top);
        let left_zz_p1 = (h_break_start - zigzag_amplitude, v_peak1_y);
        let left_zz_p2 = (h_break_start + zigzag_amplitude, v_peak2_y);
        let left_zz_p3 = (h_break_start, v_y_bottom);

        // Right zigzag (vertical)
        let right_zz_p0 = (h_break_end, v_y_top);
        let right_zz_p1 = (h_break_end - zigzag_amplitude, v_peak1_y);
        let right_zz_p2 = (h_break_end + zigzag_amplitude, v_peak2_y);
        let right_zz_p3 = (h_break_end, v_y_bottom);

        // Helper functions for intersection calculations
        fn y_at_x(p1: (f64, f64), p2: (f64, f64), x: f64) -> f64 {
            let (x1, y1) = p1;
            let (x2, y2) = p2;
            if (x2 - x1).abs() < 0.001 { return y1; }
            y1 + (x - x1) * (y2 - y1) / (x2 - x1)
        }
        fn x_at_y(p1: (f64, f64), p2: (f64, f64), y: f64) -> f64 {
            let (x1, y1) = p1;
            let (x2, y2) = p2;
            if (y2 - y1).abs() < 0.001 { return x1; }
            x1 + (y - y1) * (x2 - x1) / (y2 - y1)
        }

        // Calculate intersections for strokes
        let top_y_at_left = y_at_x(top_zz_p0, top_zz_p1, frame_x);
        let top_y_at_right = y_at_x(top_zz_p2, top_zz_p3, frame_x + frame_w);
        let bot_y_at_left = y_at_x(bot_zz_p0, bot_zz_p1, frame_x);
        let bot_y_at_right = y_at_x(bot_zz_p2, bot_zz_p3, frame_x + frame_w);
        let left_x_at_top = x_at_y(left_zz_p0, left_zz_p1, frame_y);
        let left_x_at_bottom = x_at_y(left_zz_p2, left_zz_p3, frame_y + frame_h);
        let right_x_at_top = x_at_y(right_zz_p0, right_zz_p1, frame_y);
        let right_x_at_bottom = x_at_y(right_zz_p2, right_zz_p3, frame_y + frame_h);

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
            top_zz_p0.0, top_zz_p0.1,
            top_zz_p1.0, top_zz_p1.1,
            top_zz_p2.0, top_zz_p2.1,
            top_zz_p3.0, top_zz_p3.1,
            bot_zz_p3.0, bot_zz_p3.1,
            bot_zz_p2.0, bot_zz_p2.1,
            bot_zz_p1.0, bot_zz_p1.1,
            bot_zz_p0.0, bot_zz_p0.1,
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
            left_zz_p0.0, left_zz_p0.1,
            left_zz_p1.0, left_zz_p1.1,
            left_zz_p2.0, left_zz_p2.1,
            left_zz_p3.0, left_zz_p3.1,
            right_zz_p3.0, right_zz_p3.1,
            right_zz_p2.0, right_zz_p2.1,
            right_zz_p1.0, right_zz_p1.1,
            right_zz_p0.0, right_zz_p0.1,
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
        // Top horizontal zigzag
        let top_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            top_zz_p0.0, top_zz_p0.1, top_zz_p1.0, top_zz_p1.1, 
            top_zz_p2.0, top_zz_p2.1, top_zz_p3.0, top_zz_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            top_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');

        // Bottom horizontal zigzag
        let bottom_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            bot_zz_p0.0, bot_zz_p0.1, bot_zz_p1.0, bot_zz_p1.1, 
            bot_zz_p2.0, bot_zz_p2.1, bot_zz_p3.0, bot_zz_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            bottom_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');

        // Left vertical zigzag
        let left_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            left_zz_p0.0, left_zz_p0.1, left_zz_p1.0, left_zz_p1.1, 
            left_zz_p2.0, left_zz_p2.1, left_zz_p3.0, left_zz_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            left_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');

        // Right vertical zigzag
        let right_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            right_zz_p0.0, right_zz_p0.1, right_zz_p1.0, right_zz_p1.1, 
            right_zz_p2.0, right_zz_p2.1, right_zz_p3.0, right_zz_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            right_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');
    } else if geometry.use_axis_break && !geometry.use_axis_break_y {
        // Horizontal break only (no vertical break)
        let break_start = geometry.axis_break_start_x;
        let break_end = geometry.axis_break_end_x;

        // Break indicator geometry - define the zigzag path ONCE, then derive everything from it
        // Pattern: center → left peak (steep) → right peak (long) → center (steep)
        let zigzag_amplitude = 5.0;  // How far each peak extends left/right
        let break_line_width = style.frame_stroke_width * 0.5;
        let proud_amount = 8.0;  // How far zigzags extend beyond frame edges

        // Define the 4 control points of the zigzag (top to bottom)
        // Point 0: top extension (above frame)
        // Point 1: first peak (near top, goes left)
        // Point 2: second peak (near bottom, goes right)
        // Point 3: bottom extension (below frame)
        let y_top = frame_y - proud_amount;
        let y_bottom = frame_y + frame_h + proud_amount;
        let peak1_y = frame_y + frame_h * 0.15;
        let peak2_y = frame_y + frame_h * 0.85;

        // Left zigzag control points
        let left_p0 = (break_start, y_top);
        let left_p1 = (break_start - zigzag_amplitude, peak1_y);
        let left_p2 = (break_start + zigzag_amplitude, peak2_y);
        let left_p3 = (break_start, y_bottom);

        // Right zigzag control points (identical shape, translated)
        let right_p0 = (break_end, y_top);
        let right_p1 = (break_end - zigzag_amplitude, peak1_y);
        let right_p2 = (break_end + zigzag_amplitude, peak2_y);
        let right_p3 = (break_end, y_bottom);

        // Helper: interpolate x at given y along a line segment
        fn x_at_y(p1: (f64, f64), p2: (f64, f64), y: f64) -> f64 {
            let (x1, y1) = p1;
            let (x2, y2) = p2;
            if (y2 - y1).abs() < 0.001 { return x1; }
            x1 + (y - y1) * (x2 - x1) / (y2 - y1)
        }

        // Calculate where zigzag crosses frame boundaries
        // Top edge (y = frame_y): on segment p0 → p1
        let left_x_at_top = x_at_y(left_p0, left_p1, frame_y);
        let right_x_at_top = x_at_y(right_p0, right_p1, frame_y);
        // Bottom edge (y = frame_y + frame_h): on segment p2 → p3
        let left_x_at_bottom = x_at_y(left_p2, left_p3, frame_y + frame_h);
        let right_x_at_bottom = x_at_y(right_p2, right_p3, frame_y + frame_h);

        // Left portion: fill edge follows zigzag exactly
        let left_fill_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
            frame_x, frame_y,                      // Top-left corner
            left_x_at_top, frame_y,                // Where zigzag crosses top edge
            left_p1.0, left_p1.1,                  // First peak
            left_p2.0, left_p2.1,                  // Second peak
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
            right_p2.0, right_p2.1,                             // Second peak - going up
            right_p1.0, right_p1.1,                             // First peak
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

        // Break indicator: dashed zigzag lines extending proud of frame
        // Uses exact same control points as fill edges
        let left_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            left_p0.0, left_p0.1,
            left_p1.0, left_p1.1,
            left_p2.0, left_p2.1,
            left_p3.0, left_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            left_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');

        let right_zigzag_path = format!(
            "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2}",
            right_p0.0, right_p0.1,
            right_p1.0, right_p1.1,
            right_p2.0, right_p2.1,
            right_p3.0, right_p3.1,
        );
        svg.push_str(&format!(
            r#"    <path d="{}" stroke="{}" stroke-width="{}" fill="none" stroke-dasharray="4,3" stroke-linecap="round" stroke-linejoin="round"/>"#,
            right_zigzag_path, style.line_color, break_line_width
        ));
        svg.push('\n');
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
    track_x!(dim_x - EXTENSION_OVERSHOOT);

    // Extension lines
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        frame_x - style.extension_line_gap, dim_y1,
        dim_x - EXTENSION_OVERSHOOT, dim_y1,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        frame_x - style.extension_line_gap, dim_y2,
        dim_x - EXTENSION_OVERSHOOT, dim_y2,
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
        let spark_width = 4.0;  // Horizontal extent of zigzag
        let spark_height = 8.0; // Vertical extent of zigzag
        
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
    let label_offset = LABEL_BUFFER + style.dimension_font_size * LABEL_FONT_OFFSET + 6.0;
    let depth_label_x = dim_x - label_offset;
    let depth_label_y = (dim_y1 + dim_y2) / 2.0;
    
    // Track left extent - the rotated label extends half its height to the left of its x position
    // (rotated -90 degrees means the text height becomes width, text is anchored at middle)
    track_x!(depth_label_x - style.dimension_font_size / 2.0);
    
    let depth_value = if geometry.use_axis_break_y {
        geometry.actual_frame_depth
    } else {
        design.frame_material_depth
    };
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">Depth: {}</text>"#,
        depth_label_x, depth_label_y,
        dim_color, style.font_family, style.dimension_font_size,
        depth_label_x, depth_label_y,
        format_value(depth_value, unit)
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
        fw_x1, fw_y - EXTENSION_OVERSHOOT,
        dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        fw_x2, frame_y - style.extension_line_gap,
        fw_x2, fw_y - EXTENSION_OVERSHOOT,
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
        let spark_width = 8.0;  // Total width of spark symbol
        let spark_height = 4.0; // Amplitude of spark

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
    track_y!(fw_label_y - style.dimension_font_size * 0.8); // Above baseline (most of glyph height)
    track_y!(fw_label_y + style.dimension_font_size * 0.2); // Below baseline (descenders)

    // Show actual frame width (not display width) in label
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle">Width: {}</text>"#,
        (fw_x1 + fw_x2) / 2.0, fw_label_y,
        dim_color, style.font_family, style.dimension_font_size,
        format_value(geometry.actual_frame_width, unit)
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
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.5" stroke="none" rx="1"/>"#,
        geometry.rabbet_area.x,
        geometry.rabbet_area.y,
        geometry.rabbet_area.width,
        geometry.rabbet_area.height,
        style.background_color
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
    let label_spacing = 16.0; // Reduced from 18

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
        let horiz_length = 10.0;
        let horiz_end_x = mat.right_edge + horiz_length;

        svg.push_str(&generate_line_with_arrows(
            mat.right_edge + 3.0, mat.center_y,
            horiz_end_x, mat.center_y,
            dim_color, style.extension_stroke_width * 0.7,
            true, false, true, // arrow_start only, is_leader
        ));

        // 2. Angled segment to label position
        // Use label_y directly - dominant-baseline="central" centers text at this position
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            horiz_end_x, mat.center_y,
            label_base_x - 5.0, label_y,
            dim_color, style.extension_stroke_width * 0.7
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
            mat.name, format_value(mat.thickness, unit)
        ));
        svg.push('\n');
    }

    // Total stack height dimension - vertical, positioned well to the right of labels
    let stack_top = geometry.glazing.y;
    let stack_bottom = geometry.backing.y + geometry.backing.height;

    // Estimate max label width more generously
    let char_width = style.dimension_font_size * 0.85 * 0.6;
    let max_label_len = materials.iter()
        .map(|m| format!("{}: {}", m.name, format_value(m.thickness, unit)).len())
        .max()
        .unwrap_or(10);
    let max_label_width = max_label_len as f64 * char_width;

    // Position stack dimension with clearance from labels (reduced for compact layout)
    let stack_dim_x = label_base_x + max_label_width + 20.0;

    // Find total stack callout
    let total_stack = callouts.iter().find(|c| c.dimension_type == super::types::DimensionType::TotalStackHeight);
    if let Some(callout) = total_stack {
        // Extension lines - start from after label area
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            label_base_x + max_label_width + 10.0, stack_top,
            stack_dim_x + EXTENSION_OVERSHOOT, stack_top,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            label_base_x + max_label_width + 10.0, stack_bottom,
            stack_dim_x + EXTENSION_OVERSHOOT, stack_bottom,
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
        track_x!(stack_label_x + style.dimension_font_size * 0.9 / 2.0);
        
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">{}</text>"#,
            stack_label_x, stack_label_y,
            dim_color, style.font_family, style.dimension_font_size * 0.9,
            stack_label_x, stack_label_y,
            callout.label.clone()
        ));
        svg.push('\n');
    }

    // Rabbet label - below the frame with leader from rabbet area
    let rabbet_label_y = frame_y + frame_h + 18.0;
    svg.push_str(&format!(
        r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}" stroke-dasharray="3,2"/>"#,
        rabbet_center_x, geometry.rabbet_area.y + rabbet_h + 2.0,
        rabbet_center_x, rabbet_label_y - 6.0,
        dim_color, style.extension_stroke_width
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
        format!("Rabbet: {}", format_value(design.rabbet_depth, unit))
    } else {
        // Non-square rabbet - show width × depth
        format!("Rabbet: {} × {}", format_value(design.rabbet_width, unit), format_value(design.rabbet_depth, unit))
    };

    let clearance_text = if geometry.has_interference() {
        format!("{} (INTERFERENCE: {})", rabbet_label, format_value(-geometry.clearance, unit))
    } else {
        format!("{} (clearance: {})", rabbet_label, format_value(geometry.clearance, unit))
    };

    // Combined rabbet + clearance on one line
    // Estimate text width to prevent clipping at left edge (rough estimate: ~6px per char at this font size)
    let estimated_text_width = clearance_text.len() as f64 * 6.0;
    let min_x_for_centering = estimated_text_width / 2.0 + 5.0; // 5px margin from edge

    let (text_x, text_anchor) = if rabbet_center_x >= min_x_for_centering {
        (rabbet_center_x, "middle")
    } else {
        (5.0, "start") // Left-align with small margin if centering would clip
    };

    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="{}">{}</text>"#,
        text_x, rabbet_label_y,
        indicator_color, style.font_family, style.label_font_size * 0.9,
        text_anchor, clearance_text
    ));
    svg.push('\n');

    svg.push_str("  </g>\n");

    // =================================================================
    // DYNAMIC VIEWBOX: Calculate legend bounds
    // =================================================================
    // Legend is positioned below content and centered on canvas
    let materials_count = if design.has_mat() { 5 } else { 4 };
    let item_width = 80.0;
    let total_width = materials_count as f64 * item_width;
    let legend_start_x = (options.canvas_width - total_width) / 2.0;
    let legend_end_x = legend_start_x + total_width;

    let content_bottom = geometry.bounds.bottom();
    let legend_gap = 8.0;
    let legend_y = content_bottom + legend_gap;
    let legend_bottom = legend_y + style.label_font_size * 0.9 * 1.2; // text height estimate

    // Calculate final bounds including legend
    let mut min_x = content_min_x.min(legend_start_x);
    let mut max_x = content_max_x.max(legend_end_x);
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
        let viewbox_end = svg[viewbox_start..].find('"').unwrap() + viewbox_start + 1;
        let after_viewbox = svg[viewbox_end..].find('"').map(|i| viewbox_end + i).unwrap_or(svg.len());

        // Build new SVG with dynamic viewBox
        let mut svg_with_dynamic_viewbox = String::new();
        svg_with_dynamic_viewbox.push_str(&svg[..viewbox_start]);
        svg_with_dynamic_viewbox.push_str(&format!(
            "viewBox=\"{:.2} {:.2} {:.2} {:.2}\"",
            min_x, min_y, viewbox_width, viewbox_height
        ));
        svg_with_dynamic_viewbox.push_str(&svg[after_viewbox..]);
        svg = svg_with_dynamic_viewbox;
    }

    // =================================================================
    // SELF-CENTERING: Apply horizontal centering transform
    // =================================================================
    // Calculate the horizontal offset needed to center the actual content
    // within the canvas. This dynamically adapts to content variations.
    let _content_width = content_max_x - content_min_x;
    let content_center_x = (content_min_x + content_max_x) / 2.0;
    let canvas_center_x = options.canvas_width / 2.0;
    let center_offset_x = canvas_center_x - content_center_x;

    // Build final SVG with centering transform wrapper
    let mut final_svg = String::new();
    
    // Copy everything up to and including the background rect
    // Find where the background rect ends (after "</rect>" for background)
    if let Some(bg_end) = svg.find(r#"width="100%" height="100%"/>"#) {
        // Find the end of that line
        let bg_line_end = svg[bg_end..].find('\n').map(|i| bg_end + i + 1).unwrap_or(svg.len());
        final_svg.push_str(&svg[..bg_line_end]);
        
        // Add centering transform wrapper around the content
        final_svg.push_str(&format!(
            "  <g id=\"section-content\" transform=\"translate({:.2}, 0)\">\n",
            center_offset_x
        ));
        
        // Add the rest of the content (geometry and dimensions groups)
        final_svg.push_str(&svg[bg_line_end..]);
        
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
        Some((content_min_x, content_max_x)),
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

/// Generate SVG for interior width dimension (horizontal, inside frame opening)
/// Positioned inside the frame opening with dog-leg support for narrow frames
#[allow(dead_code)]
fn svg_interior_width_dimension(
    callout: &super::types::DimensionCallout,
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();
    svg.push_str("  <g id=\"interior-width-dimension\">\n");

    // Use inside dimension color for interior dimensions
    let dim_color = &style.inside_dimension_color;

    // ALWAYS use frame_inner - this measures the frame's visible opening, not the mat window
    let inner_rect = &geometry.frame_inner;

    // Position inside the visible opening
    let left_x = inner_rect.left();
    let right_x = inner_rect.right();
    let span_width = right_x - left_x;

    // Position the dimension line inside the opening, below the top edge
    let offset = 25.0; // Fixed offset from top of visible opening
    let dim_y = inner_rect.top() + offset;

    // Short extension lines going DOWN from dimension line toward edge (contained within opening)
    let ext_top = dim_y - 8.0;  // Above dimension line
    let ext_bottom = dim_y + 4.0; // Below dimension line

    // Estimate label width
    let char_width = style.dimension_font_size * 0.6;
    let label_width = callout.label.len() as f64 * char_width;
    let label_padding = 30.0;
    let min_span_for_label = label_width + label_padding;

    let use_dogleg = span_width < min_span_for_label;

    if use_dogleg {
        // Dog-leg style: arrows point INWARD toward the measured span
        let center_x = (left_x + right_x) / 2.0;
        let dogleg_extension = (min_span_for_label - span_width) / 2.0 + 20.0;

        // Short extension lines (ticks at boundaries)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            left_x, ext_top, left_x, ext_bottom,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            right_x, ext_top, right_x, ext_bottom,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dog-leg lines (no markers - we'll draw arrows explicitly)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            left_x - dogleg_extension, dim_y, left_x, dim_y,
            dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            right_x, dim_y, right_x + dogleg_extension, dim_y,
            dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');

        // Explicit arrow heads pointing INWARD toward the span
        // Left arrow at outer end, pointing RIGHT (toward center)
        let arrow_size = 6.0;
        let left_arrow_tip = left_x - dogleg_extension;
        svg.push_str(&format!(
            r#"    <path d="M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z" fill="{}"/>"#,
            left_arrow_tip, dim_y,
            left_arrow_tip - arrow_size, dim_y - arrow_size / 2.0,
            left_arrow_tip - arrow_size, dim_y + arrow_size / 2.0,
            dim_color
        ));
        svg.push('\n');

        // Right arrow at outer end, pointing LEFT (toward center)
        let right_arrow_tip = right_x + dogleg_extension;
        svg.push_str(&format!(
            r#"    <path d="M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z" fill="{}"/>"#,
            right_arrow_tip, dim_y,
            right_arrow_tip + arrow_size, dim_y - arrow_size / 2.0,
            right_arrow_tip + arrow_size, dim_y + arrow_size / 2.0,
            dim_color
        ));
        svg.push('\n');

        // Label centered on the line with mask
        let char_width = style.dimension_font_size * 0.55;
        let label_text_width = callout.label.len() as f64 * char_width;
        let mask_padding = 4.0;
        let mask_width = label_text_width + mask_padding * 2.0;
        let mask_height = style.dimension_font_size + 4.0;

        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
            center_x - mask_width / 2.0, dim_y - mask_height / 2.0,
            mask_width, mask_height,
            style.background_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" dominant-baseline="central">{}</text>"#,
            center_x, dim_y,
            dim_color, style.font_family, style.dimension_font_size,
            html_escape(&callout.label)
        ));
        svg.push('\n');
    } else {
        // Standard style: arrows point inward

        // Short extension lines (ticks at boundaries)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            left_x, ext_top, left_x, ext_bottom,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            right_x, ext_top, right_x, ext_bottom,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dimension line with inward arrows - tips land exactly at boundaries
        let int_w_line_x1 = arrow_line_endpoint_for_target(left_x, style.dimension_stroke_width, true);
        let int_w_line_x2 = arrow_line_endpoint_for_target(right_x, style.dimension_stroke_width, false);
        svg.push_str(&generate_line_with_arrows(
            int_w_line_x1, dim_y, int_w_line_x2, dim_y,
            &style.inside_dimension_color,
            style.dimension_stroke_width,
            true, true, false, // both arrows
        ));

        // Label centered on the line with mask
        let center_x = (left_x + right_x) / 2.0;
        let char_width = style.dimension_font_size * 0.55;
        let label_text_width = callout.label.len() as f64 * char_width;
        let mask_padding = 4.0;
        let mask_width = label_text_width + mask_padding * 2.0;
        let mask_height = style.dimension_font_size + 4.0;

        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
            center_x - mask_width / 2.0, dim_y - mask_height / 2.0,
            mask_width, mask_height,
            style.background_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" dominant-baseline="central">{}</text>"#,
            center_x, dim_y,
            dim_color, style.font_family, style.dimension_font_size,
            html_escape(&callout.label)
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
    svg
}

/// Generate SVG for interior height dimension (vertical, inside frame opening)
/// Positioned inside the frame opening with dog-leg support for short frames
#[allow(dead_code)]
fn svg_interior_height_dimension(
    callout: &super::types::DimensionCallout,
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();
    svg.push_str("  <g id=\"interior-height-dimension\">\n");

    // Use inside dimension color for interior dimensions
    let dim_color = &style.inside_dimension_color;

    // ALWAYS use frame_inner - this measures the frame's visible opening, not the mat window
    let inner_rect = &geometry.frame_inner;

    // Position inside the visible opening
    let top_y = inner_rect.top();
    let bottom_y = inner_rect.bottom();
    let span_height = bottom_y - top_y;

    // Position the dimension line inside the opening, to the right of left edge
    let offset = 25.0;
    let dim_x = inner_rect.left() + offset;

    // Short extension lines contained within opening
    let ext_left = dim_x - 8.0;  // Left of dimension line
    let ext_right = dim_x + 4.0; // Right of dimension line

    // Estimate label height (for vertical text, width becomes height)
    let char_width = style.dimension_font_size * 0.6;
    let label_width = callout.label.len() as f64 * char_width;
    let label_padding = 30.0;
    let min_span_for_label = label_width + label_padding;

    let use_dogleg = span_height < min_span_for_label;

    if use_dogleg {
        // Dog-leg style: arrows point INWARD toward the measured span
        let center_y = (top_y + bottom_y) / 2.0;
        let dogleg_extension = (min_span_for_label - span_height) / 2.0 + 20.0;

        // Short extension lines (ticks at boundaries)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_left, top_y, ext_right, top_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_left, bottom_y, ext_right, bottom_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dog-leg lines (no markers - we'll draw arrows explicitly)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            dim_x, top_y - dogleg_extension, dim_x, top_y,
            dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            dim_x, bottom_y, dim_x, bottom_y + dogleg_extension,
            dim_color, style.dimension_stroke_width
        ));
        svg.push('\n');

        // Explicit arrow heads pointing INWARD toward the span
        // Top arrow at outer end, pointing DOWN (toward center)
        let arrow_size = 6.0;
        let top_arrow_tip = top_y - dogleg_extension;
        svg.push_str(&format!(
            r#"    <path d="M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z" fill="{}"/>"#,
            dim_x, top_arrow_tip,
            dim_x - arrow_size / 2.0, top_arrow_tip - arrow_size,
            dim_x + arrow_size / 2.0, top_arrow_tip - arrow_size,
            dim_color
        ));
        svg.push('\n');

        // Bottom arrow at outer end, pointing UP (toward center)
        let bottom_arrow_tip = bottom_y + dogleg_extension;
        svg.push_str(&format!(
            r#"    <path d="M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z" fill="{}"/>"#,
            dim_x, bottom_arrow_tip,
            dim_x - arrow_size / 2.0, bottom_arrow_tip + arrow_size,
            dim_x + arrow_size / 2.0, bottom_arrow_tip + arrow_size,
            dim_color
        ));
        svg.push('\n');

        // Label centered on the line with mask, rotated 90°
        let char_width = style.dimension_font_size * 0.55;
        let label_text_width = callout.label.len() as f64 * char_width;
        let mask_padding = 4.0;
        let mask_width = label_text_width + mask_padding * 2.0;
        let mask_height = style.dimension_font_size + 4.0;

        // For vertical text, swap dimensions for the mask
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
            dim_x - mask_height / 2.0, center_y - mask_width / 2.0,
            mask_height, mask_width,
            style.background_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" dominant-baseline="central" transform="rotate(90 {:.2} {:.2})">{}</text>"#,
            dim_x, center_y,
            dim_color, style.font_family, style.dimension_font_size,
            dim_x, center_y,
            html_escape(&callout.label)
        ));
        svg.push('\n');
    } else {
        // Standard style: arrows point inward

        // Short extension lines (ticks at boundaries)
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_left, top_y, ext_right, top_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
            ext_left, bottom_y, ext_right, bottom_y,
            dim_color, style.extension_stroke_width
        ));
        svg.push('\n');

        // Dimension line with inward arrows (vertical) - tips land exactly at boundaries
        let int_h_line_y1 = arrow_line_endpoint_for_target_y(top_y, style.dimension_stroke_width, true);
        let int_h_line_y2 = arrow_line_endpoint_for_target_y(bottom_y, style.dimension_stroke_width, false);
        svg.push_str(&generate_line_with_arrows(
            dim_x, int_h_line_y1, dim_x, int_h_line_y2,
            &style.inside_dimension_color,
            style.dimension_stroke_width,
            true, true, false, // both arrows
        ));

        // Label centered on the line with mask, rotated 90°
        let center_y = (top_y + bottom_y) / 2.0;
        let char_width = style.dimension_font_size * 0.55;
        let label_text_width = callout.label.len() as f64 * char_width;
        let mask_padding = 4.0;
        let mask_width = label_text_width + mask_padding * 2.0;
        let mask_height = style.dimension_font_size + 4.0;

        // For vertical text, swap dimensions for the mask
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
            dim_x - mask_height / 2.0, center_y - mask_width / 2.0,
            mask_height, mask_width,
            style.background_color
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}" text-anchor="middle" dominant-baseline="central" transform="rotate(90 {:.2} {:.2})">{}</text>"#,
            dim_x, center_y,
            dim_color, style.font_family, style.dimension_font_size,
            dim_x, center_y,
            html_escape(&callout.label)
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
    svg
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
        DimensionType::FrameInsideWidth
        | DimensionType::FrameInsideHeight
        | DimensionType::FrameInsideWidthInterior
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
        let ext_end_y = if going_up { dim_y - EXTENSION_OVERSHOOT } else { dim_y + EXTENSION_OVERSHOOT };

        // Extension lines - special case for MatCutWidth: both lines extend to same y-value
        // at the mat opening's bottom edge (with small offset)
        let (mat_cut_ext_start_y, mat_cut_ext_end_y) = if callout.callout.dimension_type == super::types::DimensionType::MatCutWidth {
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
        let line_x1 = arrow_line_endpoint_for_target(callout.callout.extent_start.x, style.dimension_stroke_width, true);
        let line_x2 = arrow_line_endpoint_for_target(callout.callout.extent_end.x, style.dimension_stroke_width, false);
        if style.use_tick_marks {
            // Plain line (tick marks added separately)
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                line_x1, dim_y,
                line_x2, dim_y,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        } else {
            // Line with inline arrow polygons
            let arrow_svg = generate_line_with_arrows(
                line_x1, dim_y,
                line_x2, dim_y,
                dim_color, style.dimension_stroke_width,
                true, true, false, // both arrows
            );
            // Indent the output to match context
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
        let ext_end_x = if going_right { dim_x + EXTENSION_OVERSHOOT } else { dim_x - EXTENSION_OVERSHOOT };

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
        let line_y1 = arrow_line_endpoint_for_target_y(callout.callout.extent_start.y, style.dimension_stroke_width, true);
        let line_y2 = arrow_line_endpoint_for_target_y(callout.callout.extent_end.y, style.dimension_stroke_width, false);
        if style.use_tick_marks {
            // Plain line (tick marks added separately)
            svg.push_str(&format!(
                r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
                dim_x, line_y1,
                dim_x, line_y2,
                dim_color, style.dimension_stroke_width
            ));
            svg.push('\n');
        } else {
            // Line with inline arrow polygons
            let arrow_svg = generate_line_with_arrows(
                dim_x, line_y1,
                dim_x, line_y2,
                dim_color, style.dimension_stroke_width,
                true, true, false, // both arrows
            );
            // Indent the output to match context
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
    let char_width = style.dimension_font_size * 0.55;
    let label_text_width = callout.callout.label.len() as f64 * char_width;
    let mask_padding_x = 4.0;  // Horizontal padding around text
    let mask_padding_y = 2.0;  // Vertical padding around text
    let mask_width = label_text_width + mask_padding_x * 2.0;
    let mask_height = style.dimension_font_size + mask_padding_y * 2.0;

    let (label_x, label_y, offset_applied) = if is_horizontal {
        // Horizontal dimension: label centered on the dimension line
        let mid_x = (callout.callout.extent_start.x + callout.callout.extent_end.x) / 2.0;
        let base_y = callout.dimension_line_position;

        // Mat cut width labels need extra padding from extension lines
        // Calculate offset based on scaled properties (automatically adapts to combined vs inline view)
        let mat_cut_offset = EXTENSION_OVERSHOOT + style.dimension_font_size / 2.0 + style.dimension_offset_base;
        let (label_y, offset) = if callout.callout.dimension_type == super::types::DimensionType::MatCutWidth {
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
        let mat_cut_offset = EXTENSION_OVERSHOOT + style.dimension_font_size / 2.0 + style.dimension_offset_base;
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
        dim_color, style.font_family, style.dimension_font_size,
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

    let item_width = 80.0;
    let total_width = materials.len() as f64 * item_width;

    // Center legend relative to content bounds (for dynamic viewBox) or canvas (for fixed viewBox)
    let start_x = if let Some((min_x, max_x)) = content_bounds_x {
        let content_center = (min_x + max_x) / 2.0;
        content_center - total_width / 2.0
    } else {
        (canvas_width - total_width) / 2.0
    };

    // Position legend tightly below the content bounds
    let content_bottom = geometry.bounds.bottom();
    let legend_gap = 8.0; // Tight gap between content and legend
    let legend_y = content_bottom + legend_gap;

    for (i, (name, pattern)) in materials.iter().enumerate() {
        let x = start_x + i as f64 * item_width;
        let fill = get_fill_for_pattern(pattern);
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="12" height="12" fill="{}" stroke="{}" stroke-width="0.5"/>"#,
            x, legend_y - 10.0, fill, style.line_color
        ));
        svg.push_str(&format!(
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}">{}</text>"#,
            x + 16.0, legend_y, style.dimension_color, style.font_family, style.label_font_size * 0.9, name
        ));
        svg.push('\n');
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

    let title = "Frame Design";
    let (outside_h, outside_w) = design.get_frame_outside_dimensions();
    let subtitle = format!(
        "{:.2}\" × {:.2}\" outside",
        outside_h, outside_w
    );

    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="25" fill="{}" font-family="{}" font-size="{}" font-weight="bold">{}</text>"#,
        options.canvas_width / 2.0, style.line_color, style.font_family, style.title_font_size, title
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <text x="{:.2}" y="45" fill="{}" font-family="{}" font-size="{}" text-anchor="middle">{}</text>"#,
        options.canvas_width / 2.0, style.dimension_color, style.font_family, style.label_font_size, subtitle
    ));
    svg.push('\n');

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
        println!("SECTION SVG:\n{}", result.svg);
    }

    #[test]
    fn test_plan_svg_output() {
        let design = test_design();
        let options = DiagramOptions::default();

        let result = generate_diagram(&design, &options);
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
}
