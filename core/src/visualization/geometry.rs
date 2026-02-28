// Geometry calculations for visualization
//
// Converts FrameDesign measurements into drawable coordinates,
// handling scaling to fit canvas with proper margins.

use crate::frame::FrameDesign;
use crate::conversions::{format_dimension, format_value, Unit};
use super::types::{AnnotationBounds, DetailMode, Point, Rect, ThumbnailLabelPosition};
use super::style::{DiagramStyle, LABEL_MASK_PADDING_X, THUMBNAIL_MINI_MAX_WIDTH};

// ============================================================================
// PLAN VIEW CONSTANTS
// ============================================================================

/// Corner detail visibility: detail_ratio threshold above which internal
/// details (rabbet, mat overlap) are too cramped → show corner detail inset.
const CORNER_STROKE_RATIO: f64 = 0.035;

/// Per-axis break threshold: frame band / outer dimension below this → needs break.
const AXIS_BREAK_RATIO: f64 = 0.025;

/// Extreme ratio: override user preference and force break at ~1%.
const FORCE_BREAK_RATIO: f64 = 0.010;

/// Hard cap on visual aspect ratio — prevents unreadably thin rendering.
const MAX_VISUAL_ASPECT_RATIO: f64 = 3.0;

/// Fixed pixel gap for axis break indicator (not scale-dependent).
const BREAK_GAP_PX: f64 = 8.0;

/// Break position bias: offset breaks so the top-left corner gets more area.
const BREAK_CENTER_BIAS_X: f64 = 0.75;
const BREAK_CENTER_BIAS_Y: f64 = 0.25;

/// Minimum rendered frame band width in pixels for break mode.
const TARGET_BAND_PX: f64 = 6.0;

/// Minimum inches of artwork to show per axis in break mode.
const MIN_DISPLAY_INCHES: f64 = 3.0;

/// Single-axis break improvement threshold: skip break if display AR
/// doesn't improve over true AR by at least this factor.
const BREAK_IMPROVEMENT_THRESHOLD: f64 = 0.90;

// ============================================================================
// SECTION VIEW CONSTANTS
// ============================================================================

/// Axis break threshold for section view (both horizontal and vertical).
const SECTION_AXIS_BREAK_THRESHOLD: f64 = 3.0;

/// Width of outer edge portion shown after break (inches).
const SECTION_OUTER_EDGE_WIDTH: f64 = 0.4;

/// Visual gap for horizontal break indicator in section view (inches).
const SECTION_BREAK_GAP_X: f64 = 0.077;

/// Visual gap for vertical break indicator in section view (inches).
const SECTION_BREAK_GAP_Y: f64 = 0.11;

/// Extra frame body shown beyond rabbet in section view (inches).
const SECTION_INNER_PORTION_EXTRA: f64 = 0.5;

/// Minimum dimension for section view calculations (inches).
const SECTION_MIN_DIMENSION: f64 = 0.1;

/// Minimum rabbet width for section view calculations.
const SECTION_MIN_RABBET_WIDTH: f64 = 0.01;

/// Material layer extends past rabbet by this multiplier for visibility.
const SECTION_MATERIALS_OVERHANG: f64 = 1.5;

/// Layer display width as multiple of rabbet width.
const SECTION_LAYER_WIDTH_MULTIPLIER: f64 = 2.5;

/// Minimum/maximum pixels per inch for section view.
const SECTION_MIN_SCALE: f64 = 20.0;
const SECTION_MAX_SCALE: f64 = 300.0;

/// Rabbet label: leader line height (px).
const RABBET_LABEL_LEADER: f64 = 18.0;

/// Rabbet label: text height as multiplier of font_size.
const RABBET_LABEL_FONT_MULTIPLIER: f64 = 2.2;

// ============================================================================
// CORNER DETAIL CONSTANTS
// ============================================================================

/// Corner detail box width as ratio of canvas width.
const CORNER_DETAIL_WIDTH_RATIO: f64 = 0.30;

/// Cap corner detail relative to rendered frame size (max dimension).
const CORNER_DETAIL_FRAME_CAP: f64 = 0.80;

/// Minimum and maximum box width (px).
const CORNER_DETAIL_MIN_WIDTH: f64 = 80.0;
const CORNER_DETAIL_MAX_WIDTH: f64 = 213.0;

/// Box height = width / this ratio.
const CORNER_DETAIL_ASPECT_RATIO: f64 = 1.15;

/// Frame band drawn at this fraction of box width.
const CORNER_DETAIL_FRAME_BAND_RATIO: f64 = 0.21;

/// Label font size as fraction of box height.
const CORNER_DETAIL_LABEL_FONT_RATIO: f64 = 0.065;

/// Box X position: nominally extends this fraction of box_w left of frame_outer.
const CORNER_DETAIL_X_OVERHANG: f64 = 0.15;

/// Minimum corner origin X as fraction of box width from box left.
const CORNER_DETAIL_CORNER_X_MIN: f64 = 0.30;

/// Corner origin Y as fraction of box height from box top.
const CORNER_DETAIL_CORNER_Y: f64 = 0.76;

/// Standard Y offset: frame_outer.bottom() - box_h * this.
const CORNER_DETAIL_Y_OFFSET: f64 = 0.85;

/// Axis-break Y blend weight toward artwork center.
const CORNER_DETAIL_CENTER_WEIGHT: f64 = 0.65;

/// Computed display artwork dimensions after axis break compression.
struct DisplayDimensions {
    artwork_w: f64,
    artwork_h: f64,
    use_break_x: bool,
    use_break_y: bool,
}

/// Result of the axis break decision pass.
struct BreakDecision {
    use_break_x: bool,
    use_break_y: bool,
    use_corner_detail: bool,
    /// Frame band width, clamped to minimum 0.001".
    frame_band: f64,
}

