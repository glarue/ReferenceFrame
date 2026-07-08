//! Frame weight estimation with honest error propagation, plus hanging-wire
//! tension derived from it.
//!
//! The frame's wood volume is exact — no miter approximations: the moulding
//! ring's plan area times its depth, minus the rabbet channel ring times the
//! channel depth. Sheet components (glazing, mat, backing, artwork) are area
//! x thickness. Every material carries a low/typical/high density range from
//! presets.json (sourced values), and ranges propagate through to the total
//! and to wire tension.
//!
//! Wire tension per leg for a frame of weight `W` hung on one hook is
//! `W / (2 sin θ)` where θ is each leg's angle from horizontal — the reason
//! taut wire is dangerous: the multiplier grows without bound as slack goes
//! to zero.

use serde::{Deserialize, Serialize};

use crate::frame::FrameDesign;
use crate::hanging::{hanging_layout, HangingParams};

const IN3_PER_FT3: f64 = 1728.0;

/// A density with its plausible range, in lb/ft^3.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaterialDensity {
    pub lb_ft3: f64,
    pub low: f64,
    pub high: f64,
}

impl From<&crate::presets::MaterialSpec> for MaterialDensity {
    fn from(spec: &crate::presets::MaterialSpec) -> Self {
        Self {
            lb_ft3: spec.lb_ft3,
            low: spec.low,
            high: spec.high,
        }
    }
}

impl MaterialDensity {
    fn weigh(&self, volume_in3: f64) -> ComponentWeight {
        let v = volume_in3 / IN3_PER_FT3;
        ComponentWeight {
            low: v * self.low,
            typical: v * self.lb_ft3,
            high: v * self.high,
        }
    }
}

/// A weight in pounds with its propagated range.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ComponentWeight {
    pub low: f64,
    pub typical: f64,
    pub high: f64,
}

impl ComponentWeight {
    fn add(&self, other: &ComponentWeight) -> ComponentWeight {
        ComponentWeight {
            low: self.low + other.low,
            typical: self.typical + other.typical,
            high: self.high + other.high,
        }
    }

    fn scale(&self, k: f64) -> ComponentWeight {
        ComponentWeight {
            low: self.low * k,
            typical: self.typical * k,
            high: self.high * k,
        }
    }
}

/// Densities to use for each component. Defaults come from presets.json:
/// generic frame wood (wide pine-to-oak range), float glass glazing,
/// paper-core matboard, foamcore backing, photo-paper artwork.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeightParams {
    pub wood: MaterialDensity,
    pub glazing: MaterialDensity,
    pub matboard: MaterialDensity,
    pub backing: MaterialDensity,
    pub artwork: MaterialDensity,
}

impl Default for WeightParams {
    fn default() -> Self {
        let m = crate::presets::get_materials();
        Self {
            wood: m.wood_default().into(),
            glazing: (&m.sheet["glass"]).into(),
            matboard: (&m.sheet["matboard_paper"]).into(),
            backing: (&m.sheet["foamcore"]).into(),
            artwork: (&m.sheet["photo_paper"]).into(),
        }
    }
}

/// Wire tension for the estimated weight at the current hanging geometry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TensionEstimate {
    /// Each leg's angle from horizontal at the hook, degrees
    pub wire_angle_deg: f64,
    /// Tension per leg as a multiple of total frame weight: 1 / (2 sin θ)
    pub multiplier: f64,
    /// Tension per leg in pounds (weight range x multiplier)
    pub per_leg_lb: ComponentWeight,
}

/// The full estimate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightEstimate {
    pub frame: ComponentWeight,
    pub glazing: ComponentWeight,
    pub matboard: ComponentWeight,
    pub backing: ComponentWeight,
    pub artwork: ComponentWeight,
    pub total: ComponentWeight,
    /// Exact moulding wood volume, cubic inches
    pub frame_wood_volume_in3: f64,
    /// None when the frame is too narrow for hangers
    pub tension: Option<TensionEstimate>,
}

/// Exact moulding wood volume in cubic inches: the frame ring times its
/// depth, minus the rabbet channel ring times the channel depth.
pub fn frame_wood_volume_in3(design: &FrameDesign) -> f64 {
    let (ih, iw) = design.get_frame_inside_dimensions();
    let (oh, ow) = design.get_frame_outside_dimensions();
    let ring_area = (ow * oh - iw * ih).max(0.0);

    let rw = design.rabbet_width.min(design.frame_material_width);
    let rd = design.rabbet_depth.min(design.frame_material_depth);
    let channel_area = ((iw + 2.0 * rw) * (ih + 2.0 * rw) - iw * ih).max(0.0);

    ring_area * design.frame_material_depth - channel_area * rd
}

