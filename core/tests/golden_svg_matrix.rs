//! Golden SVG regression test matrix.
//!
//! Generates SVGs for a matrix of designs x view options x display settings
//! and compares them against stored golden files.
//!
//! - Normal run: compare generated SVG against golden file, fail on mismatch.
//! - `UPDATE_GOLDEN=1 cargo test`: overwrite golden files with current output.
//! - First run (no golden file): auto-create it (no env var needed).

use std::path::PathBuf;

use referenceframe_core::{FrameDesign, FrameStyle};
use referenceframe_core::visualization::{
    DiagramOptions, ViewOption, generate_diagram,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Canonical base design with every field set explicitly (no reliance on
/// presets.json defaults).  Callers override the fields they care about.
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

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden_svgs")
}

/// Compare `svg` against the golden file at `name.svg`.
///
/// - If `UPDATE_GOLDEN=1`, always write.
/// - If the golden file does not exist, create it (first-run friendly).
/// - Otherwise assert equality.
/// Every `&` in emitted SVG must start a known entity — a bare ampersand is
/// invalid XML and breaks browser rendering even though string comparison
/// passes. (Caught in the wild via a "Joinery & Hanging" label.)
fn assert_valid_entities(name: &str, svg: &str) {
    for (i, _) in svg.match_indices('&') {
        let rest = &svg[i + 1..];
        let ok = ["amp;", "lt;", "gt;", "quot;", "apos;", "#"]
            .iter()
            .any(|e| rest.starts_with(e));
        assert!(
            ok,
            "`{name}`: bare `&` at byte {i} is invalid XML: ...{}...",
            &svg[i.saturating_sub(40)..(i + 20).min(svg.len())]
        );
    }
}

fn assert_golden(name: &str, svg: &str) {
    assert_valid_entities(name, svg);
    let path = golden_dir().join(format!("{name}.svg"));
    let update = std::env::var("UPDATE_GOLDEN").map(|v| v == "1").unwrap_or(false);

    if update || !path.exists() {
        std::fs::write(&path, svg).unwrap_or_else(|e| {
            panic!("Failed to write golden file {}: {e}", path.display());
        });
        // On first creation we don't fail — the file is simply recorded.
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("Failed to read golden file {}: {e}", path.display());
    });

    if expected != svg {
        // Write the actual output next to the golden for easy diffing.
        let actual_path = golden_dir().join(format!("{name}.actual.svg"));
        let _ = std::fs::write(&actual_path, svg);
        panic!(
            "Golden SVG mismatch for `{name}`.\n\
             Golden: {}\n\
             Actual: {}\n\
             Run with UPDATE_GOLDEN=1 to accept the new output.",
            path.display(),
            actual_path.display(),
        );
    }
}

// ---------------------------------------------------------------------------
// Design builders
// ---------------------------------------------------------------------------

fn standard_8x10() -> FrameDesign {
    FrameDesign {
        artwork_width: 10.0,
        artwork_height: 8.0,
        frame_material_width: 1.0,
        frame_material_depth: 0.75,
        rabbet_width: 0.375,
        rabbet_depth: 0.375,
        ..base_design()
    }
}

fn matted_16x20() -> FrameDesign {
    FrameDesign {
        artwork_width: 20.0,
        artwork_height: 16.0,
        mat_width_top_bottom: 2.0,
        mat_width_sides: 2.0,
        frame_material_width: 1.5,
        frame_material_depth: 0.75,
        rabbet_width: 0.375,
        rabbet_depth: 0.375,
        ..base_design()
    }
}

fn wide_50x18() -> FrameDesign {
    FrameDesign {
        artwork_width: 50.0,
        artwork_height: 18.0,
        frame_material_width: 1.0,
        ..base_design()
    }
}

fn tall_8x60() -> FrameDesign {
    FrameDesign {
        artwork_width: 8.0,
        artwork_height: 60.0,
        frame_material_width: 1.0,
        ..base_design()
    }
}

fn small_4x6() -> FrameDesign {
    FrameDesign {
        artwork_width: 6.0,
        artwork_height: 4.0,
        frame_material_width: 0.75,
        ..base_design()
    }
}

fn dual_break() -> FrameDesign {
    FrameDesign {
        artwork_width: 80.0,
        artwork_height: 80.0,
        frame_material_width: 1.0,
        ..base_design()
    }
}

fn asymmetric_mat() -> FrameDesign {
    FrameDesign {
        artwork_width: 14.0,
        artwork_height: 11.0,
        mat_width_top_bottom: 2.5,
        mat_width_sides: 2.0,
        frame_material_width: 1.0,
        symmetrical_mat: false,
        ..base_design()
    }
}

/// Sight-size frame: opening = artwork, no lip over the art face.
fn sight_size_11x14() -> FrameDesign {
    FrameDesign {
        artwork_width: 14.0,
        artwork_height: 11.0,
        frame_material_width: 1.0,
        frame_material_depth: 0.75,
        rabbet_width: 0.375,
        rabbet_depth: 0.375,
        frame_style: FrameStyle::SightSize,
        ..base_design()
    }
}

