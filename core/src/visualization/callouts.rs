// Dimension callout generation
//
// Generates dimension callouts from a FrameDesign, assigning
// priorities and preferred placement sides.

use crate::frame::FrameDesign;
use crate::conversions::{format_dimension, Unit};
use super::types::{
    DimensionCallout, DimensionType, Point,
};
use super::geometry::PlanViewGeometry;
use super::style::DiagramStyle;

/// Tolerance for comparing mat cut dimensions (1/32").
const MAT_DIMENSION_TOLERANCE: f64 = 0.03125;

/// Generate all dimension callouts for a plan view
/// Shows essential dimensions for frame construction
/// Labels include descriptive prefixes (Outside:, Inside:, etc.)
pub fn generate_plan_callouts(
    design: &FrameDesign,
    geometry: &PlanViewGeometry,
    unit_mm: bool,
    use_tape_segments: bool,
    use_decimal: bool,
    style: &DiagramStyle,
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    // Helper closure for formatting dimensions
    let fmt = |value: f64| format_dimension(value, unit, use_tape_segments, use_decimal);

    // Stroke width adjustments - stroke is centered on the path
    // Visual edge is at rect_position ± stroke_width/2
    let frame_half_stroke = style.frame_stroke_width / 2.0;
    let mat_half_stroke = style.mat_stroke_width / 2.0;

    // Frame outside WIDTH - on top (primary dimension)
    // Extends from visual outer left edge to visual outer right edge
    let (frame_h, frame_w) = design.get_frame_outside_dimensions();
    callouts.push(DimensionCallout::new(
        frame_w,
        format!("Outside: {}", fmt(frame_w)),
        DimensionType::FrameOutsideWidth,
        Point::new(geometry.frame_outer.left() - frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
    ));

    // Frame outside HEIGHT - on right (primary dimension)
    // Extends from visual outer top edge to visual outer bottom edge
    callouts.push(DimensionCallout::new(
        frame_h,
        format!("Outside: {}", fmt(frame_h)),
        DimensionType::FrameOutsideHeight,
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.bottom() + frame_half_stroke),
    ));

    // Visible opening WIDTH - on top (combined with outside width)
    // Extends from visual inner left edge to visual inner right edge
    let (inside_h, inside_w) = design.get_frame_inside_dimensions();
    callouts.push(DimensionCallout::new(
        inside_w,
        format!("Inside: {}", fmt(inside_w)),
        DimensionType::FrameInsideWidthInterior,
        Point::new(geometry.frame_inner.left() + frame_half_stroke, geometry.frame_inner.top() + frame_half_stroke),
        Point::new(geometry.frame_inner.right() - frame_half_stroke, geometry.frame_inner.top() + frame_half_stroke),
    ));

    // Visible opening HEIGHT - on right
    // Extends from visual inner top edge to visual inner bottom edge
    callouts.push(DimensionCallout::new(
        inside_h,
        format!("Inside: {}", fmt(inside_h)),
        DimensionType::FrameInsideHeightInterior,
        Point::new(geometry.frame_inner.right() - frame_half_stroke, geometry.frame_inner.top() + frame_half_stroke),
        Point::new(geometry.frame_inner.right() - frame_half_stroke, geometry.frame_inner.bottom() - frame_half_stroke),
    ));

    // Mat cut dimensions (if mat is present)
    // Shows both width (left/right borders) and height (top/bottom borders) when they differ
    if design.has_mat() {
        if let Some(mat_opening) = &geometry.mat_opening {
            // Mat cut WIDTH (horizontal dimension, uses left/right borders)
            let mat_visible_sides = design.mat_width_sides;
            let mat_cut_width = mat_visible_sides + design.rabbet_width;
            // Use the pre-computed extent when available (two-pass: geometry.rs chose the side
            // before thumbnail placement so the decision is consistent with what was reserved).
            // Fall back to choose_mat_cut_extent for callers that didn't go through from_design.
            let (mat_cut_start, mat_cut_end) = if let Some((start, end)) = geometry.annotation_bounds.mat_cut_extent {
                (start, end)
            } else {
                PlanViewGeometry::choose_mat_cut_extent(
                    &geometry.frame_inner,
                    &geometry.content_area,
                    mat_opening,
                    &geometry.annotation_bounds.occupied_rects(),
                    style,
                )
            };
            callouts.push(DimensionCallout::new(
                mat_cut_width,
                format!("Mat Cut: {} ({} visible)",
                    fmt(mat_cut_width),
                    fmt(mat_visible_sides)),
                DimensionType::MatCutWidth,
                mat_cut_start,
                mat_cut_end,
            ));

            // Mat cut HEIGHT (vertical dimension, uses top/bottom borders) - only if different from width
            let mat_visible_tb = design.mat_width_top_bottom;
            if (mat_visible_tb - mat_visible_sides).abs() > MAT_DIMENSION_TOLERANCE {
                let mat_cut_height = mat_visible_tb + design.rabbet_width;
                callouts.push(DimensionCallout::new(
                    mat_cut_height,
                    format!("Mat Cut: {} ({} visible)",
                        fmt(mat_cut_height),
                        fmt(mat_visible_tb)),
                    DimensionType::MatCutHeight,  // Use MatCutHeight which has Side::Left preference
                    // Place extent at frame_outer.left() so extension lines stay in the left
                    // margin and don't cross through the frame corner material.
                    // Y span: content_area.top() → mat_opening.top() = mat_cut_height * scale.
                    Point::new(geometry.frame_outer.left() - frame_half_stroke, geometry.content_area.top()),
                    Point::new(geometry.frame_outer.left() - frame_half_stroke, mat_opening.top() + mat_half_stroke),
                ));
            }
        }
    }

    // Note: Frame width and Rabbet depth are shown in Section View
    // since they are depth/profile dimensions, not plan dimensions

    callouts
}

