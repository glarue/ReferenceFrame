//! Grain for one moulding piece, in piece-local coordinates: u runs along the
//! length [0, L], v across the width [0, fw], both in px. Port of `side_grain`
//! from `tools/wood-fit/gen_fit.py`.
//!
//! Layers, bottom to top: colour streaks, growth-zone bands, ring traces, pores,
//! ray flecks. Each layer draws from its own random substream.

use super::appearance::{Figure, LineMode, WoodAppearance};
use super::noise::{clamp01, fbm, substream, Rng};
use super::path::{ccw, dash_list, op2, poly_d, ribbon_poly_d, smooth_d, Pt};
use std::fmt::Write;

/// Thinnest stroke worth drawing at preview scale (px)
const FLOOR_LINE_PX: f64 = 0.45;
const FLOOR_PORE_PX: f64 = 0.4;
/// Scale LOD: rings closer than this (px) are decimated -- every k-th ring is drawn,
/// with opacity raised so overall darkness holds. At validation scale k = 1.
const MIN_RING_PX: f64 = 2.5;
/// Variable-width ribbons only where the trace is at least this wide (px); thinner
/// traces are plain strokes (the width variation would be invisible).
const RIBBON_MIN_PX: f64 = 1.0;
/// Scale LOD for pores: rows closer than this (px) are thinned, opacity compensates.
const MIN_PORE_ROW_PX: f64 = 1.2;
/// Scale LOD for colour streaks: minimum spacing (px) of their outline samples.
const STREAK_STEP_PX: f64 = 24.0;

/// Counts of emitted elements (for tests and payload tuning).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GrainStats {
    pub streaks: usize,
    pub zones: usize,
    pub lines: usize,
    pub pores: usize,
    pub flecks: usize,
}

/// A run of points on the face, with the apparent-width multiplier (R/x, capped)
/// and an apex flag per point (apexes survive point thinning).
#[derive(Default)]
struct Run {
    pts: Vec<Pt>,
    mul: Vec<f64>,
    apex: Vec<bool>,
}

impl Run {
    fn push(&mut self, p: Pt, mul: f64, apex: bool) {
        self.pts.push(p);
        self.mul.push(mul);
        self.apex.push(apex);
    }

    fn straight(pts: Vec<Pt>) -> Self {
        let n = pts.len();
        Run { pts, mul: vec![1.0; n], apex: vec![false; n] }
    }

    /// Simplified for compact paths: Ramer-Douglas-Peucker at a sub-pixel tolerance,
    /// with arch apexes as fixed break points (they always survive). Polylines through
    /// these points beat fitted cubics here: the noise-warped grain lines wiggle every few
    /// px, so a cubic (6 numbers) rarely replaces more than ~3 vertices (2 numbers each).
    fn thinned(&self) -> (Vec<Pt>, Vec<f64>) {
        let n = self.pts.len();
        if n <= 3 {
            return (self.pts.clone(), self.mul.clone());
        }
        let mut keep = vec![false; n];
        let mut start = 0;
        for i in 1..n {
            if self.apex[i] || i == n - 1 {
                rdp(&self.pts, start, i, RDP_EPS_PX, &mut keep);
                start = i;
            }
        }
        let pts = (0..n).filter(|&i| keep[i]).map(|i| self.pts[i]).collect();
        let mul = (0..n).filter(|&i| keep[i]).map(|i| self.mul[i]).collect();
        (pts, mul)
    }
}

/// Tolerance for path simplification (px): invisible at any display scale we use.
const RDP_EPS_PX: f64 = 0.25;

/// Ramer-Douglas-Peucker over pts[a..=b]; marks kept indices (iterative).
fn rdp(pts: &[Pt], a: usize, b: usize, eps: f64, keep: &mut [bool]) {
    keep[a] = true;
    keep[b] = true;
    let mut stack = vec![(a, b)];
    while let Some((i, j)) = stack.pop() {
        if j <= i + 1 {
            continue;
        }
        let (p, q) = (pts[i], pts[j]);
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = dx.hypot(dy);
        let (mut best, mut idx) = (0.0, i);
        for (k, r) in pts.iter().enumerate().take(j).skip(i + 1) {
            let d = if len == 0.0 {
                (r.0 - p.0).hypot(r.1 - p.1)
            } else {
                ((r.0 - p.0) * dy - (r.1 - p.1) * dx).abs() / len
            };
            if d > best {
                best = d;
                idx = k;
            }
        }
        if best > eps {
            keep[idx] = true;
            stack.push((i, idx));
            stack.push((idx, j));
        }
    }
}