// ---------------------------------------------------------------------------
// Options builders
// ---------------------------------------------------------------------------

fn opts_plan_inches() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width: 800.0,
        canvas_height: 600.0,
        ..Default::default()
    }
}

fn opts_section_inches() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_width: 800.0,
        canvas_height: 600.0,
        ..Default::default()
    }
}

fn opts_both_inches() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::Both,
        canvas_width: 800.0,
        canvas_height: 600.0,
        ..Default::default()
    }
}

fn opts_plan_mm() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width: 800.0,
        canvas_height: 600.0,
        unit_mm: true,
        ..Default::default()
    }
}

fn opts_plan_decimal() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width: 800.0,
        canvas_height: 600.0,
        use_decimal_display: true,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Matrix definition
// ---------------------------------------------------------------------------

struct MatrixEntry {
    name: &'static str,
    design_fn: fn() -> FrameDesign,
    options_fn: fn() -> DiagramOptions,
}

/// Full test matrix.
///
/// Every design gets PlanOnly/inches (7 entries).
/// A representative subset also gets section, both, mm, and decimal views.
const MATRIX: &[MatrixEntry] = &[
    // -- All designs x PlanOnly x inches (7) --
    MatrixEntry { name: "standard_8x10_plan_inches",   design_fn: standard_8x10,   options_fn: opts_plan_inches },
    MatrixEntry { name: "matted_16x20_plan_inches",    design_fn: matted_16x20,    options_fn: opts_plan_inches },
    MatrixEntry { name: "wide_50x18_plan_inches",      design_fn: wide_50x18,      options_fn: opts_plan_inches },
    MatrixEntry { name: "tall_8x60_plan_inches",       design_fn: tall_8x60,       options_fn: opts_plan_inches },
    MatrixEntry { name: "small_4x6_plan_inches",       design_fn: small_4x6,       options_fn: opts_plan_inches },
    MatrixEntry { name: "dual_break_plan_inches",      design_fn: dual_break,       options_fn: opts_plan_inches },
    MatrixEntry { name: "asymmetric_mat_plan_inches",  design_fn: asymmetric_mat,   options_fn: opts_plan_inches },

    // -- All designs x SectionOnly x inches (7) --
    MatrixEntry { name: "standard_8x10_section_inches",   design_fn: standard_8x10,   options_fn: opts_section_inches },
    MatrixEntry { name: "matted_16x20_section_inches",    design_fn: matted_16x20,    options_fn: opts_section_inches },
    MatrixEntry { name: "wide_50x18_section_inches",      design_fn: wide_50x18,      options_fn: opts_section_inches },
    MatrixEntry { name: "tall_8x60_section_inches",       design_fn: tall_8x60,       options_fn: opts_section_inches },
    MatrixEntry { name: "small_4x6_section_inches",       design_fn: small_4x6,       options_fn: opts_section_inches },
    MatrixEntry { name: "dual_break_section_inches",      design_fn: dual_break,       options_fn: opts_section_inches },
    MatrixEntry { name: "asymmetric_mat_section_inches",  design_fn: asymmetric_mat,   options_fn: opts_section_inches },

    // -- All designs x Both x inches (7) --
    MatrixEntry { name: "standard_8x10_both_inches",   design_fn: standard_8x10,   options_fn: opts_both_inches },
    MatrixEntry { name: "matted_16x20_both_inches",    design_fn: matted_16x20,    options_fn: opts_both_inches },
    MatrixEntry { name: "wide_50x18_both_inches",      design_fn: wide_50x18,      options_fn: opts_both_inches },
    MatrixEntry { name: "tall_8x60_both_inches",       design_fn: tall_8x60,       options_fn: opts_both_inches },
    MatrixEntry { name: "small_4x6_both_inches",       design_fn: small_4x6,       options_fn: opts_both_inches },
    MatrixEntry { name: "dual_break_both_inches",      design_fn: dual_break,       options_fn: opts_both_inches },
    MatrixEntry { name: "asymmetric_mat_both_inches",  design_fn: asymmetric_mat,   options_fn: opts_both_inches },

    // -- Subset x PlanOnly x mm (representative 4) --
    MatrixEntry { name: "standard_8x10_plan_mm",   design_fn: standard_8x10,   options_fn: opts_plan_mm },
    MatrixEntry { name: "matted_16x20_plan_mm",    design_fn: matted_16x20,    options_fn: opts_plan_mm },
    MatrixEntry { name: "wide_50x18_plan_mm",      design_fn: wide_50x18,      options_fn: opts_plan_mm },
    MatrixEntry { name: "asymmetric_mat_plan_mm",  design_fn: asymmetric_mat,   options_fn: opts_plan_mm },

    // -- Subset x PlanOnly x decimal (representative 4) --
    MatrixEntry { name: "standard_8x10_plan_decimal",   design_fn: standard_8x10,   options_fn: opts_plan_decimal },
    MatrixEntry { name: "matted_16x20_plan_decimal",    design_fn: matted_16x20,    options_fn: opts_plan_decimal },
    MatrixEntry { name: "small_4x6_plan_decimal",       design_fn: small_4x6,       options_fn: opts_plan_decimal },
    MatrixEntry { name: "dual_break_plan_decimal",      design_fn: dual_break,       options_fn: opts_plan_decimal },

    // -- Subset x SectionOnly x mm (2) --
    MatrixEntry { name: "standard_8x10_section_mm",  design_fn: standard_8x10,  options_fn: opts_section_mm },
    MatrixEntry { name: "matted_16x20_section_mm",   design_fn: matted_16x20,   options_fn: opts_section_mm },

    // -- Subset x Both x mm (2) --
    MatrixEntry { name: "standard_8x10_both_mm",  design_fn: standard_8x10,  options_fn: opts_both_mm },
    MatrixEntry { name: "matted_16x20_both_mm",   design_fn: matted_16x20,   options_fn: opts_both_mm },

    // -- Sight-size (no lip; opening = artwork), inches --
    MatrixEntry { name: "sight_size_11x14_plan_inches",    design_fn: sight_size_11x14, options_fn: opts_plan_inches },
    MatrixEntry { name: "sight_size_11x14_section_inches", design_fn: sight_size_11x14, options_fn: opts_section_inches },
    MatrixEntry { name: "sight_size_11x14_both_inches",    design_fn: sight_size_11x14, options_fn: opts_both_inches },

    // -- Spline / hanging overlays (opt-in flags), inches --
    MatrixEntry { name: "spline_standard_8x10_section_inches", design_fn: standard_8x10, options_fn: opts_section_spline },
    MatrixEntry { name: "spline_standard_8x10_plan_inches",    design_fn: standard_8x10, options_fn: opts_plan_spline },
    MatrixEntry { name: "hanging_matted_16x20_plan_inches",    design_fn: matted_16x20,  options_fn: opts_plan_hanging },
    MatrixEntry { name: "overlays_matted_16x20_both_inches",   design_fn: matted_16x20,  options_fn: opts_both_overlays },

    // -- Interference warning (backdropped, drawn above the legend) --
    MatrixEntry { name: "interference_8x10_section_inches", design_fn: interference_8x10, options_fn: opts_section_inches },

    // -- Portrait (phone) canvas: overlay card moves below the content --
    MatrixEntry { name: "overlays_matted_16x20_plan_portrait", design_fn: matted_16x20, options_fn: opts_portrait_overlays },
];

