//! Wood-grain generator utility (WOOD_GRAIN_PLAN.md phase 1).
//!
//!   cargo run --example wood_grain -- sizes            # preview payload per species
//!   cargo run --example wood_grain -- boards <dir> [n] # 3"x6" boards @150 px/in, n seeds (default 3)
//!   cargo run --example wood_grain -- frames <dir>     # preview-scale frames per species
//!   cargo run --example wood_grain -- diagrams <dir>   # full diagrams with `DiagramOptions::wood`
//!   cargo run --example wood_grain -- swatches <dir>   # picker swatches (192 px) per species with a look,
//!                                                      # per tone, reshuffles 0-2
//!
//! Boards feed the private tools/wood-fit validation (statistics vs the prototype).

use referenceframe_core::presets::get_materials;
use referenceframe_core::visualization::{
    board_svg, frame_face_svg, generate_diagram_with_style, wood_appearance, wood_looks, wood_swatch_svg, DepthCues,
    DiagramOptions, DiagramStyle, FaceDepths, FrameFace, ViewOption, WoodAppearance, WoodLod, WoodRender, WoodTone,
};
use referenceframe_core::{FrameDesign, FrameStyle};
use std::path::Path;

const DEPTHS: FaceDepths = FaceDepths { frame_material_depth: 0.75, rabbet_depth: 0.375, glazing_thickness: 0.093 };

fn species() -> Vec<(String, &'static WoodAppearance)> {
    let mut v: Vec<_> = get_materials().woods.keys().map(|k| (k.clone(), wood_appearance(k, None))).collect();
    v.push(("white_oak_qs".into(), wood_appearance("white_oak", Some("quartersawn"))));
    v
}

/// Preview-like frame: 18x22" outer, 1.5" moulding at `ppi`.
fn frame_doc(a: &WoodAppearance, ppi: f64, lod: WoodLod) -> String {
    let (w, h, fw, pad) = (18.0 * ppi, 22.0 * ppi, 1.5 * ppi, 20.0);
    let f = frame_face_svg(&FrameFace {
        appearance: a, x: pad, y: pad, width: w, height: h, frame_width: fw, px_per_in: ppi, seed: 5, lod,
        depth: DepthCues::InnerAndWall, depths: DEPTHS, id_prefix: "wg-",
    });
    let (tw, th) = (w + 2.0 * pad, h + 2.0 * pad);
    let (ix, iw, ih) = (pad + fw, w - 2.0 * fw, h - 2.0 * fw);
    let (under, face, inner) = (f.under, f.face, f.inner_shadow);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{tw}" height="{th}" viewBox="0 0 {tw} {th}"><rect width="{tw}" height="{th}" fill="#fff"/>{under}{face}<rect x="{pad}" y="{pad}" width="{w}" height="{h}" fill="none" stroke="#333" stroke-width="1.5"/><rect x="{ix}" y="{ix}" width="{iw}" height="{ih}" fill="#f4f1ea"/>{inner}<rect x="{ix}" y="{ix}" width="{iw}" height="{ih}" fill="none" stroke="#333" stroke-width="1.2"/></svg>"##
    )
}

fn design(art_w: f64, art_h: f64, mat: f64, fw: f64) -> FrameDesign {
    FrameDesign {
        artwork_width: art_w, artwork_height: art_h, mat_width_top_bottom: mat, mat_width_sides: mat,
        mat_overlap: 0.125, rabbet_width: 0.375, rabbet_depth: 0.375, frame_material_width: fw,
        matboard_thickness: 0.055, artwork_thickness: 0.008, backing_thickness: 0.125, glazing_thickness: 0.093,
        frame_material_depth: 0.75, assembly_margin: 0.0625, symmetrical_mat: true, no_artwork_margin: false,
        frame_style: FrameStyle::Rabbet, float_reveal: 0.0,
    }
}