/// Decide which axes need axis breaks and whether corner detail is needed.
///
/// Evaluates frame proportions, label widths, and detail visibility to
/// determine the optimal break strategy before geometry is computed.
fn decide_axis_breaks(
    design: &FrameDesign,
    frame_outer_width: f64,
    frame_outer_height: f64,
    native_scale: f64,
    style: &DiagramStyle,
    detail_mode: DetailMode,
    corner_detail_enabled: bool,
    axis_breaks_enabled: bool,
    unit_mm: bool,
    use_tape_segments: bool,
    use_decimal: bool,
) -> BreakDecision {
    let frame_band = design.frame_material_width.max(0.001);
    let frame_face_px = frame_band * native_scale;
    let detail_stroke_px = style.extension_stroke_width;
    let detail_ratio = if frame_face_px > 0.0 {
        detail_stroke_px / frame_face_px
    } else {
        1.0
    };

    // Per-axis: which dimensions are geometrically extreme?
    let needs_break_x = frame_outer_width > 0.0
        && frame_band / frame_outer_width < AXIS_BREAK_RATIO;
    let needs_break_y = frame_outer_height > 0.0
        && frame_band / frame_outer_height < AXIS_BREAK_RATIO;

    let force_break_x = frame_outer_width > 0.0
        && frame_band / frame_outer_width < FORCE_BREAK_RATIO;
    let force_break_y = frame_outer_height > 0.0
        && frame_band / frame_outer_height < FORCE_BREAK_RATIO;

    // Minimum rendered dimension: if the short side would render too narrow
    // for callout labels, force a break on the long axis to reclaim space.
    let unit = if unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt_dim = |v: f64| format_dimension(v, unit, use_tape_segments, use_decimal);
    let (inside_h, inside_w) = design.get_frame_inside_dimensions();
    let fs = style.label_font_size;

    let width_labels = [
        format!("Outside: {}", fmt_dim(frame_outer_width)),
        format!("Inside: {}", fmt_dim(inside_w)),
    ];
    let min_width_px = width_labels.iter().map(|l| {
        effective_label_width(l, fs)
    }).fold(0.0_f64, f64::max) + 8.0;

    let height_labels = [
        format!("Outside: {}", fmt_dim(frame_outer_height)),
        format!("Inside: {}", fmt_dim(inside_h)),
    ];
    let min_height_px = height_labels.iter().map(|l| {
        effective_label_width(l, fs)
    }).fold(0.0_f64, f64::max) + 8.0;

    let scaled_width_native = frame_outer_width * native_scale;
    let scaled_height_native = frame_outer_height * native_scale;
    let force_break_x_label = scaled_height_native < min_height_px && frame_outer_width > frame_outer_height * 2.0;
    let force_break_y_label = scaled_width_native < min_width_px && frame_outer_height > frame_outer_width * 2.0;

    let force_breaks = force_break_x || force_break_y || force_break_x_label || force_break_y_label;

    let needs_corner_detail = detail_ratio > CORNER_STROKE_RATIO;
    let needs_breaks = needs_break_x || needs_break_y;

    let use_breaks = match detail_mode {
        DetailMode::Auto => (needs_breaks && axis_breaks_enabled) || force_breaks,
        DetailMode::None => false,
    };

    let use_corner_detail = match detail_mode {
        DetailMode::Auto => needs_corner_detail && corner_detail_enabled,
        DetailMode::None => false,
    };

    let (use_break_x, use_break_y) = if use_breaks {
        (needs_break_x || force_break_x || force_break_x_label,
         needs_break_y || force_break_y || force_break_y_label)
    } else {
        (false, false)
    };

    BreakDecision {
        use_break_x,
        use_break_y,
        use_corner_detail,
        frame_band,
    }
}

/// Compute compressed display artwork dimensions for axis break mode.
///
/// Determines how much of the artwork to show along each axis, applying
/// aspect ratio caps and label-fit refinement. May cancel breaks if the
/// stable scale already provides adequate detail.
fn compute_display_dimensions(
    design: &FrameDesign,
    frame_outer_width: f64,
    frame_outer_height: f64,
    mut use_break_x: bool,
    mut use_break_y: bool,
    frame_band: f64,
    available_width: f64,
    available_height: f64,
    canvas_width: f64,
    canvas_height: f64,
    native_scale_x: f64,
    style: &DiagramStyle,
) -> DisplayDimensions {
    let non_artwork_w = frame_outer_width - design.artwork_width;
    let non_artwork_h = frame_outer_height - design.artwork_height;
    let min_scale = TARGET_BAND_PX / frame_band;
    let min_display = MIN_DISPLAY_INCHES;

    // Helper: compute dual-axis uniform compression (preserves aspect ratio)
    let dual_axis_uniform = |scale: f64| -> (f64, f64) {
        let d_w = if design.artwork_width > 0.0 {
            (available_width / scale - non_artwork_w) / design.artwork_width
        } else { 1.0 };
        let d_h = if design.artwork_height > 0.0 {
            (available_height / scale - non_artwork_h) / design.artwork_height
        } else { 1.0 };
        let d = d_w.min(d_h).clamp(0.0, 1.0);
        (
            (d * design.artwork_width).max(min_display).min(design.artwork_width),
            (d * design.artwork_height).max(min_display).min(design.artwork_height),
        )
    };

    // Break budget: asymmetric margins
    let break_off_near = style.margin;
    let break_off_far = style.margin + style.dimension_offset_base + style.dimension_offset_step;
    let break_avail_x = canvas_width - break_off_near - break_off_far;
    let break_avail_y = canvas_height - break_off_near - break_off_far;

    // Cancel breaks when the width-limited scale already provides adequate frame detail.
    let stable_scale = native_scale_x;
    if stable_scale >= min_scale {
        use_break_x = false;
        use_break_y = false;
    }

    let (mut display_artwork_w, mut display_artwork_h, mut use_break_x, mut use_break_y) = match (use_break_x, use_break_y) {
        (true, true) => {
            let (dw, dh) = dual_axis_uniform(min_scale);
            (dw, dh, dw < design.artwork_width, dh < design.artwork_height)
        }
        (true, false) => {
            let dh = design.artwork_height;
            let max_fits = (break_avail_x / min_scale - non_artwork_w).max(min_display);
            let dw = max_fits.min(design.artwork_width);
            let min_dw = (non_artwork_h + dh - non_artwork_w).max(min_display);
            let dw = dw.max(min_dw);
            (dw, dh, dw < design.artwork_width, false)
        }
        (false, true) => {
            let dw = design.artwork_width;
            let max_fits = (break_avail_y / min_scale - non_artwork_h).max(min_display);
            let dh = max_fits.min(design.artwork_height);
            let min_dh = (non_artwork_w + dw - non_artwork_h).max(min_display);
            let dh = dh.max(min_dh);
            (dw, dh, false, dh < design.artwork_height)
        }
        _ => (design.artwork_width, design.artwork_height, false, false),
    };

    // Cap the display aspect ratio to the true frame ratio
    let true_ratio = if frame_outer_height > 0.0 && frame_outer_width > 0.0 {
        (frame_outer_width / frame_outer_height).max(frame_outer_height / frame_outer_width)
    } else {
        1.3
    };
    let max_display_ratio = true_ratio;
    {
        let disp_w = non_artwork_w + display_artwork_w;
        let disp_h = non_artwork_h + display_artwork_h;
        let ratio = disp_w / disp_h;
        if ratio > max_display_ratio {
            let target_w = disp_h * max_display_ratio;
            display_artwork_w = (target_w - non_artwork_w).max(min_display);
            use_break_x = display_artwork_w < design.artwork_width;
        } else if ratio > 0.0 && 1.0 / ratio > max_display_ratio {
            let target_h = disp_w * max_display_ratio;
            display_artwork_h = (target_h - non_artwork_h).max(min_display);
            use_break_y = display_artwork_h < design.artwork_height;
        }
    }

    // Label-fit aspect ratio cap (MAX_VISUAL_ASPECT_RATIO)
    {
        let disp_w = non_artwork_w + display_artwork_w;
        let disp_h = non_artwork_h + display_artwork_h;
        if disp_h > disp_w * MAX_VISUAL_ASPECT_RATIO {
            let target_h = disp_w * MAX_VISUAL_ASPECT_RATIO;
            display_artwork_h = (target_h - non_artwork_h).max(min_display);
            use_break_y = display_artwork_h < design.artwork_height;
        } else if disp_w > disp_h * MAX_VISUAL_ASPECT_RATIO {
            let target_w = disp_h * MAX_VISUAL_ASPECT_RATIO;
            display_artwork_w = (target_w - non_artwork_w).max(min_display);
            use_break_x = display_artwork_w < design.artwork_width;
        }
    }

    // Label-fit refinement
    {
        let disp_w = non_artwork_w + display_artwork_w;
        let disp_h = non_artwork_h + display_artwork_h;
        let is_portrait = disp_h > disp_w;

        let narrow_label_px = if is_portrait && use_break_y {
            let outside_val = format_value(frame_outer_width, Unit::Inches);
            let inside_val = format_value(design.artwork_width, Unit::Inches);
            let outside_w = estimate_text_width("Outside:", style.label_font_size)
                .max(estimate_text_width(&outside_val, style.label_font_size));
            let inside_w = estimate_text_width("Inside:", style.label_font_size)
                .max(estimate_text_width(&inside_val, style.label_font_size));
            let mask_pad = LABEL_MASK_PADDING_X * 4.0;
            outside_w.max(inside_w) + mask_pad
        } else if !is_portrait && use_break_x {
            let outside_val = format_value(frame_outer_height, Unit::Inches);
            let inside_val = format_value(design.artwork_height, Unit::Inches);
            let outside_w = estimate_text_width("Outside:", style.label_font_size)
                .max(estimate_text_width(&outside_val, style.label_font_size));
            let inside_w = estimate_text_width("Inside:", style.label_font_size)
                .max(estimate_text_width(&inside_val, style.label_font_size));
            let mask_pad = LABEL_MASK_PADDING_X * 4.0;
            outside_w.max(inside_w) + mask_pad
        } else {
            0.0
        };

        if narrow_label_px > 0.0 {
            let (projected_px, is_height_limited) = if is_portrait {
                (disp_w * available_height / disp_h, true)
            } else {
                (disp_h * available_width / disp_w, false)
            };

            if projected_px < narrow_label_px * 1.5 {
                let target_label = narrow_label_px * 1.5;
                if is_height_limited {
                    let target_h = disp_w * available_height / target_label;
                    display_artwork_h = (target_h - non_artwork_h).max(min_display);
                    use_break_y = display_artwork_h < design.artwork_height;
                } else {
                    let target_w = disp_h * available_width / target_label;
                    display_artwork_w = (target_w - non_artwork_w).max(min_display);
                    use_break_x = display_artwork_w < design.artwork_width;
                }
            }
        }
    }

    DisplayDimensions {
        artwork_w: display_artwork_w,
        artwork_h: display_artwork_h,
        use_break_x,
        use_break_y,
    }
}

