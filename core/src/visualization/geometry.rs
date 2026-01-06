// Geometry calculations for visualization
//
// Converts FrameDesign measurements into drawable coordinates,
// handling scaling to fit canvas with proper margins.

use crate::frame::FrameDesign;
use crate::conversions::{format_value, Unit};
use super::types::{Point, Rect};
use super::style::DiagramStyle;

// SVG layout constants - must match values in svg.rs
const EXTENSION_OVERSHOOT: f64 = 8.0;
const LABEL_BUFFER: f64 = 2.0;
const LABEL_FONT_OFFSET: f64 = 0.4;

/// Helper to estimate text width based on character count and font size
fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    // Average character width is approximately 0.6x font size for proportional fonts
    text.len() as f64 * font_size * 0.6
}

/// Computed geometry for rendering a plan view
#[derive(Debug, Clone)]
pub struct PlanViewGeometry {
    /// Frame outer rectangle
    pub frame_outer: Rect,
    /// Frame inner rectangle (visible opening)
    pub frame_inner: Rect,
    /// Mat visible area (if mat present)
    pub mat_visible: Option<Rect>,
    /// Mat opening (artwork window)
    pub mat_opening: Option<Rect>,
    /// Artwork rectangle (content area)
    pub artwork: Rect,
    /// Content area (under rabbet)
    pub content_area: Rect,
    /// Scale factor from inches to canvas units
    pub scale: f64,
    /// Origin offset (for centering)
    pub origin: Point,
}

/// Computed geometry for rendering a section view
#[derive(Debug, Clone)]
pub struct SectionViewGeometry {
    /// Overall bounds
    pub bounds: Rect,
    /// Frame profile rectangle
    pub frame_profile: Rect,
    /// Glazing layer
    pub glazing: Rect,
    /// Matboard layer (if present)
    pub matboard: Option<Rect>,
    /// Artwork layer
    pub artwork: Rect,
    /// Backing layer
    pub backing: Rect,
    /// Assembly margin layer (unfilled space for assembly tolerance)
    pub assembly_margin: Rect,
    /// Rabbet area indicator
    pub rabbet_area: Rect,
    /// Total stack height
    pub stack_height: f64,
    /// Assembly margin value (inches)
    pub assembly_margin_value: f64,
    /// Rabbet width (horizontal lip overlap)
    pub rabbet_width: f64,
    /// Rabbet depth (vertical z-axis depth)
    pub rabbet_depth: f64,
    /// Clearance (positive = OK, negative = interference)
    pub clearance: f64,
    /// Scale factor
    pub scale: f64,
    /// Origin offset
    pub origin: Point,
    /// Whether to use axis break for wide frames
    pub use_axis_break: bool,
    /// X position where the break starts (right edge of outer portion)
    pub axis_break_start_x: f64,
    /// X position where the break ends (left edge of inner portion)
    pub axis_break_end_x: f64,
    /// Width of the outer edge portion shown after the break
    pub outer_edge_width: f64,
    /// Actual frame width in inches (for dimension label)
    pub actual_frame_width: f64,
    /// Whether to use vertical axis break for deep frames
    pub use_axis_break_y: bool,
    /// Y position where the vertical break starts (bottom edge of top portion)
    pub axis_break_start_y: f64,
    /// Y position where the vertical break ends (top edge of bottom portion)
    pub axis_break_end_y: f64,
    /// Height of the top edge portion shown after the break
    pub outer_edge_depth: f64,
    /// Actual frame depth in inches (for dimension label)
    pub actual_frame_depth: f64,
}

