//! Plan view SVG rendering.
//!
//! Contains the plan (top-down) view builder, corner detail inset,
//! viewBox computation, and dimension callout renderer.

use crate::frame::FrameDesign;
use crate::conversions::{format_dimension, Unit};
use super::types::{
    DiagramOptions, PositionedCallout,
    Rect, Side, TextAnchor, ThumbnailLabelPosition,
};
use super::style::{DiagramStyle, LABEL_MASK_PADDING_X};
use super::geometry::{CornerDetailGeometry, PlanViewGeometry, estimate_text_width};
use super::svg_util::*;
use super::layout::LayoutResult;

/// Artwork outline (dashed, inside the mat opening).
const ARTWORK_OUTLINE_DASH: &str = "4,2";
const ARTWORK_OUTLINE_OPACITY: f64 = 0.6;
const MAT_OVERLAP_DASH: &str = "3,2";
const MAT_OVERLAP_OPACITY: f64 = 0.4;

/// Stroke spec for one plan-view rectangle outline.
struct RectStroke<'a> {
    rect: &'a Rect,
    color: &'a str,
    width: f64,
    dasharray: Option<&'a str>,
    opacity: f64,
}

/// Every rect outline the plan view draws, defined once. The no-break path
/// renders these fields in place (interleaved with the overlap fills); the
/// axis-break path draws them all up front via `in_break_order`, before the
/// zigzag ribbons mask the break zones.
struct PlanRectStrokes<'a> {
    frame_outer: RectStroke<'a>,
    frame_inner: RectStroke<'a>,
    content_boundary: RectStroke<'a>,
    /// Present when there is a mat.
    mat_opening: Option<RectStroke<'a>>,
    /// Present when there is a mat.
    artwork: Option<RectStroke<'a>>,
    /// Present when the lip over the art renders wider than 0.5px.
    /// NOTE: outlines the same rect as `content_boundary` (double stroke; audit 5c).
    rabbet_overlap: Option<RectStroke<'a>>,
    /// Present when the mat overlaps the art by more than 0.5px.
    mat_overlap: Option<RectStroke<'a>>,
}

impl<'a> PlanRectStrokes<'a> {
    fn new(
        design: &FrameDesign,
        geometry: &'a PlanViewGeometry,
        style: &'a DiagramStyle,
        content_edge_color: &'a str,
        rabbet_scaled: f64,
    ) -> Self {
        let solid = |rect: &'a Rect, width: f64| RectStroke {
            rect, color: &style.line_color, width, dasharray: None, opacity: 1.0,
        };
        let mat_overlap_visible = geometry.mat_opening.is_some()
            && design.mat_overlap * geometry.scale > 0.5
            && design.has_mat();
        Self {
            frame_outer: solid(&geometry.frame_outer, style.frame_stroke_width),
            frame_inner: solid(&geometry.frame_inner, style.frame_stroke_width),
            content_boundary: RectStroke {
                rect: &geometry.content_area, color: content_edge_color,
                width: style.extension_stroke_width, dasharray: Some(DASH_BOUNDARY),
                opacity: OPACITY_CONTENT_BOUNDARY,
            },
            mat_opening: geometry.mat_opening.as_ref()
                .map(|mo| solid(mo, style.mat_stroke_width)),
            artwork: geometry.mat_opening.as_ref().map(|_| RectStroke {
                rect: &geometry.artwork, color: &style.artwork_color,
                width: style.extension_stroke_width, dasharray: Some(ARTWORK_OUTLINE_DASH),
                opacity: ARTWORK_OUTLINE_OPACITY,
            }),
            rabbet_overlap: (rabbet_scaled > 0.5).then_some(RectStroke {
                rect: &geometry.content_area, color: content_edge_color,
                width: style.extension_stroke_width * 0.8, dasharray: Some(DASH_ASSEMBLY_MARGIN),
                opacity: OPACITY_CONTENT_BOUNDARY,
            }),
            mat_overlap: mat_overlap_visible.then_some(RectStroke {
                rect: &geometry.artwork, color: &style.mat_overlap_color,
                width: style.extension_stroke_width * 0.8, dasharray: Some(MAT_OVERLAP_DASH),
                opacity: MAT_OVERLAP_OPACITY,
            }),
        }
    }

    /// Draw order used by the axis-break path.
    fn in_break_order(&self) -> Vec<&RectStroke<'a>> {
        let mut v = vec![&self.frame_outer, &self.frame_inner, &self.content_boundary];
        v.extend(self.mat_opening.iter());
        v.extend(self.artwork.iter());
        v.extend(self.rabbet_overlap.iter());
        v.extend(self.mat_overlap.iter());
        v
    }
}

