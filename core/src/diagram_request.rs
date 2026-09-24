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
//! (default true), `show_spline`/`show_hanging` (default false) and
//! `spline_params`/`hanging_params` (default: presets) may be omitted.

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
    use crate::visualization::{generate_diagram, DetailMode, ViewOption};

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
