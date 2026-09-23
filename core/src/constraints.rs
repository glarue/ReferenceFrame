//! Input-constraint policy: the one place both platforms clamp user input.
//!
//! [`apply_input_constraints`] normalizes a design (symmetric mat, no-margin
//! overlap, non-negative float reveal) and clamps values the physical design
//! can't hold, honoring the user's [`ValidationConfig`] (min lip, min face,
//! min visible opening). Every clamp is reported as a [`ConstraintAdjustment`]
//! with a short, unit-aware message suitable for a snackbar/notice.
//! Normalizations are silent — they follow directly from flags the user set.
//!
//! This is not validation: it fixes what can be fixed on input. Anything left
//! out of range is reported by `validation::validate_design`.

use serde::{Deserialize, Serialize};

use crate::conversions::{format_value, Unit};
use crate::frame::{FrameDesign, FrameStyle};
use crate::validation::ValidationConfig;

/// Smallest frame/rabbet dimension kept after clamping (1/16").
const MIN_DIMENSION: f64 = 0.0625;

/// Tolerance so values already at a limit (e.g. stored after a previous
/// clamp, or round-tripped through mm) don't produce repeat notices.
const EPS: f64 = 1e-9;

/// One value the policy changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintAdjustment {
    /// `FrameDesign` field name (e.g. "rabbet_width")
    pub field: String,
    /// Value before clamping (inches)
    pub old: f64,
    /// Value after clamping (inches)
    pub new: f64,
    /// Short user-facing message, e.g. `Rabbet width limited to 5/8" (leaves 1/8" lip)`
    pub message: String,
}

/// The constrained design plus what changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintOutcome {
    pub design: FrameDesign,
    pub adjustments: Vec<ConstraintAdjustment>,
}

