// Diagram styling definitions
//
// Defines colors, line weights, and typography for the
// warm, woodworking-plan aesthetic.

use serde::{Deserialize, Serialize};

/// Fill patterns for materials in section view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPatterns {
    /// Pattern for frame wood
    pub frame: FillPattern,
    /// Pattern for glazing (glass/acrylic)
    pub glazing: FillPattern,
    /// Pattern for matboard
    pub matboard: FillPattern,
    /// Pattern for artwork
    pub artwork: FillPattern,
    /// Pattern for backing
    pub backing: FillPattern,
}

impl Default for MaterialPatterns {
    fn default() -> Self {
        Self {
            frame: FillPattern::Solid("#8B6914".to_string()), // Wood brown
            glazing: FillPattern::Solid("#B8D4E3".to_string()), // Light blue
            matboard: FillPattern::Solid("#F5F0E1".to_string()), // Cream
            artwork: FillPattern::Hatched {
                color: "#E8E8E8".to_string(),
                line_color: "#CCCCCC".to_string(),
                spacing: 3.0,
            },
            backing: FillPattern::Solid("#A0A0A0".to_string()), // Gray (distinct from wood frame)
        }
    }
}

/// Fill pattern for a material in section view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillPattern {
    /// Solid color fill
    Solid(String),
    /// Diagonal hatching
    Hatched {
        color: String,
        line_color: String,
        spacing: f64,
    },
    /// Cross-hatching
    CrossHatched {
        color: String,
        line_color: String,
        spacing: f64,
    },
}

/// Complete diagram style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramStyle {
    // Colors - warm palette
    /// Background color (cream/warm white)
    pub background_color: String,
    /// Primary line color (dark brown/sepia)
    pub line_color: String,
    /// Dimension text and lines color (default/general)
    pub dimension_color: String,
    /// Inside dimension color (frame inside width/height)
    pub inside_dimension_color: String,
    /// Outside dimension color (frame outside width/height)
    pub outside_dimension_color: String,
    /// Mat dimension color (mat cut width, mat opening)
    pub mat_dimension_color: String,
    /// Artwork dimension color (artwork width/height indicator)
    pub artwork_dimension_color: String,
    /// Accent color for highlights
    pub accent_color: String,
    /// Warning/alert color (for interference)
    pub warning_color: String,
    /// Success color (for clearance OK)
    pub success_color: String,

    // Line weights (in SVG units)
    /// Frame outline stroke width
    pub frame_stroke_width: f64,
    /// Mat/opening stroke width
    pub mat_stroke_width: f64,
    /// Dimension line stroke width
    pub dimension_stroke_width: f64,
    /// Extension line stroke width
    pub extension_stroke_width: f64,

    // Typography
    /// Font family stack
    pub font_family: String,
    /// Dimension label font size
    pub dimension_font_size: f64,
    /// General label font size
    pub label_font_size: f64,
    /// Title font size
    pub title_font_size: f64,

    // Dimension line styling
    /// Gap between geometry and extension line start
    pub extension_line_gap: f64,
    /// How far extension lines extend past dimension line
    pub extension_line_overshoot: f64,
    /// Size of tick marks at dimension line ends
    pub tick_size: f64,
    /// Use tick marks instead of arrows
    pub use_tick_marks: bool,
    /// Base offset for first dimension level from geometry
    pub dimension_offset_base: f64,
    /// Additional offset per stacking level
    pub dimension_offset_step: f64,

    // Section view materials
    pub material_patterns: MaterialPatterns,

    // Layout margins
    /// Margin around the diagram content
    pub margin: f64,
    /// Minimum spacing between labels
    pub label_spacing: f64,
}

