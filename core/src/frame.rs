// Frame design models and calculations
//
// Ported from Python frame.py with identical calculation behavior

use serde::{Deserialize, Serialize};
use crate::presets;

/// Represents a standard or custom frame size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSize {
    pub name: String,
    pub height: f64,  // inches
    pub width: f64,   // inches
}

impl FrameSize {
    pub fn new(name: String, height: f64, width: f64) -> Self {
        Self {
            name,
            height,
            width,
        }
    }
}

/// Complete frame design with all dimensions and materials
///
/// `#[serde(default)]` keeps old saved JSON (history entries, shared designs)
/// loadable when new fields are added later: missing fields fall back to the
/// presets.json defaults via the `Default` impl below.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameDesign {
    // Artwork dimensions
    pub artwork_width: f64,
    pub artwork_height: f64,

    // Mat configuration
    pub mat_width_top_bottom: f64,
    pub mat_width_sides: f64,
    pub mat_overlap: f64,

    // Frame dimensions
    /// Rabbet width - horizontal distance the lip extends over content (XY plane overlap)
    pub rabbet_width: f64,
    /// Rabbet depth - z-axis depth of the rabbet cutout (vertical space for materials)
    pub rabbet_depth: f64,
    pub frame_material_width: f64,

    // Material thicknesses (z-axis)
    pub matboard_thickness: f64,
    pub artwork_thickness: f64,
    pub backing_thickness: f64,
    pub glazing_thickness: f64,
    pub frame_material_depth: f64,
    pub assembly_margin: f64,

    // Flags
    pub symmetrical_mat: bool,
    pub no_artwork_margin: bool,
}

impl Default for FrameDesign {
    fn default() -> Self {
        // Load defaults from presets.json (single source of truth)
        let d = presets::get_defaults();
        Self {
            artwork_width: d.artwork_width,
            artwork_height: d.artwork_height,
            mat_width_top_bottom: d.mat_width,
            mat_width_sides: d.mat_width,
            mat_overlap: d.mat_overlap,
            rabbet_width: d.rabbet_width,
            rabbet_depth: d.rabbet_depth,
            frame_material_width: d.frame_material_width,
            matboard_thickness: d.matboard_thickness,
            artwork_thickness: d.artwork_thickness,
            backing_thickness: d.backing_thickness,
            glazing_thickness: d.glazing_thickness,
            frame_material_depth: d.frame_material_depth,
            assembly_margin: d.assembly_margin,
            symmetrical_mat: d.symmetrical_mat,
            no_artwork_margin: false,
        }
    }
}

impl FrameDesign {
    /// Create a new frame design with custom artwork dimensions
    pub fn new(artwork_height: f64, artwork_width: f64) -> Self {
        Self {
            artwork_height,
            artwork_width,
            ..Default::default()
        }
    }

    /// Enforce internal constraints on this design (symmetry, clamping, normalization).
    ///
    /// This is NOT validation against user-configurable limits -- for that,
    /// use `validation::validate_design()` with a `ValidationConfig`.
    pub fn enforce_constraints(&mut self) {
        // Enforce symmetrical mat if flag is set
        if self.symmetrical_mat && self.mat_width_sides != self.mat_width_top_bottom {
            self.mat_width_sides = self.mat_width_top_bottom;
        }

        // When no_artwork_margin is set, mat opening equals artwork size
        if self.no_artwork_margin {
            self.mat_overlap = 0.0;
        }

        // Clamp mat_overlap to sensible maximum (can't overlap more than half the artwork)
        let max_overlap_h = self.artwork_height / 2.0 - 0.125; // Leave at least 1/4" visible
        let max_overlap_w = self.artwork_width / 2.0 - 0.125;
        let max_overlap = max_overlap_h.min(max_overlap_w).max(0.0);
        if self.mat_overlap > max_overlap {
            self.mat_overlap = max_overlap;
        }

        // Enforce minimum dimensions
        const MIN_DIMENSION: f64 = 0.0625; // 1/16 inch minimum
        self.frame_material_width = self.frame_material_width.max(MIN_DIMENSION);
        self.frame_material_depth = self.frame_material_depth.max(MIN_DIMENSION);
        self.rabbet_width = self.rabbet_width.max(MIN_DIMENSION);
        self.rabbet_depth = self.rabbet_depth.max(MIN_DIMENSION);

        // Rabbet constraints: rabbet must fit within frame
        // Rabbet depth cannot exceed frame depth
        if self.rabbet_depth > self.frame_material_depth {
            self.rabbet_depth = self.frame_material_depth;
        }
        // Rabbet width cannot exceed frame width
        if self.rabbet_width > self.frame_material_width {
            self.rabbet_width = self.frame_material_width;
        }
    }

    /// Check if this design includes matting
    pub fn has_mat(&self) -> bool {
        self.mat_width_sides > 0.0 || self.mat_width_top_bottom > 0.0
    }

    /// Helper: add border to both dimensions
    fn add_border(&self, height: f64, width: f64, border: f64) -> (f64, f64) {
        (height + (2.0 * border), width + (2.0 * border))
    }