/// Helper to estimate text width based on character count and font size
/// Uses character-aware widths for more accurate proportional font estimation
pub fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    // Use character-aware width estimation for proportional fonts
    // Different characters have different widths
    let mut width = 0.0;
    for c in text.chars() {
        let char_width_factor = match c {
            // Very narrow characters
            ' ' => 0.28,
            '1' | 'i' | 'l' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' => 0.4,
            'I' | 'j' | 't' | 'f' | 'r' => 0.45,
            // Brackets and parens (narrow in Inter/SF/Arial)
            '(' | ')' | '[' | ']' => 0.35,
            // Wide characters
            'm' | 'w' | 'M' | 'W' => 0.9,
            // Fraction slash (common in inches display)
            '/' => 0.4,
            // Digits (tabular width in Inter/SF)
            '0'..='9' => 0.6,
            // Most lowercase letters
            'a'..='z' => 0.58,
            // Most uppercase letters
            'A'..='Z' => 0.72,
            // Quote/inch marks
            '"' => 0.45,
            // Default for other characters
            _ => 0.6,
        };
        width += font_size * char_width_factor;
    }
    // Add 3% safety margin to ensure boxes are never too tight
    width * 1.03
}

/// Estimate the effective display width of a label, accounting for two-line split.
/// Labels containing ": " are rendered as two lines; the display width is the max
/// of the two parts rather than the full single-line width.
pub fn effective_label_width(label: &str, font_size: f64) -> f64 {
    if let Some(pos) = label.find(": ") {
        let prefix = &label[..pos + 1];
        let value = label[pos + 2..].trim_start();
        estimate_text_width(prefix, font_size)
            .max(estimate_text_width(value, font_size))
    } else {
        estimate_text_width(label, font_size)
    }
}

/// Geometry for the corner detail inset overlay
#[derive(Debug, Clone)]
pub struct CornerDetailGeometry {
    /// White background box position/size in SVG coords
    pub box_rect: Rect,
    /// Where the outside corner of the zoomed L-shape sits in SVG coords
    pub corner_origin: Point,
    /// Pixels per inch for the zoomed view
    pub detail_scale: f64,
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
    /// Whether to use axis break on X axis (horizontal compression)
    pub use_axis_break_x: bool,
    /// Whether to use axis break on Y axis (vertical compression)
    pub use_axis_break_y: bool,
    /// Canvas X where break gap begins
    pub break_x_start: f64,
    /// Canvas X where break gap ends
    pub break_x_end: f64,
    /// Canvas Y where break gap begins
    pub break_y_start: f64,
    /// Canvas Y where break gap ends
    pub break_y_end: f64,
    /// Proportional thumbnail rect (true aspect ratio silhouette), shown only when breaks active
    pub thumbnail: Option<Rect>,
    /// Whether thumbnail is positioned below (landscape) vs left (portrait)
    pub thumbnail_below: bool,
    /// Corner detail inset overlay (shown when breaks active)
    pub corner_detail: Option<CornerDetailGeometry>,
    /// Where the thumbnail label text is positioned
    pub thumbnail_label_position: ThumbnailLabelPosition,
    /// Bounding boxes for floating annotations (for collision detection and viewBox)
    pub annotation_bounds: AnnotationBounds,
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
    /// Gap between content bottom and legend (computed once, used by SVG renderer)
    pub legend_gap: f64,
}

