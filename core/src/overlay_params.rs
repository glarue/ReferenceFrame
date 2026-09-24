//! Spline/hanging parameter overrides and the JSON both bindings return for
//! the joinery, hanging and weight cards.
//!
//! The apps send overrides (user-customized kerf, wall, hanger drop, …) as
//! `{"spline": {...}, "hanging": {...}}`; anything missing falls back to the
//! presets.json defaults.

use serde::Deserialize;

use crate::frame::FrameDesign;
use crate::hanging::{hanging_layout, HangingParams};
use crate::joinery::{spline_envelope, SplineParams};
use crate::weight::{estimate_weight, WeightParams};

/// Optional overrides for the spline and hanging parameters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
pub struct OverlayParams {
    pub spline: Option<SplineParams>,
    pub hanging: Option<HangingParams>,
}

impl OverlayParams {
    /// Parse `{"spline": {...}, "hanging": {...}}`. A missing section, or a
    /// missing field inside one, takes the presets.json default. `None` or
    /// JSON that doesn't parse means no overrides at all (overlays never
    /// fail a render).
    pub fn from_json(json: Option<&str>) -> Self {
        json.and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default()
    }

    /// Spline parameters: the override, else the presets defaults
    pub fn spline(&self) -> SplineParams {
        self.spline.unwrap_or_default()
    }

    /// Hanging parameters: the override, else the presets defaults
    pub fn hanging(&self) -> HangingParams {
        self.hanging.unwrap_or_default()
    }
}

/// Spline slot planning as JSON `{"params": …, "envelope": …}`, or `"null"`
/// when the moulding can't hold a slot with these parameters.
pub fn spline_envelope_json(design: &FrameDesign, overlays: &OverlayParams) -> String {
    let params = overlays.spline();
    match spline_envelope(design, &params) {
        Some(env) => serde_json::json!({ "params": params, "envelope": env }).to_string(),
        None => "null".to_string(),
    }
}

/// Hanging hardware layout as JSON, or `"null"` when the frame is too narrow.
pub fn hanging_layout_json(design: &FrameDesign, overlays: &OverlayParams) -> String {
    match hanging_layout(design, &overlays.hanging()) {
        Some(layout) => serde_json::to_string(&layout).unwrap_or_else(|_| "null".to_string()),
        None => "null".to_string(),
    }
}

/// Weight + wire-tension estimate as JSON. The keys pick materials-index
/// entries (defaults "generic" wood, "glass", "foamcore"; unknown keys keep
/// the default — see [`WeightParams::from_material_keys`]); the hanging
/// override sets the wire geometry for the tension.
pub fn weight_estimate_json(
    design: &FrameDesign,
    wood_key: Option<&str>,
    glazing_key: Option<&str>,
    backing_key: Option<&str>,
    overlays: &OverlayParams,
) -> String {
    let params = WeightParams::from_material_keys(wood_key, glazing_key, backing_key);
    let est = estimate_weight(design, &params, &overlays.hanging());
    serde_json::to_string(&est).unwrap_or_else(|_| "null".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_override_keeps_given_fields_and_defaults_the_rest() {
        // Before `#[serde(default)]` on the param structs, one missing field
        // dropped both overrides.
        let o = OverlayParams::from_json(Some(
            r#"{"spline": {"min_wall": 0.25}, "hanging": {"drop_fraction": 0.5}}"#,
        ));
        let spline_defaults = SplineParams::default();
        let hanging_defaults = HangingParams::default();
        assert_eq!(o.spline(), SplineParams { min_wall: 0.25, ..spline_defaults });
        assert_eq!(o.hanging(), HangingParams { drop_fraction: 0.5, ..hanging_defaults });
    }

    #[test]
    fn missing_or_bad_json_means_defaults() {
        for json in [None, Some(""), Some("{oops"), Some("{}"), Some("null")] {
            let o = OverlayParams::from_json(json);
            assert_eq!(o.spline(), SplineParams::default(), "{json:?}");
            assert_eq!(o.hanging(), HangingParams::default(), "{json:?}");
        }
    }

    #[test]
    fn card_json_shapes() {
        let design = FrameDesign::default();
        let none = OverlayParams::default();
        let spline: serde_json::Value =
            serde_json::from_str(&spline_envelope_json(&design, &none)).unwrap();
        assert!(spline["params"]["slot_thickness"].is_number());
        assert!(!spline["envelope"]["recommended"].as_array().unwrap().is_empty());
        let hanging: serde_json::Value =
            serde_json::from_str(&hanging_layout_json(&design, &none)).unwrap();
        assert!(hanging["wire_cut_length"].as_f64().unwrap() > 0.0);

        // Unknown material keys fall back to the defaults
        assert_eq!(
            weight_estimate_json(&design, Some("unobtainium"), None, Some("nope"), &none),
            weight_estimate_json(&design, None, None, None, &none),
        );
        assert_ne!(
            weight_estimate_json(&design, Some("basswood"), Some("acrylic"), None, &none),
            weight_estimate_json(&design, None, None, None, &none),
        );
    }
}
