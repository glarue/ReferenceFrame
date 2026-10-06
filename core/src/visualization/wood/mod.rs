//! Procedural wood-grain frame faces (docs/plans/WOOD_GRAIN_PLAN.md).
//!
//! Vector-only output (no SVG filters, no percentage lengths), so it renders the
//! same in browsers and flutter_svg. Per-species parameters come from
//! `core/data/wood_appearance.json`.
//!
//! Diagrams opt in through `DiagramOptions::wood` ([`WoodRender`]); with it unset
//! the output is unchanged.

mod appearance;
mod grain;
mod noise;
mod path;

pub use appearance::{wood_appearance, Figure, LineMode, WoodAppearance, WoodPalette, WoodParams, WoodStructure};
pub use grain::GrainStats;

use crate::frame::FrameDesign;
use noise::mix32;
use serde::{Deserialize, Serialize};
use path::{num, op2, poly_d};
use std::fmt::Write;

/// Level of detail. `Flat` is for animation frames: base colour, tone, seams and
/// depth cues only (cheap enough to regenerate every frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WoodLod {
    Flat,
    #[default]
    Grain,
}

/// Depth cues (light from the upper left).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthCues {
    None,
    /// Lip shadow on the mat/art along the top and left inner edges (plan view)
    #[default]
    Inner,
    /// Inner shadow plus the frame's shadow on the wall, bottom and right (live preview)
    InnerAndWall,
}

/// Heights that size the depth cues (inches).
#[derive(Debug, Clone, Copy)]
pub struct FaceDepths {
    /// Overall frame thickness: how far the frame stands off the wall
    pub frame_material_depth: f64,
    pub rabbet_depth: f64,
    pub glazing_thickness: f64,
}

impl FaceDepths {
    pub fn from_design(d: &FrameDesign) -> Self {
        Self {
            frame_material_depth: d.frame_material_depth,
            rabbet_depth: d.rabbet_depth,
            glazing_thickness: d.glazing_thickness,
        }
    }

    /// How far the face stands above the mat/art: the lip (in front of the rabbet) plus the glazing.
    fn face_above_mat(&self) -> f64 {
        (self.frame_material_depth - self.rabbet_depth + self.glazing_thickness).max(0.0)
    }
}

/// Wood-grain rendering for a diagram (`DiagramOptions::wood`).
///
/// JSON: `{"species": "red_oak", "variant": null, "lod": "grain", "depth": "inner", "reshuffle": 0}`;
/// every field but `species` is optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WoodRender {
    /// A `materials.woods` key; unknown keys fall back to the generic wood
    pub species: String,
    /// Figure variant, e.g. `"quartersawn"`
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub lod: WoodLod,
    #[serde(default)]
    pub depth: DepthCues,
    /// Bump to draw different boards for the same species
    #[serde(default)]
    pub reshuffle: u32,
}

impl WoodRender {
    pub fn appearance(&self) -> &'static WoodAppearance {
        wood_appearance(&self.species, self.variant.as_deref())
    }

    /// Diagram face: the frame's outer rectangle in px at `scale` px/in.
    pub(crate) fn frame_face(&self, design: &FrameDesign, x: f64, y: f64, width: f64, height: f64, scale: f64) -> FrameFaceSvg {
        // ids must be unique per inline SVG on a page (web history renders several)
        let id_prefix = format!("wg{:x}-", mix32(self.seed() ^ (width * 64.0) as u32 ^ ((height * 64.0) as u32).rotate_left(16)));
        frame_face_svg(&self.face(design, x, y, width, height, scale, &id_prefix))
    }

    /// The corner-detail inset's zoomed corner at `scale` px/in (see [`corner_face_svg`]).
    pub(crate) fn corner_face(&self, design: &FrameDesign, cx: f64, cy: f64, right: f64, up: f64, scale: f64) -> String {
        let id_prefix = format!("wd{:x}-", mix32(self.seed() ^ (scale * 64.0) as u32));
        corner_face_svg(&self.face(design, 0.0, 0.0, 0.0, 0.0, scale, &id_prefix), cx, cy, right, up)
    }

    #[allow(clippy::too_many_arguments)]
    fn face<'a>(&self, design: &FrameDesign, x: f64, y: f64, width: f64, height: f64, scale: f64, id_prefix: &'a str) -> FrameFace<'a> {
        FrameFace {
            appearance: self.appearance(),
            x, y, width, height,
            frame_width: design.frame_material_width * scale,
            px_per_in: scale,
            seed: self.seed(),
            lod: self.lod,
            depth: self.depth,
            depths: FaceDepths::from_design(design),
            id_prefix,
        }
    }

    /// Stable per species/variant: the same design always shows the same boards.
    pub fn seed(&self) -> u32 {
        seed_for(&self.species, self.variant.as_deref(), self.reshuffle)
    }
}