    /// Calculate visible (face) dimensions of the frame opening
    ///
    /// With mat: mat opening + visible mat borders
    /// Without mat: artwork sits in rabbet, so frame opening is smaller than artwork
    pub fn get_visible_dimensions(&self) -> (f64, f64) {
        if self.has_mat() {
            let (mat_opening_height, mat_opening_width) = self.get_mat_opening_dimensions();
            let visible_height = mat_opening_height + (2.0 * self.mat_width_top_bottom);
            let visible_width = mat_opening_width + (2.0 * self.mat_width_sides);
            (visible_height, visible_width)
        } else {
            // Without mat, artwork sits directly in rabbet - frame overlaps by rabbet_width
            let visible_height = self.artwork_height - (2.0 * self.rabbet_width);
            let visible_width = self.artwork_width - (2.0 * self.rabbet_width);
            (visible_height, visible_width)
        }
    }

    /// Calculate inside (cut) dimensions of the frame
    ///
    /// These match the visible dimensions
    pub fn get_frame_inside_dimensions(&self) -> (f64, f64) {
        self.get_visible_dimensions()
    }

    /// Calculate outside dimensions of the frame
    ///
    /// Inside dimensions plus frame material border on each side
    pub fn get_frame_outside_dimensions(&self) -> (f64, f64) {
        let (inside_h, inside_w) = self.get_frame_inside_dimensions();
        self.add_border(inside_h, inside_w, self.frame_material_width)
    }

    /// Calculate total wood length required to build the frame
    ///
    /// Outer perimeter plus margin for each of four pieces
    pub fn get_total_wood_length(&self, saw_margin: f64, error_margin: f64) -> f64 {
        let (outside_height, outside_width) = self.get_frame_outside_dimensions();
        let base_length = 2.0 * (outside_width + outside_height);
        let total_margin = 4.0 * (saw_margin + error_margin);
        base_length + total_margin
    }

    /// Calculate physical dimensions of the matboard
    ///
    /// Matboard extends into the rabbet area on all sides (by rabbet_width)
    pub fn get_matboard_dimensions(&self) -> (f64, f64) {
        let (inside_h, inside_w) = self.get_frame_inside_dimensions();
        self.add_border(inside_h, inside_w, self.rabbet_width)
    }

    /// Calculate dimensions of the opening cut in the matboard
    ///
    /// If no_artwork_margin is true, returns full artwork dimensions
    /// Otherwise, subtracts twice the mat_overlap
    pub fn get_mat_opening_dimensions(&self) -> (f64, f64) {
        if self.no_artwork_margin {
            (self.artwork_height, self.artwork_width)
        } else {
            let mat_opening_height = self.artwork_height - (2.0 * self.mat_overlap);
            let mat_opening_width = self.artwork_width - (2.0 * self.mat_overlap);
            (mat_opening_height, mat_opening_width)
        }
    }

    /// Calculate mat border cut width
    ///
    /// Visual mat width + rabbet width (portion hidden under frame lip)
    pub fn get_matboard_cut_dimensions(&self) -> (f64, f64) {
        let top_bottom_cut = self.mat_width_top_bottom + self.rabbet_width;
        let side_cut = self.mat_width_sides + self.rabbet_width;
        (top_bottom_cut, side_cut)
    }

    /// Calculate required rabbet z-axis depth based on material thicknesses
    pub fn get_rabbet_z_depth_required(&self) -> f64 {
        self.glazing_thickness
            + self.artwork_thickness
            + self.backing_thickness
            + if self.has_mat() { self.matboard_thickness } else { 0.0 }
            + self.assembly_margin
    }

    /// Generate cut list with dimensions for each frame piece
    pub fn get_cut_list(&self) -> CutList {
        let (inside_height, inside_width) = self.get_frame_inside_dimensions();
        let (outside_height, outside_width) = self.get_frame_outside_dimensions();

        CutList {
            horizontal_pieces: vec![FramePiece {
                quantity: 2,
                inside_length: inside_width,
                outside_length: outside_width,
                width: self.frame_material_width,
            }],
            vertical_pieces: vec![FramePiece {
                quantity: 2,
                inside_length: inside_height,
                outside_length: outside_height,
                width: self.frame_material_width,
            }],
        }
    }

