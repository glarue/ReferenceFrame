//! Input validation for frame designs
//!
//! Provides configurable validation with:
//! - Hard errors (block visualization/export)
//! - Soft warnings (informational)
//! - User-configurable limits stored in localStorage

use serde::{Deserialize, Serialize};

use crate::conversions;
use crate::frame::FrameDesign;

/// Validation configuration with user-adjustable limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    // Structural minimums (must leave this much material after rabbet cut)
    /// Minimum remaining lip width after rabbet width cut (inches)
    pub min_lip_width: f64,
    /// Minimum remaining face depth after rabbet depth cut (inches)
    pub min_face_depth: f64,

    // Frame material bounds
    /// Minimum frame moulding width (inches)
    pub min_frame_width: f64,
    /// Maximum frame moulding width (inches)
    pub max_frame_width: f64,
    /// Minimum frame moulding depth (inches)
    pub min_frame_depth: f64,
    /// Maximum frame moulding depth (inches)
    pub max_frame_depth: f64,

    // Opening bounds
    /// Minimum opening dimension (inches)
    pub min_opening: f64,
    /// Maximum opening dimension (inches)
    pub max_opening: f64,

    // Rabbet bounds
    /// Minimum rabbet dimension (inches)
    pub min_rabbet: f64,
    /// Maximum rabbet dimension (inches)
    pub max_rabbet: f64,

    // Material thickness bounds
    pub min_glazing: f64,
    pub max_glazing: f64,
    pub min_matboard: f64,
    pub max_matboard: f64,
    pub min_artwork: f64,
    pub max_artwork: f64,
    pub min_backing: f64,
    pub max_backing: f64,
    pub min_margin: f64,
    pub max_margin: f64,

    // Soft warning thresholds
    /// Warn if artwork extends less than this past opening per side (inches)
    pub warn_artwork_opening_overlap: f64,
    /// Warn if aspect ratio exceeds this value
    pub warn_extreme_aspect_ratio: f64,

    // Mat constraints
    /// Minimum visible artwork through mat opening per side (inches)
    pub min_visible_opening: f64,
    /// Warn if mat opening dimension is smaller than this (inches)
    pub warn_min_mat_opening: f64,
    /// Minimum mat overlap (inches)
    pub min_mat_overlap: f64,
    /// Maximum mat overlap (inches)
    pub max_mat_overlap: f64,
}

impl ValidationConfig {
    /// Create default validation config with sensible limits
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize to JSON string for localStorage
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<ValidationConfig, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Parse error: {}", e))
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            // Structural minimums
            min_lip_width: 0.125,      // 1/8"
            min_face_depth: 0.125,     // 1/8"

            // Frame material bounds
            min_frame_width: 0.5,      // 1/2"
            max_frame_width: 12.0,
            min_frame_depth: 0.5,      // 1/2" (per user request)
            max_frame_depth: 6.0,

            // Opening bounds
            min_opening: 0.5,          // 1/2"
            max_opening: 120.0,        // 10 feet

            // Rabbet bounds
            min_rabbet: 0.125,         // 1/8"
            max_rabbet: 3.0,

            // Material thickness bounds
            min_glazing: 0.0,          // 0 = no glazing
            max_glazing: 0.5,
            min_matboard: 0.0,         // 0 = no mat
            max_matboard: 0.5,
            min_artwork: 0.001,        // ~paper thickness
            max_artwork: 2.0,          // Canvas on deep stretcher
            min_backing: 0.03125,      // 1/32"
            max_backing: 0.5,
            min_margin: 0.0,
            max_margin: 0.25,

            // Warning thresholds
            warn_artwork_opening_overlap: 0.25,  // 1/4" per side
            warn_extreme_aspect_ratio: 10.0,

            // Mat constraints
            min_visible_opening: 0.125,          // 1/8" minimum visible artwork per side
            warn_min_mat_opening: 1.0,           // 1" minimum mat opening before warning
            min_mat_overlap: 0.0625,             // 1/16" minimum
            max_mat_overlap: 6.0,                // 6" maximum (very generous)
        }
    }
}

