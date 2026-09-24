// WASM bindings for browser JavaScript
//
// This module exposes the Rust API to JavaScript via wasm-bindgen
// It wraps the pure Rust types from referenceframe_core with wasm_bindgen annotations

use wasm_bindgen::prelude::*;
use referenceframe_core::{
    conversions::{self, Unit},
    frame::{FrameDesign, FrameStyle},
    shareable_url::{self, ShareableParams},
    overlay_params::{self, OverlayParams},
    presets,
    version,
    history,
};

/// Get the version of the core library compiled into this WASM build
/// (core's `CARGO_PKG_VERSION`, e.g. "1.10.0"). Same value as `getCoreVersion`.
#[wasm_bindgen(js_name = "getWasmVersion")]
pub fn get_wasm_version() -> String {
    version::get_core_version().to_string()
}

/// WASM-friendly wrapper for FrameDesign
#[wasm_bindgen]
pub struct WasmFrameDesign {
    pub(crate) inner: FrameDesign,
}

#[wasm_bindgen]
impl WasmFrameDesign {
    #[wasm_bindgen(constructor)]
    pub fn new(artwork_height: f64, artwork_width: f64) -> WasmFrameDesign {
        WasmFrameDesign {
            inner: FrameDesign::new(artwork_height, artwork_width),
        }
    }

    // Getters for all fields
    #[wasm_bindgen(getter, js_name = "artworkHeight")]
    pub fn artwork_height(&self) -> f64 {
        self.inner.artwork_height
    }

    #[wasm_bindgen(setter, js_name = "artworkHeight")]
    pub fn set_artwork_height(&mut self, value: f64) {
        self.inner.artwork_height = value;
    }

    #[wasm_bindgen(getter, js_name = "artworkWidth")]
    pub fn artwork_width(&self) -> f64 {
        self.inner.artwork_width
    }

    #[wasm_bindgen(setter, js_name = "artworkWidth")]
    pub fn set_artwork_width(&mut self, value: f64) {
        self.inner.artwork_width = value;
    }

    #[wasm_bindgen(getter, js_name = "matWidthTopBottom")]
    pub fn mat_width_top_bottom(&self) -> f64 {
        self.inner.mat_width_top_bottom
    }

    #[wasm_bindgen(setter, js_name = "matWidthTopBottom")]
    pub fn set_mat_width_top_bottom(&mut self, value: f64) {
        self.inner.mat_width_top_bottom = value;
    }

    #[wasm_bindgen(getter, js_name = "matWidthSides")]
    pub fn mat_width_sides(&self) -> f64 {
        self.inner.mat_width_sides
    }

    #[wasm_bindgen(setter, js_name = "matWidthSides")]
    pub fn set_mat_width_sides(&mut self, value: f64) {
        self.inner.mat_width_sides = value;
    }

    #[wasm_bindgen(getter, js_name = "matOverlap")]
    pub fn mat_overlap(&self) -> f64 {
        self.inner.mat_overlap
    }

    #[wasm_bindgen(setter, js_name = "matOverlap")]
    pub fn set_mat_overlap(&mut self, value: f64) {
        self.inner.mat_overlap = value;
    }

    #[wasm_bindgen(getter, js_name = "rabbetWidth")]
    pub fn rabbet_width(&self) -> f64 {
        self.inner.rabbet_width
    }

    #[wasm_bindgen(setter, js_name = "rabbetWidth")]
    pub fn set_rabbet_width(&mut self, value: f64) {
        self.inner.rabbet_width = value;
    }

    #[wasm_bindgen(getter, js_name = "rabbetDepth")]
    pub fn rabbet_depth(&self) -> f64 {
        self.inner.rabbet_depth
    }

    #[wasm_bindgen(setter, js_name = "rabbetDepth")]
    pub fn set_rabbet_depth(&mut self, value: f64) {
        self.inner.rabbet_depth = value;
    }

    #[wasm_bindgen(getter, js_name = "frameWidth")]
    pub fn frame_width(&self) -> f64 {
        self.inner.frame_material_width
    }

    #[wasm_bindgen(setter, js_name = "frameWidth")]
    pub fn set_frame_width(&mut self, value: f64) {
        self.inner.frame_material_width = value;
    }

    #[wasm_bindgen(getter, js_name = "frameDepth")]
    pub fn frame_depth(&self) -> f64 {
        self.inner.frame_material_depth
    }

    #[wasm_bindgen(setter, js_name = "frameDepth")]
    pub fn set_frame_depth(&mut self, value: f64) {
        self.inner.frame_material_depth = value;
    }

    // Material thickness getters/setters

    #[wasm_bindgen(getter, js_name = "glazingThickness")]
    pub fn glazing_thickness(&self) -> f64 {
        self.inner.glazing_thickness
    }

