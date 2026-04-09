//! Geometry snapshot export and comparison utilities.
//!
//! Provides JSON serialization of diagram geometry (rectangles, callouts,
//! positioned labels) for external tools and for regression testing via
//! golden-file comparison.
//!
//! Two tiers of checks:
//!   Tier 1 -- Invariants: must always hold (no overlaps, elements in bounds, etc.)
//!   Tier 2 -- Regression: numerical values compared with tolerance against golden files
//!
//! Golden files live in `core/tests/snapshots/<name>.json`.
//! Set env var `UPDATE_SNAPSHOTS=1` to regenerate them.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::frame::FrameDesign;
use super::types::Rect;
use super::geometry::PlanViewGeometry;
use super::layout::LayoutResult;
use super::style::DiagramStyle;
use super::callouts::generate_plan_callouts;
use super::layout::layout_plan_callouts;
use super::svg::run_collision_pass_for_snapshot;

/// A single element in the snapshot, identified by a stable string key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSnapshot {
    pub key: String,
    pub rect: Rect,
}

/// Captured layout properties for one frame + canvas size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    /// Human-readable label for the test case
    pub label: String,
    /// Rendering scale (px/inch)
    pub scale: f64,
    /// Whether X / Y axis breaks are active
    pub axis_break_x: bool,
    pub axis_break_y: bool,
    /// ViewBox as (x, y, w, h) — extracted from generated SVG
    pub viewbox: [f64; 4],
    /// Final positioned callouts: DimensionType name → dim_line_position
    pub callout_positions: BTreeMap<String, f64>,
    /// Final rects for floating annotation elements
    pub corner_detail: Option<Rect>,
    pub thumbnail: Option<Rect>,
    /// Number of element pairs that overlap (with 2px margin) — must be 0
    pub overlap_count: usize,
    /// Whether all element rects fall within the viewBox
    pub all_within_viewbox: bool,
}

/// Compute a snapshot for the given design at 390×844 canvas (iPhone portrait).
pub fn compute_snapshot(label: &str, design: &FrameDesign) -> LayoutSnapshot {
    let canvas_w = 390.0_f64;
    let canvas_h = 844.0_f64;
    let style = DiagramStyle::default();

    let mut geometry = PlanViewGeometry::from_design_with_mode(
        design, canvas_w, canvas_h, &style,
        super::types::DetailMode::Auto, true, true,
        false, false, false,
    );

    let callouts = generate_plan_callouts(design, &geometry, false, false, false, &style);
    let mut layout = layout_plan_callouts(&callouts, &geometry, &style);
    run_collision_pass_for_snapshot(&mut geometry, &mut layout, &style);

    // Collect all element rects for overlap + bounds checks
    let elements = collect_elements(&geometry, &layout);

    // Compute viewBox from element bounds + margin
    let padding = style.margin;
    let viewbox = compute_viewbox(&elements, padding);

    let overlap_count = count_overlaps(&elements, 2.0);
    let all_within_viewbox = elements.iter().all(|e| rect_within(e.rect, viewbox));

    let callout_positions = layout.positioned_callouts.iter()
        .map(|pc| {
            let key = format!("{:?}", pc.callout.dimension_type);
            (key, round2(pc.dimension_line_position))
        })
        .collect();

    LayoutSnapshot {
        label: label.to_string(),
        scale: round2(geometry.scale),
        axis_break_x: geometry.use_axis_break_x,
        axis_break_y: geometry.use_axis_break_y,
        viewbox: [round2(viewbox[0]), round2(viewbox[1]), round2(viewbox[2]), round2(viewbox[3])],
        callout_positions,
        corner_detail: geometry.corner_detail.as_ref().map(|cd| round_rect(cd.box_rect)),
        thumbnail: geometry.thumbnail.map(round_rect),
        overlap_count,
        all_within_viewbox,
    }
}

/// Collect all layout elements into a flat list for analysis.
fn collect_elements(geometry: &PlanViewGeometry, layout: &LayoutResult) -> Vec<ElementSnapshot> {
    let mut elements = Vec::new();

    elements.push(ElementSnapshot {
        key: "frame_outer".to_string(),
        rect: geometry.frame_outer,
    });

    if let Some(cd) = &geometry.corner_detail {
        elements.push(ElementSnapshot {
            key: "corner_detail".to_string(),
            rect: cd.box_rect,
        });
    }

    if let Some(thumb) = geometry.thumbnail {
        elements.push(ElementSnapshot {
            key: "thumbnail".to_string(),
            rect: thumb,
        });
    }

    for (i, pc) in layout.positioned_callouts.iter().enumerate() {
        elements.push(ElementSnapshot {
            key: format!("callout_label_{:?}_{}", pc.callout.dimension_type, i),
            rect: pc.label_bounds,
        });
    }

    elements
}

