//! One JSON options object for the bindings' SVG diagram entry points.
//!
//! A request is a [`DiagramOptions`] object (snake_case keys, flattened)
//! plus a `theme` choosing the [`DiagramStyle`]:
//!
//! ```json
//! { "view": "Both", "canvas_width": 1200, "canvas_height": 1200,
//!   "include_title_block": true, "title_text": null, "unit_mm": false,
//!   "use_tape_segments": false, "use_decimal_display": false,
//!   "show_callouts": true, "show_spline": false, "show_hanging": false,
//!   "theme": "pdf" }
//! ```
//!
//! `view` is `"PlanOnly"`, `"SectionOnly"` or `"Both"`; `detail_mode`
//! (`"Auto"`/`"None"`), `corner_detail_enabled`/`axis_breaks_enabled`
//! (default true), `show_spline`/`show_hanging` (default false),
//! `spline_params`/`hanging_params` (default: presets) and `wood`
//! (default null = no grain; see [`crate::visualization::WoodRender`]) may be omitted.

use serde::Deserialize;

use crate::frame::FrameDesign;
use crate::visualization::{generate_diagram_with_style, DiagramOptions, DiagramStyle};

/// Diagram color scheme.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramTheme {
    /// On-screen light (`DiagramStyle::default()`)
    #[default]
    Light,
    /// On-screen dark (`DiagramStyle::for_dark()`)
    Dark,
    /// PDF/print: always light, print-tuned (`DiagramStyle::for_pdf()`)
    Pdf,
}

impl DiagramTheme {
    pub fn style(self) -> DiagramStyle {
        match self {
            DiagramTheme::Light => DiagramStyle::default(),
            DiagramTheme::Dark => DiagramStyle::for_dark(),
            DiagramTheme::Pdf => DiagramStyle::for_pdf(),
        }
    }
}

/// Diagram options plus theme, as sent by the web and iOS bindings.
#[derive(Debug, Clone, Deserialize)]
pub struct DiagramRequest {
    #[serde(flatten)]
    pub options: DiagramOptions,
    #[serde(default)]
    pub theme: DiagramTheme,
}

impl DiagramRequest {
    /// Parse a request; the error names the offending/missing field.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Render the diagram SVG for `design`.
    pub fn render(&self, design: &FrameDesign) -> String {
        generate_diagram_with_style(design, &self.options, &self.theme.style()).svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::{generate_diagram, DepthCues, DetailMode, ViewOption, WoodLod, WoodRender};

    const REQUIRED: &str = r#""canvas_width": 1200, "canvas_height": 700,
        "include_title_block": false, "unit_mm": false, "use_tape_segments": false,
        "use_decimal_display": false, "show_callouts": true"#;

    #[test]
    fn minimal_request_takes_documented_defaults() {
        let r = DiagramRequest::from_json(&format!(r#"{{"view": "Both", {REQUIRED}}}"#)).unwrap();
        let o = &r.options;
        assert_eq!(r.theme, DiagramTheme::Light);
        assert_eq!(o.view, ViewOption::Both);
        assert_eq!((o.canvas_width, o.canvas_height), (1200.0, 700.0));
        assert_eq!(o.title_text, None);
        assert_eq!(o.detail_mode, DetailMode::Auto);
        assert!(o.corner_detail_enabled && o.axis_breaks_enabled);
        assert!(!o.show_spline && !o.show_hanging);
        assert!(o.spline_params.is_none() && o.hanging_params.is_none());
        assert!(o.wood.is_none());
    }

    #[test]
    fn full_request_parses_every_field() {
        let r = DiagramRequest::from_json(&format!(
            r#"{{"view": "PlanOnly", {REQUIRED}, "title_text": "Hall", "detail_mode": "None",
               "corner_detail_enabled": false, "axis_breaks_enabled": false,
               "show_spline": true, "show_hanging": true,
               "spline_params": {{"slot_thickness": 0.1, "min_wall": 0.2}},
               "hanging_params": {{"drop_fraction": 0.25, "slack_fraction": 0.05, "wrap_allowance": 2.5}},
               "theme": "dark"}}"#
        ))
        .unwrap();
        let o = &r.options;
        assert_eq!(r.theme, DiagramTheme::Dark);
        assert_eq!(o.title_text.as_deref(), Some("Hall"));
        assert_eq!(o.detail_mode, DetailMode::None);
        assert!(!o.corner_detail_enabled && !o.axis_breaks_enabled);
        assert!(o.show_spline && o.show_hanging);
        assert_eq!(o.spline_params.unwrap().min_wall, 0.2);
        assert_eq!(o.hanging_params.unwrap().wrap_allowance, 2.5);
    }

    #[test]
    fn wood_takes_documented_defaults() {
        let r = DiagramRequest::from_json(&format!(r#"{{"view": "PlanOnly", {REQUIRED}, "wood": {{"species": "red_oak"}}}}"#))
            .unwrap();
        let want = WoodRender { species: "red_oak".into(), variant: None, lod: WoodLod::Grain, depth: DepthCues::Inner, reshuffle: 0 };
        assert_eq!(r.options.wood, Some(want));
        let r = DiagramRequest::from_json(&format!(
            r#"{{"view": "PlanOnly", {REQUIRED}, "wood": {{"species": "white_oak", "variant": "quartersawn",
               "lod": "flat", "depth": "inner_and_wall", "reshuffle": 3}}}}"#
        ))
        .unwrap();
        let w = r.options.wood.unwrap();
        assert_eq!((w.variant.as_deref(), w.lod, w.depth, w.reshuffle), (Some("quartersawn"), WoodLod::Flat, DepthCues::InnerAndWall, 3));
        assert!(DiagramRequest::from_json(&format!(r#"{{"view": "PlanOnly", {REQUIRED}, "wood": {{"species": "red_oak", "lod": "fine"}}}}"#)).is_err());
    }

    #[test]
    fn null_or_omitted_wood_renders_as_before() {
        let design = FrameDesign::default();
        for view in ["PlanOnly", "SectionOnly", "Both"] {
            let omitted = DiagramRequest::from_json(&format!(r#"{{"view": "{view}", {REQUIRED}}}"#)).unwrap();
            let null = DiagramRequest::from_json(&format!(r#"{{"view": "{view}", {REQUIRED}, "wood": null}}"#)).unwrap();
            assert_eq!(omitted.render(&design), null.render(&design), "{view}");
            assert!(!null.render(&design).contains(r#"id="wood""#));
        }
    }

    #[test]
    fn missing_required_field_is_an_error() {
        let err = DiagramRequest::from_json(r#"{"view": "Both", "canvas_width": 10}"#).unwrap_err();
        assert!(err.contains("missing field"), "{err}");
        assert!(DiagramRequest::from_json(&format!(r#"{{"view": "Both", {REQUIRED}, "theme": "sepia"}}"#)).is_err());
    }

    #[test]
    fn render_matches_direct_generation() {
        let design = FrameDesign::default();
        let r = DiagramRequest::from_json(&format!(r#"{{"view": "Both", {REQUIRED}}}"#)).unwrap();
        assert_eq!(r.render(&design), generate_diagram(&design, &r.options).svg);
        let dark = DiagramRequest { theme: DiagramTheme::Dark, ..r.clone() };
        let pdf = DiagramRequest { theme: DiagramTheme::Pdf, ..r };
        assert_ne!(dark.render(&design), pdf.render(&design));
    }
}
