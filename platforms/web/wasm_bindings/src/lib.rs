// WASM bindings for browser JavaScript
//
// This module exposes the Rust API to JavaScript via wasm-bindgen
// It wraps the pure Rust types from referenceframe_core with wasm_bindgen annotations

use wasm_bindgen::prelude::*;
use referenceframe_core::{
    conversions::{format_value, format_value_with_decimal, inches_to_mm, mm_to_inches, Unit},
    frame::FrameDesign,
    aspect_ratio::AspectLockState,
    shareable_url::{ShareableParams, generate_shareable_url, decode_shareable_url},
    defaults::*,
};

/// Get the WASM build version for debugging
#[wasm_bindgen(js_name = "getWasmVersion")]
pub fn get_wasm_version() -> String {
    "2026-01-09-dogleg-no-offset".to_string()
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

    /// Create with default values
    #[wasm_bindgen(js_name = "default")]
    pub fn default_design() -> WasmFrameDesign {
        WasmFrameDesign {
            inner: FrameDesign::default(),
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

    #[wasm_bindgen(getter, js_name = "rabbet_width")]
    pub fn rabbet_width(&self) -> f64 {
        self.inner.rabbet_width
    }

    #[wasm_bindgen(setter, js_name = "rabbet_width")]
    pub fn set_rabbet_width(&mut self, value: f64) {
        self.inner.rabbet_width = value;
    }

    #[wasm_bindgen(getter, js_name = "rabbet_depth")]
    pub fn rabbet_depth(&self) -> f64 {
        self.inner.rabbet_depth
    }

    #[wasm_bindgen(setter, js_name = "rabbet_depth")]
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

    #[wasm_bindgen(getter, js_name = "includeMat")]
    pub fn has_mat(&self) -> bool {
        self.inner.has_mat()
    }

    // Calculation methods

    /// Validate and enforce constraints
    pub fn validate(&mut self) {
        self.inner.validate();
    }

    /// Get visible (face) dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getVisibleDimensions")]
    pub fn get_visible_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_visible_dimensions();
        vec![h, w]
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

    /// Get matboard dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getMatboardDimensions")]
    pub fn get_matboard_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_matboard_dimensions();
        vec![h, w]
    }

    /// Get mat opening dimensions - returns [height, width]
    #[wasm_bindgen(js_name = "getMatOpeningDimensions")]
    pub fn get_mat_opening_dimensions(&self) -> Vec<f64> {
        let (h, w) = self.inner.get_mat_opening_dimensions();
        vec![h, w]
    }

    /// Get matboard cut dimensions - returns [top_bottom, sides]
    #[wasm_bindgen(js_name = "getMatboardCutDimensions")]
    pub fn get_matboard_cut_dimensions(&self) -> Vec<f64> {
        let (tb, s) = self.inner.get_matboard_cut_dimensions();
        vec![tb, s]
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

/// WASM-friendly wrapper for AspectLockState
#[wasm_bindgen]
pub struct WasmAspectLock {
    inner: AspectLockState,
}

#[wasm_bindgen]
impl WasmAspectLock {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmAspectLock {
        WasmAspectLock {
            inner: AspectLockState::new(),
        }
    }

    pub fn locked(&self) -> bool {
        self.inner.locked()
    }

    pub fn ratio(&self) -> Option<f64> {
        self.inner.ratio()
    }

    pub fn lock(&mut self, height: f64, width: f64) -> bool {
        self.inner.lock(height, width)
    }

    pub fn unlock(&mut self) {
        self.inner.unlock();
    }

    pub fn toggle(&mut self, height: f64, width: f64) -> bool {
        self.inner.toggle(height, width)
    }

    pub fn invert(&mut self) {
        self.inner.invert();
    }

    #[wasm_bindgen(js_name = "getWidthForHeight")]
    pub fn get_width_for_height(&self, height: f64, step: f64) -> f64 {
        self.inner.get_width_for_height(height, step)
    }

    #[wasm_bindgen(js_name = "getHeightForWidth")]
    pub fn get_height_for_width(&self, width: f64, step: f64) -> f64 {
        self.inner.get_height_for_width(width, step)
    }
}

// Free functions

#[wasm_bindgen(js_name = "inchesToMm")]
pub fn wasm_inches_to_mm(inches: f64) -> f64 {
    inches_to_mm(inches)
}

#[wasm_bindgen(js_name = "mmToInches")]
pub fn wasm_mm_to_inches(mm: f64) -> f64 {
    mm_to_inches(mm)
}

#[wasm_bindgen(js_name = "formatValue")]
pub fn wasm_format_value(value: f64, unit_mm: bool) -> String {
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    format_value(value, unit)
}

#[wasm_bindgen(js_name = "formatValueWithDecimal")]
pub fn wasm_format_value_with_decimal(value: f64, unit_mm: bool) -> String {
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    format_value_with_decimal(value, unit)
}

#[wasm_bindgen(js_name = "getAspectRatioDisplay")]
pub fn wasm_get_aspect_ratio_display(height: f64, width: f64) -> String {
    referenceframe_core::aspect_ratio::get_aspect_ratio_display(height, width)
}

#[wasm_bindgen(js_name = "generateShareableUrl")]
pub fn wasm_generate_shareable_url(params_json: &str) -> Result<String, JsValue> {
    let params: ShareableParams = serde_json::from_str(params_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    Ok(generate_shareable_url(&params))
}

#[wasm_bindgen(js_name = "decodeShareableUrl")]
pub fn wasm_decode_shareable_url(url: &str) -> Result<String, JsValue> {
    let params = decode_shareable_url(url)
        .map_err(|e| JsValue::from_str(&format!("Decode error: {}", e)))?;
    serde_json::to_string(&params)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// Visualization functions

/// Generate a plan view SVG diagram
#[wasm_bindgen(js_name = "generatePlanViewSvg")]
pub fn wasm_generate_plan_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
) -> String {
    use referenceframe_core::visualization::{generate_diagram, DiagramOptions, ViewOption};

    let options = DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width,
        canvas_height,
        include_title_block: false,
        unit_mm,
    };

    let result = generate_diagram(&design.inner, &options);
    result.svg
}

/// Generate a section view SVG diagram
#[wasm_bindgen(js_name = "generateSectionViewSvg")]
pub fn wasm_generate_section_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
) -> String {
    use referenceframe_core::visualization::{generate_diagram, DiagramOptions, ViewOption};

    let options = DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_width,
        canvas_height,
        include_title_block: false,
        unit_mm,
    };

    let result = generate_diagram(&design.inner, &options);
    result.svg
}

/// Generate combined view SVG (for PDF export)
#[wasm_bindgen(js_name = "generateCombinedViewSvg")]
pub fn wasm_generate_combined_view_svg(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    include_title: bool,
) -> String {
    wasm_generate_combined_view_svg_with_style(design, canvas_width, canvas_height, unit_mm, include_title, false)
}

/// Generate combined view SVG with optional PDF styling
#[wasm_bindgen(js_name = "generateCombinedViewSvgForPdf")]
pub fn wasm_generate_combined_view_svg_with_style(
    design: &WasmFrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    unit_mm: bool,
    include_title: bool,
    for_pdf: bool,
) -> String {
    use referenceframe_core::visualization::{generate_diagram_with_style, DiagramOptions, DiagramStyle, ViewOption};

    let options = DiagramOptions {
        view: ViewOption::Both,
        canvas_width,
        canvas_height,
        include_title_block: include_title,
        unit_mm,
    };

    let style = if for_pdf {
        DiagramStyle::for_pdf()
    } else {
        DiagramStyle::default()
    };

    let result = generate_diagram_with_style(&design.inner, &options, &style);
    result.svg
}

/// Generate diagram with full options (returns JSON with svg and warnings)
#[wasm_bindgen(js_name = "generateDiagram")]
pub fn wasm_generate_diagram(
    design: &WasmFrameDesign,
    options_json: &str,
) -> Result<String, JsValue> {
    use referenceframe_core::visualization::{generate_diagram, DiagramOptions};

    let options: DiagramOptions = serde_json::from_str(options_json)
        .map_err(|e| JsValue::from_str(&format!("Options parse error: {}", e)))?;

    let result = generate_diagram(&design.inner, &options);

    let response = serde_json::json!({
        "svg": result.svg,
        "warnings": result.warnings,
    });

    serde_json::to_string(&response)
        .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))
}