/// Typical value ranges for UI hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypicalRanges {
    pub frame_width_min: f64,
    pub frame_width_max: f64,
    pub frame_depth_min: f64,
    pub frame_depth_max: f64,
    pub rabbet_width_min: f64,
    pub rabbet_width_max: f64,
    pub rabbet_depth_min: f64,
    pub rabbet_depth_max: f64,
    pub glazing_min: f64,
    pub glazing_max: f64,
    pub matboard_min: f64,
    pub matboard_max: f64,
    pub artwork_min: f64,
    pub artwork_max: f64,
    pub backing_min: f64,
    pub backing_max: f64,
    pub margin_min: f64,
    pub margin_max: f64,
}

impl TypicalRanges {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get typical range as formatted string for a field
    pub fn get_range_hint(&self, field: &str, use_mm: bool) -> String {
        let (min, max) = match field {
            "frame_width" | "frame_material_width" => (self.frame_width_min, self.frame_width_max),
            "frame_depth" | "frame_material_depth" => (self.frame_depth_min, self.frame_depth_max),
            "rabbet_width" => (self.rabbet_width_min, self.rabbet_width_max),
            "rabbet_depth" => (self.rabbet_depth_min, self.rabbet_depth_max),
            "glazing" | "glazing_thickness" => (self.glazing_min, self.glazing_max),
            "matboard" | "matboard_thickness" => (self.matboard_min, self.matboard_max),
            "artwork" | "artwork_thickness" => (self.artwork_min, self.artwork_max),
            "backing" | "backing_thickness" => (self.backing_min, self.backing_max),
            "margin" | "assembly_margin" => (self.margin_min, self.margin_max),
            _ => return String::new(),
        };

        if use_mm {
            format!("Typical: {:.1}mm - {:.1}mm", min * 25.4, max * 25.4)
        } else {
            format!("Typical: {} - {}", conversions::format_inches_as_fraction(min), conversions::format_inches_as_fraction(max))
        }
    }
}

impl Default for TypicalRanges {
    fn default() -> Self {
        Self {
            frame_width_min: 0.75,
            frame_width_max: 4.0,
            frame_depth_min: 0.5,
            frame_depth_max: 2.0,
            rabbet_width_min: 0.375,   // 3/8"
            rabbet_width_max: 0.5,     // 1/2"
            rabbet_depth_min: 0.3125,  // 5/16"
            rabbet_depth_max: 0.375,   // 3/8"
            glazing_min: 0.0625,       // 1/16"
            glazing_max: 0.25,         // 1/4"
            matboard_min: 0.0625,      // 1/16"
            matboard_max: 0.1875,      // 3/16"
            artwork_min: 0.01,
            artwork_max: 0.25,
            backing_min: 0.0625,       // 1/16"
            backing_max: 0.25,         // 1/4"
            margin_min: 0.0,
            margin_max: 0.0625,        // 1/16"
        }
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,   // Hard error - blocks visualization
    Warning, // Soft warning - informational only
}

/// A single validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub field: String,
    pub message: String,
    pub details: Option<String>,
}

impl ValidationIssue {
    pub fn error(field: &str, message: &str) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            field: field.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    pub fn error_with_details(field: &str, message: &str, details: &str) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            field: field.to_string(),
            message: message.to_string(),
            details: Some(details.to_string()),
        }
    }

    pub fn warning(field: &str, message: &str) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            field: field.to_string(),
            message: message.to_string(),
            details: None,
        }
    }

    pub fn warning_with_details(field: &str, message: &str, details: &str) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            field: field.to_string(),
            message: message.to_string(),
            details: Some(details.to_string()),
        }
    }
}

