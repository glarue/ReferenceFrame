//! Preset values and color palette
//!
//! Loads from data/presets.json - the single source of truth for all platforms.
//! JSON is embedded at compile time for zero runtime file I/O.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Raw JSON embedded at compile time
const PRESETS_JSON: &str = include_str!("../data/presets.json");

/// Parsed presets data (full JSON structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetsData {
    pub colors: ColorPalette,
    pub defaults: Defaults,
    pub validation_limits: ValidationLimits,
    pub presets: Presets,
}

// ============================================================================
// Color Palette
// ============================================================================

/// Complete color palette from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub palette: HashMap<String, String>,
    pub palette_light: HashMap<String, String>,
    pub palette_dark: HashMap<String, String>,
    pub neutrals: HashMap<String, String>,
    pub semantic: HashMap<String, String>,
}

impl ColorPalette {
    /// Get a color hex value by name (e.g., "teal", "primary", "gray_dark")
    /// Returns the hex string without # prefix
    pub fn get(&self, name: &str) -> Option<String> {
        // Check semantic colors first (may reference palette colors)
        if let Some(value) = self.semantic.get(name) {
            // If it's a reference like "blue", resolve it
            if !value.contains('.') && value.len() <= 12 {
                // Try to resolve from palette
                if let Some(hex) = self.palette.get(value) {
                    return Some(hex.clone());
                }
            }
            // Otherwise return as-is (it's a direct hex value)
            return Some(value.clone());
        }

        // Check main palette
        if let Some(hex) = self.palette.get(name) {
            return Some(hex.clone());
        }

        // Check light palette
        if let Some(hex) = self.palette_light.get(name) {
            return Some(hex.clone());
        }

        // Check dark palette
        if let Some(hex) = self.palette_dark.get(name) {
            return Some(hex.clone());
        }

        // Check neutrals
        if let Some(hex) = self.neutrals.get(name) {
            return Some(hex.clone());
        }

        None
    }

    /// Get color with # prefix for CSS/Flutter
    pub fn get_hex(&self, name: &str) -> Option<String> {
        self.get(name).map(|h| format!("#{}", h))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub artwork_height: f64,
    pub artwork_width: f64,
    pub include_mat: bool,
    pub symmetrical_mat: bool,
    pub frame_material_width: f64,
    pub frame_material_depth: f64,
    pub rabbet_width: f64,
    pub rabbet_depth: f64,
    pub mat_width: f64,
    pub mat_overlap: f64,
    pub glazing_thickness: f64,
    pub matboard_thickness: f64,
    pub artwork_thickness: f64,
    pub backing_thickness: f64,
    pub assembly_margin: f64,
    pub blade_width: f64,
}

/// Default validation thresholds loaded from JSON.
///
/// Mirrors the fields of `validation::ValidationConfig` -- that type's
/// `Default` impl reads these values, keeping presets.json the single
/// source of truth for validation limits. All dimensions in inches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationLimits {
    // Structural minimums
    pub min_lip_width: f64,
    pub min_face_depth: f64,
    // Frame material bounds
    pub min_frame_width: f64,
    pub max_frame_width: f64,
    pub min_frame_depth: f64,
    pub max_frame_depth: f64,
    // Opening bounds
    pub min_opening: f64,
    pub max_opening: f64,
    // Artwork dimension bounds
    pub max_artwork_dimension: f64,
    // Rabbet bounds
    pub min_rabbet: f64,
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
    pub warn_artwork_opening_overlap: f64,
    pub warn_extreme_aspect_ratio: f64,
    // Mat constraints
    pub min_visible_opening: f64,
    pub warn_min_mat_opening: f64,
    pub min_mat_overlap: f64,
    pub max_mat_overlap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCategory {
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presets {
    pub frame_face_width: PresetCategory,
    pub frame_depth: PresetCategory,
    pub rabbet_width: PresetCategory,
    pub rabbet_depth: PresetCategory,
    pub mat_width: PresetCategory,
    pub mat_overlap: PresetCategory,
    pub glazing: PresetCategory,
    pub matboard: PresetCategory,
    pub artwork: PresetCategory,
    pub backing: PresetCategory,
    pub assembly_margin: PresetCategory,
}

/// Get all presets data (parsed once, cached for lifetime of process)
pub fn get_presets_data() -> &'static PresetsData {
    static DATA: OnceLock<PresetsData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(PRESETS_JSON).expect("Invalid presets.json"))
}

/// Get just the defaults
pub fn get_defaults() -> &'static Defaults {
    &get_presets_data().defaults
}

/// Get just the preset arrays
pub fn get_presets() -> &'static Presets {
    &get_presets_data().presets
}

/// Get the default validation limits
pub fn get_validation_limits() -> &'static ValidationLimits {
    &get_presets_data().validation_limits
}

/// Get raw JSON string (for passing to FFI/WASM)
pub fn get_presets_json() -> &'static str {
    PRESETS_JSON
}

/// Legacy field name aliases from the original PyScript era.
///
/// Platform UI code (web JS, Flutter) sometimes uses these older names.
/// The canonical names are the JSON keys in `data/presets.json`.
/// New code should prefer the canonical (right-hand) names.
const FIELD_ALIASES: &[(&str, &str)] = &[
    ("frame_face_width", "frame_material_width"),
    ("frame_depth", "frame_material_depth"),
    ("mat_width", "mat_width_top_bottom"), // also covers mat_width_sides
    ("glazing", "glazing_thickness"),
    ("matboard", "matboard_thickness"),
    ("artwork", "artwork_thickness"),
    ("backing", "backing_thickness"),
];