fn opts_portrait_overlays() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::PlanOnly,
        canvas_width: 390.0,
        canvas_height: 844.0,
        show_spline: true,
        show_hanging: true,
        ..Default::default()
    }
}

/// Shallow rabbet: the material stack overruns the channel -> INTERFERENCE.
fn interference_8x10() -> FrameDesign {
    FrameDesign {
        rabbet_depth: 0.25,
        ..standard_8x10()
    }
}

fn opts_section_spline() -> DiagramOptions {
    DiagramOptions {
        show_spline: true,
        ..opts_section_inches()
    }
}

fn opts_plan_spline() -> DiagramOptions {
    DiagramOptions {
        show_spline: true,
        ..opts_plan_inches()
    }
}

fn opts_plan_hanging() -> DiagramOptions {
    DiagramOptions {
        show_hanging: true,
        ..opts_plan_inches()
    }
}

fn opts_both_overlays() -> DiagramOptions {
    DiagramOptions {
        show_spline: true,
        show_hanging: true,
        ..opts_both_inches()
    }
}

fn opts_section_mm() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::SectionOnly,
        canvas_width: 800.0,
        canvas_height: 600.0,
        unit_mm: true,
        ..Default::default()
    }
}

fn opts_both_mm() -> DiagramOptions {
    DiagramOptions {
        view: ViewOption::Both,
        canvas_width: 800.0,
        canvas_height: 600.0,
        unit_mm: true,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn golden_svg_matrix() {
    let mut count = 0;
    let mut failures = Vec::new();

    for entry in MATRIX {
        let design = (entry.design_fn)();
        let options = (entry.options_fn)();
        let result = generate_diagram(&design, &options);

        // Catch panics so we can report all failures, not just the first.
        let name = entry.name;
        let svg = &result.svg;
        let outcome = std::panic::catch_unwind(|| {
            assert_golden(name, svg);
        });

        if let Err(e) = outcome {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            failures.push((name, msg));
        }
        count += 1;
    }

    println!("Golden SVG matrix: {count} entries checked.");

    if !failures.is_empty() {
        let summary: Vec<String> = failures
            .iter()
            .map(|(name, msg)| format!("  - {name}: {msg}"))
            .collect();
        panic!(
            "{} golden SVG mismatch(es):\n{}",
            failures.len(),
            summary.join("\n"),
        );
    }
}