impl PlanViewGeometry {
    /// Build all rectangles from pre-computed scale and origin
    fn build_rects(design: &FrameDesign, scale: f64, origin_x: f64, origin_y: f64) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();
        let (frame_inner_height, frame_inner_width) = design.get_frame_inside_dimensions();

        let origin = Point::new(origin_x, origin_y);
        let frame_outer = Rect::new(origin_x, origin_y, frame_outer_width * scale, frame_outer_height * scale);

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

            let mat_vis = Some(frame_inner);
            let opening_x = frame_inner.x + (frame_inner.width - mat_opening_scaled_w) / 2.0;
            let opening_y = frame_inner.y + (frame_inner.height - mat_opening_scaled_h) / 2.0;
            let mat_open = Some(Rect::new(opening_x, opening_y, mat_opening_scaled_w, mat_opening_scaled_h));

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
            Rect::new(
                content_area.x + (content_area.width - artwork_scaled_w) / 2.0,
                content_area.y + (content_area.height - artwork_scaled_h) / 2.0,
                artwork_scaled_w,
                artwork_scaled_h,
            )
        } else {
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
            use_axis_break_x: false,
            use_axis_break_y: false,
            break_x_start: 0.0,
            break_x_end: 0.0,
            break_y_start: 0.0,
            break_y_end: 0.0,
            thumbnail: None,
            thumbnail_below: false,
            corner_detail: None,
            thumbnail_label_position: ThumbnailLabelPosition::Below,

            annotation_bounds: AnnotationBounds {
                corner_detail_box: None,
                thumbnail_box: None,
                thumbnail_label_position: ThumbnailLabelPosition::Below,
                mat_cut_width_label: None,
                mat_cut_height_label: None,
                mat_cut_extent: None,
            },
        }
    }

    /// Build rectangles using display (compressed) artwork dimensions for axis breaks.
    /// Frame band, mat border, and rabbet use actual design dimensions at the new scale.
    /// Only the artwork rect and the regions it affects use display dimensions.
    fn build_rects_with_display_artwork(
        design: &FrameDesign,
        scale: f64,
        origin_x: f64,
        origin_y: f64,
        display_artwork_w: f64,
        display_artwork_h: f64,
        display_outer_w: f64,
        display_outer_h: f64,
    ) -> Self {
        // Display inner = actual_inner dimensions adjusted for compressed artwork
        // The difference between outer and inner is always 2 * frame_material_width
        let display_inner_w = display_outer_w - 2.0 * design.frame_material_width;
        let display_inner_h = display_outer_h - 2.0 * design.frame_material_width;

        let origin = Point::new(origin_x, origin_y);
        let frame_outer = Rect::new(origin_x, origin_y, display_outer_w * scale, display_outer_h * scale);

        let frame_width_scaled = design.frame_material_width * scale;
        let frame_inner = Rect::new(
            origin_x + frame_width_scaled,
            origin_y + frame_width_scaled,
            display_inner_w * scale,
            display_inner_h * scale,
        );

        // Mat geometry (if mat is present)
        let (mat_visible, mat_opening) = if design.has_mat() {
            let (mat_opening_height, mat_opening_width) = design.get_mat_opening_dimensions();
            // Display mat opening: compressed similarly to artwork
            // mat_opening = artwork + 2*mat_overlap, so compress by same amount as artwork
            let display_mat_opening_w = mat_opening_width - (design.artwork_width - display_artwork_w);
            let display_mat_opening_h = mat_opening_height - (design.artwork_height - display_artwork_h);
            let mat_opening_scaled_w = display_mat_opening_w * scale;
            let mat_opening_scaled_h = display_mat_opening_h * scale;

            let mat_vis = Some(frame_inner);
            let opening_x = frame_inner.x + (frame_inner.width - mat_opening_scaled_w) / 2.0;
            let opening_y = frame_inner.y + (frame_inner.height - mat_opening_scaled_h) / 2.0;
            let mat_open = Some(Rect::new(opening_x, opening_y, mat_opening_scaled_w, mat_opening_scaled_h));

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

        // Artwork rectangle uses display dimensions
        let artwork_scaled_w = display_artwork_w * scale;
        let artwork_scaled_h = display_artwork_h * scale;
        let artwork = if design.has_mat() {
            Rect::new(
                content_area.x + (content_area.width - artwork_scaled_w) / 2.0,
                content_area.y + (content_area.height - artwork_scaled_h) / 2.0,
                artwork_scaled_w,
                artwork_scaled_h,
            )
        } else {
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
            use_axis_break_x: false,
            use_axis_break_y: false,
            break_x_start: 0.0,
            break_x_end: 0.0,
            break_y_start: 0.0,
            break_y_end: 0.0,
            thumbnail: None,
            thumbnail_below: false,
            corner_detail: None,
            thumbnail_label_position: ThumbnailLabelPosition::Below,

            annotation_bounds: AnnotationBounds {
                corner_detail_box: None,
                thumbnail_box: None,
                thumbnail_label_position: ThumbnailLabelPosition::Below,
                mat_cut_width_label: None,
                mat_cut_height_label: None,
                mat_cut_extent: None,
            },
        }
    }

    /// Calculate geometry from a frame design
    pub fn from_design(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        Self::from_design_with_mode(design, canvas_width, canvas_height, style, DetailMode::Auto, true, true, false, false, false)
    }

    /// Calculate geometry with explicit detail mode and feature flags
    pub fn from_design_with_mode(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
        detail_mode: DetailMode,
        corner_detail_enabled: bool,
        axis_breaks_enabled: bool,
        unit_mm: bool,
        use_tape_segments: bool,
        use_decimal: bool,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();

        // Calculate available canvas area (accounting for margins and dimension callouts).
        // Top and right always have callouts (frame dims); bottom and left only when mat present.
        let callout_reservation = style.dimension_offset_base + style.dimension_offset_step;
        let has_mat = design.has_mat();
        let top_reserve = callout_reservation;
        let bottom_reserve = if has_mat { callout_reservation } else { style.margin };
        let right_reserve = callout_reservation;
        let left_reserve = if has_mat { callout_reservation } else { style.margin };
        let available_width = canvas_width - 2.0 * style.margin - right_reserve - left_reserve;
        let available_height = canvas_height - 2.0 * style.margin - top_reserve - bottom_reserve;

        // Trial scale per axis: how many pixels per inch at full fit
        let native_scale_x = available_width / frame_outer_width;
        let native_scale_y = available_height / frame_outer_height;
        let native_scale = native_scale_x.min(native_scale_y);

        let bd = decide_axis_breaks(
            design, frame_outer_width, frame_outer_height, native_scale,
            style, detail_mode, corner_detail_enabled, axis_breaks_enabled,
            unit_mm, use_tape_segments, use_decimal,
        );
        let use_corner_detail = bd.use_corner_detail;
        let frame_band = bd.frame_band;
        let (use_break_x, use_break_y) = (bd.use_break_x, bd.use_break_y);

        if !use_break_x && !use_break_y {
            return Self::build_no_break_geometry(
                design, native_scale, canvas_width, canvas_height,
                use_corner_detail, style,
            );
        }

        let dd = compute_display_dimensions(
            design, frame_outer_width, frame_outer_height,
            use_break_x, use_break_y, frame_band,
            available_width, available_height, canvas_width, canvas_height,
            native_scale_x, style,
        );
        let display_artwork_w = dd.artwork_w;
        let display_artwork_h = dd.artwork_h;
        let use_break_x = dd.use_break_x;
        let use_break_y = dd.use_break_y;

        // If break computation determined everything fits uncompressed,
        // fall back to the standard path which can still apply corner detail.
        if !use_break_x && !use_break_y {
            return Self::build_no_break_geometry(
                design, native_scale, canvas_width, canvas_height,
                use_corner_detail, style,
            );
        }

        // Display outer = actual_outer - actual_artwork + display_artwork
        let display_outer_w = frame_outer_width - design.artwork_width + display_artwork_w;
        let display_outer_h = frame_outer_height - design.artwork_height + display_artwork_h;

        // Marginal break guard (single-axis only): if the break barely compresses
        // the frame, the break gap (8px) can make the visual aspect ratio as extreme
        // as (or more than) the actual frame. Skip the break when the rendered AR
        // wouldn't improve over the true AR by at least 10%.
        // Dual-axis breaks always compress meaningfully (both axes are extreme).
        let is_single_axis = use_break_x != use_break_y;
        {
            let actual_ar = (frame_outer_width / frame_outer_height)
                .max(frame_outer_height / frame_outer_width);
            let display_ar = (display_outer_w / display_outer_h)
                .max(display_outer_h / display_outer_w);
            if is_single_axis && display_ar > actual_ar * BREAK_IMPROVEMENT_THRESHOLD {
                // Break doesn't meaningfully help — fall back to no-break path
                return Self::build_no_break_geometry(
                    design, native_scale, canvas_width, canvas_height,
                    use_corner_detail, style,
                );
            }
        }

        // Final scale from display dimensions.
        // Axis break frames have no left-side callouts, so we can use asymmetric
        // margins: only `margin` on the left, full callout space on the right.
        // This gives the frame more horizontal space to expand into.
        let right_offset = style.margin + style.dimension_offset_base + style.dimension_offset_step;
        let left_offset = style.margin;
        let break_available_width = canvas_width - left_offset - right_offset;
        let scale_x = break_available_width / display_outer_w;
        let scale_y = available_height / display_outer_h;
        let scale = scale_x.min(scale_y);

        let scaled_width = display_outer_w * scale;
        let scaled_height = display_outer_h * scale;

        let origin_x = ((break_available_width - scaled_width) / 2.0 + left_offset).max(left_offset);
        let min_offset_y = style.total_callout_reserve();
        let origin_y = ((canvas_height - scaled_height) / 2.0).max(min_offset_y);

        // Build rects using display artwork dimensions
        // Frame band, mat border, rabbet all use actual dimensions at new scale
        let mut geo = Self::build_rects_with_display_artwork(
            design, scale, origin_x, origin_y,
            display_artwork_w, display_artwork_h,
            display_outer_w, display_outer_h,
        );

        // Compute break positions in canvas coords
        // Offset breaks so the top-left corner gets more visible area
        let break_center_x = geo.artwork.x + geo.artwork.width * BREAK_CENTER_BIAS_X;
        let break_center_y = geo.artwork.y + geo.artwork.height * BREAK_CENTER_BIAS_Y;

        geo.use_axis_break_x = use_break_x;
        geo.use_axis_break_y = use_break_y;

        if use_break_x {
            geo.break_x_start = break_center_x - BREAK_GAP_PX / 2.0;
            geo.break_x_end = break_center_x + BREAK_GAP_PX / 2.0;
        }
        if use_break_y {
            geo.break_y_start = break_center_y - BREAK_GAP_PX / 2.0;
            geo.break_y_end = break_center_y + BREAK_GAP_PX / 2.0;
        }

        // Corner detail when face is too narrow at this scale to show internal details.
        if use_corner_detail && design.frame_material_width > 0.0 {
            geo.corner_detail = Some(Self::compute_corner_detail(design, &geo, canvas_width, style));
        }

        // Two-pass placement: compute mat cut extent first (actual side choice + label bounds),
        // then use those bounds in the occupied list so thumbnail placement is collision-free
        // without the approximation loop of the old approach.
        let cd_occupied: Vec<Rect> = geo.corner_detail.as_ref().map(|cd| vec![cd.box_rect]).unwrap_or_default();
        let mat_cut_extent: Option<(Point, Point)> = if design.has_mat() {
            geo.mat_opening.as_ref().map(|mat_opening| {
                Self::choose_mat_cut_extent(
                    &geo.frame_inner,
                    &geo.content_area,
                    mat_opening,
                    &cd_occupied,
                    style,
                )
            })
        } else {
            None
        };

        // Build occupied list from already-placed elements (corner detail + mat cut label).
        let mut occupied: Vec<Rect> = Vec::new();
        if let Some(cd) = &geo.corner_detail {
            occupied.push(cd.box_rect);
            if let Some((ref start, ref end)) = mat_cut_extent {
                occupied.push(Self::mat_cut_label_bounds_from_extent(
                    &geo.frame_outer, start, end, style,
                ));
            }
        }

        Self::compute_thumbnail_placement(
            &mut geo, frame_outer_width, frame_outer_height,
            &occupied, mat_cut_extent, style,
        );

        geo
    }

    /// Build geometry for the no-break (standard) path.
    ///
    /// Computes origin from native scale, optionally adds corner detail,
    /// computes mat cut extent, and returns the fully-populated geometry.
    /// No thumbnail is placed on the no-break path.
    fn build_no_break_geometry(
        design: &FrameDesign,
        native_scale: f64,
        canvas_width: f64,
        canvas_height: f64,
        use_corner_detail: bool,
        style: &DiagramStyle,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();
        let scale = native_scale;
        let scaled_width = frame_outer_width * scale;
        let scaled_height = frame_outer_height * scale;

        let min_offset = style.total_callout_reserve();
        let origin_x = ((canvas_width - scaled_width) / 2.0).max(min_offset);
        let origin_y = ((canvas_height - scaled_height) / 2.0).max(min_offset);

        let mut geo = Self::build_rects(design, scale, origin_x, origin_y);

        if use_corner_detail && design.frame_material_width > 0.0 {
            geo.corner_detail = Some(Self::compute_corner_detail(design, &geo, canvas_width, style));
        }

        let cd_occupied: Vec<Rect> = geo.corner_detail.as_ref().map(|cd| vec![cd.box_rect]).unwrap_or_default();
        let mat_cut_extent: Option<(Point, Point)> = if design.has_mat() {
            geo.mat_opening.as_ref().map(|mat_opening| {
                Self::choose_mat_cut_extent(
                    &geo.frame_inner,
                    &geo.content_area,
                    mat_opening,
                    &cd_occupied,
                    style,
                )
            })
        } else {
            None
        };

        geo.annotation_bounds = AnnotationBounds {
            corner_detail_box: geo.corner_detail.as_ref().map(|cd| cd.box_rect),
            thumbnail_box: None,
            thumbnail_label_position: ThumbnailLabelPosition::Below,
            mat_cut_width_label: None,
            mat_cut_height_label: None,
            mat_cut_extent,
        };

        geo
    }

    /// Compute thumbnail sizing, preferred position, and annotation bounds.
    ///
    /// Places a proportional silhouette thumbnail in the margin around the
    /// frame, avoiding collision with corner detail and mat cut labels.
    /// The collision pass in svg.rs handles fine adjustments (nudging away
    /// from arrow stubs, callout labels, etc.).
    fn compute_thumbnail_placement(
        geo: &mut Self,
        frame_outer_width: f64,
        frame_outer_height: f64,
        occupied: &[Rect],
        mat_cut_extent: Option<(Point, Point)>,
        style: &DiagramStyle,
    ) {
        let tm = style.thumbnail_metrics();
        let is_portrait = frame_outer_height >= frame_outer_width;
        let has_cd_and_mc = occupied.len() == 2;

        // Sizing: rotation-invariant when CD + MC both present (smaller thumb to fit gap),
        // standard orientation-aware sizing otherwise.
        let (thumb_w, thumb_h) = if has_cd_and_mc {
            let frame_long = frame_outer_width.max(frame_outer_height);
            let frame_short = frame_outer_width.min(frame_outer_height);
            let mini_max_h = style.two_line_label_bounds_height() * tm.scale_factor;
            let thumb_scale = (THUMBNAIL_MINI_MAX_WIDTH / frame_long).min(mini_max_h / frame_short);
            ((frame_outer_width * thumb_scale).max(tm.min_px),
             (frame_outer_height * thumb_scale).max(tm.min_px))
        } else {
            let (thumbnail_max_w, thumbnail_max_h) = if is_portrait {
                (tm.short_dim, tm.long_dim)
            } else {
                (tm.long_dim, tm.short_dim)
            };
            let scale_w = thumbnail_max_w / frame_outer_width;
            let scale_h = thumbnail_max_h / frame_outer_height;
            let thumb_scale = scale_w.min(scale_h);
            ((frame_outer_width * thumb_scale).max(tm.min_px),
             (frame_outer_height * thumb_scale).max(tm.min_px))
        };

        let label_below_h = tm.text_below_height;

        // Preferred position: one per orientation, with CD/MC gap awareness.
        let (thumb_x, thumb_y, thumb_label_pos) = if is_portrait {
            // Left of frame, vertically centered, label below
            let x = geo.frame_outer.left() - tm.gap - thumb_w;
            let centered_y = if has_cd_and_mc {
                geo.frame_outer.top() + (geo.frame_outer.height - (thumb_h + label_below_h)) / 2.0
            } else {
                geo.frame_outer.top() + (geo.frame_outer.height - thumb_h) / 2.0
            };
            // If centered position overlaps corner detail, shift above it
            if let Some(cd) = &geo.corner_detail {
                let full_bottom = centered_y + thumb_h + label_below_h;
                if full_bottom > cd.box_rect.top() - 6.0 {
                    let shifted_y = cd.box_rect.top() - 12.0 - label_below_h - thumb_h;
                    (x, shifted_y, ThumbnailLabelPosition::Below)
                } else {
                    (x, centered_y, ThumbnailLabelPosition::Below)
                }
            } else {
                (x, centered_y, ThumbnailLabelPosition::Below)
            }
        } else if has_cd_and_mc {
            // Landscape with CD + MC: center in gap between them
            let corner_right = occupied[0].right();
            let mat_cut_left = occupied[1].left();
            let mini_gap = 10.0;
            let avail = mat_cut_left - corner_right - 2.0 * mini_gap;
            let y = geo.frame_outer.bottom() + tm.gap;
            if avail >= thumb_w {
                let x = corner_right + mini_gap + (avail - thumb_w) / 2.0;
                (x, y, ThumbnailLabelPosition::Below)
            } else {
                let x = corner_right + mini_gap;
                (x, y, ThumbnailLabelPosition::Below)
            }
        } else {
            // Landscape: bottom-right of frame
            let x = geo.frame_outer.right() - thumb_w;
            let y = geo.frame_outer.bottom() + tm.gap;
            (x, y, ThumbnailLabelPosition::Right)
        };

        geo.thumbnail = Some(Rect::new(thumb_x, thumb_y, thumb_w, thumb_h));
        geo.thumbnail_below = thumb_y > geo.frame_outer.bottom();
        geo.thumbnail_label_position = thumb_label_pos;

        geo.annotation_bounds = AnnotationBounds {
            corner_detail_box: geo.corner_detail.as_ref().map(|cd| cd.box_rect),
            thumbnail_box: geo.thumbnail,
            thumbnail_label_position: thumb_label_pos,
            mat_cut_width_label: None,
            mat_cut_height_label: None,
            mat_cut_extent,
        };
    }

    /// Compute corner detail geometry for the inset overlay.
    /// Box size is proportional to the frame diagram so it stays visually balanced.
    /// Corner origin is at bottom-left of the box; L-shape extends RIGHT and UP.
    fn compute_corner_detail(design: &FrameDesign, geo: &Self, canvas_width: f64, style: &super::DiagramStyle) -> CornerDetailGeometry {
        // Size the box relative to canvas width — the viewBox includes callout
        // margins so frame_outer is much smaller than the visible canvas.
        // Target: box should be ~30% of canvas width for readable labels.
        // Also cap relative to rendered frame size so the box doesn't dominate a
        // small frame (e.g. PDF combined view where plan canvas height is limited).
        let target_w = canvas_width * CORNER_DETAIL_WIDTH_RATIO;
        // Use max (not min) so that extreme AR frames, where the short rendered dimension
        // is very small due to scale, don't shrink the corner detail to the minimum.
        // The canvas_width target already keeps the box proportional to the viewport;
        // frame_cap just prevents it from dominating an actually small canvas (PDF combined view).
        let frame_cap = geo.frame_outer.width.max(geo.frame_outer.height) * CORNER_DETAIL_FRAME_CAP;
        let box_w = (target_w.min(frame_cap)).clamp(CORNER_DETAIL_MIN_WIDTH, CORNER_DETAIL_MAX_WIDTH);

        let box_h = box_w / CORNER_DETAIL_ASPECT_RATIO;

        // X position: nominally extends 15% of box_w to the LEFT of frame_outer.left()
        // so the L-corner aligns with the frame corner.  When a mat is present, the mat
        // cut extension lines are vertical lines at mat_opening.right(); the corner detail
        // box must not overlap them.  Shift the box LEFTWARD (there is always left margin
        // space on the break path) until its right edge clears those lines.
        let margin = 3.0;
        let natural_box_x = geo.frame_outer.left() - box_w * CORNER_DETAIL_X_OVERHANG;
        // Basic clearance from mat opening extension lines (the post-layout
        // collision pass in svg.rs handles arrow stub clearance dynamically).
        let clearance = 4.0;
        let box_x = if let Some(mat_opening) = &geo.mat_opening {
            let natural_box_right = natural_box_x + box_w;
            let needed_box_right = mat_opening.right() - clearance;
            if natural_box_right > needed_box_right {
                let shifted_x = needed_box_right - box_w;
                shifted_x.max(style.margin)
            } else {
                natural_box_x
            }
        } else {
            natural_box_x
        };

        // Cap: box right edge should not extend past the frame's vertical centerline.
        // On wide landscape frames this is a no-op (box is much smaller than half the frame).
        // On narrow portrait frames this shifts the box left so it doesn't dominate the frame.
        let frame_center_x = geo.frame_outer.x + geo.frame_outer.width / 2.0;
        let center_pad = 6.0;
        let box_x = if box_x + box_w > frame_center_x - center_pad {
            (frame_center_x - center_pad - box_w).max(style.margin)
        } else {
            box_x
        };

        // Y position: box should overlap the bottom-left corner of the frame.
        // Standard formula: frame_outer.bottom() is always inside the box (0 < 0.85 < 1).
        // For axis-break frames, blend toward artwork center to push the box lower
        // (more bottom-anchored) when space allows. But clamp at standard_y so the box
        // never rises above the frame bottom — for tall portrait frames the blend would
        // otherwise place the box in the middle of the frame, not at the corner.
        let standard_y = geo.frame_outer.bottom() - box_h * CORNER_DETAIL_Y_OFFSET;
        let artwork_center_y = geo.artwork.y + geo.artwork.height / 2.0;
        let box_y = if geo.use_axis_break_x || geo.use_axis_break_y {
            let center_weight = CORNER_DETAIL_CENTER_WEIGHT;
            let anchor_y = artwork_center_y * center_weight + geo.frame_outer.bottom() * (1.0 - center_weight);
            (anchor_y + margin).max(standard_y)
        } else {
            standard_y
        };

        // Cap: box top should not extend above the frame's horizontal centerline.
        // On short landscape frames the box would otherwise dominate the frame height.
        // Shift box down (extending below frame) rather than shrinking it.
        let frame_center_y = geo.frame_outer.y + geo.frame_outer.height / 2.0;
        let center_pad_y = 6.0;
        let box_y = if box_y < frame_center_y + center_pad_y {
            frame_center_y + center_pad_y
        } else {
            box_y
        };

        // Detail scale: zoom out so frame band is ~21% of box width.
        // Smaller ratio = frame material drawn thinner = more room for labels,
        // and allows the box itself to be slightly smaller without clipping.
        let target_frame_band = box_w * CORNER_DETAIL_FRAME_BAND_RATIO;
        let detail_scale = target_frame_band / design.frame_material_width;

        // Corner origin X: must leave room for "Rabbet" label to the left.
        // The label chain is: text(end-anchored) ← 4px gap ← dim_line(cx-6) ← corner(cx).
        // So we need: cx - 10 - text_width("Rabbet", label_font) >= box_x + padding.
        let label_font = (box_h * CORNER_DETAIL_LABEL_FONT_RATIO).min(style.dimension_font_size * 0.75);
        let rabbet_text_w = estimate_text_width("Rabbet", label_font);
        let min_corner_x = box_x + 6.0 + rabbet_text_w + 10.0 + 4.0; // pad + text + dim_offset + gap
        let corner_x = min_corner_x.max(box_x + box_w * CORNER_DETAIL_CORNER_X_MIN);
        let corner_y = box_y + box_h * CORNER_DETAIL_CORNER_Y;

        CornerDetailGeometry {
            box_rect: Rect::new(box_x, box_y, box_w, box_h),
            corner_origin: Point::new(corner_x, corner_y),
            detail_scale,
        }
    }

    /// Choose extent points for the mat cut width dimension callout.
    ///
    /// Tries bottom-left first. If the estimated label bounding box overlaps any
    /// occupied annotation rect, falls back to bottom-right. This decouples mat cut
    /// placement from knowing specifically *what* occupies the bottom-left.
    ///
    /// Note: the SVG renderer pins extension line start to mat_opening.bottom()+3 regardless
    /// of extent_y, and dimension line is anchored to frame_outer.bottom().  Overlap with
    /// the corner detail box is handled purely by z-ordering in svg.rs (mat cut geometry
    /// renders before corner detail; labels render after).
    pub fn choose_mat_cut_extent(
        frame_inner: &Rect,
        content_area: &Rect,
        mat_opening: &Rect,
        occupied: &[Rect],
        style: &DiagramStyle,
    ) -> (Point, Point) {
        let frame_half_stroke = style.frame_stroke_width / 2.0;
        let mat_half_stroke = style.mat_stroke_width / 2.0;
        let extent_y = frame_inner.bottom() - frame_half_stroke;

        // Estimate label bounds at bottom-left position
        let mat_cut_offset = style.mat_cut_label_offset();
        let label_width = estimate_text_width("Mat Cut: 2 3/8\" (2\" visible)", style.label_font_size);
        let label_height = style.two_line_label_bounds_height();

        let bottom_left_label = Rect::new(
            content_area.left(),
            frame_inner.bottom() + mat_cut_offset - label_height / 2.0,
            label_width,
            label_height,
        );

        let use_right = occupied.iter().any(|occ| bottom_left_label.overlaps_with_margin(occ, 6.0));

        if use_right {
            // Bottom-right: from mat opening right edge to content area right edge
            (
                Point::new(mat_opening.right() - mat_half_stroke, extent_y),
                Point::new(content_area.right(), extent_y),
            )
        } else {
            // Bottom-left: from content area left edge to mat opening left edge
            (
                Point::new(content_area.left(), extent_y),
                Point::new(mat_opening.left() + mat_half_stroke, extent_y),
            )
        }
    }

    /// Estimate the bounding box of the mat cut width label given its extent points.
    /// Used to reserve space for thumbnail placement.
    fn mat_cut_label_bounds_from_extent(
        frame_outer: &Rect,
        extent_start: &Point,
        extent_end: &Point,
        style: &DiagramStyle,
    ) -> Rect {
        let mat_cut_offset = style.mat_cut_label_offset();
        // MatCutWidth is priority 2, typically level 0 on the bottom side.
        let dim_line_y = frame_outer.bottom() + style.dimension_offset_base;
        let label_width = estimate_text_width("Mat Cut: 2 3/8\" (2\" visible)", style.label_font_size);
        let label_height = style.two_line_label_bounds_height();
        // Label anchors at the leftmost x of the extent (start anchor in svg_dimension)
        let label_x = extent_start.x.min(extent_end.x);
        let label_center_y = dim_line_y + mat_cut_offset;
        Rect::new(label_x, label_center_y - label_height / 2.0, label_width, label_height)
    }

    /// Convert a dimension value (inches) to canvas units
    pub fn scale_dimension(&self, value: f64) -> f64 {
        value * self.scale
    }

    /// Calculate geometry for preview mode (no callouts)
    ///
    /// Scales to maximize use of available canvas space while maintaining
    /// correct proportions. Diagram size will change when mat is toggled
    /// (because the actual frame size changes), but ratios remain accurate.
    pub fn from_design_preview(
        design: &FrameDesign,
        canvas_width: f64,
        canvas_height: f64,
        style: &DiagramStyle,
    ) -> Self {
        let (frame_outer_height, frame_outer_width) = design.get_frame_outside_dimensions();

        // Minimal margins, no callout space needed
        let available_width = canvas_width - 2.0 * style.margin;
        let available_height = canvas_height - 2.0 * style.margin;

        let scale_x = available_width / frame_outer_width;
        let scale_y = available_height / frame_outer_height;
        let scale = scale_x.min(scale_y);

        let scaled_width = frame_outer_width * scale;
        let scaled_height = frame_outer_height * scale;
        let origin_x = (canvas_width - scaled_width) / 2.0;
        let origin_y = (canvas_height - scaled_height) / 2.0;

        Self::build_rects(design, scale, origin_x, origin_y)
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
        let max_label_text = material_labels.iter()
            .max_by_key(|s| s.len())
            .unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::test_helpers::test_design;

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

    #[test]
    fn test_plan_view_large_frame_both_breaks() {
        // 250"×375" artwork with 3/4" frame — both axes need breaks
        let mut design = FrameDesign::new(375.0, 250.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        assert!(geo.use_axis_break_x, "X axis break should be triggered for 375\" wide artwork");
        assert!(geo.use_axis_break_y, "Y axis break should be triggered for 250\" tall artwork");
        assert!(geo.break_x_start > 0.0);
        assert!(geo.break_x_end > geo.break_x_start);
        assert!(geo.break_y_start > 0.0);
        assert!(geo.break_y_end > geo.break_y_start);

        // Frame band should now be visible (>= 6px target)
        let frame_band_px = design.frame_material_width * geo.scale;
        assert!(frame_band_px >= 5.5, "Frame band should be visible: {:.1} px", frame_band_px);
    }

    #[test]
    fn test_plan_view_normal_frame_no_breaks() {
        // 8"×12" artwork with 3/4" frame — no breaks needed
        let mut design = FrameDesign::new(12.0, 8.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        assert!(!geo.use_axis_break_x, "No X break needed for normal frame");
        assert!(!geo.use_axis_break_y, "No Y break needed for normal frame");
    }

    #[test]
    fn test_plan_view_tall_only_y_break() {
        // 200" tall × 8" wide artwork — Y break needed, X may or may not
        let mut design = FrameDesign::new(200.0, 8.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        // Y break triggered because height is dominant and frame band would be subpixel
        assert!(geo.use_axis_break_y, "Y axis break should be triggered for 200\" tall artwork");
        assert!(geo.break_y_start > 0.0);
        assert!(geo.break_y_end > geo.break_y_start);
    }

    #[test]
    fn test_plan_view_aspect_ratio_preserved() {
        // 250"×375" artwork — both axes huge, both should break
        let mut design = FrameDesign::new(375.0, 250.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        assert!(geo.use_axis_break_x && geo.use_axis_break_y,
            "Both axis breaks should be triggered for 250\"×375\" artwork");

        // Uniform compression preserves aspect ratio: display should be taller than wide (3:2 H:W)
        let ratio = geo.frame_outer.height / geo.frame_outer.width;
        assert!(ratio > 1.2,
            "Height/width ratio should be > 1.2 (was {:.2}), preserving portrait aspect", ratio);
    }

    #[test]
    fn test_plan_view_single_axis_break() {
        // 100"w × 10"h extreme landscape — X axis needs break (~0.71%), Y doesn't (~4.9%)
        // (ratio threshold is 3%; outer dims: ~105" wide, ~15" tall)
        let mut design = FrameDesign::new(10.0, 100.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 2.0;
        design.mat_width_sides = 2.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        assert!(geo.use_axis_break_x,
            "X axis break should be triggered for 100\" width");
        assert!(!geo.use_axis_break_y,
            "Y axis break should NOT be triggered for 10\" height");
    }

    #[test]
    fn test_plan_view_break_gap_fixed_pixels() {
        // Verify the break gap is a fixed pixel size, not scale-dependent
        let mut design = FrameDesign::new(375.0, 250.0);
        design.frame_material_width = 0.75;
        design.mat_width_top_bottom = 0.0;
        design.mat_width_sides = 0.0;

        let style = DiagramStyle::default();
        let geo = PlanViewGeometry::from_design(&design, 800.0, 600.0, &style);

        if geo.use_axis_break_x {
            let gap = geo.break_x_end - geo.break_x_start;
            assert!((gap - 8.0).abs() < 0.1, "X break gap should be ~8px, was {:.1}", gap);
        }
        if geo.use_axis_break_y {
            let gap = geo.break_y_end - geo.break_y_start;
            assert!((gap - 8.0).abs() < 0.1, "Y break gap should be ~8px, was {:.1}", gap);
        }
    }

}
