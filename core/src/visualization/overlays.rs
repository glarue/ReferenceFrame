//! Optional diagram overlays: spline (corner key) slot placement and hanging
//! hardware. Enabled per-layer via `DiagramOptions::show_spline` /
//! `show_hanging`; both default off so existing diagrams are unchanged.
//!
//! Overlays are geometry-driven: positions come from the computed view
//! geometry plus the `joinery` / `hanging` math modules, never from the
//! branch-specific frame rendering. When axis breaks compress a view the
//! linear position mapping no longer holds, so overlays are skipped there
//! rather than drawn misplaced.
//!
//! Plan-view text does NOT sit over the diagram: marks render in place, and
//! all annotation lines collect into a bordered card in the right gutter
//! (`plan_overlay_card` + `render_overlay_card`), following the Corner
//! Detail inset's sectioning language. Marks and card lines share colors so
//! they key to each other.

use crate::frame::FrameDesign;
use crate::hanging::{hanging_layout, HangingParams};
use crate::joinery::{spline_envelope, SplineParams};

use super::geometry::{estimate_text_width, PlanViewGeometry, SectionViewGeometry};
use super::style::DiagramStyle;
use super::types::DiagramOptions;

/// Spline slot fill needs a darker companion for outline/text contrast.
const SPLINE_STROKE: &str = "#2e7a63";
const SPLINE_TEXT: &str = "#0d3d30";

/// Overlay label with a semi-opaque backdrop so it stays legible over any
/// geometry or callout lines it crosses.
fn push_backdropped_text(
    svg: &mut String,
    x: f64,
    y: f64,
    anchor: &str,
    color: &str,
    style: &DiagramStyle,
    text: &str,
) {
    let text_w = estimate_text_width(text, style.dimension_font_size);
    let pad = 4.0;
    let bg_x = match anchor {
        "end" => x - text_w,
        "middle" => x - text_w / 2.0,
        _ => x,
    } - pad;
    svg.push_str(&format!(
        r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.82" rx="3"/>"#,
        bg_x,
        y - style.dimension_font_size * 0.75 - pad / 2.0,
        text_w + 2.0 * pad,
        style.dimension_font_size * 1.5 + pad,
        style.background_color
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <text transform="translate({x:.2}, {y:.2})" fill="{color}" font-family="{}" font-size="{}px" text-anchor="{anchor}" dominant-baseline="central">{text}</text>"#,
        style.font_family, style.dimension_font_size
    ));
    svg.push('\n');
}

/// Spline slots on the section-view moulding profile.
///
/// Section orientation: TOP of the profile is the front face; the rabbet
/// channel occupies the back-inner (bottom-right) region. Slot depth
/// (`z_center`) is measured from the front face, so it maps straight down
/// from the profile top. The slot is open at the outer (left) edge; the fill
/// is inset by the frame stroke so the profile outline stays crisp.
pub(crate) fn render_section_splines(
    svg: &mut String,
    geometry: &SectionViewGeometry,
    design: &FrameDesign,
    style: &DiagramStyle,
    fmt: &dyn Fn(f64) -> String,
    params: SplineParams,
) {
    if geometry.use_axis_break || geometry.use_axis_break_y {
        return;
    }
    let Some(env) = spline_envelope(design, &params) else {
        return;
    };

    let fp = &geometry.frame_profile;
    let s = geometry.scale;
    let inset = style.frame_stroke_width / 2.0;

    svg.push_str("  <g id=\"spline-slots\">\n");
    for slot in &env.recommended {
        let yc = fp.y + slot.z_center * s;
        let h = params.slot_thickness * s;
        let w = slot.max_penetration * s - inset;
        if w <= 0.0 {
            continue;
        }
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" fill-opacity="0.85" stroke="{}" stroke-width="0.8"/>"#,
            fp.x + inset,
            yc - h / 2.0,
            w,
            h,
            style.accent_color,
            SPLINE_STROKE
        ));
        svg.push('\n');
        let label = format!(
            "Spline {} · ≤ {} deep{}",
            fmt(params.slot_thickness),
            fmt(slot.max_penetration),
            if slot.over_rabbet { " (limited by rabbet)" } else { "" },
        );
        // Label sits inside the slot band when it fits; on small mouldings
        // (compact canvases) it moves beside the slot with a backdrop instead
        // of straddling the profile edge.
        if estimate_text_width(&label, style.dimension_font_size) + 20.0 <= w {
            svg.push_str(&format!(
                r#"    <text transform="translate({:.2}, {:.2})" fill="{}" font-family="{}" font-size="{}px" text-anchor="start" dominant-baseline="central">{}</text>"#,
                fp.x + inset + 10.0,
                yc,
                SPLINE_TEXT,
                style.font_family,
                style.dimension_font_size,
                label
            ));
            svg.push('\n');
        } else {
            push_backdropped_text(
                svg,
                fp.x + inset + w + 8.0,
                yc,
                "start",
                &style.accent_color,
                style,
                &label,
            );
        }
    }
    svg.push_str("  </g>\n");
}