/// Render the corner detail inset overlay for plan view.
/// Shows a zoomed bottom-left corner with frame outer, frame inner,
/// content area (matboard/artwork edge), and rabbet overlap zone.
/// Layout matches the HTML mockup: corner origin near bottom-left,
/// L-shape extends RIGHT and UP.
pub(crate) fn render_corner_detail(
    design: &FrameDesign,
    cd: &CornerDetailGeometry,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    let mut svg = String::new();
    let s = cd.detail_scale;
    // Corner origin (outside corner of frame) — near bottom-left of box
    let cx = cd.corner_origin.x;
    let cy = cd.corner_origin.y;
    let bx = cd.box_rect.x;
    let by = cd.box_rect.y;
    let bw = cd.box_rect.width;
    let bh = cd.box_rect.height;

    let frame_w = design.frame_material_width * s;
    let rabbet_w = design.rabbet_width * s;
    // Content area inset from outer edge (frame_w - rabbet_w from outer)
    let content_inset = frame_w - rabbet_w;

    // L-shape extends RIGHT from cx, and UP from cy
    let arm_right = bx + bw - cx - 16.0; // 16px: keeps matboard/artwork label inside box edge
    let arm_up = cy - by - 30.0;        // 30px: reserves space for "Corner Detail" title + breathing room

    svg.push_str("  <g id=\"corner-detail\">\n");

    // Clip path for zoomed content (inset from box edges for breathing room).
    // Left/right/bottom use a small 4px margin to keep strokes from touching the box border.
    // Top uses a larger 24px inset to avoid clipping the "Corner Detail" title text that
    // renders above the zoomed geometry but inside the box.
    let clip_inset_left = 4.0;
    let clip_inset_top = 24.0;
    let clip_inset_right = 4.0;
    let clip_inset_bottom = 4.0;
    let clip_id = "corner-detail-clip";
    svg.push_str(&format!(
        "    <defs><clipPath id=\"{}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath></defs>\n",
        clip_id, bx + clip_inset_left, by + clip_inset_top, bw - clip_inset_left - clip_inset_right, bh - clip_inset_top - clip_inset_bottom
    ));

    // Background box + title (title scales with box height)
    let title_font = (bh * 0.08).min(style.dimension_font_size * 0.9);
    let title_y = by + title_font + 5.0;
    render_inset_box(&mut svg, &cd.box_rect, "Corner Detail", title_font, title_y, style);

    // Clipped group for zoomed geometry
    svg.push_str(&format!("    <g clip-path=\"url(#{})\">\n", clip_id));

    // Content area / matboard edge color
    let content_color = if design.has_mat() {
        &style.artwork_dimension_color  // Carrot Orange for matboard
    } else {
        &style.artwork_color            // Willow Green for artwork
    };

    // Rabbet overlap zone: L-shaped fill using a single path (no overlap doubling)
    let ci_x = cx + content_inset;  // content inset x
    let ci_y = cy - content_inset;  // content inset y
    let fi_x = cx + frame_w;       // frame inner x
    let fi_y = cy - frame_w;       // frame inner y
    let top_y = cy - arm_up;       // top of visible area
    let right_x = cx + arm_right;  // right of visible area
    // L-shaped rabbet overlap fill — traces the region between the content edge
    // (matboard/artwork) and the frame inner edge, which is the rabbet overlap zone.
    //
    // ASCII diagram (corner at bottom-left, looking from outside):
    //
    //     ci_x  fi_x
    //       |    |
    //  top ─┌────┐
    //       │    │           ← vertical strip (content_inset to frame_inner)
    //  ci_y ├────┴───────┐  ← horizontal strip continues rightward
    //  fi_y │            │
    //       └────────────┘
    //                 right_x
    //
    // Path walks clockwise: start top-left → down → right → up → left → up → close.
    svg.push_str(&format!(
        "    <path d=\"M{:.2},{:.2} V{:.2} H{:.2} V{:.2} H{:.2} V{:.2} H{:.2} Z\" fill=\"{}\" fill-opacity=\"0.10\" stroke=\"none\"/>\n",
        ci_x, top_y,     // top-left of vertical strip
        ci_y,            // down to content corner Y
        right_x,         // right along content line to right edge
        fi_y,            // up to frame inner Y
        fi_x,            // left to frame inner X
        top_y,           // up to top
        ci_x,            // back to start X (close)
        content_color
    ));

    // Content area / matboard edge: dashed line
    // Horizontal (goes right from content_inset above outer line)
    svg.push_str(&format!(
        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6,3\" opacity=\"0.7\"/>\n",
        ci_x, ci_y, cx + arm_right, ci_y, content_color
    ));
    // Vertical (goes up from content_inset right of outer line)
    svg.push_str(&format!(
        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6,3\" opacity=\"0.7\"/>\n",
        ci_x, ci_y, ci_x, top_y, content_color
    ));

    // Frame outer L-shape (thick dark) — polyline for clean corner join
    let outer_sw = style.frame_stroke_width;
    svg.push_str(&format!(
        "    <polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linejoin=\"miter\"/>\n",
        cx, top_y, cx, cy, cx + arm_right, cy, style.line_color, outer_sw
    ));

    // Frame inner L-shape (thick dark) — polyline for clean corner join
    svg.push_str(&format!(
        "    <polyline points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linejoin=\"miter\"/>\n",
        fi_x, top_y, fi_x, fi_y, cx + arm_right, fi_y, style.line_color, outer_sw
    ));

    // Spline slot chord across the zoomed corner (edge-to-edge; the clip
    // trims any excess). Same slot the plan-view chords show.
    if options.show_spline {
        if let Some(leg) = super::overlays::plan_spline_leg(design, options.spline_params.unwrap_or_default(), s) {
            svg.push_str(&format!(
                "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"2\" stroke-dasharray=\"5,3\"/>\n",
                cx + leg, cy, cx, cy - leg, style.accent_color
            ));
        }
    }

    svg.push_str("    </g>\n"); // end clipped group (geometry only)

    // ============ DIMENSION CALLOUTS (outside clip, inside box clip) ============
    // Annotations are clipped to the full box rect so labels near edges stay visible
    // but don't escape the detail box boundary.
    let annot_clip_id = "corner-detail-annot-clip";
    let annot_pad = 2.0;
    svg.push_str(&format!(
        "    <defs><clipPath id=\"{}\"><rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"/></clipPath></defs>\n",
        annot_clip_id, bx + annot_pad, by + annot_pad, bw - 2.0 * annot_pad, bh - 2.0 * annot_pad
    ));
    svg.push_str(&format!("    <g clip-path=\"url(#{})\">\n", annot_clip_id));

    let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };
    let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);
    let label_font = (bh * 0.065).min(style.dimension_font_size * 0.75);

    // 1. Frame width: horizontal dimension between outer and inner, below the corner
    let frame_label = format!("Frame: {}", fmt(design.frame_material_width));
    let fw_dim_y = cy + (frame_w * 0.45).clamp(7.0, 16.0); // dimension line position below corner
    let fw_arrow = DimensionArrow::new(cx, cx + frame_w, fw_dim_y, true)
        .color(&style.outside_dimension_color)
        .extension(cy, 2.0)
        .stroke(0.75, 0.5)
        .label(&frame_label, &style.font_family, label_font)
        .label_offset(label_font + 2.0);
    svg.push_str(&fw_arrow.render());

    // 2. Rabbet: vertical dimension between content area and inner, left side
    let rb_dim_x = cx - (frame_w * 0.22).clamp(3.0, 9.0); // dimension line position left of corner
    let rb_arrow = DimensionArrow::new(ci_y, fi_y, rb_dim_x, false)
        .color(&style.inside_dimension_color)
        .extension(ci_x, -2.0) // extension lines go leftward from geometry
        .stroke(0.75, 0.5);
    svg.push_str(&rb_arrow.render());
    // Rabbet label: "Rabbet" + value, right-aligned just left of dimension line
    let rabbet_mid_y = (ci_y + fi_y) / 2.0;
    let rb_label_x = rb_dim_x - 4.0;
    svg.push_str(&format!(
        "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" text-anchor=\"end\" font-weight=\"bold\">Rabbet</text>\n",
        rb_label_x, rabbet_mid_y - 1.0,
        style.inside_dimension_color, style.font_family, label_font
    ));
    svg.push_str(&format!(
        "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" text-anchor=\"end\" font-weight=\"bold\">{}</text>\n",
        rb_label_x, rabbet_mid_y + label_font,
        style.inside_dimension_color, style.font_family, label_font,
        html_escape(&fmt(design.rabbet_width))
    ));

    // 3. Content label ("matboard" or "artwork") — centered inside the content interior.
    // The interior is the open area bounded by ci_x (left), right_x (frame arm end),
    // top_y (top of clip), and ci_y (horizontal content dashed line).
    // Centering with text-anchor="middle" keeps the label away from both frame lines.
    let content_label = if design.has_mat() { "matboard" } else { "artwork" };
    let cl_font = label_font * 0.9;
    let cl_text_w = estimate_text_width(content_label, cl_font);
    let cl_bg_pad = 2.0;

    // Horizontal center of the matboard interior, clamped to keep text inside
    let interior_left = ci_x + 1.0;
    let interior_right = right_x - 1.0;
    let label_center_x = ((interior_left + interior_right) / 2.0)
        .max(interior_left + cl_text_w / 2.0 + cl_bg_pad)
        .min(interior_right - cl_text_w / 2.0 - cl_bg_pad);

    // Vertical: upper-third of the interior, below title area
    let label_y_raw = top_y + (ci_y - top_y) * 0.30;
    let label_y = label_y_raw
        .max(by + title_font + cl_font + 6.0)
        .min(ci_y - 6.0);

    // Leader: straight vertical from horizontal dashed line up to label
    svg.push_str(&format!(
        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"0.5\" opacity=\"0.5\"/>\n",
        label_center_x, ci_y, label_center_x, label_y + cl_font * 0.15,
        content_color
    ));
    // Background-colored rect centered on the label text
    let cl_bg_h = cl_font * 1.2;
    let cl_bg_x = label_center_x - cl_text_w / 2.0 - cl_bg_pad;
    let cl_bg_y = label_y - cl_font * 0.75 - cl_bg_pad / 2.0;
    svg.push_str(&format!(
        "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\" stroke=\"none\" rx=\"1\"/>\n",
        cl_bg_x, cl_bg_y, cl_text_w + cl_bg_pad * 2.0, cl_bg_h, style.background_color
    ));
    svg.push_str(&format!(
        "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}\" text-anchor=\"middle\" font-weight=\"bold\" opacity=\"0.8\">{}</text>\n",
        label_center_x, label_y,
        content_color, style.font_family, cl_font,
        content_label
    ));

    svg.push_str("    </g>\n"); // end annotation clip group

    svg.push_str("  </g>\n");
    svg
}

