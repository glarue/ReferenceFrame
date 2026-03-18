// SVG generation for frame diagram
//
// Generates professional, warm-aesthetic SVG diagrams from
// frame designs with adaptive dimension callouts.

use crate::frame::FrameDesign;
use super::types::{
    DiagramOptions, DiagramResult, ViewOption,
    Rect, Side,
};
use super::style::DiagramStyle;
use super::geometry::{PlanViewGeometry, SectionViewGeometry};
use super::callouts::{generate_plan_callouts, generate_section_callouts};
use super::layout::{layout_plan_callouts, LayoutResult};
use super::collision::{self, FlexElement, ElementId, FlexRule, Axis};
use super::svg_util::*;
use super::section_svg::{build_section_svg, generate_title_block};
use super::plan_svg::build_plan_svg;

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
    let mut geometry = if options.show_callouts {
        PlanViewGeometry::from_design_with_mode(
            design,
            options.canvas_width,
            options.canvas_height,
            style,
            options.detail_mode,
            options.corner_detail_enabled,
            options.axis_breaks_enabled,
            options.unit_mm,
            options.use_tape_segments,
            options.use_decimal_display,
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
    let (callouts, mut layout) = if options.show_callouts {
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

    // Post-layout collision pass: detect and resolve overlaps between
    // corner detail, arrow stubs, callout labels, and thumbnail.
    run_collision_pass(&mut geometry, &mut layout, style);

    let svg = build_plan_svg(design, &geometry, &callouts, &layout, options, style);
    let frame_center_x = Some(geometry.frame_outer.center().x);

    DiagramResult {
        svg,
        warnings: layout.warnings,
        frame_center_x,
    }
}

/// Post-layout collision pass: collect positioned elements, detect overlaps,
/// and shift flexible elements to resolve collisions.
fn run_collision_pass(
    geometry: &mut PlanViewGeometry,
    layout: &mut LayoutResult,
    style: &DiagramStyle,
) {
    let mut elements: Vec<FlexElement> = Vec::new();

    let arrow_tip_size = arrow_geometry::tip_extension(style.dimension_stroke_width);
    let stub_len = arrow_tip_size * 2.5;

    // Collect arrow stub rects from bottom callouts in tight-space mode
    for (i, pc) in layout.positioned_callouts.iter().enumerate() {
        if pc.actual_side != Side::Bottom {
            continue;
        }
        let extent_span = (pc.callout.extent_end.x - pc.callout.extent_start.x).abs();
        let tight_space = extent_span < arrow_tip_size * TIGHT_SPACE_MULTIPLIER;
        if !tight_space {
            continue;
        }

        // Left stub: extends leftward from extent_start.x
        let dim_y = pc.dimension_line_position;
        let stub_height = arrow_tip_size * 2.0; // approximate visual height of arrow
        elements.push(FlexElement {
            id: ElementId::ArrowStub { callout: i, side: Side::Left },
            bounds: Rect::new(
                pc.callout.extent_start.x - stub_len,
                dim_y - stub_height / 2.0,
                stub_len,
                stub_height,
            ),
            flex: FlexRule::None, // stubs are immovable
            priority: 0,
        });

        // Right stub: extends rightward from extent_end.x
        elements.push(FlexElement {
            id: ElementId::ArrowStub { callout: i, side: Side::Right },
            bounds: Rect::new(
                pc.callout.extent_end.x,
                dim_y - stub_height / 2.0,
                stub_len,
                stub_height,
            ),
            flex: FlexRule::None,
            priority: 0,
        });
    }

    // Corner detail box (can shift left to clear arrow stubs, or right to clear left-side callout labels)
    if let Some(cd) = &geometry.corner_detail {
        let max_shift_left = cd.box_rect.x - style.margin;
        // Allow rightward shifts up to the frame's vertical centerline so the corner detail
        // can clear a left-side callout label (e.g. MatCutHeight on portrait frames).
        let frame_center_x = geometry.frame_outer.x + geometry.frame_outer.width / 2.0;
        let max_shift_right = (frame_center_x - cd.box_rect.right()).max(0.0);
        elements.push(FlexElement {
            id: ElementId::CornerDetail,
            bounds: cd.box_rect,
            flex: FlexRule::ShiftAxis {
                axis: Axis::X,
                range: (-max_shift_left, max_shift_right),
            },
            priority: 2,
        });
    }

    // Callout labels (can shift outward along their side's normal)
    for (i, pc) in layout.positioned_callouts.iter().enumerate() {
        let (axis, range) = match pc.actual_side {
            Side::Top => (Axis::Y, (-style.dimension_offset_step * 3.0, 0.0)),
            Side::Bottom => (Axis::Y, (0.0, style.dimension_offset_step * 3.0)),
            Side::Left => (Axis::X, (-style.dimension_offset_step * 3.0, 0.0)),
            Side::Right => (Axis::X, (0.0, style.dimension_offset_step * 3.0)),
        };
        elements.push(FlexElement {
            id: ElementId::Callout(i),
            bounds: pc.label_bounds,
            flex: FlexRule::ShiftAxis { axis, range },
            priority: 3,
        });
    }

    // Thumbnail: axis depends on orientation.
    // Landscape (below frame) → shift along X to dodge corner detail / mat cut.
    // Portrait (left of frame) → shift along Y to dodge corner detail.
    //
    // For portrait layout the text labels render BELOW the rect. Extend the collision
    // bounds downward to include those labels so the solver reserves enough clearance
    // above the corner detail for the rect AND its "Actual proportions" text.
    //
    // Portrait setup: left-side callout labels (e.g. MatCutHeight) share the same
    // x-band as the thumbnail. Their axis is X so they can't resolve a Y overlap —
    // the solver would oscillate. Instead:
    //   1. Pre-nudge the thumbnail below all left-side label bounds.
    //   2. Cap the upward flex range at the label floor so the solver can never push
    //      the thumbnail back into callout text.
    //   3. Extend collision bounds downward to include "Actual proportions" text labels.
    //   4. The Callout↔Thumbnail skip below prevents residual oscillation.
    if let Some(thumb) = &mut geometry.thumbnail {
        let is_below_frame = thumb.top() >= geometry.frame_outer.bottom();
        if !is_below_frame {
            for pc in layout.positioned_callouts.iter() {
                if pc.actual_side == Side::Left {
                    let floor_y = pc.label_bounds.bottom() + style.margin;
                    if thumb.y < floor_y {
                        thumb.y = floor_y;
                    }
                }
            }
        }
    }
    if let Some(thumb) = &geometry.thumbnail {
        let is_below_frame = thumb.top() >= geometry.frame_outer.bottom();
        let tm = style.thumbnail_metrics();
        let text_below = if !is_below_frame { tm.text_below_height } else { 0.0 };
        let collision_bounds = Rect::new(thumb.x, thumb.y, thumb.width, thumb.height + text_below);
        let (axis, range) = if is_below_frame {
            let max_left = thumb.x - style.margin;
            (Axis::X, (-max_left, 50.0))
        } else {
            // Floor = max of left-side callout bottoms.
            // This prevents the solver from nudging the thumbnail back above callout labels
            // (whose X-axis flex can't resolve Y overlaps). CornerDetail↔Thumbnail collisions
            // are handled by the solver directly (those pairs aren't skipped).
            let callout_floor = layout.positioned_callouts.iter()
                .filter(|pc| pc.actual_side == Side::Left)
                .map(|pc| pc.label_bounds.bottom() + style.margin)
                .fold(style.margin, f64::max);
            let max_up = (thumb.y - callout_floor).max(0.0);
            (Axis::Y, (-max_up, 50.0))
        };
        elements.push(FlexElement {
            id: ElementId::Thumbnail,
            bounds: collision_bounds,
            flex: FlexRule::ShiftAxis { axis, range },
            priority: 4,
        });
    }

    if elements.len() < 2 {
        return;
    }

    // Run the resolver.
    // Build a side lookup for each callout index, so the skip closure can check
    // whether two callouts share the same flex axis.
    let callout_sides: Vec<Side> = layout.positioned_callouts.iter()
        .map(|pc| pc.actual_side)
        .collect();

    // Skip pairs whose single-axis flex can't resolve the overlap:
    //  • Callout↔Thumbnail: left/right callouts flex on X, portrait thumbnail on Y.
    //  • Cross-side Callout↔Callout: top/bottom flex on Y, left/right on X. Shifting
    //    along one axis doesn't clear an overlap on the perpendicular axis, causing
    //    cascading shifts that collapse dimension lines to the same position.
    let skip = |a: collision::ElementId, b: collision::ElementId| {
        use collision::ElementId::{Callout, Thumbnail};
        match (&a, &b) {
            (Callout(_), Thumbnail) | (Thumbnail, Callout(_)) => true,
            (Callout(i), Callout(j)) => {
                let si = callout_sides.get(*i).copied().unwrap_or(Side::Top);
                let sj = callout_sides.get(*j).copied().unwrap_or(Side::Top);
                si.is_horizontal() != sj.is_horizontal()
            }
            _ => false,
        }
    };
    let adjustments = collision::resolve(&mut elements, 4.0, 4, Some(&skip));

    // Apply adjustments
    let mut corner_detail_dx = 0.0_f64;
    for adj in &adjustments {
        match adj.id {
            ElementId::CornerDetail => {
                if let Some(cd) = &mut geometry.corner_detail {
                    corner_detail_dx = adj.new_bounds.x - cd.box_rect.x;
                    cd.box_rect = adj.new_bounds;
                    cd.corner_origin.x += corner_detail_dx;
                    geometry.annotation_bounds.corner_detail_box = Some(adj.new_bounds);
                }
            }
            ElementId::Thumbnail => {
                if let Some(thumb) = &mut geometry.thumbnail {
                    let dx = adj.new_bounds.x - thumb.x;
                    let dy = adj.new_bounds.y - thumb.y;
                    // Apply only the positional delta — new_bounds may have extended height
                    // for collision purposes but the actual rect size stays unchanged.
                    thumb.x += dx;
                    thumb.y += dy;
                    if let Some(ab) = &mut geometry.annotation_bounds.thumbnail_box {
                        ab.x += dx;
                        ab.y += dy;
                    }
                }
            }
            ElementId::Callout(idx) => {
                if let Some(pc) = layout.positioned_callouts.get_mut(idx) {
                    let dx = adj.new_bounds.x - pc.label_bounds.x;
                    let dy = adj.new_bounds.y - pc.label_bounds.y;
                    pc.label_bounds = adj.new_bounds;
                    pc.label_position.x += dx;
                    pc.label_position.y += dy;
                    pc.dimension_line_position += dy;
                }
            }
            ElementId::ArrowStub { .. } => {}
        }
    }

    // Re-center thumbnail between corner detail and mat cut annotation,
    // but only when they share the same horizontal band (landscape layout).
    // Guard: only apply when thumbnail is below the frame (landscape) — for portrait frames the
    // thumbnail is to the left and should never be repositioned horizontally here.
    if geometry.thumbnail_below {
    if let (Some(cd), Some(thumb)) = (&geometry.corner_detail, &geometry.thumbnail) {
        let v_overlap = thumb.top() < cd.box_rect.bottom() && thumb.bottom() > cd.box_rect.top();
        if v_overlap {
            let mat_cut_left = geometry.annotation_bounds.mat_cut_extent.as_ref().map(|(start, _)| start.x);
            if let Some(mat_left) = mat_cut_left {
                let corner_right = cd.box_rect.right();
                let mini_gap = 10.0;
                let avail = mat_left - corner_right - 2.0 * mini_gap;
                if avail >= thumb.width {
                    let new_x = corner_right + mini_gap + (avail - thumb.width) / 2.0;
                    let dx = new_x - thumb.x;
                    if dx.abs() > 0.5 {
                        if let Some(thumb) = &mut geometry.thumbnail {
                            thumb.x = new_x;
                        }
                        if let Some(ab) = &mut geometry.annotation_bounds.thumbnail_box {
                            ab.x += dx;
                        }
                    }
                }
            }
        }
    }
    } // end thumbnail_below guard

    // Post-collision fallback: if thumbnail + text labels still overlap the corner detail
    // (e.g. solver was margin-constrained on extreme-AR frames), shrink the rect to fit.
    // Text labels are a fixed size; only the rect height is reduced. Portrait only.
    if !geometry.thumbnail_below {
        if let (Some(thumb), Some(cd)) = (&geometry.thumbnail, geometry.corner_detail.as_ref()) {
            let tm = style.thumbnail_metrics();
            let text_below = tm.text_below_height;
            let clearance = 4.0;
            let available_h = cd.box_rect.top() - clearance - text_below - thumb.y;
            let min_thumb_h = 10.0;
            if available_h < thumb.height && available_h >= min_thumb_h {
                if let Some(thumb) = &mut geometry.thumbnail {
                    thumb.height = available_h;
                }
            }
        }
    }
}

/// Public re-export of the collision pass for snapshot testing.
pub fn run_collision_pass_for_snapshot(
    geometry: &mut PlanViewGeometry,
    layout: &mut LayoutResult,
    style: &DiagramStyle,
) {
    run_collision_pass(geometry, layout, style);
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
        frame_center_x: None,
    }
}

/// Generate combined view for PDF export
fn generate_combined_view(
    design: &FrameDesign,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> DiagramResult {
    // Vertical stacking: plan view (top), section view (bottom)
    const MIN_GAP: f64 = 8.0; // minimum inter-view gap, always preserved

    // Account for title block height if present
    let title_height = if options.include_title_block { TITLE_BLOCK_HEIGHT } else { 0.0 };

    // Available height budget (minimum gap and title reserved)
    let available_height = options.canvas_height - MIN_GAP - title_height;

    // Initial rough split used only to generate the SVG content.
    // The actual zone heights are derived from viewBox aspect ratios below.
    let plan_height_init = available_height * PLAN_HEIGHT_RATIO;
    let section_height_init = available_height * SECTION_HEIGHT_RATIO;

    // Use full PDF font sizes without scaling — dynamic viewBox handles fitting
    let mut plan_style = style.clone();
    plan_style.margin = 5.0;

    // Section view typically has narrower content, causing larger viewBox scaling.
    // Reduce font sizes proportionally so rendered sizes match between views.
    let mut section_style = style.clone();
    section_style.margin = 5.0;
    section_style.label_font_size = (style.label_font_size * SECTION_FONT_SCALE).round();
    section_style.dimension_offset_base = style.dimension_offset_base * SECTION_DIM_OFFSET_SCALE;
    section_style.dimension_offset_step = style.dimension_offset_step * SECTION_DIM_OFFSET_SCALE;

    let plan_options = DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_height: plan_height_init,
        ..options.clone()
    };
    let section_options = DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_height: section_height_init,
        ..options.clone()
    };

    // First pass: probe both views to determine content-aware zone heights.
    let plan_probe = generate_plan_view(design, &plan_options, &plan_style);
    let section_probe = generate_section_view(design, &section_options, &section_style);

    // Extract viewBoxes to determine content-aware zone heights.
    let plan_viewbox_probe = extract_viewbox(&plan_probe.svg);
    let section_viewbox_probe = extract_viewbox(&section_probe.svg);

    // Content-aware zone heights derived from viewBox aspect ratios.
    // Natural height = the height each view needs to fill canvas_width with no side whitespace.
    // Strategy: proportional scaling when both can't fit at natural size, flooring section at
    // 70% of its natural height so the legend stays readable for portrait frames.
    let (plan_zone_h, section_zone_h) = match (plan_viewbox_probe, section_viewbox_probe) {
        (Some((_, _, pvw, pvh)), Some((_, _, svw, svh)))
            if pvw > 0.0 && pvh > 0.0 && svw > 0.0 && svh > 0.0 =>
        {
            let plan_natural = options.canvas_width * pvh / pvw;
            let section_natural = options.canvas_width * svh / svw;

            if plan_natural + section_natural <= available_height {
                // Both views fit at natural size. Give plan view maximum available space so the
                // second-pass render uses more canvas height. For portrait (height-limited) frames
                // this increases the scale, giving more room for callouts and thumbnail while
                // shrinking the gap to MIN_GAP. For landscape frames the plan viewBox doesn't
                // grow proportionally, so the gap stays larger — but never worse than before.
                const SECTION_SCALE_CAP: f64 = 1.05; // section ≤ 5% above its init height
                const SECTION_SCALE_FLOOR: f64 = 0.70; // section ≥ 70% of init height (readability)
                let section_h = section_natural
                    .min(section_height_init * SECTION_SCALE_CAP)
                    .max(section_height_init * SECTION_SCALE_FLOOR);
                let plan_h = available_height - section_h;
                (plan_h, section_h)
            } else {
                // Section gets its full natural height (fills canvas_width); plan gets the rest.
                // Section content is roughly constant across frame sizes, so a hard floor at
                // section_natural prevents it from being horizontally squished on portrait frames.
                let section_h = section_natural.min(available_height * 0.50);
                let plan_h = (available_height - section_h).max(available_height * 0.25);
                (plan_h, section_h)
            }
        }
        _ => (plan_height_init, section_height_init), // fallback: fixed split
    };

    // Second pass: re-generate both views at their actual zone heights so geometry
    // (axis breaks, thumbnail placement, corner detail) is computed for the real
    // available space rather than the initial probe estimate.
    let plan_result = if (plan_zone_h - plan_height_init).abs() > 5.0 {
        let plan_options_final = DiagramOptions {
            view: ViewOption::PlanOnly,
            canvas_height: plan_zone_h,
            ..options.clone()
        };
        generate_plan_view(design, &plan_options_final, &plan_style)
    } else {
        plan_probe
    };
    let plan_viewbox = extract_viewbox(&plan_result.svg);

    // ViewBox-centered plan height: scale so plan content fills (canvas_width - 2×margin)
    // horizontally, centering the content bounding box with equal margins on both sides.
    let plan_render_h = match plan_viewbox {
        Some((_, _, pvw, pvh)) if pvw > 0.0 => {
            let h = pvh * (options.canvas_width - 2.0 * style.margin) / pvw;
            // Cap so combined views leave at least MIN_GAP
            h.min(available_height - section_zone_h)
        }
        _ => plan_zone_h,
    };

    // Dynamic gap: absorbs leftover space so combined SVG fills the canvas height.
    let gap_between_views = MIN_GAP + (available_height - plan_render_h - section_zone_h).max(0.0);

    let section_result = if (section_zone_h - section_height_init).abs() > 5.0 {
        let section_options_final = DiagramOptions {
            view: ViewOption::SectionOnly,
            canvas_height: section_zone_h,
            ..options.clone()
        };
        generate_section_view(design, &section_options_final, &section_style)
    } else {
        section_probe
    };
    let section_viewbox = extract_viewbox(&section_result.svg);

    // Combined SVG height matches actual content — eliminates dead space at bottom
    let combined_h = title_height + plan_render_h + gap_between_views + section_zone_h;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg viewBox="0 0 {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">"#,
        options.canvas_width, combined_h
    ));
    svg.push('\n');

    if options.include_title_block {
        svg.push_str(&generate_title_block(design, options, style));
    }

    // Plan view — center content bounding box for equal margins on both sides.
    let plan_content = extract_svg_content(&plan_result.svg);
    if let Some((vx, vy, vw, vh)) = plan_viewbox {
        let (tx, ty, scale) = calculate_fit_transform(
            vx, vy, vw, vh,
            0.0, title_height, options.canvas_width, plan_render_h,
            true,
            None,
        );
        svg.push_str(&format!(
            r#"  <g id="plan-view" transform="translate({:.2}, {:.2}) scale({:.4})">{}</g>"#,
            tx, ty, scale, plan_content
        ));
    } else {
        svg.push_str(&format!(r#"  <g id="plan-view">{}</g>"#, plan_content));
    }
    svg.push('\n');

    // Section view — viewBox-centered (section is symmetric, frame center not needed)
    let section_y = title_height + plan_render_h + gap_between_views;
    let section_content = extract_svg_content(&section_result.svg);
    if let Some((vx, vy, vw, vh)) = section_viewbox {
        let (tx, ty, scale) = calculate_fit_transform(
            vx, vy, vw, vh,
            0.0, section_y, options.canvas_width, section_zone_h,
            true,
            None,
        );
        svg.push_str(&format!(
            r#"  <g id="section-view" transform="translate({:.2}, {:.2}) scale({:.4})">{}</g>"#,
            tx, ty, scale, section_content
        ));
    } else {
        svg.push_str(&format!(
            r#"  <g id="section-view" transform="translate(0, {:.2})">{}</g>"#,
            section_y, section_content
        ));
    }
    svg.push('\n');

    svg.push_str("</svg>");

    let mut warnings = plan_result.warnings;
    warnings.extend(section_result.warnings);

    DiagramResult { svg, warnings, frame_center_x: None }
}

/// Calculate transform (tx, ty, scale) to fit a source rect into a target rect.
/// Preserves aspect ratio (meet).
/// - align_top: aligns to top of target (YMin) if true, else centers vertically (YMid)
/// - frame_center_x: if Some, horizontally centers the frame body (not the viewBox midpoint)
///   in the dest rect, clamped to keep content within bounds.
fn calculate_fit_transform(
    src_x: f64, src_y: f64, src_w: f64, src_h: f64,
    dest_x: f64, dest_y: f64, dest_w: f64, dest_h: f64,
    align_top: bool,
    frame_center_x: Option<f64>,
) -> (f64, f64, f64) {
    if src_w <= 0.0 || src_h <= 0.0 || dest_w <= 0.0 || dest_h <= 0.0 {
        return (dest_x, dest_y, 1.0);
    }
    if !src_w.is_finite() || !src_h.is_finite() || !dest_w.is_finite() || !dest_h.is_finite() {
        return (dest_x, dest_y, 1.0);
    }

    let scale_x = dest_w / src_w;
    let scale_y = dest_h / src_h;
    let scale = scale_x.min(scale_y);

    let new_w = src_w * scale;
    let new_h = src_h * scale;

    // Horizontal: center frame body if available, else center viewBox (XMid)
    let offset_x = if let Some(fc_x) = frame_center_x {
        // Place the scaled frame center at the horizontal midpoint of dest
        let raw = dest_w / 2.0 - scale * (fc_x - src_x);
        // Clamp so content stays within dest bounds
        raw.max(0.0).min(dest_w - new_w)
    } else {
        (dest_w - new_w) / 2.0
    };

    let offset_y = if align_top {
        0.0
    } else {
        (dest_h - new_h) / 2.0
    };

    let tx = dest_x + offset_x - scale * src_x;
    let ty = dest_y + offset_y - scale * src_y;

    if !tx.is_finite() || !ty.is_finite() || !scale.is_finite() {
        return (dest_x, dest_y, 1.0);
    }

    (tx, ty, scale)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::test_helpers::test_design;

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
    fn test_mat_cut_height_label_present() {
        // Portrait frame with mat — should produce a Mat Cut height label on the left
        let mut design = FrameDesign::new(18.0, 32.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 1.0;
        design.frame_material_width = 0.75;
        design.rabbet_width = 0.375;
        let options = DiagramOptions {
            view: ViewOption::PlanOnly,
            ..Default::default()
        };
        let result = generate_diagram(&design, &options);
        assert!(result.svg.contains("Mat Cut"), "SVG must contain 'Mat Cut' label text");
    }

    #[test]
    fn test_tape_measure_does_not_affect_axis_break() {
        // Combined view: tape measure formatting changes section viewBox, which shifts
        // plan canvas_height, potentially toggling axis breaks on/off. This tests that
        // the break decision is stable regardless of label formatting.
        let mut design = FrameDesign::new(32.0, 18.0); // portrait
        design.mat_width_top_bottom = 1.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 0.75;
        design.rabbet_width = 0.375;

        // Scan canvas heights around iPhone 15 Pro dimensions (width-32=361)
        let widths = [361.0, 380.0, 400.0];
        let heights: Vec<f64> = (550..=850).step_by(25).map(|h| h as f64).collect();
        let break_marker = r#"stroke-dasharray="4,3""#;
        let mut any_mismatch = false;
        for &cw in &widths {
            for &ch in &heights {
                let opts_no_tape = DiagramOptions {
                    view: ViewOption::Both,
                    canvas_width: cw, canvas_height: ch,
                    use_tape_segments: false,
                    ..Default::default()
                };
                let opts_tape = DiagramOptions {
                    view: ViewOption::Both,
                    canvas_width: cw, canvas_height: ch,
                    use_tape_segments: true,
                    ..Default::default()
                };
                let r1 = generate_diagram(&design, &opts_no_tape);
                let r2 = generate_diagram(&design, &opts_tape);
                let b1 = r1.svg.contains(break_marker);
                let b2 = r2.svg.contains(break_marker);
                if b1 != b2 {
                    eprintln!("MISMATCH {:.0}x{:.0}: no_tape={} tape={}", cw, ch, b1, b2);
                    any_mismatch = true;
                }
            }
        }
        assert!(!any_mismatch, "Tape measure formatting should not change axis break presence");
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
    fn test_extreme_ar_width_labels_not_collapsed() {
        // Regression: extreme portrait frames (100"×18") had cross-side callout collisions
        // that cascaded top-side width labels to the same y-position.
        let mut design = FrameDesign::new(100.0, 18.0);
        design.mat_width_top_bottom = 1.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 0.75;
        design.rabbet_width = 0.375;

        // Small canvas height (as in combined view) triggers the collision pass
        for ch in &[322.0, 400.0, 575.0] {
            let options = DiagramOptions {
                canvas_width: 329.0,
                canvas_height: *ch,
                view: ViewOption::PlanOnly,
                ..Default::default()
            };
            let result = generate_diagram(&design, &options);

            // Extract y-coordinates of non-rotated "Outside:" and "Inside:" labels
            let mut outside_ys: Vec<f64> = Vec::new();
            let mut inside_ys: Vec<f64> = Vec::new();
            for line in result.svg.lines() {
                let t: &str = line.trim();
                if t.contains("rotate(90") { continue; }
                if t.contains("Outside:") {
                    if let Some(y) = extract_translate_y(t) { outside_ys.push(y); }
                }
                if t.contains("Inside:") {
                    if let Some(y) = extract_translate_y(t) { inside_ys.push(y); }
                }
            }
            assert!(!outside_ys.is_empty(), "canvas_h={}: no Outside width label found", ch);
            assert!(!inside_ys.is_empty(), "canvas_h={}: no Inside width label found", ch);

            // The two labels must be at distinct y positions (not collapsed)
            let gap = (outside_ys[0] - inside_ys[0]).abs();
            assert!(gap > 10.0,
                "canvas_h={}: width labels collapsed (gap={:.1}px, Outside y={:.1}, Inside y={:.1})",
                ch, gap, outside_ys[0], inside_ys[0]);
        }
    }

    /// Extract the y value from a translate(x, y) transform attribute.
    fn extract_translate_y(svg_line: &str) -> Option<f64> {
        let start = svg_line.find("translate(")? + 10;
        let rest = &svg_line[start..];
        let end = rest.find(')')?;
        let coords = &rest[..end];
        let parts: Vec<&str> = coords.split(',').collect();
        if parts.len() == 2 {
            parts[1].trim().parse().ok()
        } else {
            None
        }
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
    #[ignore] // Run manually: cargo test --lib test_dump_asym_mat_portrait_svg -- --ignored --nocapture
    fn test_dump_asym_mat_portrait_svg() {
        // Reproduces the screenshot: 18×32 artwork (height×width), asymmetric mat (1" TB / 2" sides), 3/4" frame
        let mut design = FrameDesign::new(32.0, 18.0);  // height=32, width=18 → portrait
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 1.0;
        design.mat_width_sides = 2.0;
        design.rabbet_width = 0.375;

        // Use a small canvas that forces axis breaks (matching iPhone plan-only zone)
        let options = DiagramOptions {
            view: ViewOption::PlanOnly,
            canvas_width: 343.0,
            canvas_height: 260.0,  // small enough to force axis break on 35" portrait frame
            show_callouts: true,
            ..Default::default()
        };

        let result = generate_diagram(&design, &options);
        std::fs::write("/tmp/plan_asym_mat_portrait.svg", &result.svg).unwrap();
        eprintln!("SVG written to /tmp/plan_asym_mat_portrait.svg ({} bytes)", result.svg.len());
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

#[cfg(test)]
mod thumbnail_scale_tests {
    use super::*;
    use crate::frame::FrameDesign;

    fn test_frame_18x32() -> FrameDesign {
        let mut d = FrameDesign::new(18.0, 32.0);
        d.mat_width_top_bottom = 2.125;
        d.mat_width_sides = 2.125;
        d.mat_overlap = 0.25;
        d.frame_material_width = 0.75;
        d.rabbet_width = 0.375;
        d
    }

    fn thumb_screen_size(design: &FrameDesign, canvas_w: f64, canvas_h: f64, style: &DiagramStyle) -> f64 {
        // Replicate generate_combined_view zone calculation to get the real plan scale.
        let gap = 30.0;
        let available_h = canvas_h - gap;
        let plan_h_init = available_h * 0.58;
        let section_h_init = available_h * 0.42;
        let plan_opts = DiagramOptions {
            view: ViewOption::PlanOnly,
            canvas_width: canvas_w,
            canvas_height: plan_h_init,
            ..DiagramOptions::default()
        };
        let section_opts = DiagramOptions {
            view: ViewOption::SectionOnly,
            canvas_width: canvas_w,
            canvas_height: section_h_init,
            ..DiagramOptions::default()
        };
        let plan_result = generate_diagram_with_style(design, &plan_opts, style);
        let section_probe = generate_diagram_with_style(design, &section_opts, style);
        let plan_vb = extract_viewbox(&plan_result.svg);
        let section_vb = extract_viewbox(&section_probe.svg);
        let (pvw, pvh, svw, svh) = match (plan_vb, section_vb) {
            (Some((_, _, pw, ph)), Some((_, _, sw, sh))) if pw > 0.0 && ph > 0.0 && sw > 0.0 && sh > 0.0 => (pw, ph, sw, sh),
            _ => return 0.0,
        };
        let plan_natural = canvas_w * pvh / pvw;
        let section_natural = canvas_w * svh / svw;
        let plan_zone_h = if plan_natural + section_natural <= available_h {
            plan_natural
        } else {
            let scale = available_h / (plan_natural + section_natural);
            let section_h = (section_natural * scale).max(section_natural * 0.70).min(available_h * 0.50);
            (available_h - section_h).max(available_h * 0.25)
        };
        let plan_scale = (canvas_w / pvw).min(plan_zone_h / pvh);
        println!("  viewBox: {:.1}w x {:.1}h  plan_zone_h: {:.1}  scale: {:.3}  thumb: {:.1}px",
            pvw, pvh, plan_zone_h, plan_scale, 95.0 * plan_scale);
        95.0 * plan_scale
    }

    #[test]
    fn test_thumbnail_scale_portrait_vs_landscape() {
        let style = DiagramStyle::default();
        let canvas_w = 358.0_f64;
        let canvas_h = 638.0_f64;

        // Portrait (18w x 32h)
        let portrait = test_frame_18x32();
        print!("Portrait  18×32: ");
        let portrait_thumb = thumb_screen_size(&portrait, canvas_w, canvas_h, &style);

        // Landscape (32w x 18h)
        let mut landscape = FrameDesign::new(32.0, 18.0);
        landscape.mat_width_top_bottom = 2.125;
        landscape.mat_width_sides = 2.125;
        landscape.mat_overlap = 0.25;
        landscape.frame_material_width = 0.75;
        landscape.rabbet_width = 0.375;
        print!("Landscape 32×18: ");
        let landscape_thumb = thumb_screen_size(&landscape, canvas_w, canvas_h, &style);

        let diff_pct = ((portrait_thumb - landscape_thumb) / portrait_thumb * 100.0).abs();
        println!("Difference: {:.1}%", diff_pct);
        // Expect thumbnails to be within 20% of each other (was ~45% before)
        assert!(diff_pct < 20.0, "Thumbnail size difference too large: {:.1}%", diff_pct);
    }

    #[test]
    fn test_viewbox_has_positive_dimensions() {
        // Smoke test: viewBox is valid for a typical equal-mat frame.
        let mut d = FrameDesign::new(12.0, 8.0);
        d.mat_width_top_bottom = 2.375;
        d.mat_width_sides = 2.375;
        d.mat_overlap = 0.25;
        d.frame_material_width = 0.75;
        d.rabbet_width = 0.375;

        let mut plan_style = DiagramStyle::default();
        plan_style.margin = 5.0;

        let opts = DiagramOptions {
            view: ViewOption::PlanOnly,
            canvas_width: 393.0,
            canvas_height: 406.0,
            ..DiagramOptions::default()
        };
        let result = generate_diagram_with_style(&d, &opts, &plan_style);
        let (_, _, vw, vh) = extract_viewbox(&result.svg).unwrap();
        assert!(vw > 0.0 && vh > 0.0);
        assert!(result.frame_center_x.is_some());
    }
}
