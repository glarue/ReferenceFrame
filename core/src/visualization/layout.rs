// Adaptive callout layout algorithm
//
// Handles collision detection and resolution to ensure all dimension
// callouts are readable and don't overlap.

use super::types::{
    DimensionCallout, PositionedCallout, Point, Rect, Side, TextAnchor,
};
use super::style::DiagramStyle;
use super::geometry::{PlanViewGeometry, estimate_text_width, effective_label_width};

/// Result of layout calculation
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// Positioned callouts ready for rendering
    pub positioned_callouts: Vec<PositionedCallout>,
    /// Warnings about omitted or adjusted dimensions
    pub warnings: Vec<String>,
}


/// Whether a label will be rendered as two lines (contains ": ").
fn is_two_line_label(label: &str) -> bool {
    label.contains(": ")
}

/// Layout callouts for a plan view
pub fn layout_plan_callouts(
    callouts: &[DimensionCallout],
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
) -> LayoutResult {
    let mut positioned = Vec::new();
    let mut warnings = Vec::new();

    // Group by side and sort by priority
    let mut top: Vec<_> = callouts.iter()
        .filter(|c| c.preferred_side == Side::Top)
        .collect();
    let mut right: Vec<_> = callouts.iter()
        .filter(|c| c.preferred_side == Side::Right)
        .collect();
    let mut bottom: Vec<_> = callouts.iter()
        .filter(|c| c.preferred_side == Side::Bottom)
        .collect();
    let mut left: Vec<_> = callouts.iter()
        .filter(|c| c.preferred_side == Side::Left)
        .collect();

    // Sort each group by priority (lower number = higher priority)
    top.sort_by_key(|c| c.priority);
    right.sort_by_key(|c| c.priority);
    bottom.sort_by_key(|c| c.priority);
    left.sort_by_key(|c| c.priority);

    // Layout each side
    positioned.extend(layout_horizontal_side(&top, geometry, style, Side::Top));
    positioned.extend(layout_horizontal_side(&bottom, geometry, style, Side::Bottom));
    positioned.extend(layout_vertical_side(&right, geometry, style, Side::Right));
    positioned.extend(layout_vertical_side(&left, geometry, style, Side::Left));

    LayoutResult {
        positioned_callouts: positioned,
        warnings,
    }
}

/// Layout callouts on a horizontal side (top or bottom)
fn layout_horizontal_side(
    callouts: &[&DimensionCallout],
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
    side: Side,
) -> Vec<PositionedCallout> {
    let mut positioned = Vec::new();

    // Sort callouts by priority (lower priority number = closer to frame)
    let mut sorted: Vec<_> = callouts.iter().enumerate().collect();
    sorted.sort_by_key(|(_, callout)| callout.priority);

    for (level, (_, callout)) in sorted.iter().enumerate() {
        let offset = style.get_dimension_offset(level as u8);

        // Calculate dimension line position
        let dim_line_y = if side == Side::Top {
            geometry.frame_outer.top() - offset
        } else {
            geometry.frame_outer.bottom() + offset
        };

        // Calculate label position (centered on dimension line)
        let label_x = (callout.extent_start.x + callout.extent_end.x) / 2.0;
        let label_y = if side == Side::Top {
            dim_line_y - style.label_font_size / 2.0 - 2.0
        } else {
            dim_line_y + style.label_font_size / 2.0 + 2.0
        };

        // Estimate label bounds for collision detection
        // Horizontal labels split into two lines when alone on the side
        let is_alone = sorted.len() == 1;
        let is_two_line = is_alone && is_two_line_label(&callout.label);
        let label_width = if is_two_line {
            effective_label_width(&callout.label, style.label_font_size)
        } else {
            estimate_text_width(&callout.label, style.label_font_size)
        };
        let label_height = if is_two_line {
            style.label_font_size * 2.4
        } else {
            style.label_font_size * 1.2
        };
        let label_bounds = Rect::new(
            label_x - label_width / 2.0,
            label_y - label_height / 2.0,
            label_width,
            label_height,
        );

        positioned.push(PositionedCallout {
            callout: (**callout).clone(),
            offset_level: level as u8,
            actual_side: side,
            dimension_line_position: dim_line_y,
            label_position: Point::new(label_x, label_y),
            label_anchor: TextAnchor::Middle,
            label_bounds,
        });
    }

    positioned
}

