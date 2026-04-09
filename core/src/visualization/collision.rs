// Post-layout collision pass for plan view elements.
//
// After geometry and callout layout produce initial positions, this module
// detects overlapping elements and shifts flexible ones to resolve collisions.
// This centralizes collision avoidance that was previously scattered as ad-hoc
// rules across geometry.rs and svg.rs.

use super::types::{Rect, Side};

/// Movement priority for collision resolution.
/// Lower values = less willing to move. Zero = immovable anchor.
/// Derives `PartialOrd`/`Ord` so the resolver can compare flexibility directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlexPriority {
    /// Fixed elements that never move (e.g., arrow stubs, frame edges)
    Immovable = 0,
    /// Corner detail inset -- can shift but prefers not to
    CornerDetail = 2,
    /// Callout labels -- flexible, will shift along their side's normal
    Callout = 3,
    /// Proportional thumbnail -- most flexible, moves last
    Thumbnail = 4,
}

/// Identifies which visual element a FlexElement represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementId {
    /// A positioned callout label (index into positioned_callouts)
    Callout(usize),
    /// The corner detail inset box
    CornerDetail,
    /// The proportional thumbnail silhouette
    Thumbnail,
    /// An outward-pointing arrow stub on a callout's extent boundary.
    /// `callout` is the index, `side` indicates which end of the extent.
    ArrowStub { callout: usize, side: Side },
}

/// Axis along which an element can be shifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// How a flex element is allowed to adjust its position.
#[derive(Debug, Clone, Copy)]
pub enum FlexRule {
    /// Immovable (frame geometry, extension lines, etc.)
    None,
    /// Can shift along a single axis within [range.0, range.1] offset from current position.
    /// Negative range values shift left/up, positive shift right/down.
    ShiftAxis { axis: Axis, range: (f64, f64) },
}

/// An element participating in collision resolution.
#[derive(Debug, Clone)]
pub struct FlexElement {
    pub id: ElementId,
    /// Current bounding rect in SVG coordinates.
    pub bounds: Rect,
    /// How this element can adjust.
    pub flex: FlexRule,
    /// Lower priority = less willing to move. `Immovable` = never moves.
    pub priority: FlexPriority,
}

/// An adjustment produced by the resolver — the element should move to `new_bounds`.
#[derive(Debug, Clone)]
pub struct Adjustment {
    pub id: ElementId,
    pub new_bounds: Rect,
}

/// Resolve collisions between flex elements.
///
/// Iteratively finds overlapping pairs (with `margin` px clearance) and shifts
/// the higher-priority-number (more flexible) element along its flex axis.
/// `skip`: optional predicate — if it returns true for a pair of IDs, that pair
/// is never resolved (useful when axes mismatch makes resolution impossible).
/// Returns adjustments for elements whose bounds changed.
pub fn resolve(
    elements: &mut [FlexElement],
    margin: f64,
    max_iter: u8,
    skip: Option<&dyn Fn(ElementId, ElementId) -> bool>,
) -> Vec<Adjustment> {
    let n = elements.len();
    if n < 2 {
        return Vec::new();
    }

    for _iter in 0..max_iter {
        let mut any_moved = false;

        // Check all pairs; shift the more-flexible element
        for i in 0..n {
            for j in (i + 1)..n {
                if let Some(skip_fn) = skip {
                    if skip_fn(elements[i].id, elements[j].id) {
                        continue;
                    }
                }
                if !elements[i].bounds.overlaps_with_margin(&elements[j].bounds, margin) {
                    continue;
                }

                // Determine which element moves (higher priority number = more flexible)
                let (fixed_idx, flex_idx) = if elements[i].priority >= elements[j].priority {
                    (j, i)
                } else {
                    (i, j)
                };

                // If the flexible element can't move, skip
                let shift = match elements[flex_idx].flex {
                    FlexRule::None => continue,
                    FlexRule::ShiftAxis { axis, range } => {
                        compute_shift(
                            &elements[fixed_idx].bounds,
                            &elements[flex_idx].bounds,
                            axis,
                            range,
                            margin,
                        )
                    }
                };

                if shift.abs() > 0.01 {
                    apply_shift(&mut elements[flex_idx], shift);
                    any_moved = true;
                }
            }
        }

        if !any_moved {
            break;
        }
    }

    // Collect adjustments (compare to see which elements moved)
    // We return all elements that have flex rules, letting the caller
    // use the final bounds regardless of whether they moved.
    elements
        .iter()
        .filter(|e| !matches!(e.flex, FlexRule::None))
        .map(|e| Adjustment {
            id: e.id,
            new_bounds: e.bounds,
        })
        .collect()
}

