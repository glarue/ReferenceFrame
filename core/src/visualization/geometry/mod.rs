// Geometry calculations for visualization
//
// Converts FrameDesign measurements into drawable coordinates,
// handling scaling to fit canvas with proper margins.

use crate::frame::FrameDesign;
use crate::conversions::{format_dimension, format_value, Unit};
use super::types::{AnnotationBounds, DetailMode, Point, Rect, ThumbnailLabelPosition};
use super::style::{DiagramStyle, LABEL_MASK_PADDING_X, THUMBNAIL_MINI_MAX_WIDTH};

mod plan;
mod section;

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
//
// The corner detail is a zoomed inset box overlaid on the bottom-left corner
// of the frame diagram. It shows rabbet, mat overlap, and frame band at a
// readable scale when the main diagram is too compressed (axis break mode).
//
// Spatial layout inside the box:
//   +-------------------------------+
//   |  "Rabbet" label    L-shape    |  <- CORNER_Y places the L-corner
//   |  (end-anchored)    corner     |     at 76% of box height from top
//   |                    |          |
//   |  <- CORNER_X_MIN ->          |  <- min 30% of box_w from left edge
//   +-------------------------------+     to leave room for rotated labels
//
// Box placement relative to the frame diagram:
//   - X: overhang 15% left of frame_outer so the L-corner aligns with the
//     actual frame corner; may shift further left to clear mat cut lines.
//   - Y: anchored near frame_outer.bottom() (85% overlap), with blend
//     toward artwork center on axis-break frames.
// ============================================================================

/// Corner detail box width as ratio of canvas width.
/// 30% keeps the box large enough for readable dimension labels.
const CORNER_DETAIL_WIDTH_RATIO: f64 = 0.30;

/// Cap corner detail relative to rendered frame size (max dimension).
/// Prevents the box from dominating a small canvas (e.g. PDF combined view).
const CORNER_DETAIL_FRAME_CAP: f64 = 0.80;

/// Minimum box width (px) — below this, labels become illegible.
/// Maximum box width (px) — above this, the box dominates the diagram.
const CORNER_DETAIL_MIN_WIDTH: f64 = 80.0;
const CORNER_DETAIL_MAX_WIDTH: f64 = 213.0;

/// Box height = width / this ratio. Slightly wider than tall (1.15:1)
/// to accommodate the horizontal "Rabbet" label and dimension lines.
const CORNER_DETAIL_ASPECT_RATIO: f64 = 1.15;

/// Frame band drawn at this fraction of box width. Controls zoom level:
/// smaller = more zoomed out = more room for labels inside the box.
const CORNER_DETAIL_FRAME_BAND_RATIO: f64 = 0.21;

/// Label font size as fraction of box height. Keeps labels proportional
/// to box size across different canvas dimensions.
const CORNER_DETAIL_LABEL_FONT_RATIO: f64 = 0.065;

/// Box X position: nominally extends this fraction of box_w left of frame_outer.
/// Aligns the L-corner in the box with the actual frame corner beneath it.
const CORNER_DETAIL_X_OVERHANG: f64 = 0.15;

/// Minimum corner origin X as fraction of box width from box left edge.
/// Reserves 30% of box width for the "Rabbet" label and dimension lines
/// that render to the left of the corner origin.
const CORNER_DETAIL_CORNER_X_MIN: f64 = 0.30;

/// Corner origin Y as fraction of box height from box top.
/// Places the L-corner in the lower quarter, leaving room above for
/// "Frame", "Mat overlap", and "Rabbet depth" dimension annotations.
const CORNER_DETAIL_CORNER_Y: f64 = 0.76;

/// Standard Y offset: box top sits at frame_outer.bottom() - box_h * 0.85.
/// This means the frame's bottom edge passes through the box at 85% from top,
/// visually anchoring the inset to the frame corner it magnifies.
const CORNER_DETAIL_Y_OFFSET: f64 = 0.85;

/// Axis-break Y blend weight toward artwork center. On axis-break frames
/// the frame bottom is higher than usual; blending 65% toward the artwork
/// center pushes the box lower for a more bottom-anchored appearance.
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

/// Estimate rendered text width for proportional sans-serif fonts (Inter, SF Pro, Arial).
///
/// Character width ratios are expressed as fractions of font_size, derived from
/// measuring these fonts at a 12px reference size. Each ratio represents
/// (advance width / font size) for that character class.
///
/// If the font stack changes in DiagramStyle, these ratios should be re-measured.
pub fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    // Width ratios as fraction of font_size, grouped by visual width class.
    // Named constants make it clear these are empirical measurements, not arbitrary.
    const SPACE: f64 = 0.28;
    const NARROW_PUNCT: f64 = 0.40;  // 1 i l . , : ; ! | '
    const NARROW_ALPHA: f64 = 0.45;  // I j t f r
    const BRACKET: f64 = 0.35;       // ( ) [ ]
    const WIDE: f64 = 0.90;          // m w M W
    const SLASH: f64 = 0.40;         // / (common in fraction display)
    const DIGIT: f64 = 0.60;         // 0-9 (tabular width in Inter/SF)
    const LOWER: f64 = 0.58;         // a-z (median lowercase)
    const UPPER: f64 = 0.72;         // A-Z (median uppercase)
    const QUOTE: f64 = 0.45;         // " (inch marks)
    const DEFAULT: f64 = 0.60;       // fallback for unclassified characters
    const SAFETY_MARGIN: f64 = 1.03; // 3% padding to ensure boxes are never too tight

    let mut width = 0.0;
    for c in text.chars() {
        let char_width_factor = match c {
            ' ' => SPACE,
            '1' | 'i' | 'l' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' => NARROW_PUNCT,
            'I' | 'j' | 't' | 'f' | 'r' => NARROW_ALPHA,
            '(' | ')' | '[' | ']' => BRACKET,
            'm' | 'w' | 'M' | 'W' => WIDE,
            '/' => SLASH,
            '0'..='9' => DIGIT,
            'a'..='z' => LOWER,
            'A'..='Z' => UPPER,
            '"' => QUOTE,
            _ => DEFAULT,
        };
        width += font_size * char_width_factor;
    }
    width * SAFETY_MARGIN
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