// Default constants

#[wasm_bindgen(js_name = "getDefaults")]
pub fn get_defaults() -> String {
    let defaults = serde_json::json!({
        "artworkHeight": DEFAULT_ARTWORK_HEIGHT,
        "artworkWidth": DEFAULT_ARTWORK_WIDTH,
        "matWidth": DEFAULT_MAT_WIDTH,
        "matOverlap": DEFAULT_MAT_OVERLAP,
        "frameWidth": DEFAULT_FRAME_MATERIAL_WIDTH,
        "frameDepth": DEFAULT_FRAME_THICKNESS,
        "rabbet_width": DEFAULT_RABBET_DEPTH,  // Default to same as depth (square rabbet)
        "rabbet_depth": DEFAULT_RABBET_DEPTH,
        "glazingThickness": DEFAULT_GLAZING_THICKNESS,
        "matboardThickness": DEFAULT_MATBOARD_THICKNESS,
        "artworkThickness": DEFAULT_ARTWORK_THICKNESS,
        "backingThickness": DEFAULT_BACKING_THICKNESS,
        "bladeWidth": DEFAULT_BLADE_WIDTH,
        "assemblyMargin": DEFAULT_ASSEMBLY_MARGIN,
    });
    serde_json::to_string(&defaults).unwrap_or_default()
}

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// DimensionInput is already exported via #[wasm_bindgen] in input_parser.rs
// No re-export needed here
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

    #[wasm_bindgen(js_name = "fromDecimal")]
    pub fn from_decimal(value: f64) -> DimensionInput {
        DimensionInput(input_parser::DimensionInput::from_decimal(value))
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.0.value()
    }

    #[wasm_bindgen(setter)]
    pub fn set_value(&mut self, value: f64) {
        self.0.set_value(value);
    }

    #[wasm_bindgen(getter)]
    pub fn original(&self) -> String {
        self.0.original()
    }

    #[wasm_bindgen]
    pub fn parse(&mut self, input: &str) {
        self.0.parse(input);
    }

    #[wasm_bindgen(getter, js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.0.error()
    }

    #[wasm_bindgen(getter, js_name = "wasFractional")]
    pub fn was_fractional(&self) -> bool {
        self.0.was_fractional()
    }

    #[wasm_bindgen(js_name = "asFraction")]
    pub fn as_fraction(&self, max_denominator: u32) -> String {
        self.0.as_fraction(max_denominator)
    }

    #[wasm_bindgen(js_name = "asDecimal")]
    pub fn as_decimal(&self) -> String {
        self.0.as_decimal()
    }

    #[wasm_bindgen(js_name = "format")]
    pub fn format(&self, use_fractions: bool, max_denominator: u32) -> String {
        self.0.format(use_fractions, max_denominator)
    }

    #[wasm_bindgen]
    pub fn add(&self, other: &DimensionInput) -> DimensionInput {
        DimensionInput(self.0.add(&other.0))
    }

    #[wasm_bindgen]
    pub fn subtract(&self, other: &DimensionInput) -> DimensionInput {
        DimensionInput(self.0.subtract(&other.0))
    }

    #[wasm_bindgen]
    pub fn multiply(&self, scalar: f64) -> DimensionInput {
        DimensionInput(self.0.multiply(scalar))
    }

    #[wasm_bindgen]
    pub fn divide(&self, scalar: f64) -> DimensionInput {
        DimensionInput(self.0.divide(scalar))
    }
}