impl PlanViewGeometry {
    /// Calculate geometry from a frame design
    pub fn from_design(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        // Get dimensions in inches
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();
        let (frame_inner_height, frame_inner_width) = design.get_frame_inside_dimensions();

        // Calculate available canvas area (accounting for margins and dimension callouts)
        let available_width = canvas_width - 2.0 * style.margin - 2.0 * style.dimension_offset_base - 2.0 * style.dimension_offset_step;
        let available_height = canvas_height - 2.0 * style.margin - 2.0 * style.dimension_offset_base - 2.0 * style.dimension_offset_step;

        // Calculate scale to fit
        let scale_x = available_width / frame_outer_width;
        let scale_y = available_height / frame_outer_height;
        let scale = scale_x.min(scale_y);

        // Calculate origin to center the diagram
        let scaled_width = frame_outer_width * scale;
        let scaled_height = frame_outer_height * scale;

        // Ensure minimum offset from edges to leave room for dimension callouts + labels
        // Labels extend above the dimension line by (font_size/2 + 2) and have height (font_size * 1.2)
        let label_extension = style.dimension_font_size + 4.0;
        let min_offset = style.margin + style.dimension_offset_base + style.dimension_offset_step + label_extension;
        let origin_x = ((canvas_width - scaled_width) / 2.0).max(min_offset);
        let origin_y = ((canvas_height - scaled_height) / 2.0).max(min_offset);
        let origin = Point::new(origin_x, origin_y);

        // Calculate rectangles
        let frame_outer = Rect::new(origin_x, origin_y, scaled_width, scaled_height);

        let frame_width_scaled = design.frame_material_width * scale;
        let frame_inner = Rect::new(
            origin_x + frame_width_scaled,
            origin_y + frame_width_scaled,
            frame_inner_width * scale,
            frame_inner_height * scale,
        );

        // Mat geometry (if mat is present)
        let (mat_visible, mat_opening) = if design.has_mat() {
            let (mat_opening_height, mat_opening_width) = design.get_mat_opening_dimensions();
            let mat_opening_scaled_w = mat_opening_width * scale;
            let mat_opening_scaled_h = mat_opening_height * scale;

            // Mat visible area = frame inner
            let mat_vis = Some(frame_inner);

            // Mat opening (centered within mat visible)
            let opening_x = frame_inner.x + (frame_inner.width - mat_opening_scaled_w) / 2.0;
            let opening_y = frame_inner.y + (frame_inner.height - mat_opening_scaled_h) / 2.0;
            let mat_open = Some(Rect::new(
                opening_x,
                opening_y,
                mat_opening_scaled_w,
                mat_opening_scaled_h,
            ));

            (mat_vis, mat_open)
        } else {
            (None, None)
        };

        // Content area (extends under rabbet lip by rabbet_width)
        let rabbet_width_scaled = design.rabbet_width * scale;
        let content_area = Rect::new(
            frame_inner.x - rabbet_width_scaled,
            frame_inner.y - rabbet_width_scaled,
            frame_inner.width + 2.0 * rabbet_width_scaled,
            frame_inner.height + 2.0 * rabbet_width_scaled,
        );

        // Artwork rectangle
        let artwork_scaled_w = design.artwork_width * scale;
        let artwork_scaled_h = design.artwork_height * scale;
        let artwork = if design.has_mat() {
            // With mat, artwork is positioned relative to content area
            Rect::new(
                content_area.x + (content_area.width - artwork_scaled_w) / 2.0,
                content_area.y + (content_area.height - artwork_scaled_h) / 2.0,
                artwork_scaled_w,
                artwork_scaled_h,
            )
        } else {
            // Without mat, artwork = content area
            content_area
        };

        Self {
            frame_outer,
            frame_inner,
            mat_visible,
            mat_opening,
            artwork,
            content_area,
            scale,
            origin,
        }
    }

    /// Convert a dimension value (inches) to canvas units
    pub fn scale_dimension(&self, value: f64) -> f64 {
        value * self.scale
    }

    /// Get a point on the frame outer boundary
    pub fn frame_outer_point(&self, t: f64, vertical: bool) -> Point {
        if vertical {
            Point::new(self.frame_outer.left(), self.frame_outer.top() + t * self.frame_outer.height)
        } else {
            Point::new(self.frame_outer.left() + t * self.frame_outer.width, self.frame_outer.top())
        }
    }
}