/// Compute the viewBox for a plan view from post-collision element positions.
///
/// Uses the final positioned callout label_bounds (exact drawn label extents,
/// already collision-resolved), extension line endpoints, and floating
/// annotations (thumbnail, corner detail) to determine tight content bounds.
pub(crate) fn compute_plan_viewbox(
    geometry: &PlanViewGeometry,
    layout: &LayoutResult,
    style: &DiagramStyle,
) -> (f64, f64, f64, f64) {
    // Start from frame outer bounds (with stroke)
    let mut min_x = geometry.frame_outer.left() - style.frame_stroke_width / 2.0;
    let mut max_x = geometry.frame_outer.right() + style.frame_stroke_width / 2.0;
    let mut min_y = geometry.frame_outer.top() - style.frame_stroke_width / 2.0;
    let mut max_y = geometry.frame_outer.bottom() + style.frame_stroke_width / 2.0;

    // Callouts: label_bounds is the exact drawn extent of the label text and
    // mask (computed once by layout::place_label, then collision-shifted), so no
    // per-side padding is needed here.
    for callout in &layout.positioned_callouts {
        let lb = callout.label_bounds;
        min_x = min_x.min(lb.left());
        max_x = max_x.max(lb.right());
        min_y = min_y.min(lb.top());
        max_y = max_y.max(lb.bottom());

        // Extension line endpoints: extend along extent span and overshoot
        // perpendicular to the extent direction.
        let es = &callout.callout.extent_start;
        let ee = &callout.callout.extent_end;
        let is_horiz = (es.y - ee.y).abs() < 1.0;
        if is_horiz {
            min_x = min_x.min(es.x.min(ee.x));
            max_x = max_x.max(es.x.max(ee.x));
        } else {
            min_y = min_y.min(es.y.min(ee.y));
            max_y = max_y.max(es.y.max(ee.y));
        }
    }

    // Thumbnail with its label area
    let ann = &geometry.annotation_bounds;
    if let Some(thumb) = &geometry.thumbnail {
        let tm = style.thumbnail_metrics();
        min_x = min_x.min(thumb.left());
        min_y = min_y.min(thumb.top());
        match ann.thumbnail_label_position {
            ThumbnailLabelPosition::Right => {
                let label_w = estimate_text_width("proportions", tm.font_size);
                max_x = max_x.max(thumb.right() + tm.label_gap + label_w);
                max_y = max_y.max(thumb.bottom());
            }
            ThumbnailLabelPosition::Below => {
                max_x = max_x.max(thumb.right());
                max_y = max_y.max(thumb.bottom() + tm.text_below_height);
            }
        }
    }

    // Corner detail box
    if let Some(cd_box) = &ann.corner_detail_box {
        min_x = min_x.min(cd_box.left());
        max_x = max_x.max(cd_box.right());
        min_y = min_y.min(cd_box.top());
        max_y = max_y.max(cd_box.bottom());
    }

    let padding = style.margin;
    (min_x - padding, min_y - padding, max_x - min_x + 2.0 * padding, max_y - min_y + 2.0 * padding)
}

