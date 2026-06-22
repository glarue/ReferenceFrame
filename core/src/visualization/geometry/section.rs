use super::*;

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
        let use_axis_break = design.frame_material_width > SECTION_AXIS_BREAK_THRESHOLD;
        let actual_frame_width = design.frame_material_width;

        // Display width: if using break, show:
        // - Outer edge portion (left)
        // - Gap with break indicator
        // - Rabbet area + some frame body (right)
        // Otherwise show full frame width
        let outer_edge_width = SECTION_OUTER_EDGE_WIDTH;
        let break_gap = SECTION_BREAK_GAP_X;
        let inner_portion = design.rabbet_width + SECTION_INNER_PORTION_EXTRA;

        let display_frame_width = if use_axis_break {
            outer_edge_width + break_gap + inner_portion
        } else {
            design.frame_material_width
        };

        // Vertical axis break for deep frames - show truncated frame with break indicator
        let use_axis_break_y = design.frame_material_depth > SECTION_AXIS_BREAK_THRESHOLD;
        let actual_frame_depth = design.frame_material_depth;

        // Display depth: if using break, show:
        // - Outer edge portion (top - front face)
        // - Gap with break indicator
        // - Rabbet area + some frame body (bottom)
        // Otherwise show full frame depth
        let outer_edge_depth = SECTION_OUTER_EDGE_WIDTH;
        let break_gap_y = SECTION_BREAK_GAP_Y;
        let inner_portion_y = design.rabbet_depth + SECTION_INNER_PORTION_EXTRA;

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
        //   dim_x = frame_x - style.section_depth_dim_offset
        //   extension lines extend to dim_x - style.extension_line_overshoot
        //   label_offset = style.label_offset()
        //   depth_label_x = dim_x - label_offset
        //   label is rotated, so label_font_size/2 extends left of center
        let label_offset_left = style.label_offset();
        let depth_dim_space = style.section_depth_dim_offset + style.extension_line_overshoot.max(label_offset_left + style.label_font_size / 2.0);

        // RIGHT SIDE: Material labels + stack dimension
        // Components (from svg.rs):
        //   base_offset = style.section_material_label_offset.min(scale * 0.4 + 12.0)
        //   material labels at label_base_x = material_right + base_offset
        //   max_label_width = estimated from text like "Glazing: 3/32""
        //   stack_dim_x = label_base_x + max_label_width + style.section_stack_dim_gap
        //   stack label at stack_dim_x + style.label_offset() + 4.0
        //   stack label text width (rotated, so height becomes width contribution)

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
        let fallback_label = &material_labels[0];
        let max_label_text = material_labels.iter()
            .max_by_key(|s| s.len())
            .unwrap_or(&fallback_label);
        let max_label_width = estimate_text_width(max_label_text, style.material_label_font_size());

        // Stack dimension label (e.g., "9/32"") - rotated vertically
        let _stack_label_text = format_value(total_stack, unit);
        let stack_label_offset = style.label_offset() + 4.0;
        let stack_label_width = style.label_font_size; // Rotated text, so font height is the horizontal extent

        // Total right side space needed from material right edge
        let labels_space = style.section_material_label_offset + max_label_width + style.section_stack_dim_gap + stack_label_offset + stack_label_width;

        // TOP: Width dimension callout
        // Components: line at frame_y - style.section_width_dim_offset, extension overshoot, label above
        let width_dim_space = style.section_width_dim_offset + style.extension_line_overshoot + style.label_font_size;

        // BOTTOM: Legend and rabbet label
        let legend_gap = 6.0;  // Gap between section content and legend
        // Rabbet label is now two lines (dimensions + clearance/interference)
        let rabbet_label_height = RABBET_LABEL_LEADER + font_size * RABBET_LABEL_FONT_MULTIPLIER;

        // Guard against zero/invalid dimensions
        let safe_frame_width = display_frame_width.max(SECTION_MIN_DIMENSION);
        let safe_frame_depth = display_frame_depth.max(SECTION_MIN_DIMENSION);
        let safe_rabbet_width = design.rabbet_width.max(SECTION_MIN_RABBET_WIDTH);

        // Horizontal constraint:
        // margin + depth_dim + frame_width*scale + materials_overhang*scale + labels + margin <= canvas_width
        // where materials_overhang = rabbet_width * 1.5 (layer extends 1.5x rabbet past frame)
        let fixed_horizontal = 2.0 * style.margin + depth_dim_space + labels_space;
        let scaled_horizontal_content = safe_frame_width + safe_rabbet_width * SECTION_MATERIALS_OVERHANG;
        let max_scale_x = (canvas_width - fixed_horizontal) / scaled_horizontal_content;

        // Vertical constraint:
        // margin + width_dim + frame_depth*scale + below_extension + legend_gap + legend_height + margin <= canvas_height
        // where below_extension = max(rabbet_label_height, material_overflow * scale)
        // and material_overflow = max(0, total_stack - rabbet_depth)
        let material_overflow = (total_stack - rabbet_depth).max(0.0);
        let fixed_vertical = 2.0 * style.margin + width_dim_space + legend_gap + style.legend_height;

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
        let scale = if max_scale_fit.is_finite() && max_scale_fit > 0.0 {
            max_scale_fit.max(SECTION_MIN_SCALE).min(SECTION_MAX_SCALE)
        } else {
            SECTION_MIN_SCALE // Fallback to minimum scale if calculation fails
        };

        // =================================================================
        // CENTERING CALCULATION
        // Center the entire visual content block for balanced appearance
        // =================================================================

        // Scaled content dimensions
        let frame_width_scaled = display_frame_width * scale;
        let frame_depth_scaled = display_frame_depth * scale;
        let materials_overhang = design.rabbet_width * scale * SECTION_MATERIALS_OVERHANG;
        let rabbet_h_s = rabbet_depth * scale;

        // Calculate actual below-frame extension at this scale
        let materials_extension = (material_overflow * scale).max(0.0);
        let below_frame_extension = rabbet_label_height.max(materials_extension);

        // Total content dimensions (including callouts)
        // Top: width_dim_space is ABOVE the frame origin
        // Frame: frame_depth_scaled
        // Bottom: below_frame_extension + legend_gap + legend_height
        let content_height = width_dim_space + frame_depth_scaled + below_frame_extension;
        let total_height = content_height + legend_gap + style.legend_height;
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
        let max_origin_y = canvas_height - style.margin - frame_depth_scaled - below_frame_extension - legend_gap - style.legend_height;
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

        let layer_width = rabbet_w_s * SECTION_LAYER_WIDTH_MULTIPLIER;

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

        // Bounds height: frame depth + below-frame extension (rabbet label / material overflow).
        // Does NOT include width_dim_space (which is ABOVE origin_y, for the top dimension line).
        // bounds.bottom() is used to position the legend tightly below the section content.
        let bounds = Rect::new(origin_x, origin_y, scaled_width, frame_depth_scaled + below_frame_extension);

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
            legend_gap,
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