/// Spline slot chords across each corner of the plan view, drawn dashed
/// (the slot lives on the outer edge, below the face). Chord endpoints sit
/// `2 x penetration` along each outer edge, which places the chord at the
/// slot's maximum bisector depth. Marks only — the annotation line lives in
/// the overlay card.
pub(crate) fn render_plan_splines(
    svg: &mut String,
    geometry: &PlanViewGeometry,
    design: &FrameDesign,
    style: &DiagramStyle,
    params: SplineParams,
) {
    if geometry.use_axis_break_x || geometry.use_axis_break_y {
        return;
    }
    let Some(a) = plan_spline_leg(design, params, geometry.scale) else {
        return;
    };

    let fo = &geometry.frame_outer;
    let (x0, y0) = (fo.x, fo.y);
    let (x1, y1) = (fo.x + fo.width, fo.y + fo.height);

    svg.push_str("  <g id=\"spline-corners\">\n");
    for (ax, ay, bx, by) in [
        (x0 + a, y0, x0, y0 + a),
        (x1 - a, y0, x1, y0 + a),
        (x0 + a, y1, x0, y1 - a),
        (x1 - a, y1, x1, y1 - a),
    ] {
        svg.push_str(&format!(
            r#"    <line x1="{ax:.2}" y1="{ay:.2}" x2="{bx:.2}" y2="{by:.2}" stroke="{}" stroke-width="2" stroke-dasharray="5,3"/>"#,
            style.accent_color
        ));
        svg.push('\n');
    }
    svg.push_str("  </g>\n");
}

/// Chord leg length (along each outer edge from the corner) for the first
/// recommended slot at the given scale. Shared by the plan view and the
/// corner-detail inset so both draw the same slot.
pub(crate) fn plan_spline_leg(design: &FrameDesign, params: SplineParams, scale: f64) -> Option<f64> {
    let env = spline_envelope(design, &params)?;
    let slot = env.recommended.first()?;
    Some(2.0 * slot.max_penetration * scale)
}

/// Hanging hardware on the plan view: attachment points on the side-rail
/// centerlines, hung-state wire legs, hook marker, and the hook-drop line.
/// Marks only — measurements live in the overlay card.
pub(crate) fn render_plan_hanging(
    svg: &mut String,
    geometry: &PlanViewGeometry,
    design: &FrameDesign,
    style: &DiagramStyle,
    params: HangingParams,
) {
    if geometry.use_axis_break_x || geometry.use_axis_break_y {
        return;
    }
    let Some(layout) = hanging_layout(design, &params) else {
        return;
    };

    let fo = &geometry.frame_outer;
    let fi = &geometry.frame_inner;
    let s = geometry.scale;

    let rail_left = (fo.x + fi.x) / 2.0;
    let rail_right = ((fo.x + fo.width) + (fi.x + fi.width)) / 2.0;
    let ring_y = fo.y + layout.ring_drop * s;
    let apex_x = fo.x + fo.width / 2.0;
    let apex_y = ring_y - layout.apex_rise * s;

    svg.push_str("  <g id=\"hanging-hardware\">\n");
    svg.push_str(&format!(
        r##"    <polyline points="{rail_left:.2},{ring_y:.2} {apex_x:.2},{apex_y:.2} {rail_right:.2},{ring_y:.2}" fill="none" stroke="{}" stroke-width="1.5"/>"##,
        style.line_color
    ));
    svg.push('\n');
    for x in [rail_left, rail_right] {
        svg.push_str(&format!(
            r#"    <circle cx="{x:.2}" cy="{ring_y:.2}" r="5" fill="{}"/>"#,
            style.dimension_color
        ));
        svg.push('\n');
    }
    svg.push_str(&format!(
        r#"    <path d="M{apex_x:.2},{apex_y:.2} l-6,-10 h12 Z" fill="{}"/>"#,
        style.mat_dimension_color
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"    <line x1="{apex_x:.2}" y1="{:.2}" x2="{apex_x:.2}" y2="{:.2}" stroke="{}" stroke-width="0.8" stroke-dasharray="4,3"/>"#,
        fo.y,
        apex_y - 10.0,
        style.mat_dimension_color
    ));
    svg.push('\n');
    svg.push_str("  </g>\n");
}