fn compute_viewbox(elements: &[ElementSnapshot], padding: f64) -> [f64; 4] {
    if elements.is_empty() {
        return [0.0, 0.0, 390.0, 844.0];
    }
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for e in elements {
        min_x = min_x.min(e.rect.left());
        min_y = min_y.min(e.rect.top());
        max_x = max_x.max(e.rect.right());
        max_y = max_y.max(e.rect.bottom());
    }
    [min_x - padding, min_y - padding,
     max_x - min_x + 2.0 * padding, max_y - min_y + 2.0 * padding]
}

/// Returns pairs of overlapping element keys (with margin), for the subset of
/// pairs the collision pass is responsible for. Specifically excludes:
/// - frame_outer vs anything (frame body underlies all annotations by design)
/// - callout_label vs callout_label (same-side stacking is layout's domain;
///   adjacent levels have intentional bounds overlap due to text_height > offset_step)
fn count_overlaps(elements: &[ElementSnapshot], margin: f64) -> usize {
    let mut count = 0;
    for i in 0..elements.len() {
        for j in (i + 1)..elements.len() {
            let ki = &elements[i].key;
            let kj = &elements[j].key;
            // Skip pairs the collision pass doesn't handle
            let either_is_frame = ki == "frame_outer" || kj == "frame_outer";
            let both_callout_labels = ki.starts_with("callout_label") && kj.starts_with("callout_label");
            if either_is_frame || both_callout_labels {
                continue;
            }
            let a = elements[i].rect.expand(margin / 2.0);
            let b = elements[j].rect.expand(margin / 2.0);
            if a.overlaps(&b) {
                count += 1;
            }
        }
    }
    count
}

fn rect_within(r: Rect, vb: [f64; 4]) -> bool {
    r.left() >= vb[0] - 1.0
        && r.top() >= vb[1] - 1.0
        && r.right() <= vb[0] + vb[2] + 1.0
        && r.bottom() <= vb[1] + vb[3] + 1.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn round_rect(r: Rect) -> Rect {
    Rect::new(round2(r.x), round2(r.y), round2(r.width), round2(r.height))
}

// ── Golden file helpers ───────────────────────────────────────────────────────

pub fn snapshot_path(name: &str) -> std::path::PathBuf {
    // Relative to the crate root (core/)
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("snapshots");
    p.push(format!("{}.json", name));
    p
}

pub fn load_or_create_snapshot(name: &str, actual: &LayoutSnapshot) -> LayoutSnapshot {
    let path = snapshot_path(name);
    if std::env::var("UPDATE_SNAPSHOTS").is_ok() || !path.exists() {
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir).unwrap();
        let json = serde_json::to_string_pretty(actual).unwrap();
        std::fs::write(&path, json).unwrap();
        actual.clone()
    } else {
        let json = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&json).unwrap()
    }
}

/// Compare two snapshots, returning a list of human-readable differences.
/// `tol_px` is the positional tolerance in pixels; `tol_pct` for ratios.
pub fn diff_snapshots(baseline: &LayoutSnapshot, actual: &LayoutSnapshot, tol_px: f64) -> Vec<String> {
    let mut diffs = Vec::new();

    // Tier 1: invariants
    if actual.overlap_count > 0 {
        diffs.push(format!("OVERLAP: {} element pair(s) overlap (baseline: {})",
            actual.overlap_count, baseline.overlap_count));
    }
    if !actual.all_within_viewbox {
        diffs.push("BOUNDS: some elements fall outside the viewBox".to_string());
    }

    // Tier 2: regression checks
    if actual.axis_break_x != baseline.axis_break_x {
        diffs.push(format!("axis_break_x: {} → {}", baseline.axis_break_x, actual.axis_break_x));
    }
    if actual.axis_break_y != baseline.axis_break_y {
        diffs.push(format!("axis_break_y: {} → {}", baseline.axis_break_y, actual.axis_break_y));
    }

    let scale_delta = (actual.scale - baseline.scale).abs();
    if scale_delta > tol_px {
        diffs.push(format!("scale: {:.2} → {:.2} (Δ{:.2})", baseline.scale, actual.scale, scale_delta));
    }

    for (key, &baseline_pos) in &baseline.callout_positions {
        if let Some(&actual_pos) = actual.callout_positions.get(key) {
            let delta = (actual_pos - baseline_pos).abs();
            if delta > tol_px {
                diffs.push(format!("callout[{}] dim_line: {:.1} → {:.1} (Δ{:.1}px)",
                    key, baseline_pos, actual_pos, delta));
            }
        } else {
            diffs.push(format!("callout[{}]: present in baseline, missing in actual", key));
        }
    }
    for key in actual.callout_positions.keys() {
        if !baseline.callout_positions.contains_key(key) {
            diffs.push(format!("callout[{}]: new in actual, not in baseline", key));
        }
    }

    diff_optional_rect(&mut diffs, "corner_detail", baseline.corner_detail, actual.corner_detail, tol_px);
    diff_optional_rect(&mut diffs, "thumbnail", baseline.thumbnail, actual.thumbnail, tol_px);

    diffs
}