    #[wasm_bindgen(setter, js_name = "glazingThickness")]
    pub fn set_glazing_thickness(&mut self, value: f64) {
        self.inner.glazing_thickness = value;
    }

    #[wasm_bindgen(getter, js_name = "matboardThickness")]
    pub fn matboard_thickness(&self) -> f64 {
        self.inner.matboard_thickness
    }

    #[wasm_bindgen(setter, js_name = "matboardThickness")]
    pub fn set_matboard_thickness(&mut self, value: f64) {
        self.inner.matboard_thickness = value;
    }

    #[wasm_bindgen(getter, js_name = "artworkThickness")]
    pub fn artwork_thickness(&self) -> f64 {
        self.inner.artwork_thickness
    }

    #[wasm_bindgen(setter, js_name = "artworkThickness")]
    pub fn set_artwork_thickness(&mut self, value: f64) {
        self.inner.artwork_thickness = value;
    }

    #[wasm_bindgen(getter, js_name = "backingThickness")]
    pub fn backing_thickness(&self) -> f64 {
        self.inner.backing_thickness
    }

    #[wasm_bindgen(setter, js_name = "backingThickness")]
    pub fn set_backing_thickness(&mut self, value: f64) {
        self.inner.backing_thickness = value;
    }

    #[wasm_bindgen(getter, js_name = "assemblyMargin")]
    pub fn assembly_margin(&self) -> f64 {
        self.inner.assembly_margin
    }

    #[wasm_bindgen(setter, js_name = "assemblyMargin")]
    pub fn set_assembly_margin(&mut self, value: f64) {
        self.inner.assembly_margin = value;
    }

    #[wasm_bindgen(getter, js_name = "symmetricalMat")]
    pub fn symmetrical_mat(&self) -> bool {
        self.inner.symmetrical_mat
    }

    #[wasm_bindgen(setter, js_name = "symmetricalMat")]
    pub fn set_symmetrical_mat(&mut self, value: bool) {
        self.inner.symmetrical_mat = value;
    }

    /// Frame style as a snake_case string: "rabbet", "sight_size", or "float".
    #[wasm_bindgen(getter, js_name = "frameStyle")]
    pub fn frame_style(&self) -> String {
        match self.inner.frame_style {
            FrameStyle::Rabbet => "rabbet",
            FrameStyle::SightSize => "sight_size",
            FrameStyle::Float => "float",
        }.to_string()
    }

    #[wasm_bindgen(setter, js_name = "frameStyle")]
    pub fn set_frame_style(&mut self, value: String) {
        self.inner.frame_style = match value.as_str() {
            "sight_size" => FrameStyle::SightSize,
            "float" => FrameStyle::Float,
            _ => FrameStyle::Rabbet,
        };
    }

    #[wasm_bindgen(getter, js_name = "floatReveal")]
    pub fn float_reveal(&self) -> f64 {
        self.inner.float_reveal
    }

    #[wasm_bindgen(setter, js_name = "floatReveal")]
    pub fn set_float_reveal(&mut self, value: f64) {
        self.inner.float_reveal = value;
    }

    /// Read-only, derived (there is no `include_mat` field): true when the
    /// design uses a mat, i.e. rabbet style with a non-zero border
    /// (`FrameDesign::has_mat`). Always false for sight-size/float.
    #[wasm_bindgen(getter, js_name = "includeMat")]
    pub fn has_mat(&self) -> bool {
        self.inner.has_mat()
    }

    // Calculation methods

    /// Apply the core input-constraint policy in place (mat symmetry, overlap
    /// and rabbet clamps honoring the user's min lip / min face / min visible
    /// opening, minimum dimensions).
    ///
    /// `configJson` is a `ValidationConfig` JSON (`validationConfig.toJson()`);
    /// `undefined`/`null`/unparseable uses the default limits. Returns a JSON
    /// array of what changed: `[{ "field", "old", "new", "message" }]`
    /// (`old`/`new` in inches; `message` short and unit-aware). This is NOT
    /// validation — use `validateDesign` for errors/warnings.
    #[wasm_bindgen(js_name = "applyInputConstraints")]
    pub fn apply_input_constraints(&mut self, config_json: Option<String>, use_mm: bool) -> String {
        let config: validation::ValidationConfig = config_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();
        let outcome = referenceframe_core::constraints::apply_input_constraints(&self.inner, &config, use_mm);
        self.inner = outcome.design;
        serde_json::to_string(&outcome.adjustments).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get frame inside dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getFrameInsideDimensions")]
    pub fn get_frame_inside_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_frame_inside_dimensions();
        vec![h, w]
    }

