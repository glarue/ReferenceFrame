//! Wood-grain generator utility (WOOD_GRAIN_PLAN.md phase 1).
//!
//!   cargo run --example wood_grain -- sizes            # preview payload per species
//!   cargo run --example wood_grain -- boards <dir> [n] # 3"x6" boards @150 px/in, n seeds (default 3)
//!   cargo run --example wood_grain -- frames <dir>     # preview-scale frames per species
//!
//! Boards feed the private tools/wood-fit validation (statistics vs the prototype).

use referenceframe_core::presets::get_materials;
use referenceframe_core::visualization::{
    board_svg, frame_face_svg, wood_appearance, DepthCues, FaceDepths, FrameFace, WoodAppearance, WoodLod,
};
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
        Some("frames") => {
            let dir = Path::new(args.get(2).expect("output dir"));
            std::fs::create_dir_all(dir).unwrap();
            for (key, a) in species() {
                std::fs::write(dir.join(format!("{key}.svg")), frame_doc(a, 18.0, WoodLod::Grain)).unwrap();
            }
        }
        _ => {
            println!("{:<22} {:>9} {:>9} {:>9}   (KB, Grain LOD, 18x22\" frame, 1.5\" moulding)", "species", "18 px/in", "30 px/in", "flat@30");
            for (key, a) in species() {
                let kb = |ppi: f64, lod| frame_doc(a, ppi, lod).len() as f64 / 1024.0;
                println!("{key:<22} {:>9.1} {:>9.1} {:>9.1}", kb(18.0, WoodLod::Grain), kb(30.0, WoodLod::Grain), kb(30.0, WoodLod::Flat));
            }
        }
    }
}
