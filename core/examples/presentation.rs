//! Presentation renders for review (docs/plans/PHOTO_PREVIEW_PLAN.md).
//!
//!   cargo run --example presentation -- <out_dir> [photo.jpg]
//!
//! Writes one SVG per case: mat colours, wood on/off, no mat, fill/fit, placeholder.

use referenceframe_core::presets::get_mat_colors;
use referenceframe_core::visualization::{
    generate_presentation, FitMode, PhotoFit, PresentationOptions, PresentationPhoto, WoodRender,
};
use referenceframe_core::FrameDesign;
use std::path::Path;

fn jpeg_size(b: &[u8]) -> Option<(f64, f64)> {
    // SOFn marker: FF C0..CF (not C4/C8/CC), height then width
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            return None;
        }
        let m = b[i + 1];
        let len = usize::from(b[i + 2]) << 8 | usize::from(b[i + 3]);
        if (0xC0..=0xCF).contains(&m) && ![0xC4, 0xC8, 0xCC].contains(&m) {
            let h = f64::from(u16::from(b[i + 5]) << 8 | u16::from(b[i + 6]));
            let w = f64::from(u16::from(b[i + 7]) << 8 | u16::from(b[i + 8]));
            return Some((w, h));
        }
        i += 2 + len;
    }
    None
}

fn b64(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in b.chunks(3) {
        let n = u32::from(c[0]) << 16 | u32::from(*c.get(1).unwrap_or(&0)) << 8 | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= c.len() {
                s.push(T[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(args.get(1).expect("out dir"));
    std::fs::create_dir_all(out).unwrap();
    let photo = args.get(2).map(|p| {
        let b = std::fs::read(p).unwrap();
        let (w, h) = jpeg_size(&b).expect("a JPEG");
        PresentationPhoto { width: w, height: h, fit: PhotoFit::default(), href: Some(format!("data:image/jpeg;base64,{}", b64(&b))) }
    });

    let mut d = FrameDesign::new(8.0, 12.0);
    d.mat_width_top_bottom = 2.5;
    d.mat_width_sides = 2.5;
    d.frame_material_width = 1.25;
    d.frame_material_depth = 1.0;
    let mut no_mat = d.clone();
    no_mat.mat_width_top_bottom = 0.0;
    no_mat.mat_width_sides = 0.0;

    let base = PresentationOptions {
        canvas_width: 900.0,
        canvas_height: 700.0,
        wood: None,
        frame_color: None,
        mat_color: None,
        photo: photo.clone(),
        background: Some("#ECE9E3".into()),
        margin: 24.0,
    };
    let walnut: WoodRender = serde_json::from_str(r#"{"species": "black_walnut"}"#).unwrap();
    let oak: WoodRender = serde_json::from_str(r#"{"species": "white_oak", "tone": "light"}"#).unwrap();
    let mut cases: Vec<(String, FrameDesign, PresentationOptions)> = vec![
        ("flat".into(), d.clone(), base.clone()),
        ("walnut".into(), d.clone(), PresentationOptions { wood: Some(walnut.clone()), ..base.clone() }),
        ("oak_no_mat".into(), no_mat, PresentationOptions { wood: Some(oak), ..base.clone() }),
        ("placeholder".into(), d.clone(), PresentationOptions { photo: None, wood: Some(walnut.clone()), ..base.clone() }),
    ];
    if let Some(p) = &photo {
        let fit = PresentationPhoto { fit: PhotoFit { mode: FitMode::Fit, ..PhotoFit::default() }, ..p.clone() };
        cases.push(("fit".into(), d.clone(), PresentationOptions { photo: Some(fit), wood: Some(walnut.clone()), ..base.clone() }));
        let mut portrait = d.clone();
        portrait.artwork_width = 8.0;
        portrait.artwork_height = 12.0;
        let zoom = PresentationPhoto { fit: PhotoFit { zoom: 2.0, center_x: 0.8, center_y: 0.3, ..PhotoFit::default() }, ..p.clone() };
        cases.push(("portrait_zoom".into(), portrait, PresentationOptions { photo: Some(zoom), wood: Some(walnut.clone()), ..base.clone() }));
    }
    for c in &get_mat_colors().colors {
        cases.push((format!("mat_{}", c.key), d.clone(), PresentationOptions { mat_color: Some(c.key.clone()), wood: Some(walnut.clone()), ..base.clone() }));
    }
    for (name, design, o) in cases {
        let r = generate_presentation(&design, &o);
        std::fs::write(out.join(format!("{name}.svg")), &r.svg).unwrap();
        println!("{name:16} {:>7} B  window {:.0}x{:.0}", r.svg.len(), r.layout.window.width, r.layout.window.height);
    }
}