/// Estimate the assembled frame's weight and hanging-wire tension.
pub fn estimate_weight(
    design: &FrameDesign,
    params: &WeightParams,
    hanging: &HangingParams,
) -> WeightEstimate {
    let wood_volume = frame_wood_volume_in3(design);
    let frame = params.wood.weigh(wood_volume);

    // Sheet components seat in the rabbet opening
    let (seat_h, seat_w) = design.get_matboard_dimensions();
    let seat_area = (seat_h * seat_w).max(0.0);

    let glazing = params
        .glazing
        .weigh(seat_area * design.glazing_thickness.max(0.0));

    // The mat is a ring: seat minus its window opening
    let matboard = if design.has_mat() {
        let (open_h, open_w) = design.get_mat_opening_dimensions();
        let mat_area = (seat_area - (open_h * open_w).max(0.0)).max(0.0);
        params.matboard.weigh(mat_area * design.matboard_thickness.max(0.0))
    } else {
        ComponentWeight::default()
    };

    let backing = params
        .backing
        .weigh(seat_area * design.backing_thickness.max(0.0));

    let artwork = params.artwork.weigh(
        (design.artwork_width * design.artwork_height).max(0.0)
            * design.artwork_thickness.max(0.0),
    );

    let total = frame
        .add(&glazing)
        .add(&matboard)
        .add(&backing)
        .add(&artwork);

    let tension = hanging_layout(design, hanging).and_then(|layout| {
        let theta = layout.wire_angle_deg.to_radians();
        if theta.sin() <= f64::EPSILON {
            return None; // taut wire: multiplier unbounded
        }
        let multiplier = 1.0 / (2.0 * theta.sin());
        Some(TensionEstimate {
            wire_angle_deg: layout.wire_angle_deg,
            multiplier,
            per_leg_lb: total.scale(multiplier),
        })
    });

    WeightEstimate {
        frame,
        glazing,
        matboard,
        backing,
        artwork,
        total,
        frame_wood_volume_in3: wood_volume,
        tension,
    }
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

    fn density(v: f64) -> MaterialDensity {
        MaterialDensity { lb_ft3: v, low: v, high: v }
    }

    /// 8x12 art, no mat, 3/4 x 3/4 moulding, 3/8 x 3/8 rabbet.
    fn no_mat_design() -> FrameDesign {
        FrameDesign {
            artwork_width: 12.0,
            artwork_height: 8.0,
            mat_width_top_bottom: 0.0,
            mat_width_sides: 0.0,
            frame_material_width: 0.75,
            frame_material_depth: 0.75,
            rabbet_width: 0.375,
            rabbet_depth: 0.375,
            ..FrameDesign::default()
        }
    }

    #[test]
    fn frame_volume_matches_hand_calculation() {
        // inside = art - 2*rabbet = 11.25 x 7.25; outside = 12.75 x 8.75
        // ring area = 12.75*8.75 - 11.25*7.25 = 111.5625 - 81.5625 = 30.0
        // channel ring = 12.0*8.0 - 81.5625 = 14.4375, channel depth 0.375
        // volume = 30*0.75 - 14.4375*0.375 = 22.5 - 5.4140625 = 17.0859375
        let v = frame_wood_volume_in3(&no_mat_design());
        assert_close(v, 17.0859375, "wood volume");
    }

    #[test]
    fn weights_scale_with_density_and_range_propagates() {
        let design = no_mat_design();
        let params = WeightParams {
            wood: MaterialDensity { lb_ft3: 35.0, low: 24.0, high: 48.0 },
            glazing: density(156.0),
            matboard: density(50.0),
            backing: density(3.0),
            artwork: density(60.0),
        };
        let est = estimate_weight(&design, &params, &HangingParams::default());
        // frame typical = 17.0859375 / 1728 * 35
        assert_close(est.frame.typical, 17.0859375 / 1728.0 * 35.0, "frame typical");
        assert_close(est.frame.low, 17.0859375 / 1728.0 * 24.0, "frame low");
        assert_close(est.frame.high, 17.0859375 / 1728.0 * 48.0, "frame high");
        assert!(est.total.low < est.total.typical && est.total.typical < est.total.high);
        // no mat -> zero mat weight
        assert_close(est.matboard.typical, 0.0, "no mat weighs nothing");
    }

    #[test]
    fn tension_multiplier_matches_geometry() {
        // 25% slack -> 3-4-5 legs: sin(theta) = 3/5, multiplier = 5/6
        let design = no_mat_design();
        let hanging = HangingParams {
            slack_fraction: 0.25,
            ..HangingParams::default()
        };
        let est = estimate_weight(&design, &WeightParams::default(), &hanging);
        let t = est.tension.expect("tension");
        assert_close(t.multiplier, 5.0 / 6.0, "3-4-5 multiplier");
        assert_close(
            t.per_leg_lb.typical,
            est.total.typical * 5.0 / 6.0,
            "per-leg pounds",
        );
    }

    #[test]
    fn defaults_give_plausible_small_frame_weight() {
        // Generic wood + glass on a small frame: sanity band, not exact
        let est = estimate_weight(
            &no_mat_design(),
            &WeightParams::default(),
            &HangingParams::default(),
        );
        assert!(
            est.total.typical > 0.4 && est.total.typical < 5.0,
            "small frame should weigh roughly a pound or two, got {}",
            est.total.typical
        );
        assert!(est.total.low <= est.total.typical && est.total.typical <= est.total.high);
    }
}