/// Build SVG string for plan view
pub(crate) fn build_plan_svg(
    design: &FrameDesign,
    geometry: &PlanViewGeometry,
    _callouts: &[super::types::DimensionCallout],
    layout: &LayoutResult,
    options: &DiagramOptions,
    style: &DiagramStyle,
) -> String {
    // Calculate viewBox dimensions
    let (min_x, min_y, mut viewbox_width, mut viewbox_height) = if options.show_callouts {
        compute_plan_viewbox(geometry, layout, style)
    } else {
        // Without callouts (preview mode): use fixed viewBox matching canvas dimensions
        (0.0, 0.0, options.canvas_width, options.canvas_height)
    };

    // Overlay annotation card (spline/hanging measurements) sits outside all
    // callouts — right gutter or below the content (see card_pos) — so the
    // viewBox is widened/extended to make room for it. Preview mode (no
    // callouts) draws neither the card nor the overlay marks.
    const CARD_GAP: f64 = 14.0;
    let overlay_card = if options.show_callouts {
        let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };
        let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);
        super::overlays::plan_overlay_card(geometry, design, style, options, &fmt)
    } else {
        None
    };
    let card_pos = overlay_card.as_ref().map(|card| {
        // Phone-width or portrait canvases place the card BELOW the content so
        // it never steals width from the diagram on width-fit displays. The
        // width check matters for the combined view, whose plan panel gets a
        // zone-split height that can read as landscape even on a phone.
        let card_below =
            options.canvas_width < 500.0 || options.canvas_width < options.canvas_height;
        if !card_below {
            // Wide canvas (desktop / combined panels): right gutter
            let cx = min_x + viewbox_width + CARD_GAP;
            let cy = geometry.frame_outer.y;
            viewbox_width += card.width + 2.0 * CARD_GAP;
            viewbox_height = viewbox_height.max(cy - min_y + card.height + CARD_GAP);
            (cx, cy)
        } else {
            let cx = geometry.frame_outer.x;
            let cy = min_y + viewbox_height + CARD_GAP;
            viewbox_height += card.height + 2.0 * CARD_GAP;
            viewbox_width = viewbox_width.max(cx - min_x + card.width + CARD_GAP);
            (cx, cy)
        }
    });

    // Build SVG with dynamic viewBox
    let mut svg = String::new();

    // SVG header with calculated viewBox
    svg.push_str(&format!(
        r#"<svg viewBox="{:.2} {:.2} {:.2} {:.2}" xmlns="http://www.w3.org/2000/svg">"#,
        min_x, min_y, viewbox_width, viewbox_height
    ));
    svg.push('\n');

    // Defs for patterns
    svg.push_str(&generate_defs(style));

    let has_breaks = geometry.use_axis_break_x || geometry.use_axis_break_y;

    // Conditional color: what sits in the rabbet determines the content edge color
    let content_edge_color = if design.has_mat() {
        &style.artwork_dimension_color  // Carrot Orange #f8961e (matboard edge)
    } else {
        &style.artwork_color            // Willow Green #90be6d (artwork edge)
    };

    // Rabbet overlap uses lip_over_art (zero for sight-size/float) so its fill and
    // outline vanish when the frame has no lip over the artwork.
    let rabbet_scaled = design.lip_over_art() * geometry.scale;
    // Single source for every plan-view rect stroke (color/width/dash/opacity),
    // shared by the no-break path below and the axis-break path further down.
    // The two paths still serialize with their historical attribute formats.
    let strokes = PlanRectStrokes::new(design, geometry, style, content_edge_color, rabbet_scaled);

    // When breaks are NOT active, draw full rect strokes as before.
    // When breaks ARE active, skip rect strokes here — they'll be drawn as
    // corner segments after the zigzag ribbons mask the fills.
    if !has_breaks {
        // Geometry group — full rect strokes (no breaks)
        svg.push_str("  <g id=\"geometry\">\n");
        for rs in [&strokes.frame_outer, &strokes.frame_inner] {
            svg.push_str(&svg_rect(rs.rect, rs.color, rs.width, None));
        }
        if let (Some(mo), Some(art)) = (&strokes.mat_opening, &strokes.artwork) {
            svg.push_str(&svg_rect(mo.rect, mo.color, mo.width, None));
            svg.push_str(&format!(
                "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"{}\" fill=\"none\" opacity=\"{}\"/>\n",
                art.rect.x, art.rect.y,
                art.rect.width, art.rect.height,
                art.color, art.width, art.dasharray.unwrap_or_default(), art.opacity
            ));
        }
        svg.push_str("  </g>\n");
    }

    // Frame/mat overlap visualization - semi-transparent fill showing rabbet overlap area.
    if rabbet_scaled > 0.5 {
        svg.push_str("  <g id=\"rabbet-overlap\">\n");
        let ox = geometry.content_area.x;
        let oy = geometry.content_area.y;
        let ow = geometry.content_area.width;
        let oh = geometry.content_area.height;
        let ix = geometry.frame_inner.x;
        let iy = geometry.frame_inner.y;
        let iw = geometry.frame_inner.width;
        let ih = geometry.frame_inner.height;
        let path_d = format!(
            "M{:.2},{:.2} h{:.2} v{:.2} h{:.2} Z M{:.2},{:.2} v{:.2} h{:.2} v{:.2} Z",
            ox, oy, ow, oh, -ow,
            ix, iy, ih, iw, -ih
        );
        svg.push_str(&format!(
            "    <path d=\"{}\" fill=\"{}\" fill-opacity=\"0.15\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
            path_d, content_edge_color
        ));
        if let (false, Some(rs)) = (has_breaks, &strokes.rabbet_overlap) {
            svg.push_str(&format!(
                "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"{}\" stroke-opacity=\"{}\"/>\n",
                rs.rect.x, rs.rect.y, rs.rect.width, rs.rect.height, rs.color, rs.width, rs.dasharray.unwrap_or_default(), rs.opacity
            ));
        }
        svg.push_str("  </g>\n");
    }

    // Content/matboard boundary
    if !has_breaks {
        let rs = &strokes.content_boundary;
        svg.push_str("  <g id=\"content-boundary\">\n");
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"{}\" fill=\"none\" opacity=\"{}\"/>\n",
            rs.rect.x, rs.rect.y,
            rs.rect.width, rs.rect.height,
            rs.color, rs.width, rs.dasharray.unwrap_or_default(), rs.opacity
        ));
        svg.push_str("  </g>\n");
    }

    // Mat/artwork overlap visualization
    if let Some(mat_opening) = &geometry.mat_opening {
        let mat_overlap_scaled = design.mat_overlap * geometry.scale;
        if mat_overlap_scaled > 0.5 && design.has_mat() {
            svg.push_str("  <g id=\"mat-overlap\">\n");
            let ox = geometry.artwork.x;
            let oy = geometry.artwork.y;
            let ow = geometry.artwork.width;
            let oh = geometry.artwork.height;
            let ix = mat_opening.x;
            let iy = mat_opening.y;
            let iw = mat_opening.width;
            let ih = mat_opening.height;
            let path_d = format!(
                "M{:.2},{:.2} h{:.2} v{:.2} h{:.2} Z M{:.2},{:.2} v{:.2} h{:.2} v{:.2} Z",
                ox, oy, ow, oh, -ow,
                ix, iy, ih, iw, -ih
            );
            svg.push_str(&format!(
                "    <path d=\"{}\" fill=\"{}\" fill-opacity=\"0.12\" fill-rule=\"evenodd\" stroke=\"none\"/>\n",
                path_d, style.mat_overlap_color
            ));
            if let (false, Some(rs)) = (has_breaks, &strokes.mat_overlap) {
                svg.push_str(&format!(
                    "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-dasharray=\"{}\" stroke-opacity=\"{}\"/>\n",
                    rs.rect.x, rs.rect.y, rs.rect.width, rs.rect.height, rs.color, rs.width, rs.dasharray.unwrap_or_default(), rs.opacity
                ));
            }
            svg.push_str("  </g>\n");
        }
    }

    // Plan view axis break: zigzag ribbon masking + corner strokes
    if has_breaks {
        let break_line_width = style.frame_stroke_width * 0.5;
        svg.push_str("  <g id=\"plan-axis-breaks\">\n");

        // Compute zigzag control points for each active break
        let x_zigzags = if geometry.use_axis_break_x {
            let left_zz = vertical_zigzag(geometry.break_x_start, geometry.frame_outer.top(), geometry.frame_outer.height);
            let right_zz = vertical_zigzag(geometry.break_x_end, geometry.frame_outer.top(), geometry.frame_outer.height);
            Some((left_zz, right_zz))
        } else {
            None
        };

        let y_zigzags = if geometry.use_axis_break_y {
            let top_zz = horizontal_zigzag(geometry.break_y_start, geometry.frame_outer.left(), geometry.frame_outer.width);
            let bot_zz = horizontal_zigzag(geometry.break_y_end, geometry.frame_outer.left(), geometry.frame_outer.width);
            Some((top_zz, bot_zz))
        } else {
            None
        };

        // STEP 1: Full rect strokes (ribbon masks in step 2 will clip break zones)
        // Drawing full <rect> elements gives clean mitered corners without join artifacts.
        for rs in strokes.in_break_order() {
            let dash_attr = if let Some(da) = rs.dasharray {
                format!(r#" stroke-dasharray="{}""#, da)
            } else {
                String::new()
            };
            let opacity_attr = if (rs.opacity - 1.0).abs() > 0.001 {
                format!(r#" opacity="{:.2}""#, rs.opacity)
            } else {
                String::new()
            };
            svg.push_str(&format!(
                r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="{}"{}{}/>"#,
                rs.rect.x, rs.rect.y, rs.rect.width, rs.rect.height,
                rs.color, rs.width, dash_attr, opacity_attr
            ));
            svg.push('\n');
        }

        // STEP 2: Zigzag ribbon masks (background-filled closed paths that hide break zones)
        if let Some((ref left_zz, ref right_zz)) = x_zigzags {
            let ribbon = format!(
                "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
                left_zz.p0.0, left_zz.p0.1, left_zz.p1.0, left_zz.p1.1,
                left_zz.p2.0, left_zz.p2.1, left_zz.p3.0, left_zz.p3.1,
                right_zz.p3.0, right_zz.p3.1, right_zz.p2.0, right_zz.p2.1,
                right_zz.p1.0, right_zz.p1.1, right_zz.p0.0, right_zz.p0.1,
            );
            svg.push_str(&format!(
                r#"    <path d="{}" fill="{}" stroke="none"/>"#,
                ribbon, style.background_color
            ));
            svg.push('\n');
        }

        if let Some((ref top_zz, ref bot_zz)) = y_zigzags {
            let ribbon = format!(
                "M{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} L{:.2},{:.2} Z",
                top_zz.p0.0, top_zz.p0.1, top_zz.p1.0, top_zz.p1.1,
                top_zz.p2.0, top_zz.p2.1, top_zz.p3.0, top_zz.p3.1,
                bot_zz.p3.0, bot_zz.p3.1, bot_zz.p2.0, bot_zz.p2.1,
                bot_zz.p1.0, bot_zz.p1.1, bot_zz.p0.0, bot_zz.p0.1,
            );
            svg.push_str(&format!(
                r#"    <path d="{}" fill="{}" stroke="none"/>"#,
                ribbon, style.background_color
            ));
            svg.push('\n');
        }

        // STEP 3: Zigzag indicator lines (dashed, reduced opacity so artwork indicators stay legible)
        let zz_opacity = 0.34;
        if let Some((ref left_zz, ref right_zz)) = x_zigzags {
            render_zigzag_line_with_opacity(&mut svg, left_zz, &style.line_color, break_line_width, zz_opacity);
            render_zigzag_line_with_opacity(&mut svg, right_zz, &style.line_color, break_line_width, zz_opacity);
        }
        if let Some((ref top_zz, ref bot_zz)) = y_zigzags {
            render_zigzag_line_with_opacity(&mut svg, top_zz, &style.line_color, break_line_width, zz_opacity);
            render_zigzag_line_with_opacity(&mut svg, bot_zz, &style.line_color, break_line_width, zz_opacity);
        }

        svg.push_str("  </g>\n");
    }

    // Mat cut callouts are collected here so they can be rendered AFTER the corner
    // detail box (declared outside show_callouts block for scope reasons).
    let mut mat_cut_geom = String::new();
    let mut mat_cut_labels = String::new();

    // Dimensions group (only if callouts are enabled)
    if options.show_callouts {
        // Two-pass rendering: draw lines/masks first, then labels on top.
        // This prevents an outer level's mask from covering an inner level's label.
        //
        // Mat cut callouts are collected separately and rendered AFTER the corner detail
        // box (see below). For extreme-AR frames on small screens the corner detail box can
        // geometrically overlap the mat cut extension lines — rendering mat cut on top
        // ensures those measurements are always visible.
        let is_mat_cut_callout = |c: &PositionedCallout| matches!(
            c.callout.dimension_type,
            super::types::DimensionType::MatCutWidth | super::types::DimensionType::MatCutHeight
        );
        // Overlay marks render below dimension callouts and the corner-detail
        // inset, so callout masks and the inset cleanly cover them; their
        // measurements live in the overlay card, not over the diagram.
        if options.show_hanging {
            super::overlays::render_plan_hanging(&mut svg, geometry, design, style, options.hanging_params.unwrap_or_default());
        }
        if options.show_spline {
            super::overlays::render_plan_splines(&mut svg, geometry, design, style, options.spline_params.unwrap_or_default());
        }

        let mut dimension_labels = String::new();
        svg.push_str("  <g id=\"dimensions\">\n");
        for callout in &layout.positioned_callouts {
            let (geom_svg, label_svg) = svg_dimension(callout, style, geometry);
            if is_mat_cut_callout(callout) {
                mat_cut_geom.push_str(&geom_svg);
                mat_cut_labels.push_str(&label_svg);
            } else {
                svg.push_str(&geom_svg);
                dimension_labels.push_str(&label_svg);
            }
        }
        svg.push_str(&dimension_labels);
        svg.push_str("  </g>\n");
    }

    // Artwork dimensions indicator - arrows extending to artwork boundary
    // Only show if callouts are enabled
    if options.show_callouts {
        // The artwork boundary is shown as a dashed line with stroke width extension_stroke_width * 0.8
        // We want arrow tips to land exactly at the INNER edge of that dashed stroke
        let artwork_center = geometry.artwork.center();
        let artwork_center_y = artwork_center.y;
        let unit = if options.unit_mm { Unit::Millimeters } else { Unit::Inches };

        // Always use artwork color — these arrows indicate artwork size
        let artwork_indicator_color = &style.artwork_color;

        svg.push_str(&format!(
            r#"  <g id="artwork-indicator">"#
        ));
        svg.push('\n');

        // Calculate arrow stroke width (used for marker scaling)
        let arrow_stroke_width = style.dimension_stroke_width * 0.7;

        // The artwork boundary dashed line has this stroke width
        let artwork_boundary_stroke = style.extension_stroke_width * 0.8;

        // Arrow tips should land at the inner edge of the artwork boundary stroke
        // Inner edge = geometric boundary + half the boundary stroke width
        let target_left = geometry.artwork.left() + artwork_boundary_stroke / 2.0;
        let target_right = geometry.artwork.right() - artwork_boundary_stroke / 2.0;
        let target_top = geometry.artwork.top() + artwork_boundary_stroke / 2.0;
        let target_bottom = geometry.artwork.bottom() - artwork_boundary_stroke / 2.0;

        // Calculate line endpoints so arrow tips land at targets
        let h_line_x1 = arrow_line_endpoint_for_target(target_left, arrow_stroke_width, true);
        let h_line_x2 = arrow_line_endpoint_for_target(target_right, arrow_stroke_width, false);
        let v_line_y1 = arrow_line_endpoint_for_target_y(target_top, arrow_stroke_width, true);
        let v_line_y2 = arrow_line_endpoint_for_target_y(target_bottom, arrow_stroke_width, false);

        // Artwork dimension label (compute size first so arrow lines can stop at label edges)
        let fmt = |v: f64| format_dimension(v, unit, options.use_tape_segments, options.use_decimal_display);
        let artwork_label = format!(
            "{} × {}",
            fmt(design.artwork_height),
            fmt(design.artwork_width)
        );
        let mask_margin = LABEL_MASK_PADDING_X;
        let text_bg_w = estimate_text_width(&artwork_label, style.label_font_size) + mask_margin * 2.0;
        let text_bg_h = style.label_font_size * 1.3 + mask_margin * 2.0;

        // Label background edges — arrow lines stop here instead of passing through
        let label_left = artwork_center.x - text_bg_w / 2.0;
        let label_right = artwork_center.x + text_bg_w / 2.0;
        let label_top = artwork_center_y - text_bg_h / 2.0;
        let label_bottom = artwork_center_y + text_bg_h / 2.0;

        // Horizontal line with arrows (with spark symbol if X break active)
        // Spark co-occurs with the frame break — directly signals where the compression happens.
        if geometry.use_axis_break_x {
            let break_center_x = (geometry.break_x_start + geometry.break_x_end) / 2.0;

            // Left segment: arrow to spark
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                break_center_x - SPARK_HORIZONTAL_WIDTH / 2.0, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            render_spark_symbol(&mut svg, break_center_x, artwork_center_y, true, artwork_indicator_color, arrow_stroke_width);
            // Right segment: spark to arrow
            svg.push_str(&generate_line_with_arrows(
                break_center_x + SPARK_HORIZONTAL_WIDTH / 2.0, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else if label_left > h_line_x1 && label_right < h_line_x2 {
            // Label fits inside artwork area — split into two segments at label edges
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                label_left, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            svg.push_str(&generate_line_with_arrows(
                label_right, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else {
            // Label wider than artwork area — draw full line (label bg will mask center)
            svg.push_str(&generate_line_with_arrows(
                h_line_x1, artwork_center_y,
                h_line_x2, artwork_center_y,
                artwork_indicator_color, arrow_stroke_width,
                true, true, false,
            ));
        }

        // Vertical line with arrows (with spark symbol if Y break active)
        // Spark co-occurs with the frame break — directly signals where the compression happens.
        if geometry.use_axis_break_y {
            let break_center_y = (geometry.break_y_start + geometry.break_y_end) / 2.0;

            // Top segment: arrow to spark
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, break_center_y - SPARK_VERTICAL_HEIGHT / 2.0,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            render_spark_symbol(&mut svg, artwork_center.x, break_center_y, false, artwork_indicator_color, arrow_stroke_width);
            // Bottom segment: spark to arrow
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, break_center_y + SPARK_VERTICAL_HEIGHT / 2.0,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else if label_top > v_line_y1 && label_bottom < v_line_y2 {
            // Label fits inside artwork area — split into two segments at label edges
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, label_top,
                artwork_indicator_color, arrow_stroke_width,
                true, false, false,
            ));
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, label_bottom,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                false, true, false,
            ));
        } else {
            // Label taller than artwork area — draw full line (label bg will mask center)
            svg.push_str(&generate_line_with_arrows(
                artwork_center.x, v_line_y1,
                artwork_center.x, v_line_y2,
                artwork_indicator_color, arrow_stroke_width,
                true, true, false,
            ));
        }

        // Draw background rectangle FIRST (so it appears behind the text)
        // Centered on artwork_center
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="{}" stroke="none" rx="2"/>"#,
            artwork_center.x - text_bg_w / 2.0,
            artwork_center_y - text_bg_h / 2.0,
            text_bg_w,
            text_bg_h,
            style.background_color,
            OPACITY_LABEL_BACKGROUND
        ));
        svg.push('\n');

        // Draw text SECOND (so it appears on top)
        // text-anchor="middle" for horizontal centering. For vertical centering we avoid
        // dominant-baseline="middle" (WebKit/Safari support is unreliable) and instead shift
        // the baseline down by ~0.35em (half the typical cap height). This places the visual
        // center of the glyphs at artwork_center_y, matching the background rect, and also
        // works correctly in svg2pdf.js (which ignores dominant-baseline entirely).
        let text_y = artwork_center_y + style.label_font_size * BASELINE_SHIFT_RATIO;
        svg.push_str(&format!(
            r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{:.2}px" text-anchor="middle">{}</text>"#,
            artwork_center.x, text_y,
            artwork_indicator_color, style.font_family, style.label_font_size,
            html_escape(&artwork_label)
        ));
        svg.push('\n');
        svg.push_str("  </g>\n");
    }

    // Mat cut geometry — rendered BEFORE the corner detail box so the corner detail's
    // opaque background cleanly covers extension lines that pass through the frame corner.
    // Extension lines start at mat_opening.bottom() which is inside the frame interior;
    // for narrow portrait frames they inevitably enter the corner detail box x/y range.
    if !mat_cut_geom.is_empty() {
        svg.push_str("  <g id=\"mat-cut-geom\">\n");
        svg.push_str(&mat_cut_geom);
        svg.push_str("  </g>\n");
    }

    // Corner detail inset overlay — renders after mat cut geometry so the opaque box
    // cleanly covers any overlapping extension lines inside the frame corner.
    if let Some(cd) = &geometry.corner_detail {
        svg.push_str(&render_corner_detail(design, cd, options, style));
    }

    // Mat cut labels — rendered last so text appears on top of the corner detail box.
    // The label y-position (dim_line_y + mat_cut_offset) is typically below the corner
    // detail box bottom, so labels don't visually clash with the L-shape inside the box.
    if !mat_cut_labels.is_empty() {
        svg.push_str("  <g id=\"mat-cut-labels\">\n");
        svg.push_str(&mat_cut_labels);
        svg.push_str("  </g>\n");
    }

    // Proportional thumbnail — true aspect ratio silhouette (only when breaks active)
    if let Some(thumb) = &geometry.thumbnail {
        let tm = style.thumbnail_metrics();
        svg.push_str("  <g id=\"thumbnail\">\n");
        svg.push_str(&format!(
            "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\" fill=\"none\"/>\n",
            thumb.x, thumb.y, thumb.width, thumb.height, style.muted_color, tm.stroke_width
        ));
        match geometry.thumbnail_label_position {
            ThumbnailLabelPosition::Right => {
                // Label to the right of thumbnail
                let label_x = thumb.right() + tm.label_gap;
                let label_y = thumb.y + thumb.height / 2.0 - 1.0;
                svg.push_str(&format!(
                    "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}px\" text-anchor=\"start\">Actual</text>\n",
                    label_x, label_y, style.muted_color, style.font_family, tm.font_size
                ));
                svg.push_str(&format!(
                    "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}px\" text-anchor=\"start\">proportions</text>\n",
                    label_x, label_y + tm.line_height, style.muted_color, style.font_family, tm.font_size
                ));
            }
            ThumbnailLabelPosition::Below => {
                // Label below thumbnail
                let label_x = thumb.x + thumb.width / 2.0;
                let label_y = thumb.bottom() + tm.line_height;
                svg.push_str(&format!(
                    "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}px\" text-anchor=\"middle\">Actual</text>\n",
                    label_x, label_y, style.muted_color, style.font_family, tm.font_size
                ));
                svg.push_str(&format!(
                    "    <text transform=\"translate({:.2}, {:.2})\" fill=\"{}\" font-family=\"{}\" font-size=\"{:.1}px\" text-anchor=\"middle\">proportions</text>\n",
                    label_x, label_y + tm.line_height, style.muted_color, style.font_family, tm.font_size
                ));
            }
        }
        svg.push_str("  </g>\n");
    }

    // Overlay annotation card at card_pos — right gutter or below the content
    // (viewBox already extended above)
    if let (Some(card), Some((cx, cy))) = (&overlay_card, card_pos) {
        super::overlays::render_overlay_card(&mut svg, card, cx, cy, style);
    }

    svg.push_str("</svg>");
    svg
}

/// Generate SVG for a dimension callout.
///
/// Visual grammar:
/// - Extension lines start from geometry with small gap, extend past dimension line
/// - Dimension line shortened so arrow tips end before extension lines
/// - Label text and its mask come from the layout's `LabelPlacement`
///
/// Returns (geometry_svg, label_svg) so labels can be rendered in a second pass
/// on top of all masks, preventing adjacent-level masks from hiding labels.
pub(crate) fn svg_dimension(callout: &PositionedCallout, style: &DiagramStyle, geometry: &PlanViewGeometry) -> (String, String) {
    let mut svg = String::new();

    // Determine color based on dimension type
    use super::types::DimensionType;
    let dim_color = match callout.callout.dimension_type {
        DimensionType::FrameInsideWidthInterior
        | DimensionType::FrameInsideHeightInterior => &style.inside_dimension_color,
        DimensionType::FrameOutsideWidth
        | DimensionType::FrameOutsideHeight => &style.outside_dimension_color,
        DimensionType::MatCutWidth
        | DimensionType::MatCutHeight => &style.mat_dimension_color,
        DimensionType::TotalStackHeight => &style.dimension_color,
    };

    svg.push_str(&format!(r#"    <g class="dimension">"#));
    svg.push('\n');

    // Determine if horizontal or vertical
    let is_horizontal = callout.actual_side.is_horizontal();

    // Map abstract (along, across) coordinates to concrete (x, y).
    // "along" = direction the dimension measures, "across" = perpendicular.
    //   Horizontal: along = x, across = y  →  point(a, c) = (a, c)
    //   Vertical:   along = y, across = x  →  point(a, c) = (c, a)
    let xy: fn(f64, f64) -> (f64, f64) = if is_horizontal {
        |along, across| (along, across)
    } else {
        |along, across| (across, along)
    };

    // Extract orientation-independent values:
    //   along_start/end: extent boundaries in the measurement direction
    //   geom_across:     geometry edge in the perpendicular direction
    //   dim_across:      dimension line position in the perpendicular direction
    let (along_start, along_end, geom_across) = if is_horizontal {
        (callout.callout.extent_start.x, callout.callout.extent_end.x, callout.callout.extent_start.y)
    } else {
        (callout.callout.extent_start.y, callout.callout.extent_end.y, callout.callout.extent_start.x)
    };
    let dim_across = callout.dimension_line_position;

    // Determine direction: does the dimension line extend in the positive or negative direction?
    // Top/Left = negative, Bottom/Right = positive
    let going_positive = matches!(callout.actual_side, Side::Bottom | Side::Right);

    // Extension line endpoints in the across direction
    let ext_across_start = if going_positive { geom_across + style.extension_line_gap } else { geom_across - style.extension_line_gap };
    let ext_across_end = if going_positive {
        dim_across + style.extension_line_overshoot
    } else {
        dim_across - style.extension_line_overshoot
    };

    // Special case for MatCutWidth: both extension lines start at the mat opening's bottom edge
    let (ext_across_start, ext_across_end) = if is_horizontal
        && callout.callout.dimension_type == super::types::DimensionType::MatCutWidth
    {
        if let Some(mat_opening) = &geometry.mat_opening {
            let target = mat_opening.bottom() + 3.0; // Small offset below mat opening bottom
            (target, ext_across_end)
        } else {
            (ext_across_start, ext_across_end)
        }
    } else {
        (ext_across_start, ext_across_end)
    };

    // --- Extension lines (one at each extent boundary) ---
    let (x1, y1) = xy(along_start, ext_across_start);
    let (x2, y2) = xy(along_start, ext_across_end);
    svg.push_str(&format!(
        r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        x1, y1, x2, y2, dim_color, style.extension_stroke_width
    ));
    svg.push('\n');
    let (x1, y1) = xy(along_end, ext_across_start);
    let (x2, y2) = xy(along_end, ext_across_end);
    svg.push_str(&format!(
        r#"      <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{}"/>"#,
        x1, y1, x2, y2, dim_color, style.extension_stroke_width
    ));
    svg.push('\n');

    // --- Dimension line with arrows ---
    // When space is too tight for inward-pointing arrows, flip to outward-pointing
    let extent_span = (along_end - along_start).abs();
    let tight_space = is_tight_space(extent_span, style.dimension_stroke_width, TIGHT_SPACE_MULTIPLIER);

    // arrow_line_endpoint_for_target and _y do identical math; use the x variant generically
    let line_along1 = arrow_line_endpoint_for_target(along_start, style.dimension_stroke_width, true);
    let line_along2 = arrow_line_endpoint_for_target(along_end, style.dimension_stroke_width, false);

    if tight_space {
        // Outward-pointing arrows: short stubs extending outward from extension lines
        let stub_len = arrow_stub_len(style.dimension_stroke_width);
        // Start-side arrow: points inward from outside
        let start_stub_start = along_start - stub_len;
        let start_stub_end = arrow_line_endpoint_for_target(along_start, style.dimension_stroke_width, false);
        let (sx1, sy1) = xy(start_stub_start, dim_across);
        let (sx2, sy2) = xy(start_stub_end, dim_across);
        let arrow_svg = generate_line_with_arrows(
            sx1, sy1, sx2, sy2,
            dim_color, style.dimension_stroke_width,
            false, true, false,
        );
        for line in arrow_svg.lines() {
            svg.push_str("    ");
            svg.push_str(line);
            svg.push('\n');
        }
        // End-side arrow: points inward from outside
        let end_stub_start = along_end + stub_len;
        let end_stub_end = arrow_line_endpoint_for_target(along_end, style.dimension_stroke_width, true);
        let (ex1, ey1) = xy(end_stub_start, dim_across);
        let (ex2, ey2) = xy(end_stub_end, dim_across);
        let arrow_svg = generate_line_with_arrows(
            ex1, ey1, ex2, ey2,
            dim_color, style.dimension_stroke_width,
            false, true, false,
        );
        for line in arrow_svg.lines() {
            svg.push_str("    ");
            svg.push_str(line);
            svg.push('\n');
        }
    } else {
        // Normal inward-pointing arrows
        let (x1, y1) = xy(line_along1, dim_across);
        let (x2, y2) = xy(line_along2, dim_across);
        let arrow_svg = generate_line_with_arrows(
            x1, y1, x2, y2,
            dim_color, style.dimension_stroke_width,
            true, true, false,
        );
        for line in arrow_svg.lines() {
            svg.push_str("    ");
            svg.push_str(line);
            svg.push('\n');
        }
    }

    // Collect arrowhead polygons to re-render after label mask
    // (ensures arrows are visible even when mask overlaps compressed dimensions)
    // In tight_space mode the stubs are outside the extent boundaries and never
    // covered by the mask, so no overlay is needed (and re-rendering inward
    // arrowheads would produce phantom arrows inside the narrow span).
    let mut arrow_overlay = String::new();
    if !tight_space {
        let (p1x, p1y) = xy(line_along1, dim_across);
        let (p2x, p2y) = xy(line_along2, dim_across);
        // Start arrow (pointing toward start)
        arrow_overlay.push_str(&generate_arrow_polygon(p2x, p2y, p1x, p1y, dim_color, style.dimension_stroke_width, false));
        arrow_overlay.push('\n');
        // End arrow (pointing toward end)
        arrow_overlay.push_str(&generate_arrow_polygon(p1x, p1y, p2x, p2y, dim_color, style.dimension_stroke_width, false));
        arrow_overlay.push('\n');
    }

    // Label: text lines and mask come precomputed from layout (`place_label`),
    // so what collision/viewBox fitting saw is exactly what is drawn here.
    let placement = &callout.label;
    let mask = &placement.mask;
    svg.push_str(&format!(
        r#"      <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="none"/>"#,
        mask.x, mask.y, mask.width, mask.height,
        style.background_color
    ));
    svg.push('\n');

    // Re-render arrowhead polygons on top of the mask so they're never hidden
    if !arrow_overlay.is_empty() {
        for line in arrow_overlay.lines() {
            svg.push_str("      ");
            svg.push_str(line);
            svg.push('\n');
        }
    }

    svg.push_str("    </g>\n");

    // Label SVG is returned separately and drawn in a second pass, on top of all
    // masks, so an adjacent level's mask can never hide it.
    //
    // Vertical centering uses an explicit baseline shift (BASELINE_SHIFT_RATIO)
    // instead of dominant-baseline, which flutter_svg (iOS) and svg2pdf/pdf
    // ignore. Horizontal lines use transform="translate(x,y)" (not x=/y=) so
    // vector_graphics scales font-size consistently with rotated labels.
    // Rotated lines put rotate(90) on the <text> itself (not a wrapping <g>) so
    // flutter_svg propagates the parent scale() to font-size; the rotation
    // center is the line's visual center, and the shifted local y moves the
    // baseline so the glyphs straddle it.
    let fs = style.label_font_size;
    let shift = fs * BASELINE_SHIFT_RATIO;
    let anchor = match placement.anchor {
        TextAnchor::Start => "start",
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
    };
    let mut label_svg = String::new();
    for line in &placement.lines {
        let (x, y) = (line.pos.x, line.pos.y);
        if placement.rotated {
            label_svg.push_str(&format!(
                r#"      <text transform="rotate(90 {:.2} {:.2})" x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{}px" text-anchor="{}">{}</text>"#,
                x, y, x, y + shift, dim_color, style.font_family, fs, anchor,
                html_escape(&line.text)
            ));
        } else {
            label_svg.push_str(&format!(
                r#"      <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px" text-anchor="{}">{}</text>"#,
                x, y + shift, dim_color, style.font_family, fs, anchor,
                html_escape(&line.text)
            ));
        }
        label_svg.push('\n');
    }

    (svg, label_svg)
}
