//! Section-view SVG rendering.
//!
//! Contains `build_section_svg` (the largest section-view function),
//! `generate_section_legend`, and `generate_title_block`.
//! Extracted from `svg.rs` to keep that file focused on view orchestration.

use crate::frame::{FrameDesign, FrameStyle};
use crate::conversions::{format_dimension, Unit};
use super::types::{DiagramOptions, DimensionCallout, DimensionType};
use super::style::{DiagramStyle, FillPattern};
use super::geometry::{SectionViewGeometry, estimate_text_width};
use super::svg_util::*;

/// Build SVG string for section view
///
/// Shows frame L-shape profile with materials stacked vertically.
/// Layout: Frame on left, materials stack from top to bottom in rabbet area,
/// dog-leg labels to the right for clear text positioning.
///
/// This function uses a self-centering approach: content is rendered at its
/// natural coordinates, then the actual horizontal bounds are calculated and
/// a centering transform is applied to horizontally center the content.
pub(crate) fn build_section_svg(
    design: &FrameDesign,
    geometry: &SectionViewGeometry,
    callouts: &[DimensionCallout],
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
        render_zigzag_line_with_opacity(&mut svg, &top_zz, &style.line_color, break_line_width, 0.75);
        render_zigzag_line_with_opacity(&mut svg, &bottom_zz, &style.line_color, break_line_width, 0.75);
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
        render_zigzag_line_with_opacity(&mut svg, &top_zz, &style.line_color, break_line_width, 0.75);
        render_zigzag_line_with_opacity(&mut svg, &bot_zz, &style.line_color, break_line_width, 0.75);
        render_zigzag_line_with_opacity(&mut svg, &left_zz, &style.line_color, break_line_width, 0.75);
        render_zigzag_line_with_opacity(&mut svg, &right_zz, &style.line_color, break_line_width, 0.75);
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
        render_zigzag_line_with_opacity(&mut svg, &left_zz, &style.line_color, break_line_width, 0.75);
        render_zigzag_line_with_opacity(&mut svg, &right_zz, &style.line_color, break_line_width, 0.75);
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
    let dim_x = frame_x - style.section_depth_dim_offset;
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
        let break_center_y = (geometry.axis_break_start_y + geometry.axis_break_end_y) / 2.0;

        // Line from top arrow to break
        svg.push_str(&generate_line_with_arrows(
            dim_x, depth_line_y1, dim_x, break_center_y - SPARK_VERTICAL_HEIGHT / 2.0,
            dim_color, style.dimension_stroke_width,
            true, false, false, // arrow_start only
        ));
        render_spark_symbol(&mut svg, dim_x, break_center_y, false, dim_color, style.dimension_stroke_width);
        // Line from break to bottom arrow
        svg.push_str(&generate_line_with_arrows(
            dim_x, break_center_y + SPARK_VERTICAL_HEIGHT / 2.0, dim_x, depth_line_y2,
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
    let label_offset = style.label_offset();
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
        r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}px" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">Depth: {}</text>"#,
        depth_label_x, depth_label_y,
        dim_color, style.font_family, style.label_font_size,
        depth_label_x, depth_label_y,
        fmt(depth_value)
    ));
    svg.push('\n');

    // Frame width dimension (horizontal, at top)
    // Always spans from left edge to right edge (full display width)
    // Use same offset as calculated in geometry.rs for consistency
    let fw_y = frame_y - style.section_width_dim_offset;
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
        let break_center = (geometry.axis_break_start_x + geometry.axis_break_end_x) / 2.0;

        // Line from left arrow to break
        svg.push_str(&generate_line_with_arrows(
            width_line_x1, fw_y, break_center - SPARK_HORIZONTAL_WIDTH / 2.0, fw_y,
            dim_color, style.dimension_stroke_width,
            true, false, false, // arrow_start only
        ));
        render_spark_symbol(&mut svg, break_center, fw_y, true, dim_color, style.dimension_stroke_width);
        // Line from break to right arrow
        svg.push_str(&generate_line_with_arrows(
            break_center + SPARK_HORIZONTAL_WIDTH / 2.0, fw_y, width_line_x2, fw_y,
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
        r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px" text-anchor="middle">Width: {}</text>"#,
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

    // Material thickness labels with dog-leg leader lines.
    //
    // Material labels use a dog-leg leader line layout:
    //   1. Short horizontal segment with arrow from the material layer's right edge
    //   2. Angled connector line from the horizontal endpoint to the label position
    // This keeps labels clear of the stacked material geometry while
    // maintaining visual connection to each specific layer.
    //
    // Labels are evenly spaced in a vertical column to the right, centered
    // on the material stack midpoint for a balanced appearance.
    let base_offset = style.section_material_label_offset.min(geometry.scale * 0.4 + 12.0);
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

        // Label text — slightly smaller than primary labels (secondary/contextual role)
        // Position text so baseline is slightly below label_y (visual center)
        // This makes dog-leg line hit visual center regardless of baseline rendering
        let stack_label_font = style.label_font_size * (11.0 / 13.0);
        let text_y = label_y + stack_label_font * BASELINE_SHIFT_RATIO;
        svg.push_str(&format!(
            r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{:.1}px">{}: {}</text>"#,
            label_base_x, text_y,
            dim_color, style.font_family, stack_label_font,
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
            estimate_text_width(&label, style.material_label_font_size())
        })
        .fold(0.0_f64, |a, b| a.max(b));

    // Position stack dimension with clearance from labels
    let stack_dim_x = label_base_x + max_label_width + style.section_stack_dim_gap;

    // Find total stack callout
    let total_stack = callouts.iter().find(|c| c.dimension_type == DimensionType::TotalStackHeight);
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
            r#"    <text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}px" text-anchor="middle" transform="rotate(-90 {:.2} {:.2})">{}</text>"#,
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

    // Format rabbet dimensions. Sight-size/float frames have no lip over the
    // art, so we label the channel depth and note the absence of a lip rather
    // than a "Rabbet: W × D" overlap.
    let rabbet_label = match design.frame_style {
        // Sight-size/float still have a real rabbet — the lip grabs the
        // (oversized) glazing/backing; it just clears the artwork.
        FrameStyle::SightSize =>
            format!("Sight-size · rabbet {} × {} (clears art)",
                fmt(design.rabbet_width), fmt(design.rabbet_depth)),
        FrameStyle::Float =>
            format!("Float · rabbet {} × {}", fmt(design.rabbet_width), fmt(design.rabbet_depth)),
        FrameStyle::Rabbet if (design.rabbet_width - design.rabbet_depth).abs() < 0.001 =>
            // Square rabbet - just show one value
            format!("Rabbet: {}", fmt(design.rabbet_depth)),
        FrameStyle::Rabbet =>
            // Non-square rabbet - show width × depth
            format!("Rabbet: {} × {}", fmt(design.rabbet_width), fmt(design.rabbet_depth)),
    };

    // Clearance/interference text on separate line to avoid overlap with material labels
    let clearance_line = if geometry.has_interference() {
        format!("(INTERFERENCE: {})", fmt(-geometry.clearance))
    } else {
        format!("(clearance: {})", fmt(geometry.clearance))
    };

    // Estimate text width of rabbet label to prevent clipping at left edge
    let estimated_text_width = estimate_text_width(&rabbet_label, style.material_label_font_size());
    let min_x_for_centering = estimated_text_width / 2.0 + 5.0; // 5px margin from edge

    let (text_x, text_anchor) = if rabbet_center_x >= min_x_for_centering {
        (rabbet_center_x, "middle")
    } else {
        (5.0, "start") // Left-align with small margin if centering would clip
    };

    // Line spacing for two-line label
    let line_height = style.single_line_height();

    // Rabbet dimensions on first line
    svg.push_str(&format!(
        r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px" text-anchor="{}">{}</text>"#,
        text_x, rabbet_label_y,
        indicator_color, style.font_family, style.label_font_size,
        text_anchor, rabbet_label
    ));
    svg.push('\n');

    // Clearance/interference on second line
    svg.push_str(&format!(
        r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px" text-anchor="{}">{}</text>"#,
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
    let legend_y = content_bottom + geometry.legend_gap;
    let legend_bottom = legend_y + style.single_line_height();

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

/// Generate section view legend (horizontal layout positioned below content)
pub(crate) fn generate_section_legend(
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
    let legend_y = content_bottom + geometry.legend_gap;

    let mut current_x = start_x;
    for ((name, pattern), item_width) in materials.iter().zip(item_widths.iter()) {
        let fill = get_fill_for_pattern(pattern);
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
            current_x, legend_y - 10.0, LEGEND_SWATCH_SIZE, LEGEND_SWATCH_SIZE, fill, style.line_color, LEGEND_SWATCH_STROKE
        ));
        svg.push_str(&format!(
            r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px">{}</text>"#,
            current_x + LEGEND_SWATCH_SIZE + LEGEND_SWATCH_GAP, legend_y, style.dimension_color, style.font_family, style.label_font_size, name
        ));
        svg.push('\n');
        current_x += item_width;
    }

    svg.push_str("  </g>\n");
    svg
}

/// Generate title block
pub(crate) fn generate_title_block(
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
        r#"    <text transform="translate({:.2}, 30)" fill="{}" font-family="{}" font-size="{}px" font-weight="bold" text-anchor="middle">{}</text>"#,
        options.canvas_width / 2.0, style.line_color, style.font_family, style.title_font_size, title
    ));
    svg.push('\n');

    // Only show subtitle (dimensions) when using default title
    let has_custom_title = options.title_text
        .as_ref()
        .map_or(false, |t| !t.trim().is_empty());
    if !has_custom_title {
        let (outside_h, outside_w) = design.get_frame_outside_dimensions();
        let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };
        let fmt_dim = |v: f64| format_dimension(v, unit, false, options.use_decimal_display);
        let subtitle = format!("{} × {} outside", fmt_dim(outside_h), fmt_dim(outside_w));
        svg.push_str(&format!(
            r#"    <text transform="translate({:.2}, 70)" fill="{}" font-family="{}" font-size="{}px" text-anchor="middle">{}</text>"#,
            options.canvas_width / 2.0, style.dimension_color, style.font_family, style.label_font_size, subtitle
        ));
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
    svg
}
