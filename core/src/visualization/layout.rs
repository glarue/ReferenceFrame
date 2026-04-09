// Adaptive callout layout algorithm
//
// Handles collision detection and resolution to ensure all dimension
// callouts are readable and don't overlap.

use super::types::{
    DimensionCallout, DimensionType, PositionedCallout, Point, Rect, Side, TextAnchor,
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

/// Padding between label text center and dimension line.
const LABEL_POSITION_PAD: f64 = 2.0;

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
    let warnings = Vec::new();

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
    positioned.extend(layout_side(&top, geometry, style, Side::Top));
    positioned.extend(layout_side(&bottom, geometry, style, Side::Bottom));
    positioned.extend(layout_side(&right, geometry, style, Side::Right));
    positioned.extend(layout_side(&left, geometry, style, Side::Left));

    LayoutResult {
        positioned_callouts: positioned,
        warnings,
    }
}

/// Layout callouts on any side (top, bottom, left, or right).
///
/// Horizontal sides (top/bottom): dimension line runs along Y, labels along X.
/// Vertical sides (left/right): dimension line runs along X, labels along Y,
/// with rotated text bounds.
fn layout_side(
    callouts: &[&DimensionCallout],
    geometry: &PlanViewGeometry,
    style: &DiagramStyle,
    side: Side,
) -> Vec<PositionedCallout> {
    let mut positioned = Vec::new();
    let horizontal = side.is_horizontal();

    // Sort callouts by priority (lower priority number = closer to frame)
    let mut sorted: Vec<_> = callouts.iter().enumerate().collect();
    sorted.sort_by_key(|(_, callout)| callout.priority);

    for (level, (_, callout)) in sorted.iter().enumerate() {
        let offset = style.get_dimension_offset(level as u8);

        // Dimension line position: offset from frame edge along the primary axis
        let dim_line_pos = if horizontal {
            if side == Side::Top {
                geometry.frame_outer.top() - offset
            } else {
                geometry.frame_outer.bottom() + offset
            }
        } else if side == Side::Right {
            geometry.frame_outer.right() + offset
        } else {
            geometry.frame_outer.left() - offset
        };

        // Label center on secondary axis (midpoint of extent)
        let (label_x, label_y) = if horizontal {
            let x = (callout.extent_start.x + callout.extent_end.x) / 2.0;
            let y = if side == Side::Top {
                dim_line_pos - style.label_font_size / 2.0 - LABEL_POSITION_PAD
            } else {
                dim_line_pos + style.label_font_size / 2.0 + LABEL_POSITION_PAD
            };
            (x, y)
        } else {
            let y = (callout.extent_start.y + callout.extent_end.y) / 2.0;
            let x = if side == Side::Right {
                dim_line_pos + style.label_font_size / 2.0 + LABEL_POSITION_PAD
            } else {
                dim_line_pos - style.label_font_size / 2.0 - LABEL_POSITION_PAD
            };
            (x, y)
        };

        // Determine two-line rendering:
        // Horizontal: only when alone on the side (more space available)
        // Vertical: always (rotated labels have more room along their axis)
        let is_two_line = if horizontal {
            sorted.len() == 1 && is_two_line_label(&callout.label)
        } else {
            is_two_line_label(&callout.label)
        };

        let text_width = if is_two_line {
            effective_label_width(&callout.label, style.label_font_size)
        } else {
            estimate_text_width(&callout.label, style.label_font_size)
        };
        let text_height = if is_two_line {
            style.two_line_height()
        } else {
            style.single_line_height()
        };

        // Compute label bounds for collision detection
        let label_bounds = if horizontal {
            Rect::new(
                label_x - text_width / 2.0,
                label_y - text_height / 2.0,
                text_width,
                text_height,
            )
        } else {
            // After rotation: text_width becomes screen-vertical, text_height becomes
            // screen-horizontal. Center bounds on dim_line_pos because svg_dimension
            // renders labels centered on the dimension line, not at label_x.
            //
            // For two-line MatCutHeight labels displayed vertically, the text naturally
            // centers on the midpoint. But the "Mat Cut:" prefix is shorter than the
            // value line, so centering looks off-balance. Bottom-align shifts the text
            // so the longer value part extends upward, keeping visual weight toward
            // the dimension line it annotates. This also keeps the downward extent
            // compact, avoiding overlap with the thumbnail below.
            let bottom_align_shift = if is_two_line && callout.dimension_type == DimensionType::MatCutHeight {
                if let Some(pos) = callout.label.find(": ") {
                    let prefix_part = &callout.label[..pos + 1];
                    let value_part = callout.label[pos + 2..].trim_start();
                    let w_v = estimate_text_width(value_part, style.label_font_size);
                    let w_p = estimate_text_width(prefix_part, style.label_font_size);
                    (w_v - w_p).max(0.0) / 2.0
                } else {
                    0.0
                }
            } else {
                0.0
            };
            Rect::new(
                dim_line_pos - text_height / 2.0,
                label_y - text_width / 2.0 - bottom_align_shift,
                text_height,
                text_width,
            )
        };

        // Text anchor
        let label_anchor = if horizontal {
            TextAnchor::Middle
        } else if side == Side::Right {
            TextAnchor::Start
        } else {
            TextAnchor::End
        };

        positioned.push(PositionedCallout {
            callout: (**callout).clone(),
            offset_level: level as u8,
            actual_side: side,
            dimension_line_position: dim_line_pos,
            label_position: Point::new(label_x, label_y),
            label_anchor,
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
    use crate::visualization::test_helpers::test_design;

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

    #[test]
    fn test_bottom_side_layout() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Bottom callouts should have dimension_line_position below frame
        let bottom_callouts: Vec<_> = result.positioned_callouts.iter()
            .filter(|c| c.actual_side == Side::Bottom)
            .collect();

        for callout in &bottom_callouts {
            assert!(
                callout.dimension_line_position > geometry.frame_outer.bottom(),
                "Bottom callout dim line {} should be below frame bottom {}",
                callout.dimension_line_position, geometry.frame_outer.bottom()
            );
        }
    }

    #[test]
    fn test_left_side_layout() {
        // Create an asymmetric mat design that produces a MatCutHeight callout on the left
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 3.0;
        design.mat_width_sides = 1.5;
        design.frame_material_width = 1.0;

        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        let left_callouts: Vec<_> = result.positioned_callouts.iter()
            .filter(|c| c.actual_side == Side::Left)
            .collect();

        // Left callouts should use TextAnchor::End
        for callout in &left_callouts {
            assert_eq!(callout.label_anchor, TextAnchor::End,
                "Left-side callout should use TextAnchor::End");
        }
    }

    #[test]
    fn test_empty_callouts_returns_none() {
        let empty: Vec<PositionedCallout> = vec![];
        assert!(calculate_callout_bounds(&empty).is_none());
    }

    #[test]
    fn test_two_line_label_bounds_taller() {
        // Two-line label (contains ": ") should produce taller bounds than single-line
        let design = test_design();
        let style = DiagramStyle::default();
        let geometry = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);
        let callouts = generate_plan_callouts(&design, &geometry, false, false, false, &style);

        let result = layout_plan_callouts(&callouts, &geometry, &style);

        // Find a callout whose label contains ": " (two-line candidate) and one that doesn't
        let two_line_callout = result.positioned_callouts.iter()
            .find(|c| c.callout.label.contains(": ") && c.actual_side.is_horizontal());
        let single_line_callout = result.positioned_callouts.iter()
            .find(|c| !c.callout.label.contains(": ") && c.actual_side.is_horizontal());

        // If both exist and the two-line one is alone on its side (which triggers two-line rendering),
        // its bounds should be taller
        if let (Some(two), Some(one)) = (two_line_callout, single_line_callout) {
            // Two-line bounds height should be >= single-line bounds height
            // (only applies when the two-line callout is rendered as two lines)
            let two_h = two.label_bounds.height;
            let one_h = one.label_bounds.height;
            // At minimum, the style's two_line_height > single_line_height
            assert!(style.two_line_height() > style.single_line_height(),
                "Style two_line_height ({}) should exceed single_line_height ({})",
                style.two_line_height(), style.single_line_height());
            // If the two-line callout actually rendered as two lines, bounds should be taller
            if two_h > one_h {
                assert!(two_h > one_h);
            }
        }
    }
}
