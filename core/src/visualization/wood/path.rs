//! Compact SVG path encoding for grain geometry.
//!
//! Coordinates are rounded to 0.1 px, then written as *relative* commands whose
//! deltas are taken between rounded absolute points (no drift accumulates).
//! Catmull-Rom curves use `c` once, then `s`: each segment's first control point
//! is exactly the reflection of the previous one's second, so the shorthand is
//! lossless. Repeated commands use implicit repetition. All of this is plain SVG
//! path grammar, which flutter_svg parses.

use std::fmt::Write;

pub(crate) type Pt = (f64, f64);

fn r1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// A number at 0.1 precision: `3`, `-0.5`, `12.3` (no trailing `.0`, never `-0`).
pub(crate) fn num(x: f64) -> String {
    let r = r1(x);
    if r == 0.0 {
        return "0".to_string();
    }
    let s = format!("{r:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// Two decimals, for opacities.
pub(crate) fn op2(x: f64) -> String {
    format!("{:.2}", x)
}

struct Writer {
    d: String,
    cur: Pt,
}

impl Writer {
    fn move_to(p: Pt) -> Self {
        let p = (r1(p.0), r1(p.1));
        Writer { d: format!("M{},{}", num(p.0), num(p.1)), cur: p }
    }

    /// Append a relative point (delta from the current point) and advance.
    fn rel(&mut self, p: Pt, advance: bool) {
        let p = (r1(p.0), r1(p.1));
        let _ = write!(self.d, "{},{}", num(p.0 - self.cur.0), num(p.1 - self.cur.1));
        if advance {
            self.cur = p;
        }
    }

    /// Relative point from `base` (control points are relative to the segment start).
    fn rel_from(&mut self, base: Pt, p: Pt) {
        let p = (r1(p.0), r1(p.1));
        let _ = write!(self.d, "{},{}", num(p.0 - base.0), num(p.1 - base.1));
    }

    fn line_run(&mut self, pts: &[Pt]) {
        for (i, &p) in pts.iter().enumerate() {
            self.d.push_str(if i == 0 { "l" } else { " " });
            self.rel(p, true);
        }
    }

    /// Catmull-Rom through `pts` (pts[0] is the current point).
    fn catmull_rom(&mut self, pts: &[Pt]) {
        let n = pts.len();
        for i in 0..n - 1 {
            let p0 = pts[i.saturating_sub(1)];
            let (p1, p2) = (pts[i], pts[i + 1]);
            let p3 = pts[(i + 2).min(n - 1)];
            let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
            let start = self.cur;
            if i == 0 {
                let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
                self.d.push('c');
                self.rel_from(start, c1);
                self.d.push(' ');
            } else {
                self.d.push(if i == 1 { 's' } else { ' ' });
            }
            self.rel_from(start, c2);
            self.d.push(' ');
            self.rel(p2, true);
        }
    }
}

/// Polyline (optionally closed).
pub(crate) fn poly_d(pts: &[Pt], close: bool) -> String {
    let mut w = Writer::move_to(pts[0]);
    w.line_run(&pts[1..]);
    if close {
        w.d.push('z');
    }
    w.d
}

/// Smooth open curve through the points (polyline when there are fewer than 3).
pub(crate) fn smooth_d(pts: &[Pt]) -> String {
    if pts.len() < 3 {
        return poly_d(pts, false);
    }
    let mut w = Writer::move_to(pts[0]);
    w.catmull_rom(pts);
    w.d
}

/// Same winding for every filled outline: overlapping sub-paths of one compound path
/// then union under the nonzero rule (opposite windings would cancel into holes).
pub(crate) fn ccw(mut pts: Vec<Pt>) -> Vec<Pt> {
    let n = pts.len();
    let area2: f64 = (0..n).map(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        a.0 * b.1 - b.0 * a.1
    }).sum();
    if area2 < 0.0 {
        pts.reverse();
    }
    pts
}

/// Closed variable-width outline with straight edges (for RDP-simplified centrelines,
/// whose points already follow the curve to within a fraction of a pixel).
pub(crate) fn ribbon_poly_d(pts: &[Pt], widths: &[f64]) -> String {
    let n = pts.len();
    let (mut left, mut right) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for (i, &(x, y)) in pts.iter().enumerate() {
        let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
        let (tx, ty) = (b.0 - a.0, b.1 - a.1);
        let nn = tx.hypot(ty);
        let nn = if nn == 0.0 { 1.0 } else { nn };
        let (nx, ny, h) = (-ty / nn, tx / nn, widths[i] / 2.0);
        left.push((x + nx * h, y + ny * h));
        right.push((x - nx * h, y - ny * h));
    }
    left.extend(right.into_iter().rev());
    poly_d(&ccw(left), true)
}

/// Dash list `"a b c d …"` at 0.1 precision.
pub(crate) fn dash_list(vals: &[f64]) -> String {
    vals.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_compact() {
        assert_eq!(num(3.0), "3");
        assert_eq!(num(-0.04), "0");
        assert_eq!(num(12.34), "12.3");
        assert_eq!(num(-0.55), "-0.6");
    }

    #[test]
    fn relative_paths_round_trip_to_rounded_absolutes() {
        // Re-integrate the relative polyline and compare with the rounded input.
        let pts = [(0.04, 0.0), (10.26, 3.33), (20.11, -4.97), (31.0, 0.05)];
        let d = poly_d(&pts, false);
        let body = d.trim_start_matches('M');
        let mut parts = body.split('l');
        let start: Vec<f64> = parts.next().unwrap().split(',').map(|s| s.parse().unwrap()).collect();
        let (mut x, mut y) = (start[0], start[1]);
        for (pair, want) in parts.next().unwrap().split(' ').zip(&pts[1..]) {
            let v: Vec<f64> = pair.split(',').map(|s| s.parse().unwrap()).collect();
            x += v[0];
            y += v[1];
            assert!((x - r1(want.0)).abs() < 1e-9 && (y - r1(want.1)).abs() < 1e-9, "{d}");
        }
    }

    #[test]
    fn smooth_uses_c_then_s() {
        let d = smooth_d(&[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0), (30.0, 5.0)]);
        assert!(d.starts_with("M0,0c") && d.contains('s'), "{d}");
        assert_eq!(d.matches('c').count(), 1);
    }
}