/// WASM wrapper for ParsedDimension (legacy API)
#[wasm_bindgen]
pub struct ParsedDimension(input_parser::ParsedDimension);

#[wasm_bindgen]
impl ParsedDimension {
    #[wasm_bindgen(getter)]
    pub fn decimal(&self) -> f64 {
        self.0.decimal()
    }

    #[wasm_bindgen(getter)]
    pub fn display(&self) -> String {
        self.0.display()
    }

    #[wasm_bindgen(getter, js_name = "wasFractional")]
    pub fn was_fractional(&self) -> bool {
        self.0.was_fractional()
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.0.error()
    }

    #[wasm_bindgen(getter, js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }
}

/// Parse a dimension input string (legacy API)
#[wasm_bindgen(js_name = "parseDimension")]
pub fn parse_dimension(input: &str) -> ParsedDimension {
    ParsedDimension(input_parser::parse_dimension(input))
}

/// Convert a decimal to the nearest common fraction
#[wasm_bindgen(js_name = "decimalToFraction")]
pub fn decimal_to_fraction(val: f64, max_denominator: u32) -> String {
    input_parser::decimal_to_fraction(val, max_denominator)
}

/// Check if input is a valid dimension string
#[wasm_bindgen(js_name = "isValidDimensionInput")]
pub fn is_valid_dimension_input(input: &str) -> bool {
    input_parser::is_valid_dimension_input(input)
}