/// Review cases: (name, design, view, species, callouts, depth, dark)
fn diagrams(dir: &Path) {
    let cases = [
        ("plan_matted_16x20_red_oak", design(20.0, 16.0, 2.0, 1.5), ViewOption::PlanOnly, "red_oak", true, DepthCues::Inner, false),
        ("plan_8x10_douglas_fir", design(10.0, 8.0, 0.0, 1.0), ViewOption::PlanOnly, "douglas_fir", true, DepthCues::Inner, false),
        ("plan_tall_8x60_generic", design(8.0, 60.0, 0.0, 1.0), ViewOption::PlanOnly, "generic", true, DepthCues::Inner, false),
        ("plan_dual_break_white_ash", design(80.0, 80.0, 0.0, 1.0), ViewOption::PlanOnly, "white_ash", true, DepthCues::Inner, false),
        ("plan_small_4x6_hard_maple", design(6.0, 4.0, 0.0, 0.75), ViewOption::PlanOnly, "hard_maple", true, DepthCues::Inner, false),
        ("plan_matted_16x20_walnut_dark", design(20.0, 16.0, 2.0, 1.5), ViewOption::PlanOnly, "black_walnut", true, DepthCues::Inner, true),
        ("both_matted_16x20_cherry", design(20.0, 16.0, 2.0, 1.5), ViewOption::Both, "black_cherry", true, DepthCues::Inner, false),
        ("preview_16x20_walnut", design(16.0, 20.0, 2.0, 1.5), ViewOption::PlanOnly, "black_walnut", false, DepthCues::InnerAndWall, false),
        ("preview_16x20_walnut_dark", design(16.0, 20.0, 2.0, 1.5), ViewOption::PlanOnly, "black_walnut", false, DepthCues::InnerAndWall, true),
    ];
    for (name, d, view, species, callouts, depth, dark) in cases {
        let options = DiagramOptions {
            view, show_callouts: callouts,
            canvas_width: if callouts { 800.0 } else { 400.0 }, canvas_height: if callouts { 600.0 } else { 500.0 },
            wood: Some(WoodRender { species: species.into(), variant: None, tone: WoodTone::Natural, lod: WoodLod::Grain, depth, reshuffle: 0 }),
            ..Default::default()
        };
        let style = if dark { DiagramStyle::for_dark() } else { DiagramStyle::default() };
        let svg = generate_diagram_with_style(&d, &options, &style).svg;
        // standalone files need an explicit size + background to rasterize like the apps show them
        let svg = svg.replacen("<svg ", &format!(r#"<svg style="background:{}" "#, style.background_color), 1);
        std::fs::write(dir.join(format!("{name}.svg")), &svg).unwrap();
        println!("{name:<34} {:>6.1} KB", svg.len() as f64 / 1024.0);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("boards") => {
            let dir = Path::new(args.get(2).expect("output dir"));
            std::fs::create_dir_all(dir).unwrap();
            let n: u32 = args.get(3).map_or(3, |s| s.parse().unwrap());
            for (key, a) in species() {
                for seed in (1..=n).map(|k| 11 * k) {
                    std::fs::write(dir.join(format!("{key}_{seed}.svg")), board_svg(a, 3.0, 6.0, 150.0, seed)).unwrap();
                }
            }
        }
        Some("diagrams") => {
            let dir = Path::new(args.get(2).expect("output dir"));
            std::fs::create_dir_all(dir).unwrap();
            diagrams(dir);
        }
        Some("swatches") => {
            let dir = Path::new(args.get(2).expect("output dir"));
            std::fs::create_dir_all(dir).unwrap();
            for key in wood_looks() {
                for (tone, name) in [(WoodTone::Light, "light"), (WoodTone::Natural, "natural"), (WoodTone::Dark, "dark")] {
                    for reshuffle in 0..3 {
                        let svg = wood_swatch_svg(key, tone, reshuffle, 192.0);
                        if (tone, reshuffle) == (WoodTone::Natural, 0) {
                            println!("{key:<22} {:>5.1} KB", svg.len() as f64 / 1024.0);
                        }
                        std::fs::write(dir.join(format!("{key}_{name}_{reshuffle}.svg")), svg).unwrap();
                    }
                }
            }
        }
        Some("frames") => {
            let dir = Path::new(args.get(2).expect("output dir"));
            std::fs::create_dir_all(dir).unwrap();
            for (key, a) in species() {
                std::fs::write(dir.join(format!("{key}.svg")), frame_doc(a, 18.0, WoodLod::Grain)).unwrap();
            }
        }
        _ => {
            println!("{:<22} {:>9} {:>7} {:>9} {:>7} {:>8}   (KB / elements, Grain LOD, 18x22\" frame, 1.5\" moulding)",
                "species", "18 px/in", "elems", "30 px/in", "elems", "flat@30");
            for (key, a) in species() {
                let doc = |ppi: f64, lod| frame_doc(a, ppi, lod);
                let (d18, d30) = (doc(18.0, WoodLod::Grain), doc(30.0, WoodLod::Grain));
                let n = |d: &str| d.matches("<path").count() + d.matches("<rect").count() + d.matches("<line").count();
                println!("{key:<22} {:>9.1} {:>7} {:>9.1} {:>7} {:>8.1}", d18.len() as f64 / 1024.0, n(&d18),
                    d30.len() as f64 / 1024.0, n(&d30), doc(30.0, WoodLod::Flat).len() as f64 / 1024.0);
            }
        }
    }
}