    /// Get frame outside dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getFrameOutsideDimensions")]
    pub fn get_frame_outside_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_frame_outside_dimensions();
        vec![h, w]
    }

    /// Get matboard dimensions (the exact rabbet opening) - returns [height, width]
    #[wasm_bindgen(js_name = "getMatboardDimensions")]
    pub fn get_matboard_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_matboard_dimensions();
        vec![h, w]
    }

    /// Get the drop-in cut size for glazing/backing/matboard-outer (rabbet
    /// opening minus assembly clearance per side) - returns [height, width]
    #[wasm_bindgen(js_name = "getFittedComponentDimensions")]
    pub fn get_fitted_component_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_fitted_component_dimensions();
        vec![h, w]
    }

    /// Get mat opening dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getMatOpeningDimensions")]
    pub fn get_mat_opening_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_mat_opening_dimensions();
        vec![h, w]
    }

    /// Get required rabbet z-axis depth
    #[wasm_bindgen(js_name = "getRabbetZDepthRequired")]
    pub fn get_rabbet_z_depth_required(&self) -> f64 {
        self.inner.get_rabbet_z_depth_required()
    }

    /// Get total wood length required
    #[wasm_bindgen(js_name = "getTotalWoodLength")]
    pub fn get_total_wood_length(&self, saw_margin: f64, error_margin: f64) -> f64 {
        self.inner.get_total_wood_length(saw_margin, error_margin)
    }

    /// Get cut list as JSON string
    #[wasm_bindgen(js_name = "getCutListJson")]
    pub fn get_cut_list_json(&self) -> String {
        let cut_list = self.inner.get_cut_list();
        serde_json::to_string(&cut_list).unwrap_or_default()
    }

    /// Export to JSON
    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    /// Import from JSON
    #[wasm_bindgen(js_name = "fromJson")]
    pub fn from_json(json: &str) -> Result<WasmFrameDesign, JsValue> {
        let design: FrameDesign = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        Ok(WasmFrameDesign { inner: design })
    }
}

// Free functions

#[wasm_bindgen(js_name = "inchesToMm")]
pub fn inches_to_mm(inches: f64) -> f64 {
    conversions::inches_to_mm(inches)
}

#[wasm_bindgen(js_name = "mmToInches")]
pub fn mm_to_inches(mm: f64) -> f64 {
    conversions::mm_to_inches(mm)
}

#[wasm_bindgen(js_name = "formatValue")]
pub fn format_value(value: f64, unit_mm: bool) -> String {
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    conversions::format_value(value, unit)
}

#[wasm_bindgen(js_name = "formatValueWithDecimal")]
pub fn format_value_with_decimal(value: f64, unit_mm: bool) -> String {
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    conversions::format_value_with_decimal(value, unit)
}

#[wasm_bindgen(js_name = "formatValueTapeMeasure")]
pub fn format_value_tape_measure(value: f64, unit_mm: bool) -> String {
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    conversions::format_value_tape_measure(value, unit)
}

#[wasm_bindgen(js_name = "getAspectRatioDisplay")]
pub fn get_aspect_ratio_display(height: f64, width: f64) -> String {
    referenceframe_core::aspect_ratio::get_aspect_ratio_display(height, width)
}

/// Shareable-link payload (the `?d=` value) for a design.
///
/// `bladeWidth` is the saw kerf setting (inches); `unitMm` the unit the link
/// opens in. `includeMat` is the "Include mat" switch: `false` sends no mat
/// (borders and overlap zeroed); `undefined` derives the flag from the
/// design (`ShareableParams::from_design`).
#[wasm_bindgen(js_name = "generateSharePayload")]
pub fn generate_share_payload(
    design: &WasmFrameDesign,
    blade_width: f64,
    unit_mm: bool,
    include_mat: Option<bool>,
) -> String {
    let mut params = ShareableParams::from_design(&design.inner, blade_width, unit_mm);
    if let Some(include) = include_mat {
        params = params.with_include_mat(include);
    }
    shareable_url::generate_shareable_url(&params)
}

