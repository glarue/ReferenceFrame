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
    pub materials: Materials,
    pub validation_limits: ValidationLimits,
    /// Input hint ranges; deserialized straight into `validation::TypicalRanges`
    pub typical_ranges: crate::validation::TypicalRanges,
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
    /// Extra wood per frame piece (inches), added with the blade kerf to
    /// Total Wood: `get_total_wood_length(blade_width, wood_error_margin)`
    pub wood_error_margin: f64,
    // Joinery & hanging parameters (spline kerf/wall in inches; hanger drop
    // and wire slack as fractions; wrap allowance in inches)
    pub spline_kerf: f64,
    pub spline_min_wall: f64,
    pub hanging_drop_fraction: f64,
    pub hanging_slack_fraction: f64,
    pub hanging_wrap_allowance: f64,
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

/// One material's density (lb/ft^3 at ~12% MC for woods) with its
/// plausible low/high range and a short source citation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSpec {
    pub name: String,
    pub lb_ft3: f64,
    pub low: f64,
    pub high: f64,
    pub source: String,
}

/// Sourced density index for weight estimation. Keys are stable snake_case
/// identifiers ("generic", "red_oak", "glass", "foamcore", ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Materials {
    pub woods: std::collections::BTreeMap<String, MaterialSpec>,
    pub sheet: std::collections::BTreeMap<String, MaterialSpec>,
}

impl Materials {
    /// The honest pine-to-oak bucket used when no species is chosen
    pub fn wood_default(&self) -> &MaterialSpec {
        &self.woods["generic"]
    }
}

/// Get the sourced material density index
pub fn get_materials() -> &'static Materials {
    &get_presets_data().materials
}

/// Get the default validation limits
pub fn get_validation_limits() -> &'static ValidationLimits {
    &get_presets_data().validation_limits
}

/// Get the typical value ranges used for input hints
pub fn get_typical_ranges() -> &'static crate::validation::TypicalRanges {
    &get_presets_data().typical_ranges
}

/// Get raw JSON string (for passing to FFI/WASM)
pub fn get_presets_json() -> &'static str {
    PRESETS_JSON
}

// ============================================================================
// Color Access Functions
// ============================================================================

/// Get the full color palette
pub fn get_colors() -> &'static ColorPalette {
    &get_presets_data().colors
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
        // Total Wood margin: 1/16" per piece, shared by web and iOS
        assert_eq!(defaults.wood_error_margin, 0.0625);
    }

    #[test]
    fn test_preset_values_load() {
        assert!(get_presets().backing.values.contains(&0.125));
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
        assert_eq!(colors.palette["flag_red"], "D52023");
        assert_eq!(colors.palette["dark_cyan"], "478583");
        assert_eq!(colors.palette["air_force_blue"], "7890A5");
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
    fn test_semantic_colors_match_ios_factory_mapping() {
        // iOS SemanticColorCategory factory colors are the reference
        // (platforms/mobile/lib/models/color_category.dart)
        let expected = [
            ("primary", "blue"),
            ("secondary", "dark_cyan"),
            ("success", "green"),
            ("warning", "orange"),
            ("error", "flag_red"),
            ("modified", "yellow"),
            ("cut_dimension", "red_orange"),
            ("incidental", "teal"),
            ("material_property", "air_force_blue"),
        ];
        let colors = get_colors();
        for (category, palette_key) in expected {
            assert_eq!(colors.semantic.get(category).map(String::as_str), Some(palette_key),
                "semantic.{category}");
            assert!(colors.palette.contains_key(palette_key), "{category} → unknown palette key");
        }
    }

    #[test]
    fn test_semantic_variant_references_exist() {
        // "palette_dark.x" / "palette_light.x" entries must name a real variant
        let colors = get_colors();
        for (category, value) in colors.semantic.iter().filter(|(k, _)| !k.starts_with('_')) {
            if let Some((variant, key)) = value.split_once('.') {
                let map = match variant {
                    "palette_dark" => &colors.palette_dark,
                    "palette_light" => &colors.palette_light,
                    other => panic!("semantic.{category}: unknown variant {other}"),
                };
                assert!(map.contains_key(key), "semantic.{category} → {value}");
            }
        }
    }
}