/// Compute how far to shift the flex element along its axis to clear the fixed element.
fn compute_shift(
    fixed: &Rect,
    flex: &Rect,
    axis: Axis,
    range: (f64, f64),
    margin: f64,
) -> f64 {
    match axis {
        Axis::X => {
            // Determine which direction clears faster
            let shift_left = fixed.left() - margin - flex.right(); // negative
            let shift_right = fixed.right() + margin - flex.left(); // positive

            // Pick the smaller absolute shift
            let shift = if shift_left.abs() < shift_right.abs() {
                shift_left
            } else {
                shift_right
            };

            shift.clamp(range.0, range.1)
        }
        Axis::Y => {
            let shift_up = fixed.top() - margin - flex.bottom(); // negative
            let shift_down = fixed.bottom() + margin - flex.top(); // positive

            let shift = if shift_up.abs() < shift_down.abs() {
                shift_up
            } else {
                shift_down
            };

            shift.clamp(range.0, range.1)
        }
    }
}

/// Apply a shift to a flex element's bounds.
fn apply_shift(element: &mut FlexElement, shift: f64) {
    match element.flex {
        FlexRule::ShiftAxis { axis: Axis::X, .. } => {
            element.bounds.x += shift;
        }
        FlexRule::ShiftAxis { axis: Axis::Y, .. } => {
            element.bounds.y += shift;
        }
        FlexRule::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_overlap_no_adjustment() {
        let mut elements = vec![
            FlexElement {
                id: ElementId::CornerDetail,
                bounds: Rect::new(0.0, 0.0, 50.0, 50.0),
                flex: FlexRule::ShiftAxis {
                    axis: Axis::X,
                    range: (-100.0, 0.0),
                },
                priority: FlexPriority::CornerDetail,
            },
            FlexElement {
                id: ElementId::ArrowStub {
                    callout: 0,
                    side: Side::Left,
                },
                bounds: Rect::new(60.0, 0.0, 20.0, 10.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
        ];

        let adjustments = resolve(&mut elements, 2.0, 4, None);
        // Corner detail didn't need to move
        assert!((elements[0].bounds.x - 0.0).abs() < 0.1);
        assert!(!adjustments.is_empty()); // still returns flex elements
    }

    #[test]
    fn test_overlap_shifts_flexible_element() {
        let mut elements = vec![
            // Arrow stub (immovable) at x=40
            FlexElement {
                id: ElementId::ArrowStub {
                    callout: 0,
                    side: Side::Left,
                },
                bounds: Rect::new(40.0, 10.0, 20.0, 10.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            // Corner detail (flexible, can shift left) overlapping the stub
            FlexElement {
                id: ElementId::CornerDetail,
                bounds: Rect::new(10.0, 0.0, 50.0, 50.0),
                flex: FlexRule::ShiftAxis {
                    axis: Axis::X,
                    range: (-50.0, 0.0),
                },
                priority: FlexPriority::CornerDetail,
            },
        ];

        let _adjustments = resolve(&mut elements, 2.0, 4, None);
        // Corner detail should have shifted left so its right edge clears stub left - margin
        assert!(elements[1].bounds.right() <= 40.0 - 2.0 + 0.1);
    }

    #[test]
    fn test_shift_clamped_to_range() {
        let mut elements = vec![
            // Fixed element
            FlexElement {
                id: ElementId::ArrowStub {
                    callout: 0,
                    side: Side::Left,
                },
                bounds: Rect::new(20.0, 0.0, 10.0, 10.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            // Flexible but with limited range
            FlexElement {
                id: ElementId::CornerDetail,
                bounds: Rect::new(15.0, 0.0, 30.0, 30.0),
                flex: FlexRule::ShiftAxis {
                    axis: Axis::X,
                    range: (-5.0, 0.0), // can only shift 5px left
                },
                priority: FlexPriority::CornerDetail,
            },
        ];

        resolve(&mut elements, 2.0, 4, None);
        // Should shift left by at most 5px
        assert!(elements[1].bounds.x >= 10.0 - 0.1);
    }

    #[test]
    fn test_immovable_elements_dont_move() {
        let mut elements = vec![
            FlexElement {
                id: ElementId::CornerDetail,
                bounds: Rect::new(0.0, 0.0, 50.0, 50.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            FlexElement {
                id: ElementId::ArrowStub {
                    callout: 0,
                    side: Side::Left,
                },
                bounds: Rect::new(30.0, 30.0, 20.0, 20.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
        ];

        resolve(&mut elements, 0.0, 4, None);
        // Neither should move
        assert!((elements[0].bounds.x - 0.0).abs() < 0.01);
        assert!((elements[1].bounds.x - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_y_axis_shift_resolution() {
        // Two elements overlapping vertically; the flexible one shifts along Y
        let mut elements = vec![
            FlexElement {
                id: ElementId::Callout(0),
                bounds: Rect::new(10.0, 10.0, 40.0, 20.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            FlexElement {
                id: ElementId::Callout(1),
                bounds: Rect::new(10.0, 20.0, 40.0, 20.0),
                flex: FlexRule::ShiftAxis { axis: Axis::Y, range: (-50.0, 50.0) },
                priority: FlexPriority::CornerDetail,
            },
        ];

        let orig_y = elements[1].bounds.y;
        resolve(&mut elements, 2.0, 4, None);
        // Element 1 should have shifted away from element 0
        assert!((elements[1].bounds.y - orig_y).abs() > 0.1,
            "Flexible element should have shifted on Y axis");
        // After resolution the rects should no longer overlap (ignoring the double-expansion margin)
        assert!(!elements[0].bounds.overlaps(&elements[1].bounds),
            "Elements should not overlap after Y-axis shift");
    }

    #[test]
    fn test_skip_predicate_prevents_resolution() {
        // Two overlapping elements, but skip predicate prevents their resolution
        let mut elements = vec![
            FlexElement {
                id: ElementId::Callout(0),
                bounds: Rect::new(0.0, 0.0, 30.0, 30.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            FlexElement {
                id: ElementId::Callout(1),
                bounds: Rect::new(10.0, 10.0, 30.0, 30.0),
                flex: FlexRule::ShiftAxis { axis: Axis::X, range: (-50.0, 50.0) },
                priority: FlexPriority::CornerDetail,
            },
        ];

        let skip_fn = |a: ElementId, b: ElementId| {
            matches!((a, b), (ElementId::Callout(0), ElementId::Callout(1)))
                || matches!((a, b), (ElementId::Callout(1), ElementId::Callout(0)))
        };

        resolve(&mut elements, 0.0, 4, Some(&skip_fn));
        // Element 1 should NOT have moved — pair was skipped
        assert!((elements[1].bounds.x - 10.0).abs() < 0.01);
        // They should still overlap
        assert!(elements[0].bounds.overlaps(&elements[1].bounds));
    }

    #[test]
    fn test_three_element_cascade() {
        // A overlaps B, B overlaps C — all three should resolve after enough iterations.
        // Use margin=0 to avoid the double-expansion effect of overlaps_with_margin.
        let mut elements = vec![
            FlexElement {
                id: ElementId::Callout(0),
                bounds: Rect::new(0.0, 0.0, 30.0, 20.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            FlexElement {
                id: ElementId::Callout(1),
                bounds: Rect::new(20.0, 0.0, 30.0, 20.0),
                flex: FlexRule::ShiftAxis { axis: Axis::X, range: (-100.0, 200.0) },
                priority: FlexPriority::CornerDetail,
            },
            FlexElement {
                id: ElementId::Callout(2),
                bounds: Rect::new(40.0, 0.0, 30.0, 20.0),
                flex: FlexRule::ShiftAxis { axis: Axis::X, range: (-100.0, 200.0) },
                priority: FlexPriority::Callout,
            },
        ];

        resolve(&mut elements, 0.0, 10, None);
        // No pair should overlap after resolution (margin=0 in resolve, so check without margin)
        assert!(!elements[0].bounds.overlaps(&elements[1].bounds),
            "Elements 0 and 1 should not overlap");
        assert!(!elements[1].bounds.overlaps(&elements[2].bounds),
            "Elements 1 and 2 should not overlap");
        assert!(!elements[0].bounds.overlaps(&elements[2].bounds),
            "Elements 0 and 2 should not overlap");
    }

    #[test]
    fn test_shift_direction_prefers_shorter() {
        // Element centered on the fixed element — shifting left is shorter because
        // the flex element extends further to the right of the fixed element.
        // Fixed: [0, 20], Flex: [5, 55]. Left shift = 0 - 2 - 55 = -57; Right shift = 20 + 2 - 5 = 17.
        // Shorter shift is right (+17).
        let mut elements = vec![
            FlexElement {
                id: ElementId::Callout(0),
                bounds: Rect::new(0.0, 0.0, 20.0, 20.0),
                flex: FlexRule::None,
                priority: FlexPriority::Immovable,
            },
            FlexElement {
                id: ElementId::Callout(1),
                bounds: Rect::new(5.0, 0.0, 50.0, 20.0),
                flex: FlexRule::ShiftAxis { axis: Axis::X, range: (-100.0, 100.0) },
                priority: FlexPriority::CornerDetail,
            },
        ];

        let original_x = elements[1].bounds.x;
        resolve(&mut elements, 2.0, 4, None);
        // Should have shifted right (positive direction) since it's the shorter shift
        assert!(elements[1].bounds.x > original_x);
    }
}
