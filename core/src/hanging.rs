//! Hanging hardware layout: D-ring placement and picture-wire sizing.
//!
//! All positions are measured on the back of the frame. Rings mount on the
//! centerline of each side rail, a fixed fraction of the frame height down
//! from the top edge. The wire runs between the rings with a little slack;
//! when hung, the wire forms two straight legs meeting at the hook, and the
//! apex rises above the ring line by simple right-triangle geometry.

use crate::frame::FrameDesign;

/// Ring drop as a fraction of outside frame height (industry rule of thumb).
pub const DEFAULT_DROP_FRACTION: f64 = 1.0 / 3.0;
/// Extra wire between rings beyond the straight span.
pub const DEFAULT_SLACK_FRACTION: f64 = 0.10;
/// Extra wire per end for wrapping back around itself.
pub const DEFAULT_WRAP_ALLOWANCE: f64 = 3.0;

/// Tunable inputs for hanging layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HangingParams {
    /// Ring drop from the frame top, as a fraction of outside height
    pub drop_fraction: f64,
    /// Wire slack between rings, as a fraction of the ring span
    pub slack_fraction: f64,
    /// Extra wire per end for wrapping
    pub wrap_allowance: f64,
}

impl Default for HangingParams {
    fn default() -> Self {
        Self {
            drop_fraction: DEFAULT_DROP_FRACTION,
            slack_fraction: DEFAULT_SLACK_FRACTION,
            wrap_allowance: DEFAULT_WRAP_ALLOWANCE,
        }
    }
}

/// Computed hanging layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HangingLayout {
    /// D-ring attachment point below the frame top
    pub ring_drop: f64,
    /// Horizontal distance between the two rings (side-rail centerlines)
    pub ring_span: f64,
    /// Straight-line wire length between rings when hung (span + slack)
    pub taut_length: f64,
    /// Wire to cut: taut length plus wrap allowance at both ends
    pub wire_cut_length: f64,
    /// How far the wire apex rises above the ring line at the hook
    pub apex_rise: f64,
    /// Hook position below the frame top (`ring_drop - apex_rise`);
    /// negative means the hook would sit above the frame (too much slack)
    pub hook_below_top: f64,
    /// Each wire leg's angle from horizontal at the hook, in degrees
    /// (shallow angles put high tension on the wire and hardware)
    pub wire_angle_deg: f64,
}

/// Compute the hanging layout for a design, or `None` when the frame is too
/// narrow for rings on both side rails.
pub fn hanging_layout(design: &FrameDesign, params: &HangingParams) -> Option<HangingLayout> {
    let (outside_h, outside_w) = design.get_frame_outside_dimensions();
    let ring_span = outside_w - design.frame_material_width;
    if ring_span <= 0.0 {
        return None;
    }

    let ring_drop = params.drop_fraction * outside_h;
    let taut_length = ring_span * (1.0 + params.slack_fraction.max(0.0));
    // Right triangle per leg: hypotenuse L/2, base span/2
    let apex_rise = 0.5 * (taut_length * taut_length - ring_span * ring_span).sqrt();
    let wire_angle_deg = (ring_span / taut_length).acos().to_degrees();

    Some(HangingLayout {
        ring_drop,
        ring_span,
        taut_length,
        wire_cut_length: taut_length + 2.0 * params.wrap_allowance,
        apex_rise,
        hook_below_top: ring_drop - apex_rise,
        wire_angle_deg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, what: &str) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{what}: expected {expected}, got {actual}"
        );
    }

    /// Build a design whose outside width is exactly `outside_w`.
    fn design_with_outside(outside_w: f64, outside_h: f64, frame_width: f64) -> FrameDesign {
        let rabbet = 0.375;
        FrameDesign {
            // No mat: inside = artwork - 2*rabbet, outside = inside + 2*frame_width
            artwork_width: outside_w - 2.0 * frame_width + 2.0 * rabbet,
            artwork_height: outside_h - 2.0 * frame_width + 2.0 * rabbet,
            mat_width_top_bottom: 0.0,
            mat_width_sides: 0.0,
            frame_material_width: frame_width,
            rabbet_width: rabbet,
            ..FrameDesign::default()
        }
    }

    #[test]
    fn three_four_five_triangle() {
        // Span 30, slack 25% -> taut 37.5, apex rise 11.25 (3-4-5 scaled)
        let design = design_with_outside(31.0, 24.0, 1.0);
        let params = HangingParams {
            drop_fraction: 1.0 / 3.0,
            slack_fraction: 0.25,
            wrap_allowance: 3.0,
        };
        let layout = hanging_layout(&design, &params).expect("layout");
        assert_close(layout.ring_span, 30.0, "ring span");
        assert_close(layout.taut_length, 37.5, "taut length");
        assert_close(layout.apex_rise, 11.25, "apex rise");
        assert_close(layout.ring_drop, 8.0, "ring drop");
        assert_close(layout.hook_below_top, -3.25, "hook above top flagged");
        assert_close(layout.wire_cut_length, 43.5, "cut length");
        assert_close(layout.wire_angle_deg, (0.8f64).acos().to_degrees(), "leg angle");
    }

    #[test]
    fn modest_slack_keeps_hook_below_top() {
        let design = design_with_outside(21.0, 25.0, 1.0);
        let layout = hanging_layout(&design, &HangingParams::default()).expect("layout");
        assert_close(layout.ring_span, 20.0, "ring span");
        assert_close(layout.taut_length, 22.0, "taut length");
        // apex = 0.5*sqrt(22^2 - 20^2) = 0.5*sqrt(84)
        assert_close(layout.apex_rise, 0.5 * 84.0f64.sqrt(), "apex rise");
        assert!(layout.hook_below_top > 0.0, "hook stays below the frame top");
    }

    #[test]
    fn zero_slack_is_degenerate_but_finite() {
        let design = design_with_outside(21.0, 25.0, 1.0);
        let params = HangingParams {
            slack_fraction: 0.0,
            ..HangingParams::default()
        };
        let layout = hanging_layout(&design, &params).expect("layout");
        assert_close(layout.apex_rise, 0.0, "no rise with a taut wire");
        assert_close(layout.wire_angle_deg, 0.0, "flat wire");
    }

    #[test]
    fn too_narrow_frame_has_no_layout() {
        // span = inside + frame_width = (art - 2*rabbet) + frame_width
        //      = (0.1 - 0.75) + 0.5 = -0.15 -> no layout
        let design = FrameDesign {
            artwork_width: 0.1,
            artwork_height: 10.0,
            mat_width_top_bottom: 0.0,
            mat_width_sides: 0.0,
            frame_material_width: 0.5,
            rabbet_width: 0.375,
            ..FrameDesign::default()
        };
        assert!(hanging_layout(&design, &HangingParams::default()).is_none());
    }
}