/// Get common fractions for a picker UI (returns JSON array)
#[wasm_bindgen(js_name = "getCommonFractions")]
pub fn get_common_fractions(max_denominator: u32) -> String {
    input_parser::get_common_fractions(max_denominator)
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
}

/// WASM wrapper for TypicalRanges
#[wasm_bindgen]
pub struct TypicalRanges(validation::TypicalRanges);

#[wasm_bindgen]
impl TypicalRanges {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TypicalRanges {
        TypicalRanges(validation::TypicalRanges::new())
    }

    #[wasm_bindgen(js_name = "getRangeHint")]
    pub fn get_range_hint(&self, field: &str, use_mm: bool) -> String {
        self.0.get_range_hint(field, use_mm)
    }

    // Expose all fields
    #[wasm_bindgen(getter, js_name = "frameWidthMin")]
    pub fn frame_width_min(&self) -> f64 { self.0.frame_width_min }
    #[wasm_bindgen(setter, js_name = "frameWidthMin")]
    pub fn set_frame_width_min(&mut self, val: f64) { self.0.frame_width_min = val; }

    #[wasm_bindgen(getter, js_name = "frameWidthMax")]
    pub fn frame_width_max(&self) -> f64 { self.0.frame_width_max }
    #[wasm_bindgen(setter, js_name = "frameWidthMax")]
    pub fn set_frame_width_max(&mut self, val: f64) { self.0.frame_width_max = val; }

    // ... (abbreviated for space - add all other fields similarly)
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

    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(js_name = "errorCount")]
    pub fn error_count(&self) -> usize {
        self.0.error_count()
    }

    #[wasm_bindgen(js_name = "warningCount")]
    pub fn warning_count(&self) -> usize {
        self.0.warning_count()
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> Result<String, JsValue> {
        self.0.to_json()
            .map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = "errorsJson")]
    pub fn errors_json(&self) -> Result<String, JsValue> {
        self.0.errors_json()
            .map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = "warningsJson")]
    pub fn warnings_json(&self) -> Result<String, JsValue> {
        self.0.warnings_json()
            .map_err(|e| JsValue::from_str(&e))
    }
}

/// Validate a frame design
#[wasm_bindgen(js_name = "validateDesign")]
pub fn validate_design_wasm(design: &WasmFrameDesign, config: &ValidationConfig) -> WasmValidationResult {
    // Call the core validate_design and wrap the result
    let result = validation::validate_design(&design.inner, &config.0);
    // Wrap ValidationResult in WasmValidationResult using constructor
    WasmValidationResult(validation::WasmValidationResult::new(result))
}
