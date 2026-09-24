// Adaptive callout layout algorithm
//
// Handles collision detection and resolution to ensure all dimension
// callouts are readable and don't overlap.

use super::types::{
    DimensionCallout, DimensionType, LabelLine, LabelPlacement, PositionedCallout, Point, Rect, Side,
    TextAnchor,
};
use super::style::{DiagramStyle, LABEL_MASK_PADDING_X, LABEL_MASK_PADDING_Y};
use super::geometry::{PlanViewGeometry, estimate_text_width, effective_label_width, split_two_line};

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
/// Vertical sides (left/right): dimension line runs along X, labels are
/// rotated 90° and run along Y.
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
    let outermost_level = sorted.len().saturating_sub(1);

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

        let (label, label_bounds) =
            place_label(callout, side, dim_line_pos, level == outermost_level, style);

        positioned.push(PositionedCallout {
            callout: (**callout).clone(),
            offset_level: level as u8,
            actual_side: side,
            dimension_line_position: dim_line_pos,
            label,
            label_bounds,
        });
    }

    positioned
}

/// Compute exactly where a callout's label (text lines + mask) is drawn, and
/// its visual bounds. This is the single source for the renderer
/// (`plan_svg::svg_dimension`), the collision pass, and viewBox fitting.
///
/// Rules:
/// - "Prefix: value" labels always render as two lines (`split_two_line`).
/// - Labels are centered on the dimension line. On the outermost level the
///   two-line block shifts outward (prefix away from the frame, value on the
///   line); inner levels keep both lines centered on the line.
/// - Mat Cut labels sit `mat_cut_label_offset()` outside their dimension line:
///   MatCutWidth is left-anchored ("start") at the extent's left edge;
///   MatCutHeight is two side-by-side rotated strips, bottom-aligned ("end").
/// - The mask is centered on the label's base position and breaks the
///   dimension line (for Mat Cut it sits at the offset label position).
fn place_label(
    callout: &DimensionCallout,
    side: Side,
    dim_line_pos: f64,
    is_outermost: bool,
    style: &DiagramStyle,
) -> (LabelPlacement, Rect) {
    let horizontal = side.is_horizontal();
    let fs = style.label_font_size;
    let line_gap = fs * 0.2;
    let half_line_offset = (fs + line_gap) / 2.0;
    let is_mat_cut_width = callout.dimension_type == DimensionType::MatCutWidth;
    let is_mat_cut_height = callout.dimension_type == DimensionType::MatCutHeight;
    let is_mat_cut = is_mat_cut_width || is_mat_cut_height;
    let mat_cut_offset = style.mat_cut_label_offset();
    let two_line = split_two_line(&callout.label);

    // Base label position: horizontal (x along the extent, y on the line);
    // vertical (x on the line, y along the extent).
    let (label_x, label_y) = if horizontal {
        if is_mat_cut_width {
            let left_x = callout.extent_start.x.min(callout.extent_end.x);
            (left_x, dim_line_pos + mat_cut_offset)
        } else {
            ((callout.extent_start.x + callout.extent_end.x) / 2.0, dim_line_pos)
        }
    } else {
        let mid_y = (callout.extent_start.y + callout.extent_end.y) / 2.0;
        let x = if is_mat_cut_height { dim_line_pos - mat_cut_offset } else { dim_line_pos };
        (x, mid_y)
    };

    let anchor = if is_mat_cut_width {
        TextAnchor::Start
    } else if is_mat_cut_height && two_line.is_some() {
        TextAnchor::End
    } else {
        TextAnchor::Middle
    };

    let line = |text: &str, x: f64, y: f64| LabelLine { text: text.to_string(), pos: Point::new(x, y) };
    let lines: Vec<LabelLine> = match two_line {
        None => vec![line(&callout.label, label_x, label_y)],
        Some((prefix, value)) if horizontal => {
            let (y1, y2) = if !is_mat_cut && is_outermost {
                match side {
                    Side::Top => (label_y - (fs + line_gap), label_y),
                    _ => (label_y, label_y + (fs + line_gap)),
                }
            } else {
                (label_y - half_line_offset, label_y + half_line_offset)
            };
            vec![line(prefix, label_x, y1), line(value, label_x, y2)]
        }
        Some((prefix, value)) if is_mat_cut_height => {
            // Side-by-side strips sharing one bottom edge (anchor "end"); the
            // prefix sits closer to the frame so it reads first.
            let shared_bottom_y = label_y + estimate_text_width(prefix, fs) / 2.0;
            vec![
                line(prefix, label_x + half_line_offset, shared_bottom_y),
                line(value, label_x - half_line_offset, shared_bottom_y),
            ]
        }
        Some((prefix, value)) => {
            // Prefix on the outward side of the value
            let (x1, x2) = match (side, is_outermost) {
                (Side::Right, true) => (label_x + (fs + line_gap), label_x),
                (_, true) => (label_x - (fs + line_gap), label_x),
                (Side::Right, false) => (label_x + half_line_offset, label_x - half_line_offset),
                (_, false) => (label_x - half_line_offset, label_x + half_line_offset),
            };
            vec![line(prefix, x1, label_y), line(value, x2, label_y)]
        }
    };

    // Mask: single-line tall, as wide as the widest line. Horizontal labels keep
    // extra padding for clearance from arrowheads; rotated ones use tight padding.
    let pad_along = if horizontal { LABEL_MASK_PADDING_X * 2.0 } else { LABEL_MASK_PADDING_X };
    let pad_across = LABEL_MASK_PADDING_Y;
    let mask_len = effective_label_width(&callout.label, fs) + pad_along * 2.0;
    let mask_thick = fs + pad_across * 2.0;
    let mask = if horizontal {
        let x = if is_mat_cut_width { label_x - LABEL_MASK_PADDING_X } else { label_x - mask_len / 2.0 };
        Rect::new(x, label_y - mask_thick / 2.0, mask_len, mask_thick)
    } else {
        // Side-by-side lines (inner levels, MatCutHeight) widen the mask to both lines.
        let across = if two_line.is_some() && (!is_outermost || is_mat_cut) {
            2.0 * half_line_offset + pad_across * 2.0
        } else {
            mask_thick
        };
        // MatCutHeight strips are bottom-aligned; center the mask on their combined extent.
        let center_y = match two_line {
            Some((prefix, value)) if is_mat_cut_height => {
                let w_v = estimate_text_width(value, fs);
                let w_p = estimate_text_width(prefix, fs);
                label_y - (w_v - w_p).max(0.0) / 2.0
            }
            _ => label_y,
        };
        Rect::new(label_x - across / 2.0, center_y - mask_len / 2.0, across, mask_len)
    };

    // Visual bounds: each line's em box along its anchor, plus the mask.
    let bounds = lines.iter().fold(mask, |acc, l| {
        let w = estimate_text_width(&l.text, fs);
        let start = match anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -w / 2.0,
            TextAnchor::End => -w,
        };
        let r = if horizontal {
            Rect::new(l.pos.x + start, l.pos.y - fs / 2.0, w, fs)
        } else {
            Rect::new(l.pos.x - fs / 2.0, l.pos.y + start, fs, w)
        };
        acc.union(&r)
    });

    (LabelPlacement { lines, anchor, rotated: !horizontal, mask }, bounds)
}


/// Calculate the bounding box of all dimension lines and labels.
/// Test-only for now; kept for the layout/renderer unification (audit C6).
#[cfg(test)]
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

        // The asymmetric mat puts MatCutHeight on the left: rotated, two
        // bottom-aligned strips (anchor End), prefix closer to the frame.
        assert!(!left_callouts.is_empty(), "expected a left-side MatCutHeight callout");
        for callout in &left_callouts {
            assert!(callout.label.rotated);
            assert_eq!(callout.label.anchor, TextAnchor::End,
                "Left-side MatCutHeight should use TextAnchor::End");
            assert_eq!(callout.label.lines.len(), 2);
            assert!(callout.label.lines[0].pos.x > callout.label.lines[1].pos.x,
                "prefix strip should sit closer to the frame than the value");
        }
    }

    #[test]
    fn test_empty_callouts_returns_none() {
        let empty: Vec<PositionedCallout> = vec![];
        assert!(calculate_callout_bounds(&empty).is_none());
    }
}
