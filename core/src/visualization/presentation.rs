//! Presentation render: the frame as it will look on the wall, with no
//! measurements or markings (docs/plans/PHOTO_PREVIEW_PLAN.md).
//!
//! Draws the wall shadow, the frame face (wood grain when `wood` is set, else a flat
//! colour), a filled mat with its bevel, and the lip's shadow. The window the art
//! shows through is either:
//! - left transparent, so a platform can draw the photo beneath the SVG (live
//!   previews: re-parsing an embedded photo every animation frame is too slow);
//! - filled with the photo as an embedded `<image>` (PDF, saved images, web);
//! - or filled with a neutral placeholder when there is no photo.
//!
//! [`PhotoLayout`] gives the window, the artwork rect and where the photo goes, in
//! the SVG's px, so the platform layer and the embedded image line up exactly.

use serde::{Deserialize, Serialize};
use std::fmt::Write;

use crate::frame::FrameDesign;
use crate::presets::get_mat_colors;

use super::geometry::PlanViewGeometry;
use super::style::DiagramStyle;
use super::types::Rect;
use super::wood::{inner_shadow_svg, num, wall_shadow_svg, DepthCues, FaceDepths, WoodRender};

/// Shadow length per inch of height (matches the wood face's depth cues).
const DEPTH_K: f64 = 0.6;
/// Flat frame colour when wood grain is off.
pub const FLAT_FRAME_COLOR: &str = "#3B3835";
/// The window's fill when there is no photo.
const PLACEHOLDER_COLOR: &str = "#E4E1DA";
/// Paper behind a fitted (letterboxed) photo.
const PAPER_COLOR: &str = "#FAFAF7";
/// Largest zoom into a photo.
pub const MAX_ZOOM: f64 = 5.0;
/// Narrowest bevel drawn, px (keeps it visible at small scales).
const MIN_BEVEL_PX: f64 = 0.8;

/// How a photo fills the artwork area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitMode {
    /// Covers the artwork area, cropping the overflow
    #[default]
    Fill,
    /// Whole photo inside the artwork area, on paper
    Fit,
}

/// A photo's fit, zoom (1 to [`MAX_ZOOM`]) and the photo point (0–1 of its width
/// and height) placed at the artwork's centre.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhotoFit {
    pub mode: FitMode,
    pub zoom: f64,
    pub center_x: f64,
    pub center_y: f64,
}

impl Default for PhotoFit {
    fn default() -> Self {
        Self { mode: FitMode::Fill, zoom: 1.0, center_x: 0.5, center_y: 0.5 }
    }
}

impl PhotoFit {
    /// Where a `w`×`h` px photo is drawn for the artwork rect `art`, and the fit with
    /// its zoom and centre clamped: a photo never pans past an edge of the artwork
    /// area it covers, and is centred on an axis where it is smaller.
    pub fn place(&self, art: &Rect, w: f64, h: f64) -> (Rect, PhotoFit) {
        if !(w > 0.0 && h > 0.0 && art.width > 0.0 && art.height > 0.0) {
            return (*art, PhotoFit::default());
        }
        let zoom = if self.zoom.is_finite() { self.zoom.clamp(1.0, MAX_ZOOM) } else { 1.0 };
        let (sx, sy) = (art.width / w, art.height / h);
        let base = match self.mode {
            FitMode::Fill => sx.max(sy),
            FitMode::Fit => sx.min(sy),
        };
        let (pw, ph) = (w * base * zoom, h * base * zoom);
        let axis = |start: f64, len: f64, size: f64, c: f64| -> (f64, f64) {
            let c = if c.is_finite() { c } else { 0.5 };
            if size <= len {
                (start + (len - size) / 2.0, 0.5)
            } else {
                let mid = start + len / 2.0;
                let pos = (mid - c * size).clamp(start + len - size, start);
                (pos, (mid - pos) / size)
            }
        };
        let (x, cx) = axis(art.x, art.width, pw, self.center_x);
        let (y, cy) = axis(art.y, art.height, ph, self.center_y);
        (Rect::new(x, y, pw, ph), PhotoFit { mode: self.mode, zoom, center_x: cx, center_y: cy })
    }
}

/// The photo to show: its pixel size, fit, and optionally its image data to embed
/// (a `data:` URI or URL). Without `href` the window is left transparent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationPhoto {
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub fit: PhotoFit,
    #[serde(default)]
    pub href: Option<String>,
}

