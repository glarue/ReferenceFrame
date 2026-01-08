// Dimension callout generation
//
// Generates dimension callouts from a FrameDesign, assigning
// priorities and preferred placement sides.

use crate::frame::FrameDesign;
use crate::conversions::{format_value, Unit};
use super::types::{
    DimensionCallout, DimensionType, Point, Side,
};
use super::geometry::PlanViewGeometry;
use super::style::DiagramStyle;

/// Generate all dimension callouts for a plan view
/// Shows essential dimensions for frame construction
/// Labels include descriptive prefixes (Outside:, Inside:, etc.)
pub fn generate_plan_callouts(
    design: &FrameDesign,
    geometry: &PlanViewGeometry,
    unit_mm: bool,
    style: &DiagramStyle,
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };

    // Stroke width adjustments - stroke is centered on the path
    // Visual edge is at rect_position ± stroke_width/2
    let frame_half_stroke = style.frame_stroke_width / 2.0;
    let mat_half_stroke = style.mat_stroke_width / 2.0;

    // Frame outside WIDTH - on top (primary dimension)
    // Extends from visual outer left edge to visual outer right edge
    let (frame_h, frame_w) = design.get_frame_outside_dimensions();
    callouts.push(DimensionCallout::new(
        frame_w,
        format!("Outside: {}", format_value(frame_w, unit)),
        DimensionType::FrameOutsideWidth,
        Point::new(geometry.frame_outer.left() - frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
    ));

    // Frame outside HEIGHT - on right (primary dimension)
    // Extends from visual outer top edge to visual outer bottom edge
    callouts.push(DimensionCallout::new(
        frame_h,
        format!("Outside: {}", format_value(frame_h, unit)),
        DimensionType::FrameOutsideHeight,
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.top() - frame_half_stroke),
        Point::new(geometry.frame_outer.right() + frame_half_stroke, geometry.frame_outer.bottom() + frame_half_stroke),
    ));

    // Visible opening WIDTH - on top (combined with outside width)
    // Extends from visual inner left edge to visual inner right edge
    let (inside_h, inside_w) = design.get_frame_inside_dimensions();
    callouts.push(DimensionCallout::new(
        inside_w,
        format!("Inside: {}", format_value(inside_w, unit)),
        DimensionType::FrameInsideWidthInterior,
        Point::new(geometry.frame_inner.left() + frame_half_stroke, geometry.frame_inner.top() + frame_half_stroke),
        Point::new(geometry.frame_inner.right() - frame_half_stroke, geometry.frame_inner.top() + frame_half_stroke),
    ));

    // Visible opening HEIGHT - on right
    // Extends from visual inner top edge to visual inner bottom edge
    callouts.push(DimensionCallout::new(
        inside_h,
        format!("Inside: {}", format_value(inside_h, unit)),
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
            callouts.push(DimensionCallout::new(
                mat_cut_width,
                format!("Mat Cut: {} ({} visible)",
                    format_value(mat_cut_width, unit),
                    format_value(mat_visible_sides, unit)),
                DimensionType::MatCutWidth,
                Point::new(geometry.content_area.left(), geometry.frame_inner.bottom() - frame_half_stroke),
                Point::new(mat_opening.left() + mat_half_stroke, geometry.frame_inner.bottom() - frame_half_stroke),
            ));

            // Mat cut HEIGHT (vertical dimension, uses top/bottom borders) - only if different from width
            // Tolerance of 1/32" (0.03125) to avoid showing near-identical dimensions
            let mat_visible_tb = design.mat_width_top_bottom;
            if (mat_visible_tb - mat_visible_sides).abs() > 0.03125 {
                let mat_cut_height = mat_visible_tb + design.rabbet_width;
                callouts.push(DimensionCallout::new(
                    mat_cut_height,
                    format!("Mat Cut: {} ({} visible)",
                        format_value(mat_cut_height, unit),
                        format_value(mat_visible_tb, unit)),
                    DimensionType::MatVisibleHeight,  // Use height type for vertical callout
                    // Place on LEFT side to avoid collision with outside/inside callouts on right
                    Point::new(geometry.frame_inner.left() + frame_half_stroke, geometry.content_area.top()),
                    Point::new(geometry.frame_inner.left() + frame_half_stroke, mat_opening.top() + mat_half_stroke),
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
) -> Vec<DimensionCallout> {
    let mut callouts = Vec::new();
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };

    // These will use placeholder positions - actual positions
    // calculated by SectionViewGeometry during layout

    // Frame depth
    callouts.push(DimensionCallout::new(
        design.frame_material_depth,
        format_value(design.frame_material_depth, unit),
        DimensionType::FrameDepth,
        Point::new(0.0, 0.0), // Placeholder
        Point::new(0.0, 1.0),
    ));

    // Material stack
    callouts.push(DimensionCallout::new(
        design.glazing_thickness,
        format!("Glazing {}", format_value(design.glazing_thickness, unit)),
        DimensionType::GlazingThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    if design.has_mat() {
        callouts.push(DimensionCallout::new(
            design.matboard_thickness,
            format!("Mat {}", format_value(design.matboard_thickness, unit)),
            DimensionType::MatboardThickness,
            Point::new(0.0, 0.0),
            Point::new(0.0, 1.0),
        ));
    }

    callouts.push(DimensionCallout::new(
        design.artwork_thickness,
        format!("Artwork {}", format_value(design.artwork_thickness, unit)),
        DimensionType::ArtworkThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    callouts.push(DimensionCallout::new(
        design.backing_thickness,
        format!("Backing {}", format_value(design.backing_thickness, unit)),
        DimensionType::BackingThickness,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    // Total stack and clearance
    let total_stack = design.get_rabbet_z_depth_required() - design.assembly_margin;
    callouts.push(DimensionCallout::new(
        total_stack,
        format_value(total_stack, unit),  // Just the value, context is clear
        DimensionType::TotalStackHeight,
        Point::new(0.0, 0.0),
        Point::new(0.0, 1.0),
    ));

    let clearance = design.frame_material_depth - design.get_rabbet_z_depth_required();
    let clearance_label = if clearance >= 0.0 {
        format!("Clearance {}", format_value(clearance, unit))
    } else {
        format!("INTERFERENCE {}", format_value(-clearance, unit))
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

/// Filter callouts by side
pub fn filter_by_side(callouts: &[DimensionCallout], side: Side) -> Vec<&DimensionCallout> {
    callouts
        .iter()
        .filter(|c| c.preferred_side == side)
        .collect()
}

/// Group callouts by their preferred side
pub fn group_by_side(callouts: &[DimensionCallout]) -> (
    Vec<&DimensionCallout>, // Top
    Vec<&DimensionCallout>, // Right
    Vec<&DimensionCallout>, // Bottom
    Vec<&DimensionCallout>, // Left
) {
    let top = filter_by_side(callouts, Side::Top);
    let right = filter_by_side(callouts, Side::Right);
    let bottom = filter_by_side(callouts, Side::Bottom);
    let left = filter_by_side(callouts, Side::Left);
    (top, right, bottom, left)
}

/// Sort callouts by priority (highest priority first)
pub fn sort_by_priority(callouts: &mut [&DimensionCallout]) {
    callouts.sort_by_key(|c| c.priority);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::style::DiagramStyle;

    fn test_design() -> FrameDesign {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 1.0;
        design
    }

    #[test]
    fn test_generate_plan_callouts() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, &style);

        // Should not have mat callouts
        let has_mat = callouts.iter().any(|c| c.dimension_type == DimensionType::MatOpeningWidth);
        assert!(!has_mat);
    }

    #[test]
    fn test_generate_section_callouts() {
        let design = test_design();
        let callouts = generate_section_callouts(&design, false);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, &style);

        let (top, right, bottom, left) = group_by_side(&callouts);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, true, &style);

        // Labels should contain mm
        let frame_width_callout = callouts.iter()
            .find(|c| c.dimension_type == DimensionType::FrameOutsideWidth)
            .unwrap();
        assert!(frame_width_callout.label.contains("mm"));
    }
}