    /// Create an interpolated design between two designs for animation.
    /// 
    /// This function linearly interpolates ALL numeric fields between `from` and `to`
    /// based on parameter `t`. This is used by the Flutter preview animation to create
    /// smooth, springy transitions when design values change.
    /// 
    /// # Parameters
    /// - `from`: The starting design state (before the change)
    /// - `to`: The target design state (after the change)
    /// - `t`: Interpolation parameter
    ///   - t=0.0 → returns `from` (start)
    ///   - t=1.0 → returns `to` (target)
    ///   - t>1.0 → extrapolates PAST target (spring overshoot)
    ///   - t<1.0 and t>0 → interpolates toward target
    /// 
    /// # Animation System Context
    /// 
    /// The Flutter animation uses Curves.elasticOut which outputs t values that
    /// oscillate around 1.0 (overshoot, undershoot, settle). By passing these
    /// directly to interpolate(), we get a springy bounce effect in the actual
    /// frame geometry - all dimensions animate together maintaining consistency.
    /// 
    /// For example, with elasticOut:
    /// - t≈1.1 (overshoot): frame dimensions extrapolate 10% past target
    /// - t≈0.97 (undershoot): frame bounces back, slightly smaller than target
    /// - t=1.0 (settle): frame arrives at exact target dimensions
    /// 
    /// # Boolean Fields
    /// 
    /// Boolean fields (`symmetrical_mat`, `no_artwork_margin`) use the destination
    /// value immediately - they cannot be meaningfully interpolated. Toggle changes
    /// are detected and skip animation entirely in the Flutter layer.
    pub fn interpolate(from: &FrameDesign, to: &FrameDesign, t: f64) -> FrameDesign {
        fn lerp(a: f64, b: f64, t: f64) -> f64 {
            a + (b - a) * t
        }

        FrameDesign {
            artwork_width: lerp(from.artwork_width, to.artwork_width, t),
            artwork_height: lerp(from.artwork_height, to.artwork_height, t),
            mat_width_top_bottom: lerp(from.mat_width_top_bottom, to.mat_width_top_bottom, t),
            mat_width_sides: lerp(from.mat_width_sides, to.mat_width_sides, t),
            mat_overlap: lerp(from.mat_overlap, to.mat_overlap, t),
            rabbet_width: lerp(from.rabbet_width, to.rabbet_width, t),
            rabbet_depth: lerp(from.rabbet_depth, to.rabbet_depth, t),
            frame_material_width: lerp(from.frame_material_width, to.frame_material_width, t),
            matboard_thickness: lerp(from.matboard_thickness, to.matboard_thickness, t),
            artwork_thickness: lerp(from.artwork_thickness, to.artwork_thickness, t),
            backing_thickness: lerp(from.backing_thickness, to.backing_thickness, t),
            glazing_thickness: lerp(from.glazing_thickness, to.glazing_thickness, t),
            frame_material_depth: lerp(from.frame_material_depth, to.frame_material_depth, t),
            assembly_margin: lerp(from.assembly_margin, to.assembly_margin, t),
            // Boolean fields: use destination value (can't interpolate booleans)
            symmetrical_mat: to.symmetrical_mat,
            no_artwork_margin: to.no_artwork_margin,
        }
    }
}

/// Represents a piece of frame material with dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePiece {
    pub quantity: usize,
    pub inside_length: f64,
    pub outside_length: f64,
    pub width: f64,
}

