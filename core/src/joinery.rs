//! Spline (corner key) slot planning for mitered frames.
//!
//! Models the moulding cross-section as seen at a corner. Coordinates:
//! `x` runs from the outer edge (0) toward the inner edge
//! (`frame_material_width`); `z` runs from the front face (0) toward the
//! back (`frame_material_depth`). The rabbet channel occupies the
//! back-inner region: `x > W - rabbet_width`, `z > D - rabbet_depth`,
//! leaving a solid full-width "face band" of depth `D - rabbet_depth`
//! at the front.
//!
//! A spline slot is cut across the outside of the miter joint,
//! perpendicular to the miter plane. Each slot is described by the depth
//! of its centerline from the front face (`z_center`) and how far it may
//! safely penetrate from the outer corner: slots whose depth band stays
//! clear of the rabbet channel may run nearly the full moulding width;
//! slots overlapping the channel's depth band must stop short of the
//! channel wall so the blade never breaks into the rabbet.

use serde::{Deserialize, Serialize};

use crate::frame::FrameDesign;

/// Mouldings at least this deep get two splines.
const TWO_SLOT_MIN_DEPTH: f64 = 1.25;

/// Tunable inputs for spline slot planning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplineParams {
    /// Slot (blade kerf) thickness
    pub slot_thickness: f64,
    /// Minimum wall to leave between the slot and faces or the rabbet
    pub min_wall: f64,
}

impl Default for SplineParams {
    /// Factory values come from presets.json (the single source of truth)
    fn default() -> Self {
        let d = crate::presets::get_defaults();
        Self {
            slot_thickness: d.spline_kerf,
            min_wall: d.spline_min_wall,
        }
    }
}

/// One planned spline slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SplineSlot {
    /// Slot centerline depth, measured from the front face
    pub z_center: f64,
    /// Maximum safe penetration from the outer edge, in cross-section
    pub max_penetration: f64,
    /// Maximum safe penetration measured along the miter bisector in plan
    /// view (cross-section penetration x sqrt(2))
    pub max_penetration_diagonal: f64,
    /// Whether the slot's depth band overlaps the rabbet channel band
    /// (penetration is limited by the channel wall)
    pub over_rabbet: bool,
}

/// The safe envelope for spline slots on a given design.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SplineEnvelope {
    /// Allowed range for a slot centerline (walls respected at both faces)
    pub center_range: (f64, f64),
    /// Depth band of solid full-width wood at the front: `(0, D - rabbet_depth)`
    pub face_band: (f64, f64),
    /// Depth band occupied by the rabbet channel: `(D - rabbet_depth, D)`
    pub rabbet_band: (f64, f64),
    /// Recommended slot placement(s)
    pub recommended: Vec<SplineSlot>,
}