/// A bordered annotation card holding all overlay measurements, rendered in
/// the plan view's right gutter instead of over the diagram.
pub(crate) struct OverlayCard {
    pub title: &'static str,
    /// (color, text) per line; colors key to the in-diagram marks
    pub lines: Vec<(String, String)>,
    pub width: f64,
    pub height: f64,
}

const CARD_PAD: f64 = 10.0;
const CARD_TITLE_H: f64 = 20.0;
const CARD_LINE_H: f64 = 19.0;

/// Build the card contents for the enabled overlay layers, or `None` when
/// nothing applies (layers off, breaks active, or no valid layout).
pub(crate) fn plan_overlay_card(
    geometry: &PlanViewGeometry,
    design: &FrameDesign,
    style: &DiagramStyle,
    options: &DiagramOptions,
    fmt: &dyn Fn(f64) -> String,
) -> Option<OverlayCard> {
    if geometry.use_axis_break_x || geometry.use_axis_break_y {
        return None;
    }
    let mut lines: Vec<(String, String)> = Vec::new();
    let mut has_hanging = false;
    let mut has_spline = false;

    if options.show_hanging {
        let params = options.hanging_params.unwrap_or_default();
        if let Some(layout) = hanging_layout(design, &params) {
            has_hanging = true;
            lines.push((
                style.mat_dimension_color.clone(),
                format!("hook {} below top", fmt(layout.hook_below_top)),
            ));
            lines.push((
                style.dimension_color.clone(),
                format!("hangers {} from top", fmt(layout.ring_drop)),
            ));
            lines.push((
                style.dimension_color.clone(),
                format!("wire cut {}", fmt(layout.wire_cut_length)),
            ));
        }
    }
    if options.show_spline {
        let params = options.spline_params.unwrap_or_default();
        if let Some(env) = spline_envelope(design, &params) {
            if let Some(slot) = env.recommended.first() {
                has_spline = true;
                lines.push((
                    style.accent_color.clone(),
                    format!("spline slots (dashed) · max {} deep", fmt(slot.max_penetration)),
                ));
            }
        }
    }
    if lines.is_empty() {
        return None;
    }

    // Titles are emitted raw into XML — keep them entity-free or pre-escaped
    let title = match (has_spline, has_hanging) {
        (true, true) => "Joinery &amp; Hanging",
        (true, false) => "Joinery",
        _ => "Hanging",
    };
    let text_w = lines
        .iter()
        .map(|(_, t)| estimate_text_width(t, style.dimension_font_size))
        .fold(estimate_text_width(title, style.dimension_font_size), f64::max);
    Some(OverlayCard {
        title,
        width: text_w + 2.0 * CARD_PAD + 12.0, // 12: color-key swatch column
        height: CARD_TITLE_H + lines.len() as f64 * CARD_LINE_H + CARD_PAD,
        lines,
    })
}

/// Draw the card at (x, y) — same visual language as the Corner Detail box.
pub(crate) fn render_overlay_card(
    svg: &mut String,
    card: &OverlayCard,
    x: f64,
    y: f64,
    style: &DiagramStyle,
) {
    svg.push_str("  <g id=\"overlay-card\">\n");
    svg.push_str(&format!(
        r##"    <rect x="{x:.2}" y="{y:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="#999" stroke-width="0.75" rx="4"/>"##,
        card.width, card.height, style.background_color
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r##"    <text transform="translate({:.2}, {:.2})" fill="#555" font-family="{}" font-size="{:.1}" font-weight="bold" text-anchor="middle">{}</text>"##,
        x + card.width / 2.0,
        y + 14.0,
        style.font_family,
        style.dimension_font_size * 0.9,
        card.title
    ));
    svg.push('\n');
    for (i, (color, text)) in card.lines.iter().enumerate() {
        let ly = y + CARD_TITLE_H + i as f64 * CARD_LINE_H + CARD_LINE_H / 2.0;
        svg.push_str(&format!(
            r#"    <rect x="{:.2}" y="{:.2}" width="7" height="7" rx="1.5" fill="{color}"/>"#,
            x + CARD_PAD,
            ly - 3.5,
        ));
        svg.push('\n');
        svg.push_str(&format!(
            r#"    <text transform="translate({:.2}, {:.2})" fill="{color}" font-family="{}" font-size="{}px" text-anchor="start" dominant-baseline="central">{text}</text>"#,
            x + CARD_PAD + 12.0,
            ly,
            style.font_family,
            style.dimension_font_size
        ));
        svg.push('\n');
    }
    svg.push_str("  </g>\n");
}