/// Complete cut list for frame construction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutList {
    pub horizontal_pieces: Vec<FramePiece>,
    pub vertical_pieces: Vec<FramePiece>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 0.001;

    fn assert_close(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() < TOL,
            "{label}: expected {expected}, got {actual}"
        );
    }

    fn assert_pair(actual: (f64, f64), expected: (f64, f64), label: &str) {
        assert_close(actual.0, expected.0, &format!("{label} height"));
        assert_close(actual.1, expected.1, &format!("{label} width"));
    }

    // ========================================================================
    // Construction & defaults
    // ========================================================================

    #[test]
    fn test_default_loads_from_presets() {
        let design = FrameDesign::default();
        let d = presets::get_defaults();
        assert_close(design.artwork_height, d.artwork_height, "artwork_height");
        assert_close(design.artwork_width, d.artwork_width, "artwork_width");
        assert_close(design.mat_width_top_bottom, d.mat_width, "mat_width_top_bottom");
        assert_close(design.mat_width_sides, d.mat_width, "mat_width_sides");
        assert_close(design.mat_overlap, d.mat_overlap, "mat_overlap");
        assert_close(design.frame_material_width, d.frame_material_width, "frame_material_width");
        assert_close(design.frame_material_depth, d.frame_material_depth, "frame_material_depth");
        assert_close(design.rabbet_width, d.rabbet_width, "rabbet_width");
        assert_close(design.rabbet_depth, d.rabbet_depth, "rabbet_depth");
        assert_close(design.glazing_thickness, d.glazing_thickness, "glazing");
        assert_close(design.matboard_thickness, d.matboard_thickness, "matboard");
        assert_close(design.artwork_thickness, d.artwork_thickness, "artwork");
        assert_close(design.backing_thickness, d.backing_thickness, "backing");
        assert_close(design.assembly_margin, d.assembly_margin, "assembly_margin");
    }

    #[test]
    fn test_deserialize_partial_json_fills_defaults() {
        // Old saved JSON missing fields (e.g., from before a field was added)
        // must still load, with missing fields taking their default values
        let json = r#"{"artwork_width": 14.0, "artwork_height": 11.0}"#;
        let design: FrameDesign = serde_json::from_str(json).unwrap();
        assert_close(design.artwork_width, 14.0, "artwork_width from json");
        assert_close(design.artwork_height, 11.0, "artwork_height from json");
        let d = presets::get_defaults();
        assert_close(design.frame_material_width, d.frame_material_width, "frame_width defaulted");
        assert_close(design.rabbet_depth, d.rabbet_depth, "rabbet_depth defaulted");
        assert_eq!(design.symmetrical_mat, d.symmetrical_mat);
    }

    #[test]
    fn test_new_sets_artwork_keeps_defaults() {
        let design = FrameDesign::new(10.0, 15.0);
        assert_close(design.artwork_height, 10.0, "artwork_height");
        assert_close(design.artwork_width, 15.0, "artwork_width");
        // Other fields should still be defaults
        let d = presets::get_defaults();
        assert_close(design.frame_material_width, d.frame_material_width, "frame_width from defaults");
        assert_close(design.mat_width_top_bottom, d.mat_width, "mat_width from defaults");
    }

    #[test]
    fn test_has_mat() {
        let design = FrameDesign::default();
        assert!(design.has_mat());

        let mut no_mat = FrameDesign::default();
        no_mat.mat_width_top_bottom = 0.0;
        no_mat.mat_width_sides = 0.0;
        assert!(!no_mat.has_mat());

        // One side nonzero still counts as having mat
        let mut partial = FrameDesign::default();
        partial.mat_width_top_bottom = 0.0;
        partial.mat_width_sides = 1.5;
        assert!(partial.has_mat());
    }

    // ========================================================================
    // Dimension calculations — default 8×12 with mat
    // ========================================================================
    //
    // Default: 8×12 artwork, 2" mat all sides, 1/8" overlap, 3/4" frame, 3/8" rabbet
    //
    // mat_opening      = (8 - 2×0.125, 12 - 2×0.125)         = (7.75, 11.75)
    // visible          = (7.75 + 2×2, 11.75 + 2×2)            = (11.75, 15.75)
    // frame_inside     = visible                               = (11.75, 15.75)
    // frame_outside    = (11.75 + 2×0.75, 15.75 + 2×0.75)     = (13.25, 17.25)
    // matboard_size    = (11.75 + 2×0.375, 15.75 + 2×0.375)   = (12.5, 16.5)
    // matboard_cut     = (2.0 + 0.375, 2.0 + 0.375)           = (2.375, 2.375)

    #[test]
    fn test_mat_opening_default() {
        let design = FrameDesign::default();
        assert_pair(design.get_mat_opening_dimensions(), (7.75, 11.75), "mat_opening");
    }

    #[test]
    fn test_visible_dimensions_with_mat() {
        let design = FrameDesign::default();
        assert_pair(design.get_visible_dimensions(), (11.75, 15.75), "visible");
    }

    #[test]
    fn test_frame_inside_equals_visible() {
        let design = FrameDesign::default();
        assert_pair(
            design.get_frame_inside_dimensions(),
            design.get_visible_dimensions(),
            "frame_inside == visible",
        );
    }

    #[test]
    fn test_frame_outside_default() {
        let design = FrameDesign::default();
        assert_pair(design.get_frame_outside_dimensions(), (13.25, 17.25), "frame_outside");
    }

    #[test]
    fn test_matboard_dimensions_default() {
        let design = FrameDesign::default();
        assert_pair(design.get_matboard_dimensions(), (12.5, 16.5), "matboard_size");
    }

    #[test]
    fn test_matboard_cut_dimensions_default() {
        let design = FrameDesign::default();
        assert_pair(design.get_matboard_cut_dimensions(), (2.375, 2.375), "matboard_cut");
    }

    // ========================================================================
    // Dimension calculations — no mat
    // ========================================================================
    //
    // 8×12 artwork, no mat, 3/4" frame, 3/8" rabbet
    //
    // visible          = (8 - 2×0.375, 12 - 2×0.375) = (7.25, 11.25)
    // frame_outside    = (7.25 + 2×0.75, 11.25 + 2×0.75) = (8.75, 12.75)

    #[test]
    fn test_visible_dimensions_no_mat() {
        let mut design = FrameDesign::default();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        assert_pair(design.get_visible_dimensions(), (7.25, 11.25), "visible_no_mat");
    }

    #[test]
    fn test_frame_outside_no_mat() {
        let mut design = FrameDesign::default();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        assert_pair(design.get_frame_outside_dimensions(), (8.75, 12.75), "outside_no_mat");
    }

    // ========================================================================
    // Dimension calculations — custom design (16×20, no mat, 1.5" frame, 0.5" rabbet)
    // ========================================================================
    //
    // visible      = (16 - 2×0.5, 20 - 2×0.5)       = (15.0, 19.0)
    // frame_outside = (15 + 2×1.5, 19 + 2×1.5)       = (18.0, 22.0)

    #[test]
    fn test_custom_design_dimensions() {
        let mut design = FrameDesign::new(16.0, 20.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        design.frame_material_width = 1.5;
        design.rabbet_width = 0.5;

        assert_pair(design.get_visible_dimensions(), (15.0, 19.0), "custom_visible");
        assert_pair(design.get_frame_outside_dimensions(), (18.0, 22.0), "custom_outside");
    }

    // ========================================================================
    // Asymmetrical mat
    // ========================================================================
    //
    // 8×12 artwork, top/bottom 3", sides 2", overlap 0.125"
    //
    // mat_opening  = (8 - 2×0.125, 12 - 2×0.125)        = (7.75, 11.75)
    // visible      = (7.75 + 2×3, 11.75 + 2×2)           = (13.75, 15.75)
    // frame_outside = (13.75 + 2×0.75, 15.75 + 2×0.75)   = (15.25, 17.25)
    // matboard_cut = (3 + 0.375, 2 + 0.375)               = (3.375, 2.375)

    #[test]
    fn test_asymmetrical_mat_dimensions() {
        let mut design = FrameDesign::default();
        design.symmetrical_mat = false;
        design.mat_width_top_bottom = 3.0;
        design.mat_width_sides = 2.0;

        assert_pair(design.get_visible_dimensions(), (13.75, 15.75), "asym_visible");
        assert_pair(design.get_frame_outside_dimensions(), (15.25, 17.25), "asym_outside");
        assert_pair(design.get_matboard_cut_dimensions(), (3.375, 2.375), "asym_cut");
    }

    // ========================================================================
    // Depth / material stack
    // ========================================================================

    #[test]
    fn test_rabbet_depth_required_with_mat() {
        let design = FrameDesign::default();
        // glazing 0.093 + matboard 0.055 + artwork 0.008 + backing 0.125 + margin 0.0625
        let expected = 0.093 + 0.055 + 0.008 + 0.125 + 0.0625;
        assert_close(design.get_rabbet_z_depth_required(), expected, "depth_with_mat");
    }

    #[test]
    fn test_rabbet_depth_required_no_mat() {
        let mut design = FrameDesign::default();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        // No matboard: glazing 0.093 + artwork 0.008 + backing 0.125 + margin 0.0625
        let expected = 0.093 + 0.008 + 0.125 + 0.0625;
        assert_close(design.get_rabbet_z_depth_required(), expected, "depth_no_mat");
    }

    // ========================================================================
    // Wood length & cut list
    // ========================================================================

    #[test]
    fn test_total_wood_length_default() {
        let design = FrameDesign::default();
        // outside = (13.25, 17.25)
        // perimeter = 2 × (13.25 + 17.25) = 61.0
        // blade kerf 0.125 per piece, no error margin: 4 × 0.125 = 0.5
        let expected = 61.0 + 0.5;
        assert_close(design.get_total_wood_length(0.125, 0.0), expected, "wood_length");
    }

    #[test]
    fn test_total_wood_length_with_error_margin() {
        let design = FrameDesign::default();
        // perimeter 61.0 + 4 × (0.125 + 0.25) = 61.0 + 1.5 = 62.5
        assert_close(design.get_total_wood_length(0.125, 0.25), 62.5, "wood_length_with_error");
    }

    #[test]
    fn test_cut_list_default() {
        let design = FrameDesign::default();
        let cut = design.get_cut_list();

        // Horizontal pieces (width axis): inside 15.75, outside 17.25
        assert_eq!(cut.horizontal_pieces.len(), 1);
        assert_eq!(cut.horizontal_pieces[0].quantity, 2);
        assert_close(cut.horizontal_pieces[0].inside_length, 15.75, "horiz_inside");
        assert_close(cut.horizontal_pieces[0].outside_length, 17.25, "horiz_outside");
        assert_close(cut.horizontal_pieces[0].width, 0.75, "horiz_width");

        // Vertical pieces (height axis): inside 11.75, outside 13.25
        assert_eq!(cut.vertical_pieces.len(), 1);
        assert_eq!(cut.vertical_pieces[0].quantity, 2);
        assert_close(cut.vertical_pieces[0].inside_length, 11.75, "vert_inside");
        assert_close(cut.vertical_pieces[0].outside_length, 13.25, "vert_outside");
        assert_close(cut.vertical_pieces[0].width, 0.75, "vert_width");
    }

    #[test]
    fn test_cut_list_inside_outside_relationship() {
        // For any design, outside = inside + 2 × frame_width
        let design = FrameDesign::new(10.0, 14.0);
        let cut = design.get_cut_list();
        let fw = design.frame_material_width;

        for piece in &cut.horizontal_pieces {
            assert_close(piece.outside_length, piece.inside_length + 2.0 * fw, "horiz_relation");
        }
        for piece in &cut.vertical_pieces {
            assert_close(piece.outside_length, piece.inside_length + 2.0 * fw, "vert_relation");
        }
    }

    #[test]
    fn test_cut_list_outside_length_trig_derivation() {
        // Orthogonal test: derive outside length from first principles using 45° miter geometry
        //
        // When frame pieces are cut with 45° miters at each end:
        //   - Each corner forms a 45-45-90 right triangle in cross-section
        //   - One leg of the triangle = frame_material_width (W)
        //   - The other leg = additional length added to outside edge
        //
        // For a 45-45-90 triangle, both legs are equal, so:
        //   additional_length_per_end = W
        //
        // Therefore:
        //   outside_length = inside_length + 2W
        //
        // This can also be derived using trig:
        //   tan(45°) = opposite / adjacent = W / extension
        //   Since tan(45°) = 1, extension = W

        let design = FrameDesign::new(10.0, 14.0);
        let cut = design.get_cut_list();
        let fw = design.frame_material_width;

        // Verify using 45-45-90 triangle identity
        let miter_angle = std::f64::consts::FRAC_PI_4; // 45° in radians
        let extension_per_end = fw / miter_angle.tan(); // Should equal fw since tan(45°) = 1

        for piece in &cut.horizontal_pieces {
            let expected_outside = piece.inside_length + (2.0 * extension_per_end);
            assert_close(piece.outside_length, expected_outside, "horiz_trig_derived");
        }
        for piece in &cut.vertical_pieces {
            let expected_outside = piece.inside_length + (2.0 * extension_per_end);
            assert_close(piece.outside_length, expected_outside, "vert_trig_derived");
        }
    }

    // ========================================================================
    // no_artwork_margin flag
    // ========================================================================

    #[test]
    fn test_no_artwork_margin_mat_opening_equals_artwork() {
        let mut design = FrameDesign::default();
        design.no_artwork_margin = true;
        // Mat opening should equal full artwork size (no overlap subtracted)
        assert_pair(
            design.get_mat_opening_dimensions(),
            (design.artwork_height, design.artwork_width),
            "no_margin_opening",
        );
    }

    // ========================================================================
    // enforce_constraints()
    // ========================================================================

    #[test]
    fn test_validate_symmetrical_mat_enforcement() {
        let mut design = FrameDesign::default();
        design.symmetrical_mat = true;
        design.mat_width_top_bottom = 3.0;
        design.mat_width_sides = 2.0; // different — should be overwritten
        design.enforce_constraints();
        assert_close(design.mat_width_sides, 3.0, "symmetrical_enforced");
    }

    #[test]
    fn test_validate_no_artwork_margin_clears_overlap() {
        let mut design = FrameDesign::default();
        design.no_artwork_margin = true;
        design.mat_overlap = 0.5;
        design.enforce_constraints();
        assert_close(design.mat_overlap, 0.0, "no_margin_clears_overlap");
    }

    #[test]
    fn test_validate_rabbet_clamped_to_frame() {
        let mut design = FrameDesign::default();
        design.frame_material_width = 0.75;
        design.rabbet_width = 1.5; // exceeds frame width
        design.frame_material_depth = 0.75;
        design.rabbet_depth = 1.5; // exceeds frame depth
        design.enforce_constraints();
        assert_close(design.rabbet_width, 0.75, "rabbet_width_clamped");
        assert_close(design.rabbet_depth, 0.75, "rabbet_depth_clamped");
    }

    #[test]
    fn test_validate_minimum_dimensions() {
        let mut design = FrameDesign::default();
        design.frame_material_width = 0.0;
        design.frame_material_depth = 0.0;
        design.rabbet_width = 0.0;
        design.rabbet_depth = 0.0;
        design.enforce_constraints();
        // All should be clamped to 1/16"
        assert_close(design.frame_material_width, 0.0625, "min_frame_width");
        assert_close(design.frame_material_depth, 0.0625, "min_frame_depth");
        assert_close(design.rabbet_width, 0.0625, "min_rabbet_width");
        assert_close(design.rabbet_depth, 0.0625, "min_rabbet_depth");
    }

    #[test]
    fn test_validate_mat_overlap_clamped() {
        let mut design = FrameDesign::new(4.0, 6.0);
        design.mat_overlap = 5.0; // way too big for 4×6 artwork
        design.enforce_constraints();
        // max_overlap = min(4/2 - 0.125, 6/2 - 0.125) = min(1.875, 2.875) = 1.875
        assert_close(design.mat_overlap, 1.875, "overlap_clamped");
    }

    // ========================================================================
    // Validation warnings
    // ========================================================================

    #[test]
    fn test_warning_material_stack_exceeds_frame_depth() {
        use crate::validation::{validate_design, ValidationConfig};
        let mut design = FrameDesign::default();
        design.frame_material_depth = 0.1; // way too shallow for the stack
        design.enforce_constraints(); // clamps rabbet_depth ≤ frame_material_depth
        let result = validate_design(&design, &ValidationConfig::default());
        let warnings = result.warnings();
        assert!(
            warnings.iter().any(|w| w.message.contains("exceeds rabbet depth")),
            "expected stack overflow warning, got: {:?}",
            warnings
        );
    }

    #[test]
    fn test_no_warnings_default_design() {
        use crate::validation::{validate_design, ValidationConfig};
        let design = FrameDesign::default();
        let result = validate_design(&design, &ValidationConfig::default());
        assert!(result.warnings().is_empty(), "default should have no warnings, got: {:?}", result.warnings());
    }

    // ========================================================================
    // Interpolation (animation)
    // ========================================================================

    #[test]
    fn test_interpolate_endpoints() {
        let from = FrameDesign::new(8.0, 12.0);
        let to = FrameDesign::new(16.0, 20.0);

        let at_0 = FrameDesign::interpolate(&from, &to, 0.0);
        assert_close(at_0.artwork_height, 8.0, "interp t=0 height");
        assert_close(at_0.artwork_width, 12.0, "interp t=0 width");

        let at_1 = FrameDesign::interpolate(&from, &to, 1.0);
        assert_close(at_1.artwork_height, 16.0, "interp t=1 height");
        assert_close(at_1.artwork_width, 20.0, "interp t=1 width");
    }

    #[test]
    fn test_interpolate_midpoint() {
        let from = FrameDesign::new(8.0, 12.0);
        let to = FrameDesign::new(16.0, 20.0);

        let mid = FrameDesign::interpolate(&from, &to, 0.5);
        assert_close(mid.artwork_height, 12.0, "interp t=0.5 height");
        assert_close(mid.artwork_width, 16.0, "interp t=0.5 width");
        // Mat width should also interpolate (both start at default 2.0)
        assert_close(mid.mat_width_top_bottom, 2.0, "interp t=0.5 mat_width");
    }

    #[test]
    fn test_interpolate_overshoot() {
        // elasticOut produces t > 1.0 for spring overshoot
        let from = FrameDesign::new(8.0, 12.0);
        let to = FrameDesign::new(10.0, 14.0);

        let overshoot = FrameDesign::interpolate(&from, &to, 1.1);
        // At t=1.1: 8 + (10-8)*1.1 = 8 + 2.2 = 10.2
        assert_close(overshoot.artwork_height, 10.2, "interp overshoot height");
        assert_close(overshoot.artwork_width, 14.2, "interp overshoot width");
    }

    // ========================================================================
    // no_artwork_margin — full calculation chain
    // ========================================================================
    //
    // 8×12 artwork, no_artwork_margin=true, asymmetric mat (top/bottom 3", sides 2")
    //
    // enforce_constraints: sets mat_overlap → 0
    // mat_opening   = (8, 12)                     (equals artwork, no overlap)
    // visible       = (8 + 2×3, 12 + 2×2)        = (14, 16)
    // frame_inside  = visible                     = (14, 16)
    // frame_outside = (14 + 2×0.75, 16 + 2×0.75) = (15.5, 17.5)
    // matboard_size = (14 + 2×0.375, 16 + 2×0.375) = (14.75, 16.75)
    // matboard_cut  = (3 + 0.375, 2 + 0.375)     = (3.375, 2.375)

    #[test]
    fn test_no_artwork_margin_with_asymmetric_mat() {
        let mut design = FrameDesign::default();
        design.no_artwork_margin = true;
        design.symmetrical_mat = false;
        design.mat_width_top_bottom = 3.0;
        design.mat_width_sides = 2.0;
        design.enforce_constraints();

        assert_pair(design.get_mat_opening_dimensions(), (8.0, 12.0), "no_margin_asym_opening");
        assert_pair(design.get_visible_dimensions(), (14.0, 16.0), "no_margin_asym_visible");
        assert_pair(design.get_frame_outside_dimensions(), (15.5, 17.5), "no_margin_asym_outside");
        assert_pair(design.get_matboard_dimensions(), (14.75, 16.75), "no_margin_asym_matboard");
        assert_pair(design.get_matboard_cut_dimensions(), (3.375, 2.375), "no_margin_asym_cut");
    }

    #[test]
    fn test_no_artwork_margin_visible_chain_consistency() {
        // Verify the full chain: opening → visible → inside → outside stays consistent
        let mut design = FrameDesign::default();
        design.no_artwork_margin = true;
        design.enforce_constraints();

        let (oh, ow) = design.get_mat_opening_dimensions();
        let (vh, vw) = design.get_visible_dimensions();
        let (ih, iw) = design.get_frame_inside_dimensions();
        let (ooh, oow) = design.get_frame_outside_dimensions();

        // opening = artwork exactly
        assert_close(oh, design.artwork_height, "opening == artwork_h");
        assert_close(ow, design.artwork_width, "opening == artwork_w");
        // visible = opening + 2×mat
        assert_close(vh, oh + 2.0 * design.mat_width_top_bottom, "visible = opening + mat");
        // inside == visible
        assert_close(ih, vh, "inside == visible");
        // outside = inside + 2×frame
        assert_close(ooh, ih + 2.0 * design.frame_material_width, "outside = inside + frame");
        assert_close(oow, iw + 2.0 * design.frame_material_width, "outside_w = inside_w + frame");
    }

    // ========================================================================
    // Zero-value boundary conditions
    // ========================================================================

    #[test]
    fn test_zero_mat_width_with_nonzero_rabbet() {
        // Mat width = 0 but rabbet exists: matboard_cut = 0 + rabbet
        let mut design = FrameDesign::default();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        // has_mat() is false, but matboard_cut still uses the fields
        assert_pair(
            design.get_matboard_cut_dimensions(),
            (design.rabbet_width, design.rabbet_width),
            "zero_mat_cut",
        );
    }

    #[test]
    fn test_all_zero_material_thicknesses() {
        let mut design = FrameDesign::default();
        design.glazing_thickness = 0.0;
        design.matboard_thickness = 0.0;
        design.artwork_thickness = 0.0;
        design.backing_thickness = 0.0;
        design.assembly_margin = 0.0;
        assert_close(design.get_rabbet_z_depth_required(), 0.0, "all_zero_stack");
    }

    #[test]
    fn test_rabbet_depth_no_mat_excludes_matboard() {
        // When has_mat() = false, matboard_thickness should NOT contribute
        let mut design = FrameDesign::default();
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;
        design.matboard_thickness = 1.0; // large value that should be excluded
        let depth = design.get_rabbet_z_depth_required();
        // Should be glazing + artwork + backing + margin, NOT including matboard
        let expected = design.glazing_thickness + design.artwork_thickness
            + design.backing_thickness + design.assembly_margin;
        assert_close(depth, expected, "no_mat_excludes_matboard");
    }

    #[test]
    fn test_enforce_constraints_floors_at_minimum() {
        // After enforce_constraints, zero inputs become MIN_DIMENSION
        let mut design = FrameDesign::default();
        design.frame_material_width = 0.0;
        design.rabbet_width = 0.0;
        design.enforce_constraints();
        // Calculations should still produce finite, positive results
        let (vh, vw) = design.get_visible_dimensions();
        assert!(vh > 0.0, "visible height should be positive after clamp");
        assert!(vw > 0.0, "visible width should be positive after clamp");
        let (oh, ow) = design.get_frame_outside_dimensions();
        assert!(oh > vh, "outside > visible after clamp");
        assert!(ow > vw, "outside_w > visible_w after clamp");
    }

    // ========================================================================
    // Large dimensions — verify no precision drift
    // ========================================================================

    #[test]
    fn test_large_artwork_calculations() {
        // 100×150" artwork, 4" mat, 0.25" overlap, 2" frame, 0.5" rabbet
        let mut design = FrameDesign::new(100.0, 150.0);
        design.mat_width_top_bottom = 4.0;
        design.mat_width_sides = 4.0;
        design.mat_overlap = 0.25;
        design.frame_material_width = 2.0;
        design.rabbet_width = 0.5;

        // mat_opening = (100 - 0.5, 150 - 0.5)       = (99.5, 149.5)
        // visible     = (99.5 + 8, 149.5 + 8)         = (107.5, 157.5)
        // outside     = (107.5 + 4, 157.5 + 4)         = (111.5, 161.5)
        // matboard    = (107.5 + 1, 157.5 + 1)         = (108.5, 158.5)
        assert_pair(design.get_mat_opening_dimensions(), (99.5, 149.5), "large_opening");
        assert_pair(design.get_visible_dimensions(), (107.5, 157.5), "large_visible");
        assert_pair(design.get_frame_outside_dimensions(), (111.5, 161.5), "large_outside");
        assert_pair(design.get_matboard_dimensions(), (108.5, 158.5), "large_matboard");
    }

    #[test]
    fn test_extreme_aspect_ratio_calculations() {
        // 2×48" panoramic — verify no issues with extreme L:W
        let mut design = FrameDesign::new(2.0, 48.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.mat_overlap = 0.125;

        let (vh, vw) = design.get_visible_dimensions();
        // mat_opening = (2 - 0.25, 48 - 0.25)     = (1.75, 47.75)
        // visible     = (1.75 + 4, 47.75 + 4)      = (5.75, 51.75)
        assert_close(vh, 5.75, "panoramic_visible_h");
        assert_close(vw, 51.75, "panoramic_visible_w");
        // Verify outside is always bigger than inside
        let (oh, ow) = design.get_frame_outside_dimensions();
        assert!(oh > vh, "outside_h > visible_h for panoramic");
        assert!(ow > vw, "outside_w > visible_w for panoramic");
    }

    // ========================================================================
    // Interpolation (animation)
    // ========================================================================

    #[test]
    fn test_interpolate_booleans_use_destination() {
        let mut from = FrameDesign::default();
        from.symmetrical_mat = true;
        from.no_artwork_margin = false;

        let mut to = FrameDesign::default();
        to.symmetrical_mat = false;
        to.no_artwork_margin = true;

        // Even at t=0, booleans should use 'to' values
        let result = FrameDesign::interpolate(&from, &to, 0.0);
        assert!(!result.symmetrical_mat, "bool should use destination");
        assert!(result.no_artwork_margin, "bool should use destination");
    }

    // ========================================================================
    // Round-trip integration: parse → calculate → format
    // ========================================================================
    //
    // These tests verify the full pipeline a user would experience:
    // typed input → parsed value → design calculations → formatted output

    #[test]
    fn test_roundtrip_fraction_input_through_calculations() {
        use crate::input_parser::DimensionInput;
        use crate::conversions::format_inches_as_fraction;

        // User types "8 3/4" for artwork height
        let input = DimensionInput::new("8 3/4");
        assert!(input.is_valid());
        assert_close(input.value(), 8.75, "parsed 8 3/4");

        let design = FrameDesign::new(input.value(), 12.0);
        let (visible_h, _) = design.get_visible_dimensions();
        // mat_opening_h = 8.75 - 2×0.125 = 8.5
        // visible_h = 8.5 + 2×2 = 12.5
        assert_close(visible_h, 12.5, "visible from fraction input");

        // Format back — should produce a clean fraction, not a decimal mess
        let formatted = format_inches_as_fraction(visible_h);
        assert_eq!(formatted, "12 1/2\"", "formatted visible should be clean fraction");
    }

    #[test]
    fn test_roundtrip_decimal_input_through_calculations() {
        use crate::input_parser::DimensionInput;
        use crate::conversions::format_value_with_decimal;

        // User types "10.5" for artwork width
        let input = DimensionInput::new("10.5");
        assert!(input.is_valid());

        let design = FrameDesign::new(8.0, input.value());
        let (_, outside_w) = design.get_frame_outside_dimensions();
        // visible_w = (10.5 - 0.25) + 4 = 14.25
        // outside_w = 14.25 + 1.5 = 15.75
        assert_close(outside_w, 15.75, "outside from decimal input");

        // format_value_with_decimal shows "fraction (decimal)" for inches
        let formatted = format_value_with_decimal(outside_w, crate::conversions::Unit::Inches);
        assert!(formatted.contains("15.75"), "should contain decimal value 15.75");
        assert!(formatted.contains("3/4"), "should contain fraction 3/4");
    }

    #[test]
    fn test_roundtrip_mm_input_through_calculations() {
        use crate::conversions::{mm_to_inches, format_value, Unit};

        // User enters 200mm artwork height → convert to inches → calculate → format back as mm
        let artwork_h_inches = mm_to_inches(200.0);
        assert_close(artwork_h_inches, 7.874, "200mm in inches");

        let mut design = FrameDesign::new(artwork_h_inches, mm_to_inches(300.0));
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let (visible_h, _) = design.get_visible_dimensions();
        // No mat: visible = artwork - 2×rabbet = 7.874 - 0.75 = 7.124" ≈ 180.9mm
        assert_close(visible_h, 7.124, "visible_h in inches");
        let formatted = format_value(visible_h, Unit::Millimeters);
        // Should be a reasonable mm value, not garbage
        assert!(formatted.contains("mm"), "mm format should contain 'mm'");
        // 7.124" × 25.4 = 180.95mm → formatted as "181 mm" or "180.9 mm"
        assert!(formatted.contains("180") || formatted.contains("181"),
            "expected ~181mm, got: {}", formatted);
    }

    #[test]
    fn test_roundtrip_tape_measure_format() {
        use crate::conversions::{format_value_tape_measure, Unit};

        // Standard 8×12 default: outside_h = 13.25 = 13 1/4"
        let design = FrameDesign::default();
        let (outside_h, _) = design.get_frame_outside_dimensions();
        assert_close(outside_h, 13.25, "default outside_h");

        let tape = format_value_tape_measure(outside_h, Unit::Inches);
        // Tape measure format: "13-1/4\""
        assert!(tape.contains("13"), "tape should contain whole inches");
        assert!(tape.contains("1/4"), "tape should contain 1/4 fraction");
    }
}