/// Compute the safe spline-slot envelope for a design, or `None` when the
/// moulding is too shallow to hold any slot with the requested walls.
pub fn spline_envelope(design: &FrameDesign, params: &SplineParams) -> Option<SplineEnvelope> {
    let w = design.frame_material_width;
    let d = design.frame_material_depth;
    let rabbet_w = design.rabbet_width.min(w);
    let rabbet_d = design.rabbet_depth.min(d);
    let t = params.slot_thickness;
    let wall = params.min_wall;
    let half = t / 2.0;

    let center_min = wall + half;
    let center_max = d - wall - half;
    if center_min > center_max {
        return None;
    }

    let face_depth = d - rabbet_d;
    let mut recommended = Vec::new();
    let centers: Vec<f64> = if d >= TWO_SLOT_MIN_DEPTH {
        vec![d / 3.0, 2.0 * d / 3.0]
    } else {
        // A single slot centers on the moulding depth — visually balanced on
        // the profile; when its band crosses the rabbet channel the
        // penetration rule below shortens it to stay clear of the stack.
        // Retreat to the solid face band only when the centered slot would
        // have no usable depth at all (very wide rabbets).
        let centered = d / 2.0;
        let over = centered + half > face_depth - wall;
        let centered_pen = if over { w - rabbet_w - wall } else { w - wall };
        if centered_pen > 0.0 || face_depth < t + 2.0 * wall {
            vec![centered]
        } else {
            vec![face_depth / 2.0]
        }
    };

    for c in centers {
        let z_center = c.clamp(center_min, center_max);
        // Full penetration requires `wall` clearance from the rabbet band
        let over_rabbet = z_center + half > face_depth - wall;
        let max_penetration = if over_rabbet {
            w - rabbet_w - wall
        } else {
            w - wall
        };
        if max_penetration <= 0.0 {
            continue;
        }
        // Drop a second slot that clamping has pushed onto the first
        if let Some(prev) = recommended.last() {
            let prev: &SplineSlot = prev;
            if (z_center - prev.z_center).abs() < t + wall {
                continue;
            }
        }
        recommended.push(SplineSlot {
            z_center,
            max_penetration,
            max_penetration_diagonal: max_penetration * std::f64::consts::SQRT_2,
            over_rabbet,
        });
    }

    Some(SplineEnvelope {
        center_range: (center_min, center_max),
        face_band: (0.0, face_depth),
        rabbet_band: (face_depth, d),
        recommended,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design(width: f64, depth: f64, rabbet_w: f64, rabbet_d: f64) -> FrameDesign {
        FrameDesign {
            frame_material_width: width,
            frame_material_depth: depth,
            rabbet_width: rabbet_w,
            rabbet_depth: rabbet_d,
            ..FrameDesign::default()
        }
    }

    fn assert_close(actual: f64, expected: f64, what: &str) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{what}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn single_slot_centers_on_depth_and_clears_the_stack() {
        // Default-ish moulding: 1" x 3/4", rabbet 3/8 x 3/8. Centered at
        // D/2 the band crosses the channel, so penetration shortens to
        // width - rabbet - wall.
        let env = spline_envelope(&design(1.0, 0.75, 0.375, 0.375), &SplineParams::default())
            .expect("envelope");
        assert_eq!(env.recommended.len(), 1);
        let slot = env.recommended[0];
        assert_close(slot.z_center, 0.375, "centered on moulding depth");
        assert!(slot.over_rabbet);
        assert_close(slot.max_penetration, 0.5, "shortened clear of the stack");
        assert_close(
            slot.max_penetration_diagonal,
            0.5 * std::f64::consts::SQRT_2,
            "diagonal penetration",
        );
    }

    #[test]
    fn very_wide_rabbet_retreats_to_the_face_band() {
        // Rabbet 0.9 of a 1" moulding: a centered slot would have no usable
        // depth (1 - 0.9 - 0.125 < 0), so the slot retreats to the solid
        // face band where full penetration is safe.
        let env = spline_envelope(&design(1.0, 0.75, 0.9, 0.375), &SplineParams::default())
            .expect("envelope");
        assert_eq!(env.recommended.len(), 1);
        let slot = env.recommended[0];
        assert_close(slot.z_center, 0.1875, "centered in the face band");
        assert!(!slot.over_rabbet);
        assert_close(slot.max_penetration, 0.875, "full width minus wall");
    }

    #[test]
    fn slot_over_rabbet_band_stops_at_channel_wall() {
        // Shallow moulding: face band (1/8") too thin, slot must sit across
        // the rabbet band and lose penetration to the channel wall.
        let env = spline_envelope(&design(1.0, 0.5, 0.375, 0.375), &SplineParams::default())
            .expect("envelope");
        assert_eq!(env.recommended.len(), 1);
        let slot = env.recommended[0];
        assert_close(slot.z_center, 0.25, "centered in depth");
        assert!(slot.over_rabbet);
        assert_close(slot.max_penetration, 0.5, "width - rabbet - wall");
    }

    #[test]
    fn too_shallow_moulding_has_no_envelope() {
        assert!(spline_envelope(&design(1.0, 0.25, 0.375, 0.125), &SplineParams::default())
            .is_none());
    }

    #[test]
    fn deep_moulding_gets_two_slots() {
        let env = spline_envelope(&design(1.0, 1.5, 0.375, 0.375), &SplineParams::default())
            .expect("envelope");
        assert_eq!(env.recommended.len(), 2);
        let (front, back) = (env.recommended[0], env.recommended[1]);
        assert_close(front.z_center, 0.5, "front slot at D/3");
        assert!(!front.over_rabbet);
        assert_close(front.max_penetration, 0.875, "front slot clear of rabbet");
        assert_close(back.z_center, 1.0, "back slot at 2D/3");
        assert!(back.over_rabbet);
        assert_close(back.max_penetration, 0.5, "back slot limited by channel");
    }

    #[test]
    fn envelope_bands_reflect_rabbet_geometry() {
        let env = spline_envelope(&design(1.0, 0.75, 0.375, 0.375), &SplineParams::default())
            .expect("envelope");
        assert_close(env.face_band.1, 0.375, "face band depth");
        assert_close(env.rabbet_band.0, 0.375, "rabbet band start");
        assert_close(env.rabbet_band.1, 0.75, "rabbet band end");
    }
}