/// Result of validating a design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == ValidationSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == ValidationSeverity::Warning)
    }

    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == ValidationSeverity::Error).collect()
    }

    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == ValidationSeverity::Warning).collect()
    }

    fn add(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }
}

/// WASM-friendly wrapper for ValidationResult
pub struct WasmValidationResult {
    inner: ValidationResult,
}

impl WasmValidationResult {
    /// Create from a ValidationResult (for platform bindings)
    pub fn new(result: ValidationResult) -> Self {
        WasmValidationResult { inner: result }
    }

    /// Check if there are any hard errors
    pub fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.inner.has_warnings()
    }

    /// Check if design is valid (no hard errors)
    pub fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.inner.errors().len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.inner.warnings().len()
    }

    /// Get all issues as JSON string
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Get errors only as JSON string
    pub fn errors_json(&self) -> Result<String, String> {
        let errors: Vec<_> = self.inner.errors().into_iter().cloned().collect();
        serde_json::to_string(&errors)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Get warnings only as JSON string
    pub fn warnings_json(&self) -> Result<String, String> {
        let warnings: Vec<_> = self.inner.warnings().into_iter().cloned().collect();
        serde_json::to_string(&warnings)
            .map_err(|e| format!("Serialization error: {}", e))
    }
}

/// Validate a frame design against the given configuration
pub fn validate_design(design: &FrameDesign, config: &ValidationConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    // === HARD ERRORS ===

    // Frame material width bounds
    if design.frame_material_width < config.min_frame_width {
        result.add(ValidationIssue::error_with_details(
            "frame_material_width",
            &format!("Frame width must be at least {:.3}\"", config.min_frame_width),
            &format!("Current: {:.3}\", Minimum: {:.3}\"", design.frame_material_width, config.min_frame_width),
        ));
    }
    if design.frame_material_width > config.max_frame_width {
        result.add(ValidationIssue::error_with_details(
            "frame_material_width",
            &format!("Frame width must be at most {:.1}\"", config.max_frame_width),
            &format!("Current: {:.3}\", Maximum: {:.1}\"", design.frame_material_width, config.max_frame_width),
        ));
    }

    // Frame material depth bounds
    if design.frame_material_depth < config.min_frame_depth {
        result.add(ValidationIssue::error_with_details(
            "frame_material_depth",
            &format!("Frame depth must be at least {:.3}\"", config.min_frame_depth),
            &format!("Current: {:.3}\", Minimum: {:.3}\"", design.frame_material_depth, config.min_frame_depth),
        ));
    }
    if design.frame_material_depth > config.max_frame_depth {
        result.add(ValidationIssue::error_with_details(
            "frame_material_depth",
            &format!("Frame depth must be at most {:.1}\"", config.max_frame_depth),
            &format!("Current: {:.3}\", Maximum: {:.1}\"", design.frame_material_depth, config.max_frame_depth),
        ));
    }

    // Rabbet width - structural constraint
    let max_rabbet_width = design.frame_material_width - config.min_lip_width;
    if design.rabbet_width > max_rabbet_width {
        result.add(ValidationIssue::error_with_details(
            "rabbet_width",
            &format!("Rabbet width too large - must leave at least {:.3}\" lip", config.min_lip_width),
            &format!("Current rabbet: {:.3}\", Frame width: {:.3}\", Max rabbet: {:.3}\"",
                design.rabbet_width, design.frame_material_width, max_rabbet_width),
        ));
    }
    if design.rabbet_width < config.min_rabbet {
        result.add(ValidationIssue::error_with_details(
            "rabbet_width",
            &format!("Rabbet width must be at least {:.3}\"", config.min_rabbet),
            &format!("Current: {:.3}\"", design.rabbet_width),
        ));
    }
    if design.rabbet_width > config.max_rabbet {
        result.add(ValidationIssue::error_with_details(
            "rabbet_width",
            &format!("Rabbet width must be at most {:.1}\"", config.max_rabbet),
            &format!("Current: {:.3}\"", design.rabbet_width),
        ));
    }

    // Rabbet depth - structural constraint
    let max_rabbet_depth = design.frame_material_depth - config.min_face_depth;
    if design.rabbet_depth > max_rabbet_depth {
        result.add(ValidationIssue::error_with_details(
            "rabbet_depth",
            &format!("Rabbet depth too large - must leave at least {:.3}\" face", config.min_face_depth),
            &format!("Current rabbet: {:.3}\", Frame depth: {:.3}\", Max rabbet: {:.3}\"",
                design.rabbet_depth, design.frame_material_depth, max_rabbet_depth),
        ));
    }
    if design.rabbet_depth < config.min_rabbet {
        result.add(ValidationIssue::error_with_details(
            "rabbet_depth",
            &format!("Rabbet depth must be at least {:.3}\"", config.min_rabbet),
            &format!("Current: {:.3}\"", design.rabbet_depth),
        ));
    }
    if design.rabbet_depth > config.max_rabbet {
        result.add(ValidationIssue::error_with_details(
            "rabbet_depth",
            &format!("Rabbet depth must be at most {:.1}\"", config.max_rabbet),
            &format!("Current: {:.3}\"", design.rabbet_depth),
        ));
    }

    // Helper: check value against min/max and add error if out of range
    let mut check_range = |field: &str, label: &str, value: f64, min: f64, max: f64| {
        if value < min {
            result.add(ValidationIssue::error(
                field,
                &format!("{} ({}) must be at least {}",
                    label, conversions::format_inches_as_fraction(value),
                    conversions::format_inches_as_fraction(min)),
            ));
        }
        if value > max {
            result.add(ValidationIssue::error(
                field,
                &format!("{} ({}) must be at most {}",
                    label, conversions::format_inches_as_fraction(value),
                    conversions::format_inches_as_fraction(max)),
            ));
        }
    };

    // Opening/inside dimensions
    let (opening_height, opening_width) = design.get_frame_inside_dimensions();
    check_range("artwork_width", "Frame opening width", opening_width, config.min_opening, config.max_opening);
    check_range("artwork_height", "Frame opening height", opening_height, config.min_opening, config.max_opening);

    // Material thickness bounds
    check_range("glazing_thickness", "Glazing thickness", design.glazing_thickness, config.min_glazing, config.max_glazing);
    check_range("matboard_thickness", "Matboard thickness", design.matboard_thickness, config.min_matboard, config.max_matboard);
    check_range("artwork_thickness", "Artwork thickness", design.artwork_thickness, config.min_artwork, config.max_artwork);
    check_range("backing_thickness", "Backing thickness", design.backing_thickness, config.min_backing, config.max_backing);
    check_range("assembly_margin", "Assembly margin", design.assembly_margin, config.min_margin, config.max_margin);

    // Mat overlap bounds (when mat is present)
    if design.has_mat() {
        if design.mat_overlap < config.min_mat_overlap {
            result.add(ValidationIssue::error_with_details(
                "mat_overlap",
                &format!("Mat overlap must be at least {:.4}\"", config.min_mat_overlap),
                &format!("Current: {:.3}\"", design.mat_overlap),
            ));
        }
        if design.mat_overlap > config.max_mat_overlap {
            result.add(ValidationIssue::error_with_details(
                "mat_overlap",
                &format!("Mat overlap must be at most {:.1}\"", config.max_mat_overlap),
                &format!("Current: {:.3}\". This would make the mat opening negative!", design.mat_overlap),
            ));
        }

        // Additional check: mat overlap must not exceed half the artwork dimensions
        // (otherwise mat opening would be negative)
        let max_safe_overlap_h = design.artwork_height / 2.0 - 0.5; // Leave at least 1" opening
        let max_safe_overlap_w = design.artwork_width / 2.0 - 0.5;
        if design.mat_overlap > max_safe_overlap_h {
            result.add(ValidationIssue::error_with_details(
                "mat_overlap",
                &format!("Mat overlap ({:.2}\") too large for artwork height ({:.2}\")",
                    design.mat_overlap, design.artwork_height),
                &format!("Maximum safe overlap: {:.2}\" (would leave 1\" opening)", max_safe_overlap_h),
            ));
        }
        if design.mat_overlap > max_safe_overlap_w {
            result.add(ValidationIssue::error_with_details(
                "mat_overlap",
                &format!("Mat overlap ({:.2}\") too large for artwork width ({:.2}\")",
                    design.mat_overlap, design.artwork_width),
                &format!("Maximum safe overlap: {:.2}\" (would leave 1\" opening)", max_safe_overlap_w),
            ));
        }
    }

    // === SOFT WARNINGS ===

    // Material stack overflow (already shown in viz, but also warn here)
    let total_stack = design.glazing_thickness
        + design.matboard_thickness
        + design.artwork_thickness
        + design.backing_thickness
        + design.assembly_margin;
    
    if total_stack > design.rabbet_depth {
        let overflow = total_stack - design.rabbet_depth;
        result.add(ValidationIssue::warning_with_details(
            "rabbet_depth",
            "Material stack exceeds rabbet depth",
            &format!("Stack: {:.3}\", Rabbet: {:.3}\", Overflow: {:.3}\"",
                total_stack, design.rabbet_depth, overflow),
        ));
    }

    // Mat opening size warning
    if design.has_mat() {
        let (mat_h, mat_w) = design.get_mat_opening_dimensions();
        if mat_h < config.warn_min_mat_opening || mat_w < config.warn_min_mat_opening {
            result.add(ValidationIssue::warning_with_details(
                "mat_overlap",
                &format!("Mat opening ({:.2}\" × {:.2}\") is very small", mat_h, mat_w),
                "Check mat overlap setting",
            ));
        }
    }

    // Artwork vs visible opening warnings
    // When mat is present: artwork is compared to mat opening (visible window)
    // When no mat: artwork is compared to frame inside dimensions
    // The mat opening is artwork_size - 2*mat_overlap, so by definition artwork > mat_opening
    // Thus we only need to check artwork vs frame opening when NO mat is present
    if !design.has_mat() {
        // No mat - artwork must cover the frame opening
        if design.artwork_width < opening_width {
            let gap = opening_width - design.artwork_width;
            result.add(ValidationIssue::warning_with_details(
                "artwork_width",
                "Artwork narrower than frame opening - will show gap",
                &format!("Artwork: {:.3}\", Opening: {:.3}\", Gap: {:.3}\"",
                    design.artwork_width, opening_width, gap),
            ));
        } else {
            let overlap_per_side = (design.artwork_width - opening_width) / 2.0;
            if overlap_per_side < config.warn_artwork_opening_overlap && overlap_per_side > 0.0 {
                result.add(ValidationIssue::warning_with_details(
                    "artwork_width",
                    &format!("Artwork extends only {:.3}\" past opening per side", overlap_per_side),
                    "May not secure properly under glazing",
                ));
            }
        }

        if design.artwork_height < opening_height {
            let gap = opening_height - design.artwork_height;
            result.add(ValidationIssue::warning_with_details(
                "artwork_height",
                "Artwork shorter than frame opening - will show gap",
                &format!("Artwork: {:.3}\", Opening: {:.3}\", Gap: {:.3}\"",
                    design.artwork_height, opening_height, gap),
            ));
        } else {
            let overlap_per_side = (design.artwork_height - opening_height) / 2.0;
            if overlap_per_side < config.warn_artwork_opening_overlap && overlap_per_side > 0.0 {
                result.add(ValidationIssue::warning_with_details(
                    "artwork_height",
                    &format!("Artwork extends only {:.3}\" past opening per side", overlap_per_side),
                    "May not secure properly under glazing",
                ));
            }
        }
    }
    // When mat IS present, the mat overlap setting directly controls how much 
    // artwork extends past the visible opening - this is validated separately
    // via the mat_overlap bounds check above

    // Extreme aspect ratio warning
    let (outer_h, outer_w) = design.get_frame_outside_dimensions();
    let aspect_ratio = if outer_w > outer_h {
        outer_w / outer_h
    } else {
        outer_h / outer_w
    };
    
    if aspect_ratio > config.warn_extreme_aspect_ratio {
        result.add(ValidationIssue::warning_with_details(
            "artwork",
            &format!("Extreme aspect ratio ({:.1}:1)", aspect_ratio),
            "Verify dimensions are correct",
        ));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_design() -> FrameDesign {
        let mut design = FrameDesign::default();
        design.frame_material_width = 1.5;
        design.frame_material_depth = 0.75;
        design.artwork_width = 8.5;
        design.artwork_height = 10.5;
        design.rabbet_width = 0.375;
        design.rabbet_depth = 0.375;
        design.glazing_thickness = 0.09375; // 3/32"
        design.matboard_thickness = 0.0;
        design.artwork_thickness = 0.01;
        design.backing_thickness = 0.125;
        design.assembly_margin = 0.03125;
        design
    }

    #[test]
    fn test_valid_design() {
        let design = test_design();
        let config = ValidationConfig::default();
        let result = validate_design(&design, &config);
        assert!(result.is_valid(), "Design should be valid: {:?}", result.issues);
    }

    #[test]
    fn test_rabbet_too_wide() {
        let mut design = test_design();
        design.rabbet_width = 1.4; // Almost as wide as frame
        let config = ValidationConfig::default();
        let result = validate_design(&design, &config);
        assert!(result.has_errors());
        assert!(result.issues.iter().any(|i| i.field == "rabbet_width"));
    }

    #[test]
    fn test_stack_overflow_warning() {
        let mut design = test_design();
        design.backing_thickness = 0.5; // Make stack too thick
        let config = ValidationConfig::default();
        let result = validate_design(&design, &config);
        assert!(result.has_warnings());
        assert!(result.issues.iter().any(|i| 
            i.severity == ValidationSeverity::Warning && 
            i.message.contains("stack exceeds")));
    }

    #[test]
    fn test_artwork_smaller_than_opening() {
        // When no mat is present, the frame inside dimension is derived from
        // artwork_size - 2*rabbet, so artwork always exceeds the opening.
        // To test the "artwork narrower than opening" warning, we need a
        // scenario where artwork is explicitly smaller than the computed opening.
        // This can happen with mat: if mat is present but mat_overlap is 0 and
        // artwork is smaller than mat_width*2, the opening could exceed artwork.
        //
        // Simplest approach: no mat, but override frame inside dimensions
        // by making rabbet_width = 0 so opening = artwork, then assembly_margin
        // makes the effective opening slightly larger than the artwork.
        let mut design = test_design();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        design.rabbet_width = 0.375;
        // With no mat: opening = artwork - 2*rabbet. Artwork is always > opening.
        // The "narrower" warning can't fire here because the math prevents it.
        // Instead, test that small overlap triggers the "extends only" warning.
        // opening_width = artwork - 2*rabbet = 8.5 - 0.75 = 7.75
        // overlap_per_side = (8.5 - 7.75) / 2 = 0.375
        // This overlap is fine. Make it smaller:
        design.rabbet_width = 0.05; // Very small rabbet
        // opening_width = 8.5 - 0.1 = 8.4
        // overlap_per_side = (8.5 - 8.4) / 2 = 0.05
        // warn_artwork_opening_overlap default is 0.125, so 0.05 < 0.125 → warning
        let config = ValidationConfig::default();
        let result = validate_design(&design, &config);
        assert!(result.has_warnings(), "Expected warning about small overlap: {:?}", result.issues);
        assert!(result.issues.iter().any(|i|
            i.field == "artwork_width" &&
            i.message.contains("extends only")));
    }
}
