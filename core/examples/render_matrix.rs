//! Render a matrix of representative designs to SVG for visual inspection.
//!
//! Dev harness for the code -> viz -> code iteration loop: renders every
//! matrix case to `<out_dir>` (first CLI arg, default `render_matrix_out`),
//! one SVG per case/view, plus feature renders with the spline/hanging
//! overlay flags enabled. Rasterize with e.g.
//! `qlmanage -t -s 1600 -o <out_dir> <out_dir>/*.svg` and inspect.
//!
//! Not shipped anywhere — this is tooling, not product.

use std::path::PathBuf;

use referenceframe_core::{FrameDesign, FrameStyle};
use referenceframe_core::visualization::{generate_diagram, DiagramOptions, ViewOption};

fn base_design() -> FrameDesign {
    FrameDesign {
        artwork_width: 8.0,
        artwork_height: 10.0,
        mat_width_top_bottom: 0.0,
        mat_width_sides: 0.0,
        mat_overlap: 0.125,
        rabbet_width: 0.375,
        rabbet_depth: 0.375,
        frame_material_width: 1.0,
        matboard_thickness: 0.055,
        artwork_thickness: 0.008,
        backing_thickness: 0.125,
        glazing_thickness: 0.093,
        frame_material_depth: 0.75,
        assembly_margin: 0.0625,
        symmetrical_mat: true,
        no_artwork_margin: false,
        frame_style: FrameStyle::Rabbet,
        float_reveal: 0.0,
    }
}

fn matted_16x20() -> FrameDesign {
    FrameDesign {
        artwork_width: 16.0,
        artwork_height: 20.0,
        mat_width_top_bottom: 2.0,
        mat_width_sides: 2.0,
        ..base_design()
    }
}

fn large_deep() -> FrameDesign {
    // Deep canvas moulding; rabbet depth 1" swallows the 0.9375" stack
    FrameDesign {
        artwork_width: 30.0,
        artwork_height: 40.0,
        frame_material_width: 2.0,
        frame_material_depth: 1.5,
        rabbet_depth: 1.0,
        artwork_thickness: 0.75, // canvas-ish
        glazing_thickness: 0.0,
        ..base_design()
    }
}

fn interference_case() -> FrameDesign {
    // Stack (0.29") overruns a shallow 1/4" rabbet -> INTERFERENCE warning
    FrameDesign {
        rabbet_depth: 0.25,
        ..base_design()
    }
}

fn matrix() -> Vec<(&'static str, FrameDesign)> {
    vec![
        ("rabbet_matted_16x20", matted_16x20()),
        ("rabbet_nomat_8x10", base_design()),
        ("sight_size_11x14", FrameDesign {
            artwork_width: 11.0,
            artwork_height: 14.0,
            frame_style: FrameStyle::SightSize,
            ..base_design()
        }),
        ("tall_4x36", FrameDesign {
            artwork_width: 4.0,
            artwork_height: 36.0,
            ..base_design()
        }),
        ("wide_36x4", FrameDesign {
            artwork_width: 36.0,
            artwork_height: 4.0,
            ..base_design()
        }),
        ("large_30x40_deep", large_deep()),
        ("interference_8x10", interference_case()),
    ]
}

/// Feature renders: (name, design, view, show_spline, show_hanging)
fn feature_cases() -> Vec<(&'static str, FrameDesign, ViewOption, bool, bool)> {
    vec![
        ("real_spline_section_nomat_8x10", base_design(), ViewOption::SectionOnly, true, false),
        ("real_spline_section_large_deep", large_deep(), ViewOption::SectionOnly, true, false),
        ("real_spline_plan_nomat_8x10", base_design(), ViewOption::PlanOnly, true, false),
        ("real_hanging_plan_matted_16x20", matted_16x20(), ViewOption::PlanOnly, false, true),
        ("real_both_plan_matted_16x20", matted_16x20(), ViewOption::PlanOnly, true, true),
        ("real_spline_section_sightsize", FrameDesign {
            artwork_width: 11.0,
            artwork_height: 14.0,
            frame_style: FrameStyle::SightSize,
            ..base_design()
        }, ViewOption::SectionOnly, true, false),
    ]
}

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("render_matrix_out"));
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let views = [(ViewOption::PlanOnly, "plan"), (ViewOption::SectionOnly, "section")];
    let mut count = 0;
    for (name, design) in matrix() {
        for (view, view_name) in &views {
            let options = DiagramOptions {
                view: *view,
                ..DiagramOptions::default()
            };
            let result = generate_diagram(&design, &options);
            for w in &result.warnings {
                eprintln!("warning [{name}/{view_name}]: {w}");
            }
            let path = out_dir.join(format!("{name}_{view_name}.svg"));
            std::fs::write(&path, &result.svg).expect("write svg");
            println!("{}", path.display());
            count += 1;
        }
    }

    for (name, design, view, show_spline, show_hanging) in feature_cases() {
        let options = DiagramOptions {
            view,
            show_spline,
            show_hanging,
            ..DiagramOptions::default()
        };
        let result = generate_diagram(&design, &options);
        let path = out_dir.join(format!("{name}.svg"));
        std::fs::write(&path, &result.svg).expect("write svg");
        println!("{}", path.display());
        count += 1;
    }

    eprintln!("rendered {count} SVGs to {}", out_dir.display());
}