/// Decode a shareable-link payload (the `?d=` value) to `ShareableParams`
/// JSON (snake_case keys, inches). Throws on a malformed payload.
#[wasm_bindgen(js_name = "decodeSharePayload")]
pub fn decode_share_payload(payload: &str) -> Result<String, JsValue> {
    let params = shareable_url::decode_payload(payload)
        .map_err(|e| JsValue::from_str(&format!("Decode error: {}", e)))?;
    serde_json::to_string(&params)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// Visualization functions

/// Pick the on-screen diagram style: dark mode or default (light).
/// PDF/print export uses `DiagramStyle::for_pdf()` instead and must stay light.
fn screen_style(dark_mode: bool) -> referenceframe_core::visualization::DiagramStyle {
    use referenceframe_core::visualization::DiagramStyle;
    if dark_mode {
        DiagramStyle::for_dark()
    } else {
        DiagramStyle::default()
    }
}

/// Generate a plan view SVG diagram
#[wasm_bindgen(js_name = "generatePlanViewSvg")]
pub fn generate_plan_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    use_tape_segments: bool,
    use_decimal_display: bool,
    corner_detail_enabled: bool,
    axis_breaks_enabled: bool,
    dark_mode: bool,
    show_spline: bool,
    show_hanging: bool,
    overlay_params_json: Option<String>,
) -> String {
    use referenceframe_core::visualization::{generate_diagram_with_style, DiagramOptions, ViewOption};

    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    let options = DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width,
        canvas_height,
        include_title_block: false,
        title_text: None,
        unit_mm,
        use_tape_segments,
        use_decimal_display,
        show_callouts: true,
        corner_detail_enabled,
        axis_breaks_enabled,
        show_spline,
        show_hanging,
        spline_params: overlays.spline,
        hanging_params: overlays.hanging,
        ..Default::default()
    };

    let style = screen_style(dark_mode);
    let result = generate_diagram_with_style(&design.inner, &options, &style);
    result.svg
}

/// Generate a section view SVG diagram
#[wasm_bindgen(js_name = "generateSectionViewSvg")]
pub fn generate_section_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    use_tape_segments: bool,
    use_decimal_display: bool,
    dark_mode: bool,
    show_spline: bool,
    show_hanging: bool,
    overlay_params_json: Option<String>,
) -> String {
    use referenceframe_core::visualization::{generate_diagram_with_style, DiagramOptions, ViewOption};

    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    let options = DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_width,
        canvas_height,
        include_title_block: false,
        title_text: None,
        unit_mm,
        use_tape_segments,
        use_decimal_display,
        show_callouts: true,
        show_spline,
        show_hanging,
        spline_params: overlays.spline,
        hanging_params: overlays.hanging,
        ..Default::default()
    };

    let style = screen_style(dark_mode);
    let result = generate_diagram_with_style(&design.inner, &options, &style);
    result.svg
}

/// Generate combined view SVG for on-screen display.
///
/// Honors the user's display format (tape/decimal); corner detail and axis
/// breaks are auto-enabled (they only render when geometry warrants).
#[wasm_bindgen(js_name = "generateCombinedViewSvg")]
pub fn generate_combined_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    include_title: bool,
    use_tape_segments: bool,
    use_decimal_display: bool,
    dark_mode: bool,
    show_spline: bool,
    show_hanging: bool,
    overlay_params_json: Option<String>,
) -> String {
    generate_combined_view_svg_with_title(
        design, canvas_width, canvas_height, unit_mm, include_title,
        false, None, use_tape_segments, use_decimal_display, true, true, dark_mode,
        show_spline, show_hanging, overlay_params_json,
    )
}

/// Generate combined view SVG with full control over styling and detail.
///
/// Exported to JS as `generateCombinedViewSvgForPdf`, but not PDF-only: it also
/// backs the on-screen `generateCombinedViewSvg`. `for_pdf` selects the light
/// print style (overriding `dark_mode`); otherwise `dark_mode` picks dark/light.
/// Parameters are positional: trailing args omitted from JS arrive as
/// `false`/`None`, which disables the corresponding display option.
///
/// title_text: Optional custom title for the diagram (e.g., "Living Room Landscape")
///             If None or empty, defaults to "Frame Design"
#[wasm_bindgen(js_name = "generateCombinedViewSvgForPdf")]
pub fn generate_combined_view_svg_with_title(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    include_title: bool,
    for_pdf: bool,
    title_text: Option<String>,
    use_tape_segments: bool,
    use_decimal_display: bool,
    corner_detail_enabled: bool,
    axis_breaks_enabled: bool,
    dark_mode: bool,
    show_spline: bool,
    show_hanging: bool,
    overlay_params_json: Option<String>,
) -> String {
    use referenceframe_core::visualization::{generate_diagram_with_style, DiagramOptions, DiagramStyle, ViewOption};

    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    let options = DiagramOptions {
        view: ViewOption::Both,
        canvas_width,
        canvas_height,
        include_title_block: include_title,
        title_text,
        unit_mm,
        use_tape_segments,
        use_decimal_display,
        show_callouts: true,
        corner_detail_enabled,
        axis_breaks_enabled,
        show_spline,
        show_hanging,
        spline_params: overlays.spline,
        hanging_params: overlays.hanging,
        ..Default::default()
    };

    // PDF/print stays light regardless of theme (printing dark wastes ink).
    let style = if for_pdf {
        DiagramStyle::for_pdf()
    } else if dark_mode {
        DiagramStyle::for_dark()
    } else {
        DiagramStyle::default()
    };

    let result = generate_diagram_with_style(&design.inner, &options, &style);
    result.svg
}