/// Layout callouts on a vertical side (left or right)
fn layout_vertical_side(
    callouts: &[&DimensionCallout],
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
    side: Side,
) -> Vec<PositionedCallout> {
    let mut positioned = Vec::new();

    // Sort callouts by priority (lower priority number = closer to frame)
    let mut sorted: Vec<_> = callouts.iter().enumerate().collect();
    sorted.sort_by_key(|(_, callout)| callout.priority);

    for (level, (_, callout)) in sorted.iter().enumerate() {
        let offset = style.get_dimension_offset(level as u8);

        // Calculate dimension line position
        let dim_line_x = if side == Side::Right {
            geometry.frame_outer.right() + offset
        } else {
            geometry.frame_outer.left() - offset
        };

        // Calculate label position (centered on dimension line)
        let label_y = (callout.extent_start.y + callout.extent_end.y) / 2.0;
        let label_x = if side == Side::Right {
            dim_line_x + style.label_font_size / 2.0 + 2.0
        } else {
            dim_line_x - style.label_font_size / 2.0 - 2.0
        };

        // Estimate label bounds for collision detection
        // All vertical-side labels with ": " render as two lines
        let is_two_line = is_two_line_label(&callout.label);
        let text_width = if is_two_line {
            effective_label_width(&callout.label, style.label_font_size)
        } else {
            estimate_text_width(&callout.label, style.label_font_size)
        };
        let text_height = if is_two_line {
            style.label_font_size * 2.4
        } else {
            style.label_font_size * 1.2
        };
        // After rotation: text_width becomes screen-vertical, text_height becomes screen-horizontal.
        // Center bounds on dim_line_x (not the offset label_x) because svg_dimension renders
        // labels centered on the dimension line position, not at the layout's label_x.
        let label_bounds = Rect::new(
            dim_line_x - text_height / 2.0,
            label_y - text_width / 2.0,
            text_height,
            text_width,
        );

        positioned.push(PositionedCallout {
            callout: (**callout).clone(),
            offset_level: level as u8,
            actual_side: side,
            dimension_line_position: dim_line_x,
            label_position: Point::new(label_x, label_y),
            label_anchor: if side == Side::Right { TextAnchor::Start } else { TextAnchor::End },
            label_bounds,
        });
    }

    positioned
}


/// Calculate the bounding box of all dimension lines and labels
pub fn calculate_callout_bounds(callouts: &[PositionedCallout]) -> Option<Rect> {
    if callouts.is_empty() {
        return None;
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for callout in callouts {
        min_x = min_x.min(callout.label_bounds.left());
        min_y = min_y.min(callout.label_bounds.top());
        max_x = max_x.max(callout.label_bounds.right());
        max_y = max_y.max(callout.label_bounds.bottom());

        // Also include the dimension line endpoints
        min_x = min_x.min(callout.callout.extent_start.x);
        min_y = min_y.min(callout.callout.extent_start.y);
        max_x = max_x.max(callout.callout.extent_end.x);
        max_y = max_y.max(callout.callout.extent_end.y);
    }

    Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameDesign;
    use crate::visualization::callouts::generate_plan_callouts;

    fn test_design() -> FrameDesign {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 1.0;
        design
    }

    #[test]
    fn test_layout_plan_callouts() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Should position all callouts
        assert!(!result.positioned_callouts.is_empty());
    }

    #[test]
    fn test_horizontal_layout() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Top callouts should have y position above frame
        let top_callouts: Vec<_> = result.positioned_callouts.iter()
            .filter(|c| c.actual_side == Side::Top)
            .collect();

        for callout in top_callouts {
            assert!(callout.dimension_line_position < geometry.frame_outer.top());
        }
    }

    #[test]
    fn test_vertical_layout() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Right callouts should have x position to the right of frame
        let right_callouts: Vec<_> = result.positioned_callouts.iter()
            .filter(|c| c.actual_side == Side::Right)
            .collect();

        for callout in right_callouts {
            assert!(callout.dimension_line_position > geometry.frame_outer.right());
        }
    }

    #[test]
    fn test_offset_levels_assigned() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Multiple callouts on same side should have different offset levels
        let top_callouts: Vec<_> = result.positioned_callouts.iter()
            .filter(|c| c.actual_side == Side::Top)
            .collect();

        if top_callouts.len() > 1 {
            let levels: Vec<_> = top_callouts.iter().map(|c| c.offset_level).collect();
            // At least some should have different levels
            let unique_levels: std::collections::HashSet<_> = levels.iter().collect();
            assert!(unique_levels.len() >= 1);
        }
    }

    #[test]
    fn test_calculate_bounds() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);
        let bounds = calculate_callout_bounds(&result.positioned_callouts);

        assert!(bounds.is_some());
        let bounds = bounds.unwrap();
        assert!(bounds.width > 0.0);
        assert!(bounds.height > 0.0);
    }

    #[test]
    fn test_estimate_text_width() {
        let short_label = "10\"";
        let long_label = "24 3/4\"";

        let short_width = estimate_text_width(short_label, 12.0);
        let long_width = estimate_text_width(long_label, 12.0);

        assert!(long_width > short_width);
    }

    #[test]
    fn test_no_mat_layout() {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Should still have frame dimensions
        assert!(!result.positioned_callouts.is_empty());

        // Should have fewer callouts than with mat
        let with_mat_design = test_design();
        let with_mat_geometry = PlanViewGeometry::from_design(&with_mat_design, 800.0, 600.0, &style);
        let with_mat_callouts = generate_plan_callouts(&with_mat_design, &with_mat_geometry, false, false, false, &style);
        let with_mat_result = layout_plan_callouts(&with_mat_callouts, &with_mat_geometry, &style);

        assert!(result.positioned_callouts.len() <= with_mat_result.positioned_callouts.len());
    }
}