/// Options for [`generate_presentation`] (JSON for the bindings; every field but
/// the canvas size is optional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationOptions {
    pub canvas_width: f64,
    pub canvas_height: f64,
    /// Wood grain on the face; `None` draws it flat in `frame_color`
    #[serde(default)]
    pub wood: Option<WoodRender>,
    /// Flat frame colour (`#RRGGBB`); defaults to [`FLAT_FRAME_COLOR`]
    #[serde(default)]
    pub frame_color: Option<String>,
    /// A `presets.json` `mat_colors` key (unknown or absent: the first colour)
    #[serde(default)]
    pub mat_color: Option<String>,
    #[serde(default)]
    pub photo: Option<PresentationPhoto>,
    /// Wall colour; `None` leaves the background transparent
    #[serde(default)]
    pub background: Option<String>,
    /// Space around the frame and its shadow, px
    #[serde(default = "default_margin")]
    pub margin: f64,
}

fn default_margin() -> f64 {
    16.0
}

/// Where things are, in the SVG's px.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoLayout {
    /// The visible opening: inside the mat's bevel, or the frame's sight opening
    pub window: Rect,
    /// The artwork's full extent (part of it is under the mat or the lip)
    pub artwork: Rect,
    /// Where the whole photo is drawn (clip it to `window`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<Rect>,
    /// The photo's fit, zoom and centre after clamping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fit: Option<PhotoFit>,
    /// Scale, px per inch
    pub px_per_in: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresentationResult {
    pub svg: String,
    pub layout: PhotoLayout,
}

impl PhotoLayout {
    fn new(window: Rect, artwork: Rect, photo: Option<&PresentationPhoto>, px_per_in: f64) -> Self {
        let placed = photo.map(|p| p.fit.place(&artwork, p.width, p.height));
        Self { window, artwork, photo: placed.map(|(r, _)| r), fit: placed.map(|(_, f)| f), px_per_in }
    }
}

/// The photo layout for the live preview's diagram (`show_callouts: false`), whose
/// window is the mat opening, or the frame's opening without a mat.
pub fn preview_photo_layout(
    design: &FrameDesign,
    canvas_width: f64,
    canvas_height: f64,
    style: &DiagramStyle,
    photo: Option<&PresentationPhoto>,
) -> PhotoLayout {
    let g = PlanViewGeometry::from_design_preview(design, canvas_width, canvas_height, style);
    let window = g.mat_opening.unwrap_or(g.frame_inner);
    PhotoLayout::new(window, g.artwork, photo, g.scale)
}