/// FNV-1a of the species (and variant) key, mixed with the reshuffle counter.
pub fn seed_for(species: &str, variant: Option<&str>, reshuffle: u32) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in species.bytes().chain(variant.into_iter().flat_map(|v| std::iter::once(b'/').chain(v.bytes()))) {
        h = (h ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }
    mix32(h ^ reshuffle.wrapping_mul(0x9E37_79B9))
}

/// Shadow length per inch of height (light ~60° from the wall plane).
const DEPTH_K: f64 = 0.6;

/// A frame face to draw: the outer rectangle and moulding width in px.
pub struct FrameFace<'a> {
    pub appearance: &'a WoodAppearance,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub frame_width: f64,
    pub px_per_in: f64,
    /// Selects the boards; the four pieces get independent sub-seeds
    pub seed: u32,
    pub lod: WoodLod,
    pub depth: DepthCues,
    pub depths: FaceDepths,
    /// Prefix for clipPath/gradient ids (unique per diagram on a page)
    pub id_prefix: &'a str,
}

/// SVG fragments, in drawing order relative to the rest of the diagram.
pub struct FrameFaceSvg {
    /// Beneath the frame (wall shadow)
    pub under: String,
    /// The four mitered pieces with grain, grain-direction tone and miter seams
    pub face: String,
    /// Over the mat/art inside the opening, beneath the opening's outline stroke
    pub inner_shadow: String,
    pub stats: GrainStats,
}