/// Merges same-style elements of one layer into compound paths: one element per
/// style instead of one per line/pore/fleck. Flushed per layer to keep stacking order.
#[derive(Default)]
struct Batch(Vec<(String, String)>); // (attributes, path data)

impl Batch {
    fn add(&mut self, attrs: &str, d: &str) {
        match self.0.iter_mut().find(|(a, _)| a == attrs) {
            Some((_, dd)) => dd.push_str(d),
            None => self.0.push((attrs.to_string(), d.to_string())),
        }
    }

    fn flush(&mut self, out: &mut String) {
        for (attrs, d) in self.0.drain(..) {
            let _ = write!(out, r#"<path d="{d}" {attrs}/>"#);
        }
    }
}

/// One of three evenly spread levels in [lo, hi] (the mean of a uniform draw): random
/// variation that still lets elements share a style and merge.
fn level3(r: &mut Rng, lo: f64, hi: f64) -> f64 {
    let k = (r.random() * 3.0).floor().min(2.0);
    lo + (hi - lo) * (2.0 * k + 1.0) / 6.0
}

/// Index into a small set of shared dash lists.
fn pick(r: &mut Rng, n: usize) -> usize {
    ((r.random() * n as f64) as usize).min(n - 1)
}

/// Dash patterns restart at each sub-path, so a dashed run is split at a random point
/// and drawn outward both ways: runs sharing a dash list keep independent phases
/// (no aligned "columns") with no visible change to the line.
fn phase_split(pts: &[Pt], r: &mut Rng) -> Vec<Vec<Pt>> {
    let seg: Vec<f64> = pts.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).collect();
    let total: f64 = seg.iter().sum();
    if pts.len() < 2 || total <= 0.0 {
        return vec![pts.to_vec()];
    }
    let mut s = r.random() * total;
    let mut i = 0;
    while i < seg.len() - 1 && s > seg[i] {
        s -= seg[i];
        i += 1;
    }
    let t = if seg[i] > 0.0 { s / seg[i] } else { 0.0 };
    let m = (pts[i].0 + t * (pts[i + 1].0 - pts[i].0), pts[i].1 + t * (pts[i + 1].1 - pts[i].1));
    let fwd: Vec<Pt> = std::iter::once(m).chain(pts[i + 1..].iter().copied()).collect();
    let back: Vec<Pt> = std::iter::once(m).chain(pts[..=i].iter().rev().copied()).collect();
    [fwd, back].into_iter().filter(|v| v.len() > 1).collect()
}

/// AR(1) tree-ring widths (phi 0.6) with an occasional narrow year, mean 1.
fn ring_widths(rr: &mut Rng, n: usize, cv: f64) -> Vec<f64> {
    let mut z = 0.0;
    let mut out: Vec<f64> = (0..n)
        .map(|_| {
            let g = (rr.random() + rr.random() + rr.random() - 1.5) * 2.0;
            z = 0.6 * z + 0.8 * g;
            let mut w = (1.0 + cv * z).max(0.3);
            if rr.random() < 0.08 {
                w *= 0.45;
            }
            w
        })
        .collect();
    let m = out.iter().sum::<f64>() / n as f64;
    out.iter_mut().for_each(|w| *w /= m);
    out
}

/// Long irregular dash list (lognormal-ish dashes, uniform-ish gaps, random phase).
fn long_dashes(r: &mut Rng, ppi: f64, dash_in: f64, gap_in: f64) -> String {
    let mut da = Vec::with_capacity(12);
    for _ in 0..6 {
        let t = r.random() + r.random() - 1.0;
        da.push((dash_in * ppi * (0.6 * t).exp()).max(2.0));
        da.push((gap_in * ppi * (0.3 + 1.4 * r.random())).max(2.0));
    }
    if r.random() < 0.5 {
        da.rotate_left(2);
    }
    dash_list(&da)
}

struct Ctx<'a> {
    a: &'a WoodAppearance,
    fw: f64,
    ppi: f64,
    seed: u32,
}

