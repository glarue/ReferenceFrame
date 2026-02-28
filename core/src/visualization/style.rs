// Diagram styling definitions
//
// Defines colors, line weights, and typography for the
// warm, woodworking-plan aesthetic.

use serde::{Deserialize, Serialize};

/// Minimum gap between label text and dimension line
pub const LABEL_BUFFER: f64 = 2.0;
/// Multiplier for font size to account for text height/baseline
pub const LABEL_FONT_OFFSET: f64 = 0.4;

/// Gap between dimension font size and label arrow tip (px).
pub const LABEL_ARROW_GAP: f64 = 4.0;

/// Horizontal padding around label text in SVG mask (along text direction).
pub const LABEL_MASK_PADDING_X: f64 = 2.0;
/// Vertical padding around label text in SVG mask (perpendicular to text).
pub const LABEL_MASK_PADDING_Y: f64 = 1.0;

/// Thumbnail baseline font size (used to compute scale factor).
pub const THUMBNAIL_BASELINE_FONT_SIZE: f64 = 13.0;
/// Thumbnail gap from frame edge, scaled by thumb_sf.
pub const THUMBNAIL_GAP_BASE: f64 = 24.0;
/// Minimum thumbnail dimension in pixels.
pub const THUMBNAIL_MIN_PX: f64 = 5.0;
/// Maximum width for mini thumbnail (CD+MC present).
pub const THUMBNAIL_MINI_MAX_WIDTH: f64 = 90.0;
/// Long dimension of standard thumbnail (px, before scaling).
pub const THUMBNAIL_LONG_DIM: f64 = 95.0;
/// Short dimension of standard thumbnail (px, before scaling).
pub const THUMBNAIL_SHORT_DIM: f64 = 60.0;
/// Thumbnail label line height base (px, before scaling).
pub const THUMBNAIL_LINE_HEIGHT_BASE: f64 = 10.0;
/// Thumbnail label font size base (px, before scaling).
pub const THUMBNAIL_FONT_SIZE_BASE: f64 = 8.0;
/// Thumbnail label gap base (px, before scaling).
pub const THUMBNAIL_LABEL_GAP_BASE: f64 = 8.0;
/// Thumbnail stroke width base (px, before scaling).
pub const THUMBNAIL_STROKE_BASE: f64 = 0.75;

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
    /// Content boundary color (dashed outline showing matboard/content edge)
    pub content_boundary_color: String,
    /// Artwork boundary color (Willow Green)
    pub artwork_color: String,
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

    // Section view layout constants (shared between geometry and SVG)
    /// Depth dimension line offset from frame edge (section view, left side)
    pub section_depth_dim_offset: f64,
    /// Width dimension line offset from frame top (section view)
    pub section_width_dim_offset: f64,
    /// Material label base offset from glazing right edge
    pub section_material_label_offset: f64,
    /// Gap between material labels and stack dimension line
    pub section_stack_dim_gap: f64,
    /// Legend height (space reserved below content for material legend)
    pub legend_height: f64,
}