/// Draw a frame face: four mitered pieces, each clipped to its trapezoid.
pub fn frame_face_svg(f: &FrameFace) -> FrameFaceSvg {
    let (x0, y0, w, h, fw, ppi) = (f.x, f.y, f.width, f.height, f.frame_width, f.px_per_in);
    let mut stats = GrainStats::default();

    let mut under = String::new();
    if f.depth == DepthCues::InnerAndWall {
        // soft drop shadow without filters: stacked offset rects, each minus the
        // frame's own rectangle so nothing darkens the opening (evenodd hole)
        let s = f.depths.frame_material_depth * DEPTH_K * ppi;
        for k in 1..=6 {
            let o = s * f64::from(k) / 6.0;
            let _ = write!(under, r##"<path d="M{},{}h{}v{}h{}zM{},{}v{}h{}v{}z" fill="#000" fill-opacity="0.05" fill-rule="evenodd"/>"##,
                num(x0 + o), num(y0 + o), num(w), num(h), num(-w), num(x0), num(y0), num(h), num(w), num(-h));
        }
    }

    // (origin, u axis, v axis, length): top, right, bottom, left; v points inward
    let sides = [
        ((x0, y0), (1.0, 0.0), (0.0, 1.0), w),
        ((x0 + w, y0), (0.0, 1.0), (-1.0, 0.0), h),
        ((x0 + w, y0 + h), (-1.0, 0.0), (0.0, -1.0), w),
        ((x0, y0 + h), (0.0, -1.0), (1.0, 0.0), h),
    ];
    let mut face = String::new();
    for (i, ((ox, oy), (ax, ay), (bx, by), len)) in sides.into_iter().enumerate() {
        let piece = Piece { side: i, origin: (ox, oy), u: (ax, ay), v: (bx, by), len, fw };
        piece.draw(&mut face, &mut stats, f, &[(0.0, 0.0), (len, 0.0), (len - fw, fw), (fw, fw)]);
    }
    for ((ax, ay), (bx, by)) in [
        ((x0, y0), (x0 + fw, y0 + fw)),
        ((x0 + w, y0), (x0 + w - fw, y0 + fw)),
        ((x0 + w, y0 + h), (x0 + w - fw, y0 + h - fw)),
        ((x0, y0 + h), (x0 + fw, y0 + h - fw)),
    ] {
        let _ = write!(face, r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#000" stroke-opacity="0.45" stroke-width="0.8"/>"##,
            num(ax), num(ay), num(bx), num(by));
    }

    let mut inner_shadow = String::new();
    if f.depth != DepthCues::None {
        let sw = f.depths.face_above_mat() * DEPTH_K * ppi;
        let (ix, iy, iw, ih) = (x0 + fw, y0 + fw, w - 2.0 * fw, h - 2.0 * fw);
        let p = f.id_prefix;
        let _ = write!(inner_shadow,
            r##"<linearGradient id="{p}ist" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#000" stop-opacity="0.32"/><stop offset="1" stop-color="#000" stop-opacity="0"/></linearGradient>"##);
        let _ = write!(inner_shadow,
            r##"<linearGradient id="{p}isl" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#000" stop-opacity="0.32"/><stop offset="1" stop-color="#000" stop-opacity="0"/></linearGradient>"##);
        let _ = write!(inner_shadow, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="url(#{p}ist)"/>"#, num(ix), num(iy), num(iw), num(sw));
        let _ = write!(inner_shadow, r#"<rect x="{}" y="{}" width="{}" height="{}" fill="url(#{p}isl)"/>"#, num(ix), num(iy), num(sw), num(ih));
    }

    FrameFaceSvg { under, face, inner_shadow, stats }
}

/// One mitered piece in local coordinates: u along the piece, v inward from its
/// outer edge. Sides are numbered top, right, bottom, left; odd sides run vertically.
struct Piece {
    side: usize,
    origin: (f64, f64),
    u: (f64, f64),
    v: (f64, f64),
    len: f64,
    fw: f64,
}

impl Piece {
    /// Base colour, grain and grain-direction tone, clipped to `clip` (local polygon).
    fn draw(&self, out: &mut String, stats: &mut GrainStats, f: &FrameFace, clip: &[(f64, f64)]) {
        let a = f.appearance;
        let (Piece { side, len, fw, .. }, (ox, oy), (ax, ay), (bx, by)) = (self, self.origin, self.u, self.v);
        let cid = format!("{}c{side}", f.id_prefix);
        let _ = write!(out, r#"<clipPath id="{cid}"><path d="{}"/></clipPath>"#, poly_d(clip, true));
        // The clip goes on an inner group: some renderers (the Dart `pdf` package) resolve
        // clip-path in the parent's space, ignoring the element's own transform
        let _ = write!(out, r#"<g transform="matrix({ax},{ay},{bx},{by},{},{})"><g clip-path="url(#{cid})">"#, num(ox), num(oy));
        let _ = write!(out, r#"<rect x="-20" y="-5" width="{}" height="{}" fill="{}"/>"#, num(len + 40.0), num(fw + 10.0), a.palette.base);
        if f.lod == WoodLod::Grain {
            let side_seed = mix32(f.seed.wrapping_add((*side as u32).wrapping_mul(0x9E37_79B9)));
            let (g, st) = grain::side_grain(a, *len, *fw, f.px_per_in, side_seed);
            out.push_str(&g);
            stats.streaks += st.streaks;
            stats.zones += st.zones;
            stats.lines += st.lines;
            stats.pores += st.pores;
            stats.flecks += st.flecks;
        }
        if side % 2 == 1 && a.params.grain_tone > 0.0 {
            // flat faces: the vertical pieces' grain runs at 90° to the horizontal ones'
            let _ = write!(out, r##"<rect x="-20" y="-5" width="{}" height="{}" fill="#000" fill-opacity="{}"/>"##,
                num(len + 40.0), num(fw + 10.0), op2(a.params.grain_tone));
        }
        out.push_str("</g></g>");
    }
}

/// The bottom-left corner of a frame face, zoomed (the plan view's corner-detail
/// inset): `(cx, cy)` is the outer corner, the bottom piece runs `right` px to the
/// right and the left piece `up` px upward, both cut off square (the inset clips them).
/// Same boards (sub-seeds) as the full face's bottom and left sides; no depth cues.
pub fn corner_face_svg(f: &FrameFace, cx: f64, cy: f64, right: f64, up: f64) -> String {
    let fw = f.frame_width;
    let mut out = String::new();
    let mut stats = GrainStats::default();
    // bottom: u runs leftward from the far end to the corner, as on the full face
    Piece { side: 2, origin: (cx + right, cy), u: (-1.0, 0.0), v: (0.0, -1.0), len: right, fw }
        .draw(&mut out, &mut stats, f, &[(0.0, 0.0), (right, 0.0), (right - fw, fw), (0.0, fw)]);
    Piece { side: 3, origin: (cx, cy), u: (0.0, -1.0), v: (1.0, 0.0), len: up, fw }
        .draw(&mut out, &mut stats, f, &[(0.0, 0.0), (up, 0.0), (up, fw), (fw, fw)]);
    let _ = write!(out, r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#000" stroke-opacity="0.45" stroke-width="0.8"/>"##,
        num(cx), num(cy), num(cx + fw), num(cy - fw));
    out
}

/// A flat, vertical-grain board (like the reference sample photos) as a standalone
/// SVG document -- for validating the port against the prototype's statistics.
pub fn board_svg(a: &WoodAppearance, w_in: f64, h_in: f64, ppi: f64, seed: u32) -> String {
    let (w, h) = (w_in * ppi, h_in * ppi);
    let (g, _) = grain::side_grain(a, h, w, ppi, seed);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{0}" height="{1}" viewBox="0 0 {0} {1}"><rect width="{0}" height="{1}" fill="{2}"/><g transform="matrix(0,1,1,0,0,0)">{g}</g></svg>"#,
        num(w), num(h), a.palette.base
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face<'a>(a: &'a WoodAppearance, lod: WoodLod, depth: DepthCues) -> FrameFace<'a> {
        // 18x22" outer, 1.5" moulding at 30 px/in: a generous live-preview scale
        FrameFace {
            appearance: a, x: 10.0, y: 10.0, width: 540.0, height: 660.0, frame_width: 45.0, px_per_in: 30.0,
            seed: 7, lod, depth, depths: FaceDepths { frame_material_depth: 0.75, rabbet_depth: 0.375, glazing_thickness: 0.093 },
            id_prefix: "wg-",
        }
    }

    fn all_species() -> Vec<&'static WoodAppearance> {
        let mut v: Vec<_> = crate::presets::get_materials().woods.keys().map(|k| wood_appearance(k, None)).collect();
        v.push(wood_appearance("white_oak", Some("quartersawn")));
        v
    }

    #[test]
    fn flutter_svg_safe_output() {
        for a in all_species() {
            let s = frame_face_svg(&face(a, WoodLod::Grain, DepthCues::InnerAndWall));
            let corner = corner_face_svg(&FrameFace { px_per_in: 90.0, frame_width: 90.0, ..face(a, WoodLod::Grain, DepthCues::None) },
                20.0, 300.0, 280.0, 250.0);
            let all = format!("{}{}{}{corner}", s.under, s.face, s.inner_shadow);
            assert!(!all.contains('%'), "percentage lengths throw in flutter_svg");
            assert!(!all.contains("<filter") && !all.contains("<pattern"));
            assert!(!all.contains("NaN") && !all.contains("inf"));
            assert_eq!(all.matches("<g ").count(), all.matches("</g>").count());
        }
    }

    #[test]
    fn preview_payload_within_budget() {
        // Typical live-preview scale: 18x22" frame, 1.5" moulding at 20 px/in. Budget from
        // the flutter_svg device test (~8.5 ms to parse 100 KB of grain, once per settle):
        // <= 60 KB per species, <= 80 KB for quartersawn (hundreds of ray flecks).
        let size = |a: &WoodAppearance| {
            let s = frame_face_svg(&FrameFace { width: 360.0, height: 440.0, frame_width: 30.0, px_per_in: 20.0,
                ..face(a, WoodLod::Grain, DepthCues::InnerAndWall) });
            (s.under.len() + s.face.len() + s.inner_shadow.len()) as f64 / 1024.0
        };
        for key in crate::presets::get_materials().woods.keys() {
            let kb = size(wood_appearance(key, None));
            assert!(kb <= 60.0, "{key}: {kb:.1} KB");
        }
        let kb = size(wood_appearance("white_oak", Some("quartersawn")));
        assert!(kb <= 80.0, "white_oak quartersawn: {kb:.1} KB");
    }

    #[test]
    fn flat_lod_has_no_grain() {
        let s = frame_face_svg(&face(wood_appearance("red_oak", None), WoodLod::Flat, DepthCues::Inner));
        assert_eq!(s.stats, GrainStats::default());
        assert_eq!(s.face.matches("<path").count(), 4, "only the four clip paths");
        assert!(s.face.len() < 2048);
        assert!(s.under.is_empty() && !s.inner_shadow.is_empty());
    }

    #[test]
    fn depth_cues_scale_with_design() {
        let a = wood_appearance("generic", None);
        let mut f = face(a, WoodLod::Flat, DepthCues::InnerAndWall);
        let thin = frame_face_svg(&f);
        f.depths.frame_material_depth = 1.5;
        let thick = frame_face_svg(&f);
        assert_ne!(thin.under, thick.under);
        assert_ne!(thin.inner_shadow, thick.inner_shadow);
        f.depth = DepthCues::None;
        let none = frame_face_svg(&f);
        assert!(none.under.is_empty() && none.inner_shadow.is_empty());
    }

    #[test]
    fn diagram_payload_within_budget() {
        // Diagrams render once per design change, not per animation frame: <= 100 KB
        // per species (<= 125 KB quartersawn: ray flecks, dense again in the zoomed
        // corner-detail inset) on the 800x600 canvas, from small/high-zoom frames to
        // long thin sides (axis breaks) at low zoom, and for the preview.
        use crate::visualization::{generate_diagram, DiagramOptions};
        let designs = [(10.0, 8.0, 0.0, 1.0), (20.0, 16.0, 2.0, 1.5), (8.0, 60.0, 0.0, 1.0), (80.0, 80.0, 0.0, 1.0), (6.0, 4.0, 0.0, 0.75)];
        let mut keys: Vec<(String, Option<String>, f64)> =
            crate::presets::get_materials().woods.keys().map(|k| (k.clone(), None, 100.0)).collect();
        keys.push(("white_oak".into(), Some("quartersawn".into()), 125.0));
        for (aw, ah, mat, fw) in designs {
            let mut d = FrameDesign::new(aw, ah);
            (d.mat_width_top_bottom, d.mat_width_sides, d.frame_material_width) = (mat, mat, fw);
            for (species, variant, budget) in &keys {
                for show_callouts in [true, false] {
                    let wood = WoodRender { species: species.clone(), variant: variant.clone(), lod: WoodLod::Grain,
                        depth: DepthCues::InnerAndWall, reshuffle: 0 };
                    let o = DiagramOptions { show_callouts, wood: Some(wood), ..Default::default() };
                    let kb = generate_diagram(&d, &o).svg.len() as f64 / 1024.0;
                    assert!(kb <= *budget, "{species} {variant:?} {aw}x{ah} callouts={show_callouts}: {kb:.1} KB");
                }
            }
        }
    }

    #[test]
    fn seed_is_stable_per_species() {
        assert_eq!(seed_for("red_oak", None, 0), seed_for("red_oak", None, 0));
        let seeds = [seed_for("red_oak", None, 0), seed_for("red_oak", None, 1), seed_for("white_oak", None, 0),
            seed_for("white_oak", Some("quartersawn"), 0)];
        for (i, a) in seeds.iter().enumerate() {
            assert!(seeds[i + 1..].iter().all(|b| a != b), "{seeds:?}");
        }
    }

    #[test]
    fn deterministic_frame() {
        let a = wood_appearance("black_walnut", None);
        let x = frame_face_svg(&face(a, WoodLod::Grain, DepthCues::Inner)).face;
        let y = frame_face_svg(&face(a, WoodLod::Grain, DepthCues::Inner)).face;
        assert_eq!(x, y);
    }
}