impl Ctx<'_> {
    fn warp(&self, u: f64, v: f64) -> f64 {
        let p = &self.a.params;
        p.wiggle_in * self.ppi * fbm(u / (p.wiggle_len_in * self.ppi), v / (0.8 * self.ppi), self.seed.wrapping_add(99), 4)
    }
}

/// Offsets (px) of pore rows inside one ring's earlywood band.
fn pore_offsets(a: &WoodAppearance, band: f64, rj: &mut Rng, scale: f64) -> Vec<f64> {
    let p = &a.params;
    let mut n = if p.pore_rows_by_width {
        ((f64::from(p.pore_rows) * scale).round() as i64).max(1)
    } else {
        i64::from(p.pore_rows)
    };
    n = n.min(((band / MIN_PORE_ROW_PX).floor() as i64).max(1)); // scale LOD
    if p.pore_jitter <= 0.0 {
        return (0..n).map(|k| (k as f64 + 0.5) / n as f64 * band).collect();
    }
    let j = p.pore_jitter;
    n = (n + (j * rj.uniform(-1.5, 1.5)).round() as i64).max(1);
    let mut out = Vec::new();
    for k in 0..n {
        if rj.random() < 0.25 * j {
            continue; // occasional missing row
        }
        let even = (k as f64 + 0.5) / n as f64;
        out.push(band * (even + j * rj.uniform(-0.5, 0.5) / n as f64).clamp(0.0, 1.0));
    }
    out
}

/// Cathedral (plain-sawn) ring geometry around a tilted, eccentric, drifting pith.
struct Cathedral {
    us: Vec<f64>,
    dz: Vec<f64>,
    vcs: Vec<f64>,
    ka: f64,
    kb: f64,
    cap: f64,
}

impl Cathedral {
    /// Ring R as runs on the face. Arch sides are joined through the exact apex
    /// (interpolated where R = depth), whichever end of the span it is on.
    fn contour(&self, cx: &Ctx, r: f64) -> Vec<Run> {
        let (us, dz, vcs) = (&self.us, &self.dz, &self.vcs);
        let apex = |ja: usize, jb: usize| -> Pt {
            let (qa, qb) = (r * r - dz[ja] * dz[ja], r * r - dz[jb] * dz[jb]);
            let t = qa / (qa - qb);
            (us[ja] + t * (us[jb] - us[ja]), vcs[ja] + t * (vcs[jb] - vcs[ja]))
        };
        let mut runs = Vec::new();
        let mut seg: Vec<(usize, f64)> = Vec::new();
        // trailing sentinel (q < 0) closes the last span
        for (i, q) in dz.iter().map(|d| r * r - d * d).chain(std::iter::once(-1.0)).enumerate() {
            if q >= 0.0 {
                seg.push((i, q.sqrt()));
                continue;
            }
            if seg.len() > 1 {
                let (i0, i1) = (seg[0].0, seg[seg.len() - 1].0);
                // (point, width multiplier, is_apex)
                let m = |x: f64| (r / x.max(1e-6)).min(self.cap);
                let left: Vec<(Pt, f64, bool)> = seg.iter().map(|&(j, x)| ((us[j], vcs[j] - x * self.ka), m(x), false)).collect();
                let right: Vec<(Pt, f64, bool)> = seg.iter().map(|&(j, x)| ((us[j], vcs[j] + x * self.kb), m(x), false)).collect();
                let rev = |v: &[(Pt, f64, bool)]| v.iter().rev().copied().collect::<Vec<_>>();
                let tip = |p: Pt| (p, self.cap, true);
                let (tip0, tip1) = (i0 > 0, i1 < us.len() - 1);
                let paths: Vec<Vec<(Pt, f64, bool)>> = if tip0 && tip1 {
                    let (a0, a1) = (tip(apex(i0, i0 - 1)), tip(apex(i1, i1 + 1)));
                    vec![[vec![a0], right, vec![a1], rev(&left), vec![a0]].concat()]
                } else if tip0 {
                    vec![[rev(&left), vec![tip(apex(i0, i0 - 1))], right].concat()]
                } else if tip1 {
                    vec![[left, vec![tip(apex(i1, i1 + 1))], rev(&right)].concat()]
                } else {
                    vec![left, right] // spans the whole piece: two separate flanks
                };
                for path in paths {
                    let mut run = Run::default();
                    for ((u, v), mul, ap) in path {
                        let v = v + cx.warp(u, v);
                        if (-3.0..=cx.fw + 3.0).contains(&v) {
                            run.push((u, v), mul, ap);
                        } else if run.pts.len() > 1 {
                            runs.push(std::mem::take(&mut run));
                        } else {
                            run = Run::default();
                        }
                    }
                    if run.pts.len() > 1 {
                        runs.push(run);
                    }
                }
            }
            seg.clear();
        }
        runs
    }
}