impl Default for DiagramStyle {
    fn default() -> Self {
        Self {
            // ================================================================
            // Standardized 7-color palette (shared with Web CSS & Flutter)
            // ================================================================
            // Core palette (warm → cool gradient):
            //   #f94144 Strawberry Red   - error/warning
            //   #f3722c Atomic Tangerine - modified/changed
            //   #f8961e Carrot Orange    - cut dimensions
            //   #f9c74f Tuscan Sun       - warnings
            //   #90be6d Willow Green     - success/material
            //   #46af8f Seaweed          - accent/incidental
            //   #577590 Blue Slate       - primary
            // ================================================================
            background_color: "#FFFFFF".to_string(),        // White
            line_color: "#333333".to_string(),              // Dark gray for frame lines
            dimension_color: "#577590".to_string(),         // Blue Slate - primary/default
            inside_dimension_color: "#46af8f".to_string(),  // Seaweed - inside dims
            outside_dimension_color: "#577590".to_string(), // Blue Slate - outside dims
            mat_dimension_color: "#f3722c".to_string(),     // Atomic Tangerine - mat dimensions
            artwork_dimension_color: "#f8961e".to_string(), // Carrot Orange - artwork dimensions
            accent_color: "#46af8f".to_string(),            // Seaweed - accent
            content_boundary_color: "#8B7355".to_string(),  // Warm brown - content boundary
            artwork_color: "#90be6d".to_string(),            // Willow Green - artwork boundary
            warning_color: "#f94144".to_string(),           // Strawberry Red - error
            success_color: "#90be6d".to_string(),           // Willow Green - success

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
            dimension_offset_step: 28.0, // Must be > 1.6 × label_font_size (= 20.8); 28 gives 7.2px gap (was 24 = 3.2px, visually crowded on narrow screens)

            // Materials
            material_patterns: MaterialPatterns::default(),

            // Layout - minimal margins (dynamic bounds handle label space)
            margin: 8.0,  // Small padding for visual comfort
            label_spacing: 6.0,

            // Section view layout
            section_depth_dim_offset: 18.0,
            section_width_dim_offset: 32.0,
            section_material_label_offset: 18.0,
            section_stack_dim_gap: 20.0,
            legend_height: 25.0,
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
        style.dimension_offset_step = 44.0;  // 1.6 × 24px = 38.4 minimum; 44 gives same relative margin as web default
        style.extension_line_gap = 8.0;      // Was 6.0, scaled ~1.33×
        style.extension_line_overshoot = 5.0; // Was 4.0, scaled ~1.25×

        style.margin = 4.0; // Keep tight margins to maximize diagram space
        style
    }

    /// Get the offset for a given dimension level
    pub fn get_dimension_offset(&self, level: u8) -> f64 {
        self.dimension_offset_base + (level as f64 * self.dimension_offset_step)
    }

    /// Label offset from dimension line (gap + buffer + font baseline adjustment)
    pub fn label_offset(&self) -> f64 {
        LABEL_BUFFER + self.label_font_size * LABEL_FONT_OFFSET + self.extension_line_gap
    }

    /// Single line height for labels
    pub fn single_line_height(&self) -> f64 {
        self.label_font_size * 1.2
    }

    /// Two line height for labels
    pub fn two_line_height(&self) -> f64 {
        self.label_font_size * 2.4
    }

    /// Material label font size (subordinate to primary labels)
    pub fn material_label_font_size(&self) -> f64 {
        self.label_font_size * 0.85
    }

    /// Offset from dimension line to mat cut label center.
    /// Used consistently in callouts, geometry, and SVG rendering.
    pub fn mat_cut_label_offset(&self) -> f64 {
        self.extension_line_overshoot + self.label_font_size / 2.0
            + self.dimension_offset_base
    }

    /// Extension from frame edge needed for label arrow tips.
    /// Returns `dimension_font_size + LABEL_ARROW_GAP`.
    pub fn label_extension(&self) -> f64 {
        self.dimension_font_size + LABEL_ARROW_GAP
    }

    /// Total reserve from frame edge for the outermost callout level.
    /// margin + dimension_offset_base + dimension_offset_step + label_extension.
    pub fn total_callout_reserve(&self) -> f64 {
        self.margin + self.dimension_offset_base + self.dimension_offset_step + self.label_extension()
    }

    /// Estimated height of a two-line label bounding box (for collision/bounds).
    /// Distinct from `two_line_height()` (× 2.4) which is for rendering.
    pub fn two_line_label_bounds_height(&self) -> f64 {
        self.label_font_size * 2.5
    }

    /// Compute thumbnail metrics scaled from `label_font_size`.
    pub fn thumbnail_metrics(&self) -> ThumbnailMetrics {
        let scale_factor = self.label_font_size / THUMBNAIL_BASELINE_FONT_SIZE;
        let line_height = THUMBNAIL_LINE_HEIGHT_BASE * scale_factor;
        let font_size = THUMBNAIL_FONT_SIZE_BASE * scale_factor;
        ThumbnailMetrics {
            scale_factor,
            gap: THUMBNAIL_GAP_BASE * scale_factor,
            min_px: THUMBNAIL_MIN_PX,
            long_dim: THUMBNAIL_LONG_DIM * scale_factor,
            short_dim: THUMBNAIL_SHORT_DIM * scale_factor,
            line_height,
            font_size,
            label_gap: THUMBNAIL_LABEL_GAP_BASE * scale_factor,
            stroke_width: THUMBNAIL_STROKE_BASE * scale_factor,
            text_below_height: line_height * 2.0 + font_size,
        }
    }
}

/// Pre-computed thumbnail display metrics, all in pixels.
#[derive(Debug, Clone, Copy)]
pub struct ThumbnailMetrics {
    /// label_font_size / THUMBNAIL_BASELINE_FONT_SIZE
    pub scale_factor: f64,
    /// Gap from frame edge (px)
    pub gap: f64,
    /// Minimum thumbnail dimension (px)
    pub min_px: f64,
    /// Long dimension of standard thumbnail (px)
    pub long_dim: f64,
    /// Short dimension of standard thumbnail (px)
    pub short_dim: f64,
    /// Label line height (px)
    pub line_height: f64,
    /// Label font size (px)
    pub font_size: f64,
    /// Gap between thumbnail rect and label text (px)
    pub label_gap: f64,
    /// Thumbnail outline stroke width (px)
    pub stroke_width: f64,
    /// Height of "Actual proportions" text below thumbnail (2 lines + font)
    pub text_below_height: f64,
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