/// Resolve a field name through legacy aliases.
/// Returns the canonical name if an alias matches, otherwise the input unchanged.
fn resolve_field_alias(field: &str) -> &str {
    for &(alias, canonical) in FIELD_ALIASES {
        if field == alias {
            return canonical;
        }
    }
    field
}

/// Get preset values for a specific field
pub fn get_preset_values(field: &str) -> &'static [f64] {
    let presets = get_presets();
    // Accept both legacy aliases and canonical names
    match resolve_field_alias(field) {
        "frame_material_width" => &presets.frame_face_width.values,
        "frame_material_depth" => &presets.frame_depth.values,
        "rabbet_width" => &presets.rabbet_width.values,
        "rabbet_depth" => &presets.rabbet_depth.values,
        "mat_width_top_bottom" | "mat_width_sides" => &presets.mat_width.values,
        "mat_overlap" => &presets.mat_overlap.values,
        "glazing_thickness" => &presets.glazing.values,
        "matboard_thickness" => &presets.matboard.values,
        "artwork_thickness" => &presets.artwork.values,
        "backing_thickness" => &presets.backing.values,
        "assembly_margin" => &presets.assembly_margin.values,
        _ => &[],
    }
}

/// Get default value for a specific field
pub fn get_default_value(field: &str) -> Option<f64> {
    let defaults = get_defaults();
    // Accept both legacy aliases and canonical names
    match resolve_field_alias(field) {
        "artwork_height" => Some(defaults.artwork_height),
        "artwork_width" => Some(defaults.artwork_width),
        "frame_material_width" => Some(defaults.frame_material_width),
        "frame_material_depth" => Some(defaults.frame_material_depth),
        "rabbet_width" => Some(defaults.rabbet_width),
        "rabbet_depth" => Some(defaults.rabbet_depth),
        "mat_width_top_bottom" | "mat_width_sides" => Some(defaults.mat_width),
        "mat_overlap" => Some(defaults.mat_overlap),
        "glazing_thickness" => Some(defaults.glazing_thickness),
        "matboard_thickness" => Some(defaults.matboard_thickness),
        "artwork_thickness" => Some(defaults.artwork_thickness),
        "backing_thickness" => Some(defaults.backing_thickness),
        "assembly_margin" => Some(defaults.assembly_margin),
        "blade_width" => Some(defaults.blade_width),
        _ => None,
    }
}

// ============================================================================
// Color Access Functions
// ============================================================================

/// Get the full color palette
pub fn get_colors() -> &'static ColorPalette {
    &get_presets_data().colors
}

/// Get a color hex value by name (without # prefix)
pub fn get_color(name: &str) -> Option<String> {
    get_colors().get(name)
}

/// Get a color hex value with # prefix
pub fn get_color_hex(name: &str) -> Option<String> {
    get_colors().get_hex(name)
}

/// Get colors as JSON string for FFI
pub fn get_colors_json() -> String {
    let colors = get_colors();
    serde_json::to_string(&colors).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_presets() {
        let data = get_presets_data();
        assert!(!data.presets.frame_face_width.values.is_empty());
        assert!(!data.presets.backing.values.is_empty());
    }

    #[test]
    fn test_defaults() {
        let defaults = get_defaults();
        assert_eq!(defaults.backing_thickness, 0.125);
        assert_eq!(defaults.frame_material_width, 0.75);
    }

    #[test]
    fn test_get_preset_values() {
        let backing = get_preset_values("backing_thickness");
        assert!(backing.contains(&0.125));
    }

    #[test]
    fn test_colors_load() {
        let colors = get_colors();
        assert!(!colors.palette.is_empty());
        assert!(colors.palette.contains_key("teal"));
    }

    #[test]
    fn test_full_ten_color_palette() {
        // All 10 platform palette colors must exist in base, light, and dark variants
        let colors = get_colors();
        let names = [
            "flag_red", "red", "red_orange", "orange", "yellow",
            "green", "teal", "dark_cyan", "blue", "air_force_blue",
        ];
        for name in names {
            assert!(colors.palette.contains_key(name), "palette missing {}", name);
            assert!(colors.palette_light.contains_key(name), "palette_light missing {}", name);
            assert!(colors.palette_dark.contains_key(name), "palette_dark missing {}", name);
        }
        // Spot-check hex values match the shipped platform palettes
        assert_eq!(get_color("flag_red"), Some("D52023".to_string()));
        assert_eq!(get_color("dark_cyan"), Some("478583".to_string()));
        assert_eq!(get_color("air_force_blue"), Some("7890A5".to_string()));
    }

    #[test]
    fn test_validation_limits_load() {
        let limits = get_validation_limits();
        assert_eq!(limits.min_frame_width, 0.5);
        assert_eq!(limits.max_frame_width, 12.0);
        assert_eq!(limits.min_rabbet, 0.125);
        assert_eq!(limits.max_mat_overlap, 6.0);
    }

    #[test]
    fn test_color_get() {
        // Direct palette color
        assert_eq!(get_color("teal"), Some("46AF8F".to_string()));
        // Semantic color resolves to palette
        assert_eq!(get_color("primary"), Some("577590".to_string()));
        // Neutral color
        assert_eq!(get_color("gray_dark"), Some("404040".to_string()));
    }

    #[test]
    fn test_color_hex() {
        assert_eq!(get_color_hex("teal"), Some("#46AF8F".to_string()));
    }
}