/// Grain elements for one piece; `seed` selects this piece's board.
pub(crate) fn side_grain(a: &WoodAppearance, l: f64, fw: f64, ppi: f64, seed: u32) -> (String, GrainStats) {
    let p = &a.params;
    let s = &a.structure;
    let pal = &a.palette;
    let cx = Ctx { a, fw, ppi, seed };
    let mut out = String::new();
    let mut stats = GrainStats::default();

    // --- colour streaks --------------------------------------------------------
    let mut rng = substream(seed, 1);
    for k in 0..3u32 {
        let v0 = rng.uniform(0.0, fw);
        let hw = rng.uniform(0.15, 0.4) * fw;
        let s2 = seed.wrapping_mul(7).wrapping_add(k * 1013);
        // ~1 sample per inch, smoothed when drawn (much sparser turns a pinched,
        // patchy streak into a crisp almond shape); at least STREAK_STEP_PX apart so
        // long sides at small scales don't pay for detail finer than the soft edges show
        let n_s = ((l / ppi.max(STREAK_STEP_PX)).ceil() as u32 + 2).max(12);
        let (mut top, mut bot) = (Vec::with_capacity(n_s as usize + 1), Vec::with_capacity(n_s as usize + 1));
        for i in 0..=n_s {
            let u = -10.0 + (l + 20.0) * f64::from(i) / f64::from(n_s);
            let c = v0 + fw * 0.4 * fbm(u / (3.0 * ppi), 0.5, s2, 3);
            let mut wt = hw * (1.0 + 0.6 * fbm(u / (2.0 * ppi), 3.1, s2.wrapping_add(1), 3));
            let mut wb = hw * (1.0 + 0.6 * fbm(u / (2.0 * ppi), 3.1, s2.wrapping_add(2), 3));
            if p.streak_patchy {
                // pinch to nothing in places: elongated patches, not stripes
                let pinch = clamp01(0.35 + 1.6 * fbm(u / (2.2 * ppi), 7.7, s2.wrapping_add(5), 2));
                wt *= pinch;
                wb *= pinch;
            }
            top.push((u, c - wt));
            bot.push((u, c + wb));
        }
        let col = if k % 2 == 0 { &pal.streak } else { &pal.alt };
        let op = clamp01(p.streak_op * rng.uniform(0.7, 1.4));
        if p.streak_soft {
            // nested bands -> soft edges (two bands: the third added bytes, not softness)
            let centres: Vec<f64> = top.iter().zip(&bot).map(|(t, b)| (t.1 + b.1) / 2.0).collect();
            for f in [1.0, 0.6] {
                let band: Vec<Pt> = top.iter().zip(&centres).map(|(&(u, y), &c)| (u, c + (y - c) * f))
                    .chain(bot.iter().zip(&centres).rev().map(|(&(u, y), &c)| (u, c + (y - c) * f)))
                    .collect();
                let _ = write!(out, r#"<path d="{}z" fill="{col}" fill-opacity="{}"/>"#, smooth_d(&band), op2(op / 2.0));
            }
        } else {
            let poly: Vec<Pt> = top.iter().copied().chain(bot.iter().rev().copied()).collect();
            let _ = write!(out, r#"<path d="{}z" fill="{col}" fill-opacity="{}"/>"#, smooth_d(&poly), op2(op));
        }
        stats.streaks += 1;
    }

    // --- growth rings ------------------------------------------------------------
    let sp_px = ppi / p.rings_per_in;
    let lod_k = ((MIN_RING_PX / sp_px).ceil() as usize).max(1);
    let boost = (lod_k as f64).powf(0.75);
    let wl = p.wiggle_len_in * ppi;
    let mut rr = substream(seed, 3);
    let mut rj = substream(seed, 8); // pore-row jitter
    let step = (wl / 10.0).clamp(1.0, 4.0); // resolve the wiggle wavelength
    let mut lines: Vec<Run> = Vec::new();
    let mut pore_runs: Vec<Run> = Vec::new();
    let mut zones: Vec<(Vec<Run>, f64)> = Vec::new();

    match s.figure {
        Figure::Cathedral => {
            // pith offset from the board centre: < half width puts arch tips on the face
            let vc0 = fw / 2.0 + rr.sign() * rr.uniform(0.5, 1.5) * p.pith_offset_in * ppi;
            let gamma = rr.uniform(-0.05, 0.05);
            let ecc = rr.uniform(-0.3, 0.3);
            let (ka, kb) = (1.0 + ecc, (1.0 + ecc) / (1.0 + 2.0 * ecc));
            let d0 = rr.uniform(0.57, 1.43) * p.pith_depth_in * ppi;
            let beta = rr.sign() * rr.uniform(0.6, 1.4) * p.pith_tilt;
            let n_us = ((l + 24.0) / step) as usize + 1;
            let us: Vec<f64> = (0..n_us).map(|i| -12.0 + step * i as f64).collect();
            let dz: Vec<f64> = us.iter()
                .map(|&u| d0 + beta * (u - l / 2.0) + p.depth_noise_in * ppi * fbm(u / (6.0 * ppi), 1.7, seed.wrapping_add(3), 4))
                .collect();
            let vcs: Vec<f64> = us.iter()
                .map(|&u| vc0 + gamma * (u - l / 2.0) + 0.12 * fw * fbm(u / (5.0 * ppi), 4.4, seed.wrapping_add(19), 2))
                .collect();
            let dmin = dz.iter().fold(f64::INFINITY, |m, d| m.min(d.abs()));
            let xmax = vcs.iter().fold(0.0f64, |m, &c| m.max(c.abs()).max((fw - c).abs())) / ka.min(kb) + 4.0;
            let rmax = dz.iter().fold(0.0f64, |m, d| m.max(d.abs())).hypot(xmax);
            let cat = Cathedral { us, dz, vcs, ka, kb, cap: p.apex_widen_cap };

            let ws = ring_widths(&mut rr, 600, p.ring_cv);
            let mut r = 0.0;
            let mut visible = 0usize;
            for (j, &w) in ws.iter().enumerate() {
                r += w * sp_px;
                if r > rmax {
                    break;
                }
                if r < dmin {
                    continue;
                }
                visible += 1;
                if !(visible - 1).is_multiple_of(lod_k) {
                    continue;
                }
                lines.extend(cat.contour(&cx, r));
                let nxt = ws.get(j + 1).map_or(w, |&n| n) * sp_px; // next ring's width
                if p.zone_op > 0.0 {
                    let zw = p.zone_frac * if s.zone_side > 0 { nxt } else { w * sp_px };
                    zones.push((cat.contour(&cx, r + f64::from(s.zone_side) * zw / 2.0), zw));
                }
                if s.pores {
                    // earlywood of the next ring lies just outside this boundary
                    let band = p.pore_band_frac.map_or(p.pore_band_in * ppi, |f| f * nxt);
                    for off in pore_offsets(a, band, &mut rj, (nxt / sp_px).min(1.5)) {
                        pore_runs.extend(cat.contour(&cx, r + off));
                    }
                }
            }
        }
        Figure::Quarter => {
            let tau = rr.uniform(-0.03, 0.03);
            let fan = rr.uniform(-0.02, 0.02);
            let ws = ring_widths(&mut rr, 400, p.ring_cv);
            let mut v = rr.uniform(0.0, sp_px);
            let n = ((l / step) as usize).max(30);
            let uu: Vec<f64> = (0..=n).map(|i| -10.0 + (l + 20.0) * i as f64 / n as f64).collect();
            let straight = |v0: f64| -> Run {
                let slope = tau + fan * (v0 - fw / 2.0) / fw;
                Run::straight(uu.iter().map(|&u| (u, v0 + slope * (u - l / 2.0) + cx.warp(u, v0))).collect())
            };
            let mut i = 0;
            while v < fw + 3.0 * sp_px && i < ws.len() {
                let nxt = sp_px * ws[i];
                if !i.is_multiple_of(lod_k) {
                    v += nxt;
                    i += 1;
                    continue;
                }
                lines.push(straight(v));
                if p.zone_op > 0.0 {
                    let zw = p.zone_frac * nxt;
                    zones.push((vec![straight(v + f64::from(s.zone_side) * zw / 2.0)], zw));
                }
                if s.pores {
                    let band = p.pore_band_frac.map_or(p.pore_band_in * ppi, |f| f * nxt);
                    for off in pore_offsets(a, band, &mut rj, (nxt / sp_px).min(1.5)) {
                        pore_runs.push(straight(v + off));
                    }
                }
                v += nxt;
                i += 1;
            }
        }
    }

    // width along a run: base x section geometry (R/x) x slow noise along the arc length
    let mut rw = substream(seed, 9);
    let mut widths_along = |pts: &[Pt], mul: &[f64], base: f64, gain: f64| -> Vec<f64> {
        let off = rw.uniform(0.0, 100.0);
        let mut sacc = 0.0;
        pts.iter().enumerate().map(|(i, pt)| {
            if i > 0 {
                sacc += (pt.0 - pts[i - 1].0).hypot(pt.1 - pts[i - 1].1);
            }
            let nz = 1.0 + p.thick_noise * 1.6 * fbm(sacc / (0.8 * ppi) + off, 3.3, seed.wrapping_add(61), 2);
            let m = 1.0 + gain * (mul[i] - 1.0);
            (base * m * nz.max(0.35)).max(FLOOR_LINE_PX)
        }).collect()
    };

    // --- growth-zone bands ----------------------------------------------------------
    let mut rz = substream(seed, 7);
    let zb = p.zone_break;
    let zone_dashes: Vec<String> = if zb > 0.0 {
        (0..4).map(|_| long_dashes(&mut rz, ppi, 0.5 + 0.9 * zb, 0.25 + 0.6 * zb)).collect()
    } else {
        Vec::new()
    };
    let mut batch = Batch::default();
    for (runs, zw) in &zones {
        let op = op2(clamp01(p.zone_op * boost * level3(&mut rz, 0.6, 1.4)));
        let zwq = ((zw * 2.0).round() / 2.0).max(0.5); // 0.5 px steps so rings can share a style
        for run in runs {
            let (tp, tm) = run.thinned();
            if p.ribbon && zb <= 0.0 && run.pts.len() > 2 {
                let w = widths_along(&tp, &tm, *zw, 0.5);
                batch.add(&format!(r#"fill="{}" fill-opacity="{op}""#, pal.late), &ribbon_poly_d(&tp, &w));
            } else if zb > 0.0 {
                let attrs = format!(r#"fill="none" stroke="{}" stroke-width="{zwq}" stroke-opacity="{op}" stroke-dasharray="{}""#,
                    pal.late, zone_dashes[pick(&mut rz, 4)]);
                for part in phase_split(&tp, &mut rz) {
                    batch.add(&attrs, &poly_d(&part, false));
                }
            } else {
                batch.add(&format!(r#"fill="none" stroke="{}" stroke-width="{zwq}" stroke-opacity="{op}""#, pal.late), &poly_d(&tp, false));
            }
            stats.zones += 1;
        }
    }
    batch.flush(&mut out);

    // --- ring traces -----------------------------------------------------------------
    let mut rs = substream(seed, 4);
    let lw = p.line_w_in * ppi;
    let broken_dashes: Vec<String> = (0..4).map(|_| long_dashes(&mut rs, ppi, p.line_dash_in, p.line_gap_in)).collect();
    let thick_dashes: Vec<String> = (0..4).map(|_| long_dashes(&mut rs, ppi, 0.6, 0.7)).collect();
    if s.line_mode != LineMode::None {
        for run in &lines {
            let (tp, tm) = run.thinned();
            let w = (lw * level3(&mut rs, 0.6, 1.4)).max(FLOOR_LINE_PX);
            let op = clamp01(p.line_op * boost * level3(&mut rs, 0.7, 1.3));
            stats.lines += 1;
            if p.ribbon && s.line_mode == LineMode::Continuous && run.pts.len() > 2 && w >= RIBBON_MIN_PX {
                let ww = widths_along(&tp, &tm, w, 1.0);
                batch.add(&format!(r#"fill="{}" fill-opacity="{}""#, pal.late, op2(op)), &ribbon_poly_d(&tp, &ww));
                continue;
            }
            let stroke = format!(r#"fill="none" stroke="{}" stroke-width="{:.2}" stroke-opacity="{}""#, pal.late, w, op2(op));
            if s.line_mode == LineMode::Broken {
                let attrs = format!(r#"{stroke} stroke-dasharray="{}""#, broken_dashes[pick(&mut rs, 4)]);
                for part in phase_split(&tp, &mut rs) {
                    batch.add(&attrs, &poly_d(&part, false));
                }
            } else {
                batch.add(&stroke, &poly_d(&tp, false));
            }
            if p.line_thick_var > 0.0 {
                let attrs = format!(r#"fill="none" stroke="{}" stroke-width="{:.2}" stroke-opacity="{}" stroke-linecap="round" stroke-dasharray="{}""#,
                    pal.late, w * level3(&mut rs, 1.3, 1.9), op2(op * 0.45 * p.line_thick_var), thick_dashes[pick(&mut rs, 4)]);
                for part in phase_split(&tp, &mut rs) {
                    batch.add(&attrs, &poly_d(&part, false));
                }
            }
        }
    }
    batch.flush(&mut out);

    // --- pores: aperiodic short dashes along earlywood rows --------------------------
    let mut rp = substream(seed, 6);
    let typical_band = p.pore_band_frac.map_or(p.pore_band_in * ppi, |f| f * sp_px * lod_k as f64);
    let pore_lod = (f64::from(p.pore_rows) / (typical_band / MIN_PORE_ROW_PX).floor().max(1.0)).max(1.0);
    let pore_boost = boost * pore_lod.powf(0.75);
    let (pl, pg) = (p.pore_len_in * ppi, p.pore_gap_in * ppi);
    let pw = (p.pore_w_in * ppi).max(FLOOR_PORE_PX);
    // a few shared aperiodic dash lists (lognormal-ish dashes, exponential-ish gaps)
    let pore_dashes: Vec<String> = (0..4).map(|_| {
        let mut da = Vec::with_capacity(12);
        for _ in 0..6 {
            let t = rp.random() + rp.random() + rp.random() - 1.5;
            da.push((pl * (0.5 * t).exp()).max(0.6));
            let u = rp.random();
            da.push((pg * (0.15 + 1.7 * u * u)).max(0.8));
        }
        dash_list(&da)
    }).collect();
    for run in &pore_runs {
        let (tp, _) = run.thinned();
        let pwi = pw * if p.pore_jitter > 0.0 { level3(&mut rp, 0.6, 1.4) } else { 1.0 };
        let op = clamp01(p.pore_op * pore_boost * level3(&mut rp, 0.7, 1.3));
        let attrs = format!(r#"stroke-width="{:.2}" stroke-opacity="{}" stroke-dasharray="{}""#, pwi, op2(op), pore_dashes[pick(&mut rp, 4)]);
        for part in phase_split(&tp, &mut rp) {
            batch.add(&attrs, &poly_d(&part, false));
        }
        stats.pores += 1;
    }
    if !pore_runs.is_empty() {
        let _ = write!(out, r#"<g fill="none" stroke="{}">"#, pal.late);
        batch.flush(&mut out);
        out.push_str("</g>");
    }

    // --- ray flecks: tapered ribbons along stacked arcs (quartersawn) ------------------
    if s.flecks {
        let _ = write!(out, r#"<g fill="{}">"#, pal.alt);
        let mut rf = substream(seed, 5);
        let sgn = rf.sign(); // arcs open toward +u or -u
        let mut u_k = rf.uniform(-0.5, 0.5) * p.fleck_arc_spacing_in * ppi;
        while u_k < l + 1.5 * ppi {
            let vc_k = fw * (0.5 + 0.45 * fbm(u_k / (3.0 * ppi), 6.6, seed.wrapping_add(41), 2));
            let c = sgn * p.fleck_arc_curv * rf.uniform(0.6, 1.4) / ppi; // u = u_k + c (v - vc)^2
            let mut v = -0.3 * ppi;
            while v < fw + 0.3 * ppi {
                let t = rf.random() + rf.random() + rf.random() - 1.5;
                let seg = p.fleck_len_in * ppi * (p.fleck_len_sd * 2.0 * t).exp();
                let gap = p.fleck_gap_in * ppi * (0.3 + 1.4 * rf.random());
                // walk the parabola by arc length: dv = ds / sqrt(1 + (2c(v-vc))^2)
                let (mut pts, mut sl, mut vv) = (Vec::new(), 0.0, v);
                while sl < seg && vv < fw + 0.3 * ppi {
                    pts.push((u_k + c * (vv - vc_k).powi(2), vv));
                    let ds = seg / (seg / 4.0).clamp(4.0, 8.0); // ~4 px steps, 4..8 per side
                    vv += ds / (1.0 + (2.0 * c * (vv - vc_k)).powi(2)).sqrt();
                    sl += ds;
                }
                if pts.len() >= 3 && (-0.1 * ppi..l + 0.1 * ppi).contains(&pts[pts.len() / 2].0) {
                    let hw = (0.5 * p.fleck_aspect * seg * rf.uniform(0.7, 1.3)).max(0.5);
                    let n = pts.len();
                    let (mut left, mut right) = (Vec::with_capacity(n), Vec::with_capacity(n));
                    for (i, &(pu, pv)) in pts.iter().enumerate() {
                        let (a0, b0) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
                        let (tu, tv) = (b0.0 - a0.0, b0.1 - a0.1);
                        let nn = tu.hypot(tv);
                        let nn = if nn == 0.0 { 1.0 } else { nn };
                        let (nu, nv) = (-tv / nn, tu / nn);
                        let w = hw * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).sin().powf(0.7) * rf.uniform(0.75, 1.25);
                        left.push((pu + w * nu, pv + w * nv));
                        right.push((pu - w * nu, pv - w * nv));
                    }
                    right.reverse();
                    left.extend(right);
                    let op = op2(clamp01(p.fleck_op * level3(&mut rf, 0.7, 1.3)));
                    batch.add(&format!(r#"fill-opacity="{op}""#), &poly_d(&ccw(left), true));
                    stats.flecks += 1;
                }
                v = vv + gap / (1.0 + (2.0 * c * (vv - vc_k)).powi(2)).sqrt();
            }
            u_k += p.fleck_arc_spacing_in * ppi * (0.35 * (rf.random() + rf.random() - 1.0)).exp();
        }
        batch.flush(&mut out);
        out.push_str("</g>");
    }
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::wood::appearance::wood_appearance;

    #[test]
    fn ring_widths_vary_like_tree_rings() {
        let mut rr = substream(1, 3);
        let w = ring_widths(&mut rr, 600, 0.38);
        let mean = w.iter().sum::<f64>() / w.len() as f64;
        let sd = (w.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / w.len() as f64).sqrt();
        assert!((mean - 1.0).abs() < 1e-9);
        assert!((0.25..0.7).contains(&(sd / mean)), "ring-width CV {}", sd / mean);
    }

    #[test]
    fn deterministic_per_seed() {
        let a = wood_appearance("douglas_fir", None);
        let (x, _) = side_grain(a, 400.0, 30.0, 20.0, 42);
        let (y, _) = side_grain(a, 400.0, 30.0, 20.0, 42);
        let (z, _) = side_grain(a, 400.0, 30.0, 20.0, 43);
        assert_eq!(x, y);
        assert_ne!(x, z);
    }

    #[test]
    fn ring_porous_emits_pores_but_no_ring_lines() {
        let (_, st) = side_grain(wood_appearance("red_oak", None), 450.0, 36.0, 18.0, 7);
        assert_eq!(st.lines, 0);
        assert!(st.pores > 0);
    }

    #[test]
    fn quartersawn_emits_flecks() {
        let (_, st) = side_grain(wood_appearance("white_oak", Some("quartersawn")), 450.0, 36.0, 18.0, 7);
        assert!(st.flecks > 0 && st.lines > 0);
    }

    #[test]
    fn arch_apex_is_joined() {
        // Pith centred on the face with depth crossing R mid-piece -> one run through the apex
        let a = wood_appearance("douglas_fir", None);
        let cx = Ctx { a, fw: 60.0, ppi: 20.0, seed: 1 };
        let us: Vec<f64> = (0..=150).map(|i| i as f64 * 2.0).collect();
        let dz: Vec<f64> = us.iter().map(|&u| 40.0 + 0.2 * (u - 150.0)).collect(); // R=40 at u=150
        let vcs = vec![30.0; us.len()];
        let cat = Cathedral { us, dz, vcs, ka: 1.0, kb: 1.0, cap: 2.0 };
        let runs = cat.contour(&cx, 40.0);
        assert_eq!(runs.len(), 1, "both arch sides in one run");
        assert!(runs[0].apex.iter().any(|&f| f), "apex point kept");
    }
}