/// Generate callouts for section view
pub fn generate_section_callouts(
    design: &FrameDesign,
    unit_mm: bool,
    use_tape_segments: bool,
    use_decimal: bool,
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    // Helper closure for formatting dimensions
    let fmt = |value: f64| format_dimension(value, unit, use_tape_segments, use_decimal);

    // These will use placeholder positions - actual positions
    // calculated by SectionViewGeometry during layout

    // Frame depth
    callouts.push(DimensionCallout::new(
        design.frame_material_depth,
        fmt(design.frame_material_depth),
        DimensionType::FrameDepth,
        Point::new(0.0, 0.0), // Placeholder
        Point::new(0.0, 1.0),
    ));

    // Material stack
    callouts.push(DimensionCallout::new(
        design.glazing_thickness,
        format!("Glazing {}", fmt(design.glazing_thickness)),
        DimensionType::GlazingThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    if design.has_mat() {
        callouts.push(DimensionCallout::new(
            design.matboard_thickness,
            format!("Mat {}", fmt(design.matboard_thickness)),
            DimensionType::MatboardThickness,
            Point::new(0.0, 0.0),
            Point::new(0.0, 1.0),
        ));
    }

    callouts.push(DimensionCallout::new(
        design.artwork_thickness,
        format!("Artwork {}", fmt(design.artwork_thickness)),
        DimensionType::ArtworkThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    callouts.push(DimensionCallout::new(
        design.backing_thickness,
        format!("Backing {}", fmt(design.backing_thickness)),
        DimensionType::BackingThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    // Total stack and clearance
    let total_stack = design.get_rabbet_z_depth_required() - design.assembly_margin;
    callouts.push(DimensionCallout::new(
        total_stack,
        fmt(total_stack),  // Just the value, context is clear
        DimensionType::TotalStackHeight,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    let clearance = design.frame_material_depth - design.get_rabbet_z_depth_required();
    let clearance_label = if clearance >= 0.0 {
        format!("Clearance {}", fmt(clearance))
    } else {
        format!("INTERFERENCE {}", fmt(-clearance))
    };
    callouts.push(DimensionCallout::new(
        clearance.abs(),
        clearance_label,
        DimensionType::Clearance,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    callouts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::types::Side;
    use crate::visualization::style::DiagramStyle;
    use crate::visualization::test_helpers::test_design;

    fn filter_by_side(callouts: &[DimensionCallout], side: Side) -> Vec<&DimensionCallout> {
        callouts.iter().filter(|c| c.preferred_side == side).collect()
    }

    fn group_by_side(callouts: &[DimensionCallout]) -> (
        Vec<&DimensionCallout>, Vec<&DimensionCallout>,
        Vec<&DimensionCallout>, Vec<&DimensionCallout>,
    ) {
        (filter_by_side(callouts, Side::Top), filter_by_side(callouts, Side::Right),
         filter_by_side(callouts, Side::Bottom), filter_by_side(callouts, Side::Left))
    }

    fn sort_by_priority(callouts: &mut [&DimensionCallout]) {
        callouts.sort_by_key(|c| c.priority);
    }

    #[test]
    fn test_generate_plan_callouts() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        // Should have at least frame outside and inside dimensions
        assert!(callouts.len() >= 4);

        // Check that we have width and height callouts
        let has_width = callouts.iter().any(|c| c.dimension_type == DimensionType::FrameOutsideWidth);
        let has_height = callouts.iter().any(|c| c.dimension_type == DimensionType::FrameOutsideHeight);
        assert!(has_width);
        assert!(has_height);
    }

    #[test]
    fn test_generate_plan_callouts_no_mat() {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        // Should not have mat callouts
        let has_mat = callouts.iter().any(|c| c.dimension_type == DimensionType::MatOpeningWidth);
        assert!(!has_mat);
    }

    #[test]
    fn test_generate_section_callouts() {
        let design = test_design();
        let callouts = generate_section_callouts(&design, false, false, false);

        // Should have frame depth and material thicknesses
        assert!(callouts.len() >= 5);

        let has_depth = callouts.iter().any(|c| c.dimension_type == DimensionType::FrameDepth);
        assert!(has_depth);
    }

    #[test]
    fn test_group_by_side() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let (top, right, _bottom, _left) = group_by_side(&callouts);

        // Frame outside width on top
        assert!(top.iter().any(|c| c.dimension_type == DimensionType::FrameOutsideWidth));

        // Frame outside height on right (combined with inside height)
        assert!(right.iter().any(|c| c.dimension_type == DimensionType::FrameOutsideHeight));

        // Inside/visible width on top (combined with outside width)
        assert!(top.iter().any(|c| c.dimension_type == DimensionType::FrameInsideWidthInterior));

        // Inside/visible height on right (combined with outside height)
        assert!(right.iter().any(|c| c.dimension_type == DimensionType::FrameInsideHeightInterior));
    }

    #[test]
    fn test_sort_by_priority() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let mut refs: Vec<&DimensionCallout> = callouts.iter().collect();
        sort_by_priority(&mut refs);

        // First element should have lowest priority number (highest priority)
        if refs.len() >= 2 {
            assert!(refs[0].priority <= refs[1].priority);
        }
    }

    #[test]
    fn test_callout_labels_inches() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        // Labels should contain inch marks
        let frame_width_callout = callouts.iter()
            .find(|c| c.dimension_type == DimensionType::FrameOutsideWidth)
            .unwrap();
        assert!(frame_width_callout.label.contains('"'));
    }

    #[test]
    fn test_callout_labels_mm() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, true, false, false, &style);

        // Labels should contain mm
        let frame_width_callout = callouts.iter()
            .find(|c| c.dimension_type == DimensionType::FrameOutsideWidth)
            .unwrap();
        assert!(frame_width_callout.label.contains("mm"));
    }
}

#[test]
fn test_asymmetric_mat_callout_coords() {
    use crate::frame::FrameDesign;
    use crate::visualization::geometry::PlanViewGeometry;
    use crate::visualization::style::DiagramStyle;

    let mut design = FrameDesign::new(12.0, 16.0);
    design.mat_width_top_bottom = 3.0;
    design.mat_width_sides = 1.5;
    design.frame_material_width = 1.0;
    let style = DiagramStyle::default();
    let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

    let callouts = generate_plan_callouts(&design, &geo, false, false, false, &style);
    let height_callout = callouts.iter().find(|c| c.dimension_type == DimensionType::MatCutHeight);
    let width_callout  = callouts.iter().find(|c| c.dimension_type == DimensionType::MatCutWidth);

    let scale = geo.scale;
    let mat_cut_height = design.mat_width_top_bottom + design.rabbet_width; // 3.375"
    let mat_cut_width  = design.mat_width_sides + design.rabbet_width;      // 1.875"

    let hc = height_callout.expect("MatCutHeight callout must be present for asymmetric mat");
    let wc = width_callout.expect("MatCutWidth callout must be present");

    // Height callout: vertical span should equal mat_cut_height * scale (± stroke)
    let h_span = (hc.extent_end.y - hc.extent_start.y).abs();
    let expected_h = mat_cut_height * scale;
    assert!((h_span - expected_h).abs() < 2.0,
        "MatCutHeight span {:.2} should be ≈{:.2} (mat_cut_height * scale)", h_span, expected_h);

    // Width callout: horizontal span should equal mat_cut_width * scale (± stroke)
    let w_span = (wc.extent_end.x - wc.extent_start.x).abs();
    let expected_w = mat_cut_width * scale;
    assert!((w_span - expected_w).abs() < 2.0,
        "MatCutWidth span {:.2} should be ≈{:.2} (mat_cut_width * scale)", w_span, expected_w);
}