impl SectionViewGeometry {
    /// Calculate geometry from a frame design
    ///
    /// Section view shows frame as L-shape with materials stacked vertically.
    /// This represents a cross-section view looking from the side of the frame.
    /// Materials stack from top to bottom: glazing (front), mat, artwork, backing (back).
    pub fn from_design(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        // Material thicknesses (become vertical heights in section view)
        let glazing_t = design.glazing_thickness;
        let matboard_t = if design.has_mat() { design.matboard_thickness } else { 0.0 };
        let artwork_t = design.artwork_thickness;
        let backing_t = design.backing_thickness;

        let total_stack = glazing_t + matboard_t + artwork_t + backing_t;
        let rabbet_depth = design.rabbet_depth;
        // Clearance is based on rabbet depth (z-axis space available for materials)
        let clearance = rabbet_depth - total_stack - design.assembly_margin;

        // Axis break for wide frames - show truncated frame with break indicator
        // Threshold: use break if frame width > 3.0" (keeps visualization compact)
        let axis_break_threshold = 3.0;
        let use_axis_break = design.frame_material_width > axis_break_threshold;
        let actual_frame_width = design.frame_material_width;

        // Display width: if using break, show:
        // - Outer edge portion (left)
        // - Gap with break indicator
        // - Rabbet area + some frame body (right)
        // Otherwise show full frame width
        let outer_edge_width = 0.4;  // Width of outer edge portion shown (inches)
        let break_gap = 0.11;  // Visual gap for break indicator (inches)
        let inner_portion = design.rabbet_width + 0.5;  // Rabbet + some frame body

        let display_frame_width = if use_axis_break {
            outer_edge_width + break_gap + inner_portion
        } else {
            design.frame_material_width
        };

        // Vertical axis break for deep frames - show truncated frame with break indicator
        // Threshold: use break if frame depth > 3.0" (keeps visualization compact)
        let axis_break_threshold_y = 3.0;
        let use_axis_break_y = design.frame_material_depth > axis_break_threshold_y;
        let actual_frame_depth = design.frame_material_depth;

        // Display depth: if using break, show:
        // - Outer edge portion (top - front face)
        // - Gap with break indicator
        // - Rabbet area + some frame body (bottom)
        // Otherwise show full frame depth
        let outer_edge_depth = 0.4;  // Height of outer edge portion shown (inches)
        let break_gap_y = 0.11;  // Visual gap for break indicator (inches)
        let inner_portion_y = design.rabbet_depth + 0.5;  // Rabbet + some frame body

        let display_frame_depth = if use_axis_break_y {
            outer_edge_depth + break_gap_y + inner_portion_y
        } else {
            design.frame_material_depth
        };

        // =================================================================
        // DYNAMIC SPACE CALCULATION
        // Compute pixel reserves based on actual style and content
        // These formulas match the SVG rendering code in svg.rs
        // =================================================================

        let font_size = style.dimension_font_size;
        let unit = Unit::Inches; // Use inches for width estimation (worst case)

        // LEFT SIDE: Depth dimension callout
        // Components (from svg.rs):
        //   dim_x = frame_x - 30.0  (dimension line offset)
        //   extension lines extend to dim_x - EXTENSION_OVERSHOOT
        //   label_offset = LABEL_BUFFER + font_size * LABEL_FONT_OFFSET + 6.0
        //   depth_label_x = dim_x - label_offset
        //   label is rotated, so font_size/2 extends left of center
        let dim_line_offset = 18.0;  // Reduced from 30.0 to minimize left margin
        let label_offset_left = LABEL_BUFFER + font_size * LABEL_FONT_OFFSET + 6.0;
        let depth_dim_space = dim_line_offset + EXTENSION_OVERSHOOT.max(label_offset_left + font_size / 2.0);

        // RIGHT SIDE: Material labels + stack dimension
        // Components (from svg.rs):
        //   base_offset = 18.0.min(scale * 0.4 + 20.0) - use 18.0 as max (reduced from 35.0)
        //   material labels at label_base_x = material_right + base_offset
        //   max_label_width = estimated from text like "Glazing: 3/32""
        //   stack_dim_x = label_base_x + max_label_width + 20.0
        //   stack label at stack_dim_x + label_offset + 4.0
        //   stack label text width (rotated, so height becomes width contribution)
        let base_offset = 18.0;  // Reduced from 35.0 to minimize right margin
        
        // Estimate max material label width - format: "Material: X/X""
        // Longest material name is "Artwork" (7 chars), typical value "15/16"" (6 chars)
        // Plus ": " (2 chars) = ~15 chars, but use actual values for accuracy
        let material_labels = [
            format!("Glazing: {}", format_value(design.glazing_thickness, unit)),
            format!("Mat: {}", format_value(design.matboard_thickness, unit)),
            format!("Artwork: {}", format_value(design.artwork_thickness, unit)),
            format!("Backing: {}", format_value(design.backing_thickness, unit)),
            format!("Margin: {}", format_value(design.assembly_margin, unit)),
        ];
        let max_label_text = material_labels.iter()
            .max_by_key(|s| s.len())
            .unwrap();
        let material_label_font_size = font_size * 0.85;
        let max_label_width = estimate_text_width(max_label_text, material_label_font_size);
        
        // Stack dimension label (e.g., "9/32"") - rotated vertically
        let _stack_label_text = format_value(total_stack, unit);
        let stack_label_offset = LABEL_BUFFER + font_size * LABEL_FONT_OFFSET + 6.0 + 4.0;
        let stack_label_width = font_size; // Rotated text, so font height is the horizontal extent
        
        // Total right side space needed from material right edge
        let labels_space = base_offset + max_label_width + 20.0 + stack_label_offset + stack_label_width;

        // TOP: Width dimension callout
        // Components: line at frame_y - offset, extension overshoot, label above
        // MUST MATCH svg.rs: fw_y = frame_y - 32.0
        let width_line_offset = 32.0;
        let width_dim_space = width_line_offset + EXTENSION_OVERSHOOT + font_size;

        // BOTTOM: Legend and rabbet label
        let legend_gap = 10.0;  // Reduced from 22.0 to minimize bottom margin
        let legend_height = 25.0;
        let rabbet_label_height = 18.0 + font_size; // Leader line + text

        // Guard against zero/invalid dimensions - use sensible minimums
        let min_dimension = 0.1; // Minimum 0.1 inch for any dimension
        let safe_frame_width = display_frame_width.max(min_dimension);
        let safe_frame_depth = display_frame_depth.max(min_dimension);
        let safe_rabbet_width = design.rabbet_width.max(0.01);

        // Horizontal constraint:
        // margin + depth_dim + frame_width*scale + materials_overhang*scale + labels + margin <= canvas_width
        // where materials_overhang = rabbet_width * 1.5 (layer extends 1.5x rabbet past frame)
        let fixed_horizontal = 2.0 * style.margin + depth_dim_space + labels_space;
        let scaled_horizontal_content = safe_frame_width + safe_rabbet_width * 1.5;
        let max_scale_x = (canvas_width - fixed_horizontal) / scaled_horizontal_content;

        // Vertical constraint:
        // margin + width_dim + frame_depth*scale + below_extension + legend_gap + legend_height + margin <= canvas_height
        // where below_extension = max(rabbet_label_height, material_overflow * scale)
        // and material_overflow = max(0, total_stack - rabbet_depth)
        let material_overflow = (total_stack - rabbet_depth).max(0.0);
        let fixed_vertical = 2.0 * style.margin + width_dim_space + legend_gap + legend_height;

        // Two vertical constraints:
        // 1. Label constraint: scale <= (canvas_height - fixed_vertical - rabbet_label_height) / frame_depth
        // 2. Material constraint: scale <= (canvas_height - fixed_vertical) / (frame_depth + material_overflow)
        let max_scale_y_label = (canvas_height - fixed_vertical - rabbet_label_height) / safe_frame_depth;
        let max_scale_y_material = if material_overflow > 0.0 {
            (canvas_height - fixed_vertical) / (safe_frame_depth + material_overflow)
        } else {
            f64::MAX // No material overflow, label constraint dominates
        };
        let max_scale_y = max_scale_y_label.min(max_scale_y_material);

        // Take the more restrictive constraint
        let max_scale_fit = max_scale_x.min(max_scale_y);

        // Apply scale limits - guard against NaN/infinity
        let min_scale = 20.0;  // Minimum pixels per inch for readability
        let max_scale = 300.0; // Increased from 200.0 to allow larger diagrams in PDFs
        let scale = if max_scale_fit.is_finite() && max_scale_fit > 0.0 {
            max_scale_fit.max(min_scale).min(max_scale)
        } else {
            min_scale // Fallback to minimum scale if calculation fails
        };

        // =================================================================
        // CENTERING CALCULATION
        // Center the entire visual content block for balanced appearance
        // =================================================================

        // Scaled content dimensions
        let frame_width_scaled = display_frame_width * scale;
        let frame_depth_scaled = display_frame_depth * scale;
        let materials_overhang = design.rabbet_width * scale * 1.5;
        let rabbet_h_s = rabbet_depth * scale;

        // Calculate actual below-frame extension at this scale
        let materials_extension = (material_overflow * scale).max(0.0);
        let below_frame_extension = rabbet_label_height.max(materials_extension);

        // Total content dimensions (including callouts)
        // Top: width_dim_space is ABOVE the frame origin
        // Frame: frame_depth_scaled
        // Bottom: below_frame_extension + legend_gap + legend_height
        let content_height = width_dim_space + frame_depth_scaled + below_frame_extension;
        let total_height = content_height + legend_gap + legend_height;
        let drawn_content_width = frame_width_scaled + materials_overhang;

        // Total horizontal content block width (including asymmetric callout spaces)
        // This is what we want to center in the canvas
        let total_content_width = depth_dim_space + drawn_content_width + labels_space;

        // Horizontal centering: center the ENTIRE content block for visual balance
        // content_block_start = where the depth dimension area starts
        // frame starts at content_block_start + depth_dim_space
        let content_block_start = (canvas_width - total_content_width) / 2.0;
        let min_origin_x = style.margin + depth_dim_space;
        let origin_x = (content_block_start + depth_dim_space).max(min_origin_x);

        // Vertical centering: center the full content block (including top callout)
        // origin_y is where the FRAME starts, so we need width_dim_space above it
        let min_origin_y = style.margin + width_dim_space;
        let max_origin_y = canvas_height - style.margin - frame_depth_scaled - below_frame_extension - legend_gap - legend_height;
        let centered_y = style.margin + width_dim_space + (canvas_height - total_height - 2.0 * style.margin) / 2.0;
        let origin_y = centered_y.max(min_origin_y).min(max_origin_y).max(min_origin_y);

        let origin = Point::new(origin_x, origin_y);

        // For compatibility with existing code
        let frame_depth_s = frame_depth_scaled;

        // Bounds width should match actual drawn content
        let scaled_width = drawn_content_width;

        // Frame profile - use display width and depth (may be truncated with axis breaks)
        // Note: frame_depth_s already calculated above for centering
        let frame_width_s = display_frame_width * scale;
        let frame_profile = Rect::new(origin_x, origin_y, frame_width_s, frame_depth_s);

        // Horizontal axis break positions (in canvas coordinates)
        // outer_edge_width_s = scaled outer edge portion
        // break_gap_s = scaled gap for break indicator
        let outer_edge_width_s = outer_edge_width * scale;
        let break_gap_s = break_gap * scale;

        let (axis_break_start_x, axis_break_end_x) = if use_axis_break {
            // break_start = right edge of outer portion
            // break_end = left edge of inner portion
            let start = origin_x + outer_edge_width_s;
            let end = origin_x + outer_edge_width_s + break_gap_s;
            (start, end)
        } else {
            (0.0, 0.0)
        };

        // Vertical axis break positions (in canvas coordinates)
        let outer_edge_depth_s = outer_edge_depth * scale;
        let break_gap_y_s = break_gap_y * scale;

        let (axis_break_start_y, axis_break_end_y) = if use_axis_break_y {
            // break_start = bottom edge of top portion
            // break_end = top edge of bottom portion (rabbet area)
            let start = origin_y + outer_edge_depth_s;
            let end = origin_y + outer_edge_depth_s + break_gap_y_s;
            (start, end)
        } else {
            (0.0, 0.0)
        };

        // Rabbet dimensions (scaled) - can be non-square
        // rabbet_width = horizontal lip overlap (how far frame extends over content)
        // rabbet_depth = vertical z-axis depth of cutout (space for materials)
        // Note: rabbet_h_s already calculated above for vertical centering
        let rabbet_w_s = design.rabbet_width * scale;  // Horizontal (lip width)

        // Frame orientation in diagram:
        // - TOP = front of frame (visible face when hanging)
        // - BOTTOM = back of frame (against wall)
        //
        // The L-shape has:
        // - Main frame body from top to (frame_depth - rabbet_depth)
        // - Step/lip at y = frame_depth - rabbet_depth
        // - Rabbet cutout from (frame_depth - rabbet_depth) to frame_depth
        //
        // Materials sit in the rabbet, pressed UP against the lip:
        // - Glazing at top (touches lip)
        // - Backing at bottom (toward back of frame)

        // Materials position - they sit in the rabbet, pressed against the lip
        let content_x = origin_x + frame_width_s - rabbet_w_s;
        
        // Lip position: the lip is at (displayed_frame_depth - rabbet_depth) from origin
        // This formula works whether or not axis break is used, because frame_depth_s
        // already uses display_frame_depth (truncated when axis break is active)
        let lip_y = origin_y + frame_depth_s - rabbet_h_s;
        
        let layer_width = rabbet_w_s * 2.5; // Extend past rabbet for visibility

        // Stack from lip downward: glazing first (pressed against lip), then mat, artwork, backing
        let mut current_y = lip_y;

        let glazing = Rect::new(
            content_x,
            current_y,
            layer_width,
            glazing_t * scale,
        );
        current_y += glazing_t * scale;

        let matboard = if design.has_mat() {
            let r = Rect::new(
                content_x,
                current_y,
                layer_width,
                matboard_t * scale,
            );
            current_y += matboard_t * scale;
            Some(r)
        } else {
            None
        };

        let artwork = Rect::new(
            content_x,
            current_y,
            layer_width,
            artwork_t * scale,
        );
        current_y += artwork_t * scale;

        let backing = Rect::new(
            content_x,
            current_y,
            layer_width,
            backing_t * scale,
        );
        current_y += backing_t * scale;

        // Assembly margin - unfilled space representing tolerance for assembly
        let assembly_margin_rect = Rect::new(
            content_x,
            current_y,
            layer_width,
            design.assembly_margin * scale,
        );

        // Rabbet area - the actual rabbet cutout at bottom-right of L-shape
        // Same formula works with or without axis break since frame_depth_s is already truncated
        let rabbet_area = Rect::new(
            origin_x + frame_width_s - rabbet_w_s,
            origin_y + frame_depth_s - rabbet_h_s,
            rabbet_w_s,
            rabbet_h_s,
        );

        // Bounds height should match the content_height used for vertical centering
        // This ensures the legend is positioned correctly relative to the centered content
        // content_height was calculated earlier as: frame_depth_s + below_frame_extension
        let bounds = Rect::new(origin_x, origin_y, scaled_width, content_height);

        Self {
            bounds,
            frame_profile,
            glazing,
            matboard,
            artwork,
            backing,
            assembly_margin: assembly_margin_rect,
            rabbet_area,
            stack_height: total_stack,
            assembly_margin_value: design.assembly_margin,
            rabbet_width: design.rabbet_width,
            rabbet_depth,
            clearance,
            scale,
            origin,
            use_axis_break,
            axis_break_start_x,
            axis_break_end_x,
            outer_edge_width,
            actual_frame_width,
            use_axis_break_y,
            axis_break_start_y,
            axis_break_end_y,
            outer_edge_depth,
            actual_frame_depth,
        }
    }