/// Spline slot planning data as JSON (params + envelope), or "null" when the
/// moulding can't hold a slot. `overlay_params_json` is
/// `{"spline": {...}, "hanging": {...}}` overrides (core `OverlayParams`;
/// missing parts use the presets).
#[wasm_bindgen(js_name = "getSplineEnvelope")]
pub fn get_spline_envelope(design: &WasmFrameDesign, overlay_params_json: Option<String>) -> String {
    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    overlay_params::spline_envelope_json(&design.inner, &overlays)
}

/// Hanging hardware layout as JSON, or "null" when the frame is too narrow.
#[wasm_bindgen(js_name = "getHangingLayout")]
pub fn get_hanging_layout(design: &WasmFrameDesign, overlay_params_json: Option<String>) -> String {
    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    overlay_params::hanging_layout_json(&design.inner, &overlays)
}

/// Sourced material-density index (for species/material pickers)
#[wasm_bindgen(js_name = "getMaterialsJson")]
pub fn get_materials_json() -> String {
    serde_json::to_string(referenceframe_core::presets::get_materials()).unwrap_or_default()
}

/// Weight + wire-tension estimate as JSON. `wood_key`/`glazing_key`/`backing_key`
/// select entries from the materials index (defaults: "generic" wood, "glass",
/// "foamcore"); a missing or unknown key falls back to that default.
/// `overlay_params_json` supplies hanging-parameter overrides.
#[wasm_bindgen(js_name = "getWeightEstimate")]
pub fn get_weight_estimate(
    design: &WasmFrameDesign,
    wood_key: Option<String>,
    glazing_key: Option<String>,
    backing_key: Option<String>,
    overlay_params_json: Option<String>,
) -> String {
    let overlays = OverlayParams::from_json(overlay_params_json.as_deref());
    overlay_params::weight_estimate_json(
        &design.inner,
        wood_key.as_deref(),
        glazing_key.as_deref(),
        backing_key.as_deref(),
        &overlays,
    )
}

// Default constants

/// Factory defaults as camelCase JSON (`presets::get_defaults_json`, shared
/// with the mobile bridge)
#[wasm_bindgen(js_name = "getDefaults")]
pub fn get_defaults() -> String {
    presets::get_defaults_json()
}

// ============================================================================
// Presets & Defaults (from data/presets.json — single source of truth)
// ============================================================================

/// Get all presets and defaults as JSON (single source of truth)
#[wasm_bindgen(js_name = "getPresetsJson")]
pub fn get_presets_json() -> String {
    presets::get_presets_json().to_string()
}

// ============================================================================
// Version Information
// ============================================================================

/// Get core library version (e.g., "1.5.3")
#[wasm_bindgen(js_name = "getCoreVersion")]
pub fn get_core_version() -> String {
    version::get_core_version().to_string()
}

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// Core is a pure rlib with no wasm-bindgen dependency, so input-parsing types
// are exposed to JS through the newtype wrappers below.
// ============================================================================
// WASM Wrappers for Input Parsing Types
// ============================================================================

use referenceframe_core::input_parser;

/// WASM wrapper for DimensionInput
#[wasm_bindgen]
pub struct DimensionInput(input_parser::DimensionInput);

#[wasm_bindgen]
impl DimensionInput {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &str) -> DimensionInput {
        DimensionInput(input_parser::DimensionInput::new(input))
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.0.value()
    }

    #[wasm_bindgen(setter)]
    pub fn set_value(&mut self, value: f64) {
        self.0.set_value(value);
    }

    #[wasm_bindgen]
    pub fn parse(&mut self, input: &str) {
        self.0.parse(input);
    }

    #[wasm_bindgen(getter, js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(getter, js_name = "wasFractional")]
    pub fn was_fractional(&self) -> bool {
        self.0.was_fractional()
    }

    #[wasm_bindgen(js_name = "asFraction")]
    pub fn as_fraction(&self, max_denominator: u32) -> String {
        self.0.as_fraction(max_denominator)
    }
}

// ============================================================================
// WASM Wrappers for Validation Types
// ============================================================================

use referenceframe_core::validation;

/// WASM wrapper for ValidationConfig
#[wasm_bindgen]
pub struct ValidationConfig(validation::ValidationConfig);