fn diff_optional_rect(diffs: &mut Vec<String>, name: &str, baseline: Option<Rect>, actual: Option<Rect>, tol: f64) {
    match (baseline, actual) {
        (Some(b), Some(a)) => {
            for (field, bv, av) in [("x", b.x, a.x), ("y", b.y, a.y),
                                    ("w", b.width, a.width), ("h", b.height, a.height)] {
                let delta = (av - bv).abs();
                if delta > tol {
                    diffs.push(format!("{}.{}: {:.1} → {:.1} (Δ{:.1}px)", name, field, bv, av, delta));
                }
            }
        }
        (Some(_), None) => diffs.push(format!("{}: present in baseline, missing in actual", name)),
        (None, Some(_)) => diffs.push(format!("{}: new in actual, not in baseline", name)),
        (None, None) => {}
    }
}

// ── Test cases ────────────────────────────────────────────────────────────────

pub fn test_designs() -> Vec<(&'static str, FrameDesign)> {
    let mut cases = Vec::new();

    // Standard frame, no mat
    let mut d = FrameDesign::new(10.0, 8.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("8x10_no_mat", d));

    // Standard frame with mat
    let mut d = FrameDesign::new(20.0, 16.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 2.0;
    d.mat_width_sides = 2.0;
    cases.push(("16x20_with_mat", d));

    // Extreme portrait — triggers Y axis break
    let mut d = FrameDesign::new(100.0, 5.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("5x100_portrait_break", d));

    // Extreme landscape — triggers X axis break
    let mut d = FrameDesign::new(5.0, 100.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("100x5_landscape_break", d));

    // Tall portrait, label-fit triggers break
    let mut d = FrameDesign::new(20.0, 3.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("3x20_label_fit_break", d));

    // Both axes extreme
    let mut d = FrameDesign::new(375.0, 250.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("250x375_dual_break", d));

    // Small dense frame
    let mut d = FrameDesign::new(6.0, 4.0);
    d.frame_material_width = 0.75;
    d.mat_width_top_bottom = 0.0;
    d.mat_width_sides = 0.0;
    cases.push(("4x6_small", d));

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_snapshots() {
        let cases = test_designs();
        let mut all_diffs: Vec<(String, Vec<String>)> = Vec::new();

        for (name, design) in &cases {
            let actual = compute_snapshot(name, design);
            let baseline = load_or_create_snapshot(name, &actual);
            let diffs = diff_snapshots(&baseline, &actual, 2.0);
            if !diffs.is_empty() {
                all_diffs.push((name.to_string(), diffs));
            }
        }

        if !all_diffs.is_empty() {
            let mut msg = String::from("Layout snapshot regressions:\n");
            for (name, diffs) in &all_diffs {
                msg.push_str(&format!("\n  [{}]\n", name));
                for d in diffs {
                    msg.push_str(&format!("    - {}\n", d));
                }
            }
            msg.push_str("\nRun with UPDATE_SNAPSHOTS=1 to accept new baselines.");
            panic!("{}", msg);
        }
    }

    #[test]
    fn test_layout_invariants() {
        // Invariants must hold regardless of snapshots
        let cases = test_designs();
        let mut failures: Vec<String> = Vec::new();

        for (name, design) in &cases {
            let snap = compute_snapshot(name, design);
            if snap.overlap_count > 0 {
                failures.push(format!("[{}] {} overlapping element pair(s)", name, snap.overlap_count));
            }
            if !snap.all_within_viewbox {
                failures.push(format!("[{}] elements outside viewBox", name));
            }
        }

        if !failures.is_empty() {
            panic!("Layout invariant failures:\n{}", failures.join("\n"));
        }
    }
}