/// The frame as it will look: no measurements, labels or construction lines.
pub fn generate_presentation(design: &FrameDesign, options: &PresentationOptions) -> PresentationResult {
    let (cw, ch) = (options.canvas_width, options.canvas_height);
    let (outer_h, outer_w) = design.get_frame_outside_dimensions();
    // fit the frame plus its wall shadow (bottom and right)
    let shadow_in = design.frame_material_depth.max(0.0) * DEPTH_K;
    let scale = ((cw - 2.0 * options.margin) / (outer_w + shadow_in))
        .min((ch - 2.0 * options.margin) / (outer_h + shadow_in));
    if !(scale.is_finite() && scale > 0.0) {
        let empty = Rect::new(0.0, 0.0, 0.0, 0.0);
        return PresentationResult {
            svg: format!(r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg"></svg>"#, num(cw.max(0.0)), num(ch.max(0.0))),
            layout: PhotoLayout::new(empty, empty, None, 0.0),
        };
    }
    let ox = (cw - (outer_w + shadow_in) * scale) / 2.0;
    let oy = (ch - (outer_h + shadow_in) * scale) / 2.0;
    let g = PlanViewGeometry::from_design_at(design, scale, ox, oy);

    let mat = get_mat_colors().get(options.mat_color.as_deref());
    let bevel = (design.matboard_thickness * scale).max(MIN_BEVEL_PX);
    let window = match &g.mat_opening {
        Some(mo) => inset(mo, bevel),
        None => g.frame_inner,
    };
    let layout = PhotoLayout::new(window, g.artwork, options.photo.as_ref(), scale);

    let mut svg = String::new();
    let _ = write!(svg, r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">"#, num(cw), num(ch));

    // wall (with the window cut out, so a photo drawn beneath shows through)
    if let Some(bg) = &options.background {
        let _ = write!(svg, r#"<path d="M0,0h{}v{}h{}z{}" fill="{}" fill-rule="evenodd"/>"#,
            num(cw), num(ch), num(-cw), hole(&window), attr(bg));
    }

    // window: embedded photo, placeholder, or nothing
    match (&options.photo, &layout.photo) {
        (Some(PresentationPhoto { href: Some(href), fit, .. }), Some(pr)) => {
            let _ = write!(svg, r#"<clipPath id="pwin"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath><g clip-path="url(#pwin)">"#,
                num(window.x), num(window.y), num(window.width), num(window.height));
            if fit.mode == FitMode::Fit {
                let _ = write!(svg, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{PAPER_COLOR}"/>"#,
                    num(window.x), num(window.y), num(window.width), num(window.height));
            }
            // two decimals: the photo's edges must meet the window's exactly
            let _ = write!(svg, r#"<image x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" preserveAspectRatio="none" xlink:href="{}"/></g>"#,
                pr.x, pr.y, pr.width, pr.height, attr(href));
        }
        (None, _) => {
            let _ = write!(svg, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{PLACEHOLDER_COLOR}"/>"#,
                num(window.x), num(window.y), num(window.width), num(window.height));
        }
        _ => {}
    }

    // mat face and bevel (light from the upper left: the top and left bevels face away)
    if let Some(mo) = &g.mat_opening {
        let fi = &g.frame_inner;
        let _ = write!(svg, r#"<path d="M{},{}h{}v{}h{}z{}" fill="{}" fill-rule="evenodd"/>"#,
            num(fi.x), num(fi.y), num(fi.width), num(fi.height), num(-fi.width), hole(mo), attr(&mat.hex));
        let (o, i) = (mo, &window);
        let sides = [
            ([(o.left(), o.top()), (o.right(), o.top()), (i.right(), i.top()), (i.left(), i.top())], Some(0.16)),
            ([(o.left(), o.top()), (i.left(), i.top()), (i.left(), i.bottom()), (o.left(), o.bottom())], Some(0.08)),
            ([(o.right(), o.top()), (o.right(), o.bottom()), (i.right(), i.bottom()), (i.right(), i.top())], None),
            ([(o.left(), o.bottom()), (i.left(), i.bottom()), (i.right(), i.bottom()), (o.right(), o.bottom())], None),
        ];
        for (pts, shade) in sides {
            let d = poly(&pts);
            let _ = write!(svg, r#"<path d="{d}" fill="{}"/>"#, attr(&mat.core));
            if let Some(a) = shade {
                let _ = write!(svg, r##"<path d="{d}" fill="#000" fill-opacity="{a}"/>"##);
            }
        }
    }

    // frame
    let o = &g.frame_outer;
    let depths = FaceDepths::from_design(design);
    match &options.wood {
        Some(wood) => {
            let wood = WoodRender { depth: DepthCues::InnerAndWall, ..wood.clone() };
            let f = wood.frame_face(design, o.x, o.y, o.width, o.height, scale);
            svg.push_str(&f.under);
            svg.push_str(&f.face);
            svg.push_str(&f.inner_shadow);
        }
        None => {
            let color = options.frame_color.as_deref().unwrap_or(FLAT_FRAME_COLOR);
            svg.push_str(&wall_shadow_svg(o.x, o.y, o.width, o.height, &depths, scale));
            flat_face(&mut svg, o, &g.frame_inner, color);
            let fi = &g.frame_inner;
            svg.push_str(&inner_shadow_svg(fi.x, fi.y, fi.width, fi.height, &depths, scale, "pf-"));
        }
    }

    svg.push_str("</svg>");
    PresentationResult { svg, layout }
}

/// A flat frame face: four mitered pieces, the top and left a little lighter.
fn flat_face(svg: &mut String, o: &Rect, i: &Rect, color: &str) {
    let mut d = format!("M{},{}h{}v{}h{}z", num(o.x), num(o.y), num(o.width), num(o.height), num(-o.width));
    d.push_str(&hole(i));
    let _ = write!(svg, r#"<path d="{d}" fill="{}" fill-rule="evenodd"/>"#, attr(color));
    let pieces = [
        ([(o.left(), o.top()), (o.right(), o.top()), (i.right(), i.top()), (i.left(), i.top())], "#FFF", 0.10),
        ([(o.left(), o.top()), (i.left(), i.top()), (i.left(), i.bottom()), (o.left(), o.bottom())], "#FFF", 0.05),
        ([(o.right(), o.top()), (o.right(), o.bottom()), (i.right(), i.bottom()), (i.right(), i.top())], "#000", 0.10),
        ([(o.left(), o.bottom()), (i.left(), i.bottom()), (i.right(), i.bottom()), (o.right(), o.bottom())], "#000", 0.16),
    ];
    for (pts, c, a) in pieces {
        let _ = write!(svg, r#"<path d="{}" fill="{c}" fill-opacity="{a}"/>"#, poly(&pts));
    }
    for ((ax, ay), (bx, by)) in [
        ((o.left(), o.top()), (i.left(), i.top())),
        ((o.right(), o.top()), (i.right(), i.top())),
        ((o.right(), o.bottom()), (i.right(), i.bottom())),
        ((o.left(), o.bottom()), (i.left(), i.bottom())),
    ] {
        let _ = write!(svg, r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#000" stroke-opacity="0.45" stroke-width="0.8"/>"##,
            num(ax), num(ay), num(bx), num(by));
    }
}

fn inset(r: &Rect, d: f64) -> Rect {
    let (dx, dy) = (d.min(r.width / 2.0), d.min(r.height / 2.0));
    Rect::new(r.x + dx, r.y + dy, r.width - 2.0 * dx, r.height - 2.0 * dy)
}

/// A rect subpath wound the other way (an evenodd hole).
fn hole(r: &Rect) -> String {
    format!("M{},{}v{}h{}v{}z", num(r.x), num(r.y), num(r.height), num(r.width), num(-r.height))
}

fn poly(pts: &[(f64, f64)]) -> String {
    let mut d = String::new();
    for (k, (x, y)) in pts.iter().enumerate() {
        let _ = write!(d, "{}{},{}", if k == 0 { "M" } else { "L" }, num(*x), num(*y));
    }
    d.push('z');
    d
}

/// Escapes a value for a double-quoted attribute.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::test_helpers::test_design;

    fn opts() -> PresentationOptions {
        serde_json::from_str(r#"{"canvas_width": 600, "canvas_height": 500}"#).unwrap()
    }

    fn photo(w: f64, h: f64, href: Option<&str>) -> PresentationPhoto {
        PresentationPhoto { width: w, height: h, fit: PhotoFit::default(), href: href.map(str::to_string) }
    }

    fn inside(inner: &Rect, outer: &Rect) -> bool {
        let e = 1e-6;
        inner.left() >= outer.left() - e && inner.right() <= outer.right() + e
            && inner.top() >= outer.top() - e && inner.bottom() <= outer.bottom() + e
    }

    #[test]
    fn has_no_markings() {
        let r = generate_presentation(&test_design(), &opts());
        for bad in ["<text", "stroke-dasharray", "matboard", "artwork"] {
            assert!(!r.svg.contains(bad), "{bad}");
        }
        assert!(!r.svg.contains('%'), "percentage lengths throw in flutter_svg");
    }

    #[test]
    fn window_is_inside_the_mat_opening_and_the_artwork() {
        let d = test_design();
        let r = generate_presentation(&d, &opts());
        let l = &r.layout;
        let (mo_h, mo_w) = d.get_mat_opening_dimensions();
        let bevel = d.matboard_thickness * l.px_per_in;
        assert!((l.window.width - (mo_w * l.px_per_in - 2.0 * bevel)).abs() < 1e-6);
        assert!((l.window.height - (mo_h * l.px_per_in - 2.0 * bevel)).abs() < 1e-6);
        assert!(inside(&l.window, &l.artwork));
        // fits the canvas with its shadow
        let (oh, ow) = d.get_frame_outside_dimensions();
        let s = d.frame_material_depth * DEPTH_K;
        assert!((ow + s) * l.px_per_in <= 600.0 - 32.0 + 1e-6 && (oh + s) * l.px_per_in <= 500.0 - 32.0 + 1e-6);
    }

    #[test]
    fn without_a_mat_the_window_is_the_frame_opening() {
        let mut d = test_design();
        d.mat_width_top_bottom = 0.0;
        d.mat_width_sides = 0.0;
        let r = generate_presentation(&d, &opts());
        let g = PlanViewGeometry::from_design_at(&d, r.layout.px_per_in, 0.0, 0.0);
        assert!((r.layout.window.width - g.frame_inner.width).abs() < 1e-6);
        assert!(inside(&r.layout.window, &r.layout.artwork));
        // no mat drawn
        assert!(!r.svg.contains(&get_mat_colors().colors[0].hex));
    }

    #[test]
    fn the_mat_overlap_or_the_lip_hides_the_artworks_edges() {
        // with a mat: the full artwork is drawn; the mat overlap (plus the bevel) covers its edges
        let d = test_design();
        let l = generate_presentation(&d, &opts()).layout;
        let s = l.px_per_in;
        assert!((l.artwork.width - d.artwork_width * s).abs() < 1e-6);
        let hidden = (l.artwork.width - l.window.width) / 2.0 / s;
        assert!((hidden - (d.mat_overlap + d.matboard_thickness)).abs() < 1e-6, "{hidden}");

        // no mat: the frame's lip over the rabbet covers them
        let mut d = test_design();
        d.mat_width_top_bottom = 0.0;
        d.mat_width_sides = 0.0;
        let l = generate_presentation(&d, &opts()).layout;
        let s = l.px_per_in;
        assert!((l.artwork.width - d.artwork_width * s).abs() < 1e-6);
        let hidden = (l.artwork.width - l.window.width) / 2.0 / s;
        assert!((hidden - d.lip_over_art()).abs() < 1e-6, "{hidden}");
        assert!(d.lip_over_art() > 0.0);
    }

    #[test]
    fn window_fill_depends_on_the_photo() {
        let placeholder = generate_presentation(&test_design(), &opts()).svg;
        assert!(placeholder.contains(PLACEHOLDER_COLOR));

        let layered = generate_presentation(&test_design(), &PresentationOptions { photo: Some(photo(1200.0, 800.0, None)), ..opts() }).svg;
        assert!(!layered.contains(PLACEHOLDER_COLOR) && !layered.contains("<image"));

        let href = "data:image/jpeg;base64,AAAA";
        let embedded = generate_presentation(&test_design(), &PresentationOptions { photo: Some(photo(1200.0, 800.0, Some(href))), ..opts() }).svg;
        assert!(embedded.contains(href) && embedded.contains(r#"clip-path="url(#pwin)""#));
    }

    #[test]
    fn mat_colour_and_flat_or_wood_frame() {
        let black = get_mat_colors().get(Some("black")).hex.clone();
        let r = generate_presentation(&test_design(), &PresentationOptions { mat_color: Some("black".into()), ..opts() });
        assert!(r.svg.contains(&black) && r.svg.contains(FLAT_FRAME_COLOR));

        let wood: WoodRender = serde_json::from_str(r#"{"species": "cherry"}"#).unwrap();
        let w = generate_presentation(&test_design(), &PresentationOptions { wood: Some(wood), ..opts() });
        assert!(!w.svg.contains(FLAT_FRAME_COLOR) && w.svg.contains("clipPath"));
    }

    #[test]
    fn wall_leaves_the_window_open() {
        let r = generate_presentation(&test_design(), &PresentationOptions { background: Some("#ECE8E1".into()), ..opts() });
        assert!(r.svg.contains(r##"fill="#ECE8E1" fill-rule="evenodd""##));
    }

    #[test]
    fn filled_photo_covers_the_artwork_and_clamps_its_pan() {
        let art = Rect::new(10.0, 20.0, 300.0, 200.0);
        let (r, f) = PhotoFit::default().place(&art, 1000.0, 1000.0);
        assert!(inside(&art, &r));
        assert!((r.width - 300.0).abs() < 1e-9 && f.center_y == 0.5);

        let far = PhotoFit { center_y: 0.0, zoom: 9.0, ..PhotoFit::default() };
        let (r, f) = far.place(&art, 1000.0, 1000.0);
        assert_eq!(f.zoom, MAX_ZOOM);
        assert!((r.top() - art.top()).abs() < 1e-9, "panned to the top edge, no further");
        assert!(inside(&art, &r) && f.center_y > 0.0);
    }

    #[test]
    fn fitted_photo_sits_inside_the_artwork_centred() {
        let art = Rect::new(0.0, 0.0, 300.0, 200.0);
        let (r, f) = PhotoFit { mode: FitMode::Fit, center_x: 0.0, ..PhotoFit::default() }.place(&art, 1000.0, 1000.0);
        assert!(inside(&r, &art));
        assert!((r.width - 200.0).abs() < 1e-9 && (r.x - 50.0).abs() < 1e-9 && f.center_x == 0.5);
    }

    #[test]
    fn degenerate_inputs_dont_panic() {
        let (r, _) = PhotoFit { zoom: f64::NAN, ..PhotoFit::default() }.place(&Rect::new(0.0, 0.0, 10.0, 10.0), 0.0, 5.0);
        assert_eq!(r.width, 10.0);
        let tiny: PresentationOptions = serde_json::from_str(r#"{"canvas_width": 10, "canvas_height": 10, "margin": 20}"#).unwrap();
        assert!(generate_presentation(&test_design(), &tiny).svg.starts_with("<svg"));
    }

    #[test]
    fn preview_layout_uses_the_mat_opening() {
        let d = test_design();
        let style = DiagramStyle::default();
        let l = preview_photo_layout(&d, 400.0, 300.0, &style, Some(&photo(800.0, 600.0, None)));
        let g = PlanViewGeometry::from_design_preview(&d, 400.0, 300.0, &style);
        assert_eq!(l.window, g.mat_opening.unwrap());
        assert!(inside(&l.artwork, &l.photo.unwrap()));
    }
}