#[wasm_bindgen]
impl ValidationConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ValidationConfig {
        ValidationConfig(validation::ValidationConfig::new())
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> Result<String, JsValue> {
        self.0.to_json()
            .map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = "fromJson")]
    pub fn from_json(json: &str) -> Result<ValidationConfig, JsValue> {
        validation::ValidationConfig::from_json(json)
            .map(ValidationConfig)
            .map_err(|e| JsValue::from_str(&e))
    }

    // Expose all fields as getters/setters
    #[wasm_bindgen(getter, js_name = "minLipWidth")]
    pub fn min_lip_width(&self) -> f64 { self.0.min_lip_width }
    #[wasm_bindgen(setter, js_name = "minLipWidth")]
    pub fn set_min_lip_width(&mut self, val: f64) { self.0.min_lip_width = val; }

    #[wasm_bindgen(getter, js_name = "minFaceDepth")]
    pub fn min_face_depth(&self) -> f64 { self.0.min_face_depth }
    #[wasm_bindgen(setter, js_name = "minFaceDepth")]
    pub fn set_min_face_depth(&mut self, val: f64) { self.0.min_face_depth = val; }

    #[wasm_bindgen(getter, js_name = "minFrameWidth")]
    pub fn min_frame_width(&self) -> f64 { self.0.min_frame_width }
    #[wasm_bindgen(setter, js_name = "minFrameWidth")]
    pub fn set_min_frame_width(&mut self, val: f64) { self.0.min_frame_width = val; }

    #[wasm_bindgen(getter, js_name = "maxFrameWidth")]
    pub fn max_frame_width(&self) -> f64 { self.0.max_frame_width }
    #[wasm_bindgen(setter, js_name = "maxFrameWidth")]
    pub fn set_max_frame_width(&mut self, val: f64) { self.0.max_frame_width = val; }

    #[wasm_bindgen(getter, js_name = "minFrameDepth")]
    pub fn min_frame_depth(&self) -> f64 { self.0.min_frame_depth }
    #[wasm_bindgen(setter, js_name = "minFrameDepth")]
    pub fn set_min_frame_depth(&mut self, val: f64) { self.0.min_frame_depth = val; }

    #[wasm_bindgen(getter, js_name = "maxFrameDepth")]
    pub fn max_frame_depth(&self) -> f64 { self.0.max_frame_depth }
    #[wasm_bindgen(setter, js_name = "maxFrameDepth")]
    pub fn set_max_frame_depth(&mut self, val: f64) { self.0.max_frame_depth = val; }

    #[wasm_bindgen(getter, js_name = "minOpening")]
    pub fn min_opening(&self) -> f64 { self.0.min_opening }
    #[wasm_bindgen(setter, js_name = "minOpening")]
    pub fn set_min_opening(&mut self, val: f64) { self.0.min_opening = val; }

    #[wasm_bindgen(getter, js_name = "maxOpening")]
    pub fn max_opening(&self) -> f64 { self.0.max_opening }
    #[wasm_bindgen(setter, js_name = "maxOpening")]
    pub fn set_max_opening(&mut self, val: f64) { self.0.max_opening = val; }

    #[wasm_bindgen(getter, js_name = "maxArtworkDimension")]
    pub fn max_artwork_dimension(&self) -> f64 { self.0.max_artwork_dimension }
    #[wasm_bindgen(setter, js_name = "maxArtworkDimension")]
    pub fn set_max_artwork_dimension(&mut self, val: f64) { self.0.max_artwork_dimension = val; }

    #[wasm_bindgen(getter, js_name = "minRabbet")]
    pub fn min_rabbet(&self) -> f64 { self.0.min_rabbet }
    #[wasm_bindgen(setter, js_name = "minRabbet")]
    pub fn set_min_rabbet(&mut self, val: f64) { self.0.min_rabbet = val; }

    #[wasm_bindgen(getter, js_name = "maxRabbet")]
    pub fn max_rabbet(&self) -> f64 { self.0.max_rabbet }
    #[wasm_bindgen(setter, js_name = "maxRabbet")]
    pub fn set_max_rabbet(&mut self, val: f64) { self.0.max_rabbet = val; }

    #[wasm_bindgen(getter, js_name = "minGlazing")]
    pub fn min_glazing(&self) -> f64 { self.0.min_glazing }
    #[wasm_bindgen(setter, js_name = "minGlazing")]
    pub fn set_min_glazing(&mut self, val: f64) { self.0.min_glazing = val; }

    #[wasm_bindgen(getter, js_name = "maxGlazing")]
    pub fn max_glazing(&self) -> f64 { self.0.max_glazing }
    #[wasm_bindgen(setter, js_name = "maxGlazing")]
    pub fn set_max_glazing(&mut self, val: f64) { self.0.max_glazing = val; }

    #[wasm_bindgen(getter, js_name = "minMatboard")]
    pub fn min_matboard(&self) -> f64 { self.0.min_matboard }
    #[wasm_bindgen(setter, js_name = "minMatboard")]
    pub fn set_min_matboard(&mut self, val: f64) { self.0.min_matboard = val; }

    #[wasm_bindgen(getter, js_name = "maxMatboard")]
    pub fn max_matboard(&self) -> f64 { self.0.max_matboard }
    #[wasm_bindgen(setter, js_name = "maxMatboard")]
    pub fn set_max_matboard(&mut self, val: f64) { self.0.max_matboard = val; }

    #[wasm_bindgen(getter, js_name = "minArtwork")]
    pub fn min_artwork(&self) -> f64 { self.0.min_artwork }
    #[wasm_bindgen(setter, js_name = "minArtwork")]
    pub fn set_min_artwork(&mut self, val: f64) { self.0.min_artwork = val; }

    #[wasm_bindgen(getter, js_name = "maxArtwork")]
    pub fn max_artwork(&self) -> f64 { self.0.max_artwork }
    #[wasm_bindgen(setter, js_name = "maxArtwork")]
    pub fn set_max_artwork(&mut self, val: f64) { self.0.max_artwork = val; }

    #[wasm_bindgen(getter, js_name = "minBacking")]
    pub fn min_backing(&self) -> f64 { self.0.min_backing }
    #[wasm_bindgen(setter, js_name = "minBacking")]
    pub fn set_min_backing(&mut self, val: f64) { self.0.min_backing = val; }

    #[wasm_bindgen(getter, js_name = "maxBacking")]
    pub fn max_backing(&self) -> f64 { self.0.max_backing }
    #[wasm_bindgen(setter, js_name = "maxBacking")]
    pub fn set_max_backing(&mut self, val: f64) { self.0.max_backing = val; }

    #[wasm_bindgen(getter, js_name = "minMargin")]
    pub fn min_margin(&self) -> f64 { self.0.min_margin }
    #[wasm_bindgen(setter, js_name = "minMargin")]
    pub fn set_min_margin(&mut self, val: f64) { self.0.min_margin = val; }

    #[wasm_bindgen(getter, js_name = "maxMargin")]
    pub fn max_margin(&self) -> f64 { self.0.max_margin }
    #[wasm_bindgen(setter, js_name = "maxMargin")]
    pub fn set_max_margin(&mut self, val: f64) { self.0.max_margin = val; }

    #[wasm_bindgen(getter, js_name = "warnArtworkOpeningOverlap")]
    pub fn warn_artwork_opening_overlap(&self) -> f64 { self.0.warn_artwork_opening_overlap }
    #[wasm_bindgen(setter, js_name = "warnArtworkOpeningOverlap")]
    pub fn set_warn_artwork_opening_overlap(&mut self, val: f64) { self.0.warn_artwork_opening_overlap = val; }

    #[wasm_bindgen(getter, js_name = "warnExtremeAspectRatio")]
    pub fn warn_extreme_aspect_ratio(&self) -> f64 { self.0.warn_extreme_aspect_ratio }
    #[wasm_bindgen(setter, js_name = "warnExtremeAspectRatio")]
    pub fn set_warn_extreme_aspect_ratio(&mut self, val: f64) { self.0.warn_extreme_aspect_ratio = val; }

    // Mat constraints
    #[wasm_bindgen(getter, js_name = "minVisibleOpening")]
    pub fn min_visible_opening(&self) -> f64 { self.0.min_visible_opening }
    #[wasm_bindgen(setter, js_name = "minVisibleOpening")]
    pub fn set_min_visible_opening(&mut self, val: f64) { self.0.min_visible_opening = val; }

    #[wasm_bindgen(getter, js_name = "warnMinMatOpening")]
    pub fn warn_min_mat_opening(&self) -> f64 { self.0.warn_min_mat_opening }
    #[wasm_bindgen(setter, js_name = "warnMinMatOpening")]
    pub fn set_warn_min_mat_opening(&mut self, val: f64) { self.0.warn_min_mat_opening = val; }

    #[wasm_bindgen(getter, js_name = "minMatOverlap")]
    pub fn min_mat_overlap(&self) -> f64 { self.0.min_mat_overlap }
    #[wasm_bindgen(setter, js_name = "minMatOverlap")]
    pub fn set_min_mat_overlap(&mut self, val: f64) { self.0.min_mat_overlap = val; }

    #[wasm_bindgen(getter, js_name = "maxMatOverlap")]
    pub fn max_mat_overlap(&self) -> f64 { self.0.max_mat_overlap }
    #[wasm_bindgen(setter, js_name = "maxMatOverlap")]
    pub fn set_max_mat_overlap(&mut self, val: f64) { self.0.max_mat_overlap = val; }
}