    /// Check if there's clearance interference
    pub fn has_interference(&self) -> bool {
        self.clearance < 0.0
    }

    /// Convert a dimension value (inches) to canvas units
    pub fn scale_dimension(&self, value: f64) -> f64 {
        value * self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_design() -> FrameDesign {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;
        design.frame_material_width = 1.0;
        design
    }

    #[test]
    fn test_plan_view_geometry() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        // Frame outer should be largest
        assert!(geo.frame_outer.width > geo.frame_inner.width);
        assert!(geo.frame_outer.height > geo.frame_inner.height);

        // Scale should be positive
        assert!(geo.scale > 0.0);
    }

    #[test]
    fn test_plan_view_no_mat() {
        let mut design = FrameDesign::new(12.0, 16.0);
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        assert!(geo.mat_visible.is_none());
        assert!(geo.mat_opening.is_none());
    }

    #[test]
    fn test_section_view_geometry() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geo = SectionViewGeometry::from_design(&design, 800.0, 600.0, &style);

        // Stack height should be positive
        assert!(geo.stack_height > 0.0);

        // Scale should be positive
        assert!(geo.scale > 0.0);
    }

    #[test]
    fn test_section_view_clearance() {
        let mut design = test_design();
        design.frame_material_depth = 1.0; // Deep frame
        design.rabbet_depth = 1.0; // Rabbet depth determines clearance
        design.glazing_thickness = 0.125;
        design.matboard_thickness = 0.0625;
        design.artwork_thickness = 0.01;
        design.backing_thickness = 0.125;
        design.assembly_margin = 0.125;

        let style = DiagramStyle::default();
        let geo = SectionViewGeometry::from_design(&design, 800.0, 600.0, &style);

        // With 1" rabbet depth and thin materials, should have clearance
        assert!(!geo.has_interference());
        assert!(geo.clearance > 0.0);
    }

    #[test]
    fn test_section_view_interference() {
        let mut design = test_design();
        design.frame_material_depth = 0.25; // Very shallow frame
        design.rabbet_depth = 0.25; // Shallow rabbet causes interference
        design.glazing_thickness = 0.125;
        design.matboard_thickness = 0.125;
        design.artwork_thickness = 0.125;
        design.backing_thickness = 0.125;
        design.assembly_margin = 0.125;

        let style = DiagramStyle::default();
        let geo = SectionViewGeometry::from_design(&design, 800.0, 600.0, &style);

        // With 0.25" depth and thick materials, should have interference
        assert!(geo.has_interference());
    }

    #[test]
    fn test_scale_dimension() {
        let design = test_design();
        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        let scaled = geo.scale_dimension(1.0);
        assert!((scaled - geo.scale).abs() < 0.001);
    }

    #[test]
    fn test_vertical_axis_break_triggered() {
        let mut design = test_design();
        design.frame_material_depth = 5.0; // > 4" threshold
        design.frame_material_width = 0.75; // < 4" threshold, no horizontal break
        
        let style = DiagramStyle::default();
        let geo = SectionViewGeometry::from_design(&design, 700.0, 400.0, &style);

        // Vertical axis break should be triggered
        assert!(geo.use_axis_break_y, "use_axis_break_y should be true for 5\" depth");
        assert!(!geo.use_axis_break, "use_axis_break should be false for 0.75\" width");
        
        // axis_break_start_y and end_y should be set
        assert!(geo.axis_break_start_y > 0.0, "axis_break_start_y should be set");
        assert!(geo.axis_break_end_y > geo.axis_break_start_y, "axis_break_end_y should be > start");
        
        // Frame profile should use truncated depth
        // display_depth = 0.4 + 0.15 + (rabbet_depth + 0.5) ≈ 1.05 + rabbet_depth
        assert!(geo.frame_profile.height < 5.0 * geo.scale, 
            "Frame height {} should be less than full 5\" * scale {} = {}",
            geo.frame_profile.height, geo.scale, 5.0 * geo.scale);
        
        // Actual frame depth should still be recorded
        assert!((geo.actual_frame_depth - 5.0).abs() < 0.01, 
            "actual_frame_depth should be 5.0, got {}", geo.actual_frame_depth);
    }
}