/// Apply the input-constraint policy to a copy of `design`.
///
/// Normalizations (silent):
/// - `symmetrical_mat` → sides mirror top/bottom
/// - `no_artwork_margin` → `mat_overlap` = 0
/// - `float_reveal` ≥ 0
///
/// Clamps (reported):
/// - frame width/depth and rabbet width/depth ≥ 1/16"
/// - `mat_overlap` ≤ min(artwork)/2 − `min_visible_opening`, floored at 0
///   (only when the design has a mat)
/// - `rabbet_width` ≤ frame width − `min_lip_width` (Rabbet and SightSize;
///   Float only ≤ frame width until Float Phase 2)
/// - `rabbet_depth` ≤ frame depth − `min_face_depth`
///
/// When the configured lip/face would leave less than a 1/16" rabbet, the
/// rabbet is only held to the frame itself; validation flags the rest.
/// Non-finite values are left untouched for validation to report.
pub fn apply_input_constraints(
    design: &FrameDesign,
    config: &ValidationConfig,
    use_mm: bool,
) -> ConstraintOutcome {
    let unit = if use_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |v: f64| format_value(v, unit);
    let mut d = design.clone();
    let mut adjustments = Vec::new();
    let mut record = |field: &str, old: f64, new: f64, message: String| {
        adjustments.push(ConstraintAdjustment { field: field.to_string(), old, new, message });
    };

    // --- Normalizations (silent) ---
    if d.symmetrical_mat && d.mat_width_sides != d.mat_width_top_bottom {
        d.mat_width_sides = d.mat_width_top_bottom;
    }
    if d.no_artwork_margin {
        d.mat_overlap = 0.0;
    }
    if d.float_reveal < 0.0 {
        d.float_reveal = 0.0;
    }

    // --- Minimum dimensions ---
    for (field, label, value) in [
        ("frame_material_width", "Frame width", &mut d.frame_material_width),
        ("frame_material_depth", "Frame depth", &mut d.frame_material_depth),
        ("rabbet_width", "Rabbet width", &mut d.rabbet_width),
        ("rabbet_depth", "Rabbet depth", &mut d.rabbet_depth),
    ] {
        if *value < MIN_DIMENSION {
            let old = *value;
            *value = MIN_DIMENSION;
            record(field, old, MIN_DIMENSION,
                format!("{} raised to {} (minimum)", label, fmt(MIN_DIMENSION)));
        }
    }

    // --- Mat overlap: keep min_visible_opening of art showing each side ---
    if d.has_mat() {
        let min_dim = d.artwork_height.min(d.artwork_width);
        let max_overlap = (min_dim / 2.0 - config.min_visible_opening).max(0.0);
        if d.mat_overlap > max_overlap + EPS {
            let old = d.mat_overlap;
            d.mat_overlap = max_overlap;
            let why = if max_overlap > 0.0 {
                format!("keeps a {} opening", fmt(2.0 * config.min_visible_opening))
            } else {
                "artwork too small for an overlap".to_string()
            };
            record("mat_overlap", old, max_overlap,
                format!("Mat overlap limited to {} ({})", fmt(max_overlap), why));
        }
    }

    // --- Rabbet width: leave the configured lip ---
    // Float is held only to the frame width until Float Phase 2 defines its
    // channel (validation exempts it from the lip check too).
    let frame_w = d.frame_material_width;
    let lip_max = frame_w - config.min_lip_width;
    let (max_rw, why_rw) = if d.frame_style != FrameStyle::Float && lip_max >= MIN_DIMENSION {
        (lip_max, format!("leaves {} lip", fmt(config.min_lip_width)))
    } else {
        (frame_w, "frame width".to_string())
    };
    if d.rabbet_width > max_rw + EPS {
        let old = d.rabbet_width;
        d.rabbet_width = max_rw;
        record("rabbet_width", old, max_rw,
            format!("Rabbet width limited to {} ({})", fmt(max_rw), why_rw));
    }

    // --- Rabbet depth: leave the configured face ---
    let frame_d = d.frame_material_depth;
    let face_max = frame_d - config.min_face_depth;
    let (max_rd, why_rd) = if face_max >= MIN_DIMENSION {
        (face_max, format!("leaves {} face", fmt(config.min_face_depth)))
    } else {
        (frame_d, "frame depth".to_string())
    };
    if d.rabbet_depth > max_rd + EPS {
        let old = d.rabbet_depth;
        d.rabbet_depth = max_rd;
        record("rabbet_depth", old, max_rd,
            format!("Rabbet depth limited to {} ({})", fmt(max_rd), why_rd));
    }

    ConstraintOutcome { design: d, adjustments }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn apply(design: &FrameDesign) -> ConstraintOutcome {
        apply_input_constraints(design, &ValidationConfig::default(), false)
    }

    fn only_field<'a>(o: &'a ConstraintOutcome, field: &str) -> &'a ConstraintAdjustment {
        let hits: Vec<_> = o.adjustments.iter().filter(|a| a.field == field).collect();
        assert_eq!(hits.len(), 1, "expected one {field} adjustment: {:?}", o.adjustments);
        hits[0]
    }

    #[test]
    fn default_design_is_untouched() {
        let d = FrameDesign::default();
        let o = apply(&d);
        assert!(o.adjustments.is_empty(), "{:?}", o.adjustments);
        assert_eq!(o.design, d);
    }

    #[test]
    fn rabbet_width_leaves_min_lip() {
        let d = FrameDesign { frame_material_width: 0.75, rabbet_width: 0.75, ..FrameDesign::new(8.0, 10.0) };
        let o = apply(&d);
        assert!(close(o.design.rabbet_width, 0.625));
        let a = only_field(&o, "rabbet_width");
        assert!(close(a.old, 0.75) && close(a.new, 0.625));
        assert_eq!(a.message, "Rabbet width limited to 5/8\" (leaves 1/8\" lip)");
    }

    #[test]
    fn rabbet_width_lip_applies_to_sight_size_but_not_float() {
        let sight = FrameDesign {
            frame_style: FrameStyle::SightSize,
            frame_material_width: 0.75,
            rabbet_width: 0.75,
            ..FrameDesign::new(8.0, 10.0)
        };
        assert!(close(apply(&sight).design.rabbet_width, 0.625));

        // Float (Phase 2 pending): only held to the frame width
        let float = FrameDesign { frame_style: FrameStyle::Float, ..sight.clone() };
        let o = apply(&float);
        assert!(close(o.design.rabbet_width, 0.75));
        assert!(o.adjustments.is_empty(), "{:?}", o.adjustments);
        let over = FrameDesign { rabbet_width: 1.0, ..float };
        let a = only_field(&apply(&over), "rabbet_width").clone();
        assert_eq!(a.message, "Rabbet width limited to 3/4\" (frame width)");
    }

    #[test]
    fn rabbet_depth_leaves_min_face() {
        let d = FrameDesign { frame_material_depth: 0.75, rabbet_depth: 1.0, ..FrameDesign::new(8.0, 10.0) };
        let o = apply(&d);
        assert!(close(o.design.rabbet_depth, 0.625));
        assert_eq!(only_field(&o, "rabbet_depth").message,
            "Rabbet depth limited to 5/8\" (leaves 1/8\" face)");
    }

    #[test]
    fn thin_frame_falls_back_to_frame_bound() {
        // 0.15" frame − 1/8" lip leaves < 1/16": hold the rabbet to the frame only
        let d = FrameDesign {
            frame_material_width: 0.15, rabbet_width: 0.5,
            frame_material_depth: 0.1, rabbet_depth: 0.5,
            ..FrameDesign::new(8.0, 10.0)
        };
        let o = apply(&d);
        assert!(close(o.design.rabbet_width, 0.15));
        assert!(close(o.design.rabbet_depth, 0.1));
        assert!(only_field(&o, "rabbet_depth").message.ends_with("(frame depth)"));
    }

    #[test]
    fn mat_overlap_keeps_min_visible_opening() {
        let d = FrameDesign { mat_overlap: 5.0, ..FrameDesign::new(4.0, 6.0) };
        let o = apply(&d);
        // min(4, 6)/2 − 1/8 = 1.875
        assert!(close(o.design.mat_overlap, 1.875));
        assert_eq!(only_field(&o, "mat_overlap").message,
            "Mat overlap limited to 1 7/8\" (keeps a 1/4\" opening)");
    }

    #[test]
    fn mat_overlap_floors_at_zero_for_tiny_art() {
        let d = FrameDesign { mat_overlap: 0.125, ..FrameDesign::new(0.2, 6.0) };
        let o = apply(&d);
        assert!(close(o.design.mat_overlap, 0.0));
        assert!(only_field(&o, "mat_overlap").message.contains("artwork too small"));
    }

    #[test]
    fn mat_overlap_ignored_without_mat() {
        let no_mat = FrameDesign {
            mat_width_top_bottom: 0.0, mat_width_sides: 0.0, mat_overlap: 5.0,
            ..FrameDesign::new(4.0, 6.0)
        };
        assert!(apply(&no_mat).adjustments.is_empty());
        let sight = FrameDesign { frame_style: FrameStyle::SightSize, mat_overlap: 5.0, ..FrameDesign::new(4.0, 6.0) };
        assert!(apply(&sight).adjustments.is_empty());
    }

    #[test]
    fn minimum_dimensions_are_raised() {
        let d = FrameDesign {
            frame_material_width: 0.0, frame_material_depth: 0.0,
            rabbet_width: 0.0, rabbet_depth: 0.0,
            ..FrameDesign::new(8.0, 10.0)
        };
        let o = apply(&d);
        for f in ["frame_material_width", "frame_material_depth", "rabbet_width", "rabbet_depth"] {
            assert!(close(only_field(&o, f).new, MIN_DIMENSION), "{f}");
        }
        assert_eq!(only_field(&o, "frame_material_width").message, "Frame width raised to 1/16\" (minimum)");
    }

    #[test]
    fn custom_config_is_honored() {
        let config = ValidationConfig {
            min_lip_width: 0.25,
            min_face_depth: 0.25,
            min_visible_opening: 0.5,
            ..ValidationConfig::default()
        };
        let d = FrameDesign {
            frame_material_width: 1.0, rabbet_width: 1.0,
            frame_material_depth: 1.0, rabbet_depth: 1.0,
            mat_overlap: 3.0,
            ..FrameDesign::new(4.0, 6.0)
        };
        let o = apply_input_constraints(&d, &config, false);
        assert!(close(o.design.rabbet_width, 0.75));
        assert!(close(o.design.rabbet_depth, 0.75));
        assert!(close(o.design.mat_overlap, 1.5)); // 4/2 − 1/2
        assert_eq!(only_field(&o, "rabbet_width").message, "Rabbet width limited to 3/4\" (leaves 1/4\" lip)");
        assert_eq!(only_field(&o, "mat_overlap").message, "Mat overlap limited to 1 1/2\" (keeps a 1\" opening)");
    }

    #[test]
    fn messages_are_unit_aware() {
        let d = FrameDesign { frame_material_width: 0.75, rabbet_width: 0.75, ..FrameDesign::new(8.0, 10.0) };
        let o = apply_input_constraints(&d, &ValidationConfig::default(), true);
        assert_eq!(only_field(&o, "rabbet_width").message,
            "Rabbet width limited to 15.9 mm (leaves 3.2 mm lip)");
    }

    #[test]
    fn values_already_at_a_limit_are_silent() {
        let d = FrameDesign { frame_material_width: 0.75, rabbet_width: 0.625, ..FrameDesign::new(8.0, 10.0) };
        assert!(apply(&d).adjustments.is_empty());
    }

    #[test]
    fn normalizations_are_silent() {
        let d = FrameDesign {
            symmetrical_mat: true, mat_width_top_bottom: 3.0, mat_width_sides: 2.0,
            no_artwork_margin: true, mat_overlap: 0.5,
            float_reveal: -1.0,
            ..FrameDesign::new(8.0, 10.0)
        };
        let o = apply(&d);
        assert!(o.adjustments.is_empty(), "{:?}", o.adjustments);
        assert!(close(o.design.mat_width_sides, 3.0));
        assert!(close(o.design.mat_overlap, 0.0));
        assert!(close(o.design.float_reveal, 0.0));
    }

    #[test]
    fn non_finite_values_are_left_for_validation() {
        let d = FrameDesign { frame_material_width: f64::NAN, ..FrameDesign::new(8.0, 10.0) };
        let o = apply(&d);
        assert!(o.design.frame_material_width.is_nan(), "NaN must survive so validation reports it");
    }

    #[test]
    fn outcome_serializes_for_bindings() {
        let d = FrameDesign { frame_material_width: 0.75, rabbet_width: 0.75, ..FrameDesign::new(8.0, 10.0) };
        let json = serde_json::to_value(apply(&d).adjustments).unwrap();
        assert_eq!(json[0]["field"], "rabbet_width");
        assert_eq!(json[0]["new"], 0.625);
        assert!(json[0]["message"].is_string());
    }
}
