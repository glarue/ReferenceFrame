//! Procedural wood-grain frame faces (docs/plans/WOOD_GRAIN_PLAN.md).
//!
//! Vector-only output (no SVG filters, no percentage lengths), so it renders the
//! same in browsers and flutter_svg. Per-species parameters come from
//! `core/data/wood_appearance.json`.
//!
//! Phase 1: the generator and data only. Nothing in the diagram pipeline calls
//! it yet, so existing diagrams and golden SVGs are unchanged.

mod appearance;
mod grain;
mod noise;
mod path;

pub use appearance::{wood_appearance, Figure, LineMode, WoodAppearance, WoodPalette, WoodParams, WoodStructure};
pub use grain::GrainStats;

use crate::frame::FrameDesign;
use noise::mix32;
use path::{num, op2, poly_d};
use std::fmt::Write;

/// Level of detail. `Flat` is for animation frames: base colour, tone, seams and
/// depth cues only (cheap enough to regenerate every frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoodLod {
    Flat,
    Grain,
}

/// Depth cues (light from the upper left).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthCues {
    None,
    /// Lip shadow on the mat/art along the top and left inner edges (plan view)
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
    let a = f.appearance;
    let (x0, y0, w, h, fw, ppi) = (f.x, f.y, f.width, f.height, f.frame_width, f.px_per_in);
    let mut stats = GrainStats::default();

    let mut under = String::new();
    if f.depth == DepthCues::InnerAndWall {
        // soft drop shadow without filters: stacked offset rects
        let s = f.depths.frame_material_depth * DEPTH_K * ppi;
        for k in 1..=6 {
            let o = s * f64::from(k) / 6.0;
            let _ = write!(under, r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#000" fill-opacity="0.05"/>"##,
                num(x0 + o), num(y0 + o), num(w), num(h));
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
        let cid = format!("{}c{i}", f.id_prefix);
        let _ = write!(face, r#"<clipPath id="{cid}"><path d="{}"/></clipPath>"#,
            poly_d(&[(0.0, 0.0), (len, 0.0), (len - fw, fw), (fw, fw)], true));
        let _ = write!(face, r#"<g transform="matrix({ax},{ay},{bx},{by},{},{})" clip-path="url(#{cid})">"#, num(ox), num(oy));
        let _ = write!(face, r#"<rect x="-20" y="-5" width="{}" height="{}" fill="{}"/>"#, num(len + 40.0), num(fw + 10.0), a.palette.base);
        if f.lod == WoodLod::Grain {
            let side_seed = mix32(f.seed.wrapping_add((i as u32).wrapping_mul(0x9E37_79B9)));
            let (g, st) = grain::side_grain(a, len, fw, ppi, side_seed);
            face.push_str(&g);
            stats.streaks += st.streaks;
            stats.zones += st.zones;
            stats.lines += st.lines;
            stats.pores += st.pores;
            stats.flecks += st.flecks;
        }
        if i % 2 == 1 && a.params.grain_tone > 0.0 {
            // flat faces: the vertical pieces' grain runs at 90° to the horizontal ones'
            let _ = write!(face, r##"<rect x="-20" y="-5" width="{}" height="{}" fill="#000" fill-opacity="{}"/>"##,
                num(len + 40.0), num(fw + 10.0), op2(a.params.grain_tone));
        }
        face.push_str("</g>");
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
            let all = format!("{}{}{}", s.under, s.face, s.inner_shadow);
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
    fn deterministic_frame() {
        let a = wood_appearance("black_walnut", None);
        let x = frame_face_svg(&face(a, WoodLod::Grain, DepthCues::Inner)).face;
        let y = frame_face_svg(&face(a, WoodLod::Grain, DepthCues::Inner)).face;
        assert_eq!(x, y);
    }
}