/// Get a range hint string for a specific field
#[wasm_bindgen(js_name = "getTypicalRangeHint")]
pub fn get_typical_range_hint(field: &str, use_mm: bool) -> String {
    let ranges = validation::TypicalRanges::new();
    ranges.get_range_hint(field, use_mm)
}

/// Re-export WasmValidationResult from core with wasm_bindgen
#[wasm_bindgen]
pub struct WasmValidationResult(validation::WasmValidationResult);

#[wasm_bindgen]
impl WasmValidationResult {
    #[wasm_bindgen(js_name = "hasErrors")]
    pub fn has_errors(&self) -> bool {
        self.0.has_errors()
    }

    #[wasm_bindgen(js_name = "hasWarnings")]
    pub fn has_warnings(&self) -> bool {
        self.0.has_warnings()
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> Result<String, JsValue> {
        self.0.to_json()
            .map_err(|e| JsValue::from_str(&e))
    }
}

/// Validate a frame design
#[wasm_bindgen(js_name = "validateDesign")]
pub fn validate_design(design: &WasmFrameDesign, config: &ValidationConfig, use_mm: bool) -> WasmValidationResult {
    // Call the core validate_design and wrap the result
    let result = validation::validate_design(&design.inner, &config.0, use_mm);
    // Wrap ValidationResult in WasmValidationResult using constructor
    WasmValidationResult(validation::WasmValidationResult::new(result))
}

// ============================================================================
// History Functions (JSON-based API for simplicity)
// ============================================================================

/// Parse history JSON, returning the parse error message on failure
fn parse_history(json: &str) -> Result<history::DesignHistory, String> {
    history::DesignHistory::from_json(json).map_err(|e| e.to_string())
}

/// Build a JSON error response preserving the original history
/// (used only by `addToHistory`, whose success shape is also an object)
fn history_error(msg: &str, original_json: &str) -> String {
    serde_json::json!({
        "error": msg,
        "history": original_json,
    }).to_string()
}

/// Create empty history with default max entries
#[wasm_bindgen(js_name = "createHistory")]
pub fn create_history() -> String {
    let history = history::DesignHistory::new();
    history.to_json().unwrap_or_else(|_| "{}".to_string())
}

/// Add entry to history
///
/// Returns JSON with updated history and whether it was a new entry:
/// `{ "history": "...", "isNew": true/false }`
/// On parse error returns: `{ "error": "...", "history": <original> }`
/// If force_new is true, always creates a new entry even if design already exists.
///
/// An empty `title` auto-generates one from the artwork size in the user's
/// unit: `useMm` true → `215.9 mm × 279.4 mm Frame`, false/omitted →
/// `8 1/2" × 11" Frame`.
#[wasm_bindgen(js_name = "addToHistory")]
pub fn add_to_history(history_json: &str, design_json: &str, timestamp: i64, title: &str, force_new: bool, use_mm: Option<bool>) -> String {
    let mut hist = match parse_history(history_json) {
        Ok(h) => h,
        Err(e) => return history_error(&format!("history parse: {}", e), history_json),
    };

    let design: FrameDesign = match serde_json::from_str(design_json) {
        Ok(d) => d,
        Err(e) => return history_error(&format!("design parse: {}", e), history_json),
    };

    let is_new = if title.is_empty() {
        hist.add_entry_auto_title(design, timestamp, force_new, use_mm.unwrap_or(false))
    } else {
        hist.add_entry(design, timestamp, title.to_string(), force_new)
    };

    let updated_history = hist.to_json().unwrap_or_else(|_| history_json.to_string());
    serde_json::json!({
        "history": updated_history,
        "isNew": is_new
    }).to_string()
}

/// Get history entry at index as JSON
///
/// Returns the entry JSON (`{ design, timestamps, title }`), or an empty
/// string if the index is invalid or `history_json` fails to parse
/// (matches the mobile bridge).
#[wasm_bindgen(js_name = "getHistoryEntry")]
pub fn get_history_entry(history_json: &str, index: usize) -> String {
    let hist = match parse_history(history_json) {
        Ok(h) => h,
        Err(_) => return String::new(),
    };

    match hist.get(index) {
        Some(entry) => serde_json::to_string(entry).unwrap_or_default(),
        None => String::new(),
    }
}

/// Remove history entry at index
///
/// Returns the updated history JSON. If `history_json` fails to parse, returns
/// it unchanged (matches the mobile bridge), so callers can always persist the
/// result without clobbering stored history.
#[wasm_bindgen(js_name = "removeHistoryEntry")]
pub fn remove_history_entry(history_json: &str, index: usize) -> String {
    let mut hist = match parse_history(history_json) {
        Ok(h) => h,
        Err(_) => return history_json.to_string(),
    };

    hist.remove(index);
    hist.to_json().unwrap_or_else(|_| history_json.to_string())
}

/// Update history entry title
///
/// Returns the updated history JSON. If `history_json` fails to parse, returns
/// it unchanged (matches the mobile bridge).
#[wasm_bindgen(js_name = "updateHistoryTitle")]
pub fn update_history_title(history_json: &str, index: usize, title: &str) -> String {
    let mut hist = match parse_history(history_json) {
        Ok(h) => h,
        Err(_) => return history_json.to_string(),
    };

    hist.update_title(index, title.to_string());
    hist.to_json().unwrap_or_else(|_| history_json.to_string())
}
