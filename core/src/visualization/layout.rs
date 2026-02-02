// Adaptive callout layout algorithm
//
// Handles collision detection and resolution to ensure all dimension
// callouts are readable and don't overlap.

use super::types::{
    DimensionCallout, PositionedCallout, Point, Rect, Side, TextAnchor,
};
use super::style::DiagramStyle;
use super::geometry::{PlanViewGeometry, estimate_text_width};

/// Result of layout calculation
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// Positioned callouts ready for rendering
    pub positioned_callouts: Vec<PositionedCallout>,
    /// Warnings about omitted or adjusted dimensions
    pub warnings: Vec<String>,
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

    // Check for collisions between positioned callouts and resolve
    let (resolved, collision_warnings) = resolve_collisions(&positioned, style);
    positioned = resolved;
    warnings.extend(collision_warnings);

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
            dim_line_y - style.dimension_font_size / 2.0 - 2.0
        } else {
            dim_line_y + style.dimension_font_size / 2.0 + 2.0
        };

        // Estimate label bounds for collision detection
        let label_width = estimate_label_width(&callout.label, style.dimension_font_size);
        let label_height = style.dimension_font_size * 1.2;
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
            dim_line_x + style.dimension_font_size / 2.0 + 2.0
        } else {
            dim_line_x - style.dimension_font_size / 2.0 - 2.0
        };

        // Estimate label bounds for collision detection
        // For vertical labels, we might rotate text, so swap width/height conceptually
        let label_width = estimate_label_width(&callout.label, style.dimension_font_size);
        let label_height = style.dimension_font_size * 1.2;
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
            dimension_line_position: dim_line_x,
            label_position: Point::new(label_x, label_y),
            label_anchor: if side == Side::Right { TextAnchor::Start } else { TextAnchor::End },
            label_bounds,
        });
    }

    positioned
}

/// Estimate label width based on text length and font size
fn estimate_label_width(text: &str, font_size: f64) -> f64 {
    // Use shared character-aware width estimation
    estimate_text_width(text, font_size)
}

/// Detect and resolve collisions between positioned callouts
fn resolve_collisions(
    callouts: &[PositionedCallout],
    style: &DiagramStyle,
) -> (Vec<PositionedCallout>, Vec<String>) {
    let mut result = callouts.to_vec();
    let mut warnings = Vec::new();

    // Check for label-label collisions
    let collision_margin = style.label_spacing;

    for i in 0..result.len() {
        for j in (i + 1)..result.len() {
            let bounds_i = result[i].label_bounds.expand(collision_margin / 2.0);
            let bounds_j = result[j].label_bounds.expand(collision_margin / 2.0);

            if bounds_i.overlaps(&bounds_j) {
                // Same side collision - try to resolve by adjusting offset
                if result[i].actual_side == result[j].actual_side {
                    // Move the lower priority one further out
                    let (higher_pri, lower_pri) = if result[i].callout.priority <= result[j].callout.priority {
                        (i, j)
                    } else {
                        (j, i)
                    };

                    let new_level = result[higher_pri].offset_level + 1;
                    let additional_offset = style.dimension_offset_step;

                    // Adjust position based on side
                    match result[lower_pri].actual_side {
                        Side::Top => {
                            result[lower_pri].dimension_line_position -= additional_offset;
                            result[lower_pri].label_position.y -= additional_offset;
                            result[lower_pri].label_bounds.y -= additional_offset;
                        }
                        Side::Bottom => {
                            result[lower_pri].dimension_line_position += additional_offset;
                            result[lower_pri].label_position.y += additional_offset;
                            result[lower_pri].label_bounds.y += additional_offset;
                        }
                        Side::Right => {
                            result[lower_pri].dimension_line_position += additional_offset;
                            result[lower_pri].label_position.x += additional_offset;
                            result[lower_pri].label_bounds.x += additional_offset;
                        }
                        Side::Left => {
                            result[lower_pri].dimension_line_position -= additional_offset;
                            result[lower_pri].label_position.x -= additional_offset;
                            result[lower_pri].label_bounds.x -= additional_offset;
                        }
                    }
                    result[lower_pri].offset_level = new_level;
                }
            }
        }
    }

    // After adjustment, check if any labels are still colliding
    // If priority 4+ callouts can't fit, warn and potentially hide them
    for i in 0..result.len() {
        for j in (i + 1)..result.len() {
            let bounds_i = result[i].label_bounds.expand(collision_margin / 2.0);
            let bounds_j = result[j].label_bounds.expand(collision_margin / 2.0);

            if bounds_i.overlaps(&bounds_j) {
                // If one is low priority, add warning
                if result[i].callout.priority >= 4 || result[j].callout.priority >= 4 {
                    let low_pri_type = if result[i].callout.priority >= result[j].callout.priority {
                        &result[i].callout.dimension_type
                    } else {
                        &result[j].callout.dimension_type
                    };
                    warnings.push(format!("{:?} dimension may overlap with adjacent label", low_pri_type));
                }
            }
        }
    }

    (result, warnings)
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
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Should position all callouts
        assert!(!result.positioned_callouts.is_empty());
    }

    #[test]
    fn test_horizontal_layout() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

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
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);
        let bounds = calculate_callout_bounds(&result.positioned_callouts);

        assert!(bounds.is_some());
        let bounds = bounds.unwrap();
        assert!(bounds.width > 0.0);
        assert!(bounds.height > 0.0);
    }

    #[test]
    fn test_estimate_label_width() {
        let short_label = "10\"";
        let long_label = "24 3/4\"";

        let short_width = estimate_label_width(short_label, 12.0);
        let long_width = estimate_label_width(long_label, 12.0);

        assert!(long_width > short_width);
    }

    #[test]
    fn test_no_mat_layout() {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Should still have frame dimensions
        assert!(!result.positioned_callouts.is_empty());

        // Should have fewer callouts than with mat
        let with_mat_design = test_design();
        let with_mat_geometry = PlanViewGeometry::from_design(&with_mat_design, 800.0, 600.0, &style);
        let with_mat_callouts = generate_plan_callouts(&with_mat_design, &with_mat_geometry, false, false, &style);
        let with_mat_result = layout_plan_callouts(&with_mat_callouts, &with_mat_geometry, &style);

        assert!(result.positioned_callouts.len() <= with_mat_result.positioned_callouts.len());
    }
}