impl Default for DiagramStyle {
    fn default() -> Self {
        Self {
            // Color palette matching PyScript UI design system
            background_color: "#FFFFFF".to_string(),  // White
            line_color: "#333333".to_string(),        // Dark gray for frame lines
            dimension_color: "#277da1".to_string(),   // Cerulean (rf-primary-blue) - default
            inside_dimension_color: "#43aa8b".to_string(),  // Seagrass (rf-seagrass) - inside dims
            outside_dimension_color: "#577590".to_string(), // Blue-slate (rf-primary-blue-dark) - outside dims
            mat_dimension_color: "#f3722c".to_string(),     // Warm orange - mat dimensions
            artwork_dimension_color: "#9b5de5".to_string(), // Purple/violet - artwork dimensions
            accent_color: "#43aa8b".to_string(),      // Seagrass (rf-seagrass)
            warning_color: "#f94144".to_string(),     // Strawberry red (rf-error-red)
            success_color: "#90be6d".to_string(),     // Willow green (rf-success-green)

            // Line weights
            frame_stroke_width: 2.5,
            mat_stroke_width: 1.5,
            dimension_stroke_width: 0.75,
            extension_stroke_width: 0.5,

            // Typography - clean, readable, slightly warm
            font_family: "Inter, system-ui, -apple-system, sans-serif".to_string(),
            dimension_font_size: 12.0,  // Good for inline display
            label_font_size: 13.0,      // Good for inline display
            title_font_size: 18.0,

            // Dimension styling - arrows for professional look
            extension_line_gap: 6.0,
            extension_line_overshoot: 4.0,
            tick_size: 8.0,
            use_tick_marks: false, // Use arrows instead
            dimension_offset_base: 22.0, // Compact spacing from geometry
            dimension_offset_step: 18.0, // Tighter stacking

            // Materials
            material_patterns: MaterialPatterns::default(),

            // Layout - minimal margins (dynamic bounds handle label space)
            margin: 8.0,  // Small padding for visual comfort
            label_spacing: 6.0,
        }
    }
}

impl DiagramStyle {
    /// Create a style optimized for PDF export (larger text for print readability)
    pub fn for_pdf() -> Self {
        let mut style = Self::default();
        // Increased ~25% from previous PDF sizes for better print readability
        // After 0.8× combined view scaling: 17.6pt/19.2pt/21.6pt
        style.dimension_font_size = 22.0;
        style.label_font_size = 24.0;
        style.title_font_size = 27.0;

        // Scale spacing proportionally to prevent overlap
        style.dimension_offset_base = 28.0;  // Was 22.0, scaled ~1.27×
        style.dimension_offset_step = 23.0;  // Was 18.0, scaled ~1.28×
        style.extension_line_gap = 8.0;      // Was 6.0, scaled ~1.33×
        style.extension_line_overshoot = 5.0; // Was 4.0, scaled ~1.25×

        style.margin = 4.0; // Keep tight margins to maximize diagram space
        style
    }

    /// Create a high-contrast style (for accessibility)
    pub fn high_contrast() -> Self {
        let mut style = Self::default();
        style.background_color = "#FFFFFF".to_string();
        style.line_color = "#000000".to_string();
        style.dimension_color = "#333333".to_string();
        style.frame_stroke_width = 2.5;
        style
    }

    /// Get the offset for a given dimension level
    pub fn get_dimension_offset(&self, level: u8) -> f64 {
        self.dimension_offset_base + (level as f64 * self.dimension_offset_step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let style = DiagramStyle::default();
        assert_eq!(style.background_color, "#FFFFFF");
        assert!(!style.use_tick_marks); // Now using arrows instead
    }

    #[test]
    fn test_pdf_style() {
        let style = DiagramStyle::for_pdf();
        assert!(style.dimension_font_size > DiagramStyle::default().dimension_font_size);
        assert!(style.label_font_size > DiagramStyle::default().label_font_size);
        assert!(style.dimension_offset_base > DiagramStyle::default().dimension_offset_base);
    }

    #[test]
    fn test_dimension_offset() {
        let style = DiagramStyle::default();
        let offset_0 = style.get_dimension_offset(0);
        let offset_1 = style.get_dimension_offset(1);
        assert!(offset_1 > offset_0);
        assert!((offset_1 - offset_0 - style.dimension_offset_step).abs() < 0.001);
    }

    #[test]
    fn test_material_patterns() {
        let patterns = MaterialPatterns::default();
        match &patterns.frame {
            FillPattern::Solid(color) => assert!(color.starts_with('#')),
            _ => panic!("Expected solid fill for frame"),
        }
    }
}
