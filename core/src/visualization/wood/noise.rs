//! Deterministic hashing, value noise and per-layer random streams.
//!
//! Basic float ops only (plus `floor`), so a given seed renders the same grain
//! everywhere. Ported from the prototype in the private `tools/wood-fit` repo
//! (`proto_after.py`): the noise is bit-compatible with it; the random streams
//! are SplitMix64 rather than Python's Mersenne Twister.

/// lowbias32 finalizer: the seed enters non-linearly, so seed, seed+1, seed+131…
/// give unrelated fields.
pub(crate) fn mix32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^ (x >> 16)
}

/// Lattice hash in [0, 1].
fn h2(ix: i64, iy: i64, seed: u32) -> f64 {
    let inner = mix32((iy.wrapping_mul(668_265_263) as u32).wrapping_add(mix32(seed)));
    let h = mix32((ix.wrapping_mul(374_761_393) as u32).wrapping_add(inner));
    f64::from(h & 0x00FF_FFFF) / f64::from(0x00FF_FFFFu32)
}

/// Smoothstep-interpolated value noise in [-1, 1].
fn vnoise(x: f64, y: f64, seed: u32) -> f64 {
    let (fx0, fy0) = (x.floor(), y.floor());
    let (ix, iy) = (fx0 as i64, fy0 as i64);
    let (fx, fy) = (x - fx0, y - fy0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (a, b) = (h2(ix, iy, seed), h2(ix + 1, iy, seed));
    let (c, d) = (h2(ix, iy + 1, seed), h2(ix + 1, iy + 1, seed));
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    (top + (bot - top) * sy) * 2.0 - 1.0
}

/// Per-octave sub-lattice offsets + non-integer lacunarity: octave lattices never
/// coincide, so there is no flat spot at each base-lattice node.
const OCT_OFF: [(f64, f64); 5] = [(0.0, 0.0), (0.317, 0.651), (0.773, 0.139), (0.459, 0.887), (0.128, 0.402)];

/// Fractal value noise, normalized to about [-1, 1].
pub(crate) fn fbm(x: f64, y: f64, seed: u32, octaves: usize) -> f64 {
    let (mut s, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for (o, &(ox, oy)) in OCT_OFF.iter().enumerate().take(octaves) {
        s += amp * vnoise(x * f + ox, y * f + oy, mix32(seed.wrapping_mul(31).wrapping_add(o as u32)));
        norm += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    s / norm
}

pub(crate) fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// SplitMix64 stream.
pub(crate) struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub(crate) fn random(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub(crate) fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// +1 or -1 with equal probability.
    pub(crate) fn sign(&mut self) -> f64 {
        if self.random() < 0.5 { 1.0 } else { -1.0 }
    }
}

/// Independent stream per layer: one layer's element count never shifts another
/// layer's draws (stable while a dimension animates).
pub(crate) fn substream(seed: u32, tag: u32) -> Rng {
    Rng(u64::from(mix32(mix32(seed).wrapping_add(tag))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_matches_python_prototype() {
        // Reference values from tools/wood-fit/proto_after.py (mix32, fbm)
        assert_eq!(mix32(0), 0);
        assert_eq!(mix32(1), 0x6889_90C0);
        assert_eq!(mix32(12345), 0x912E_FCF7);
        for (x, y, seed, oct, want) in [
            (0.37, 0.11, 7u32, 4usize, -0.186_652_808_520_328_4),
            (-3.2, 5.9, 4_000_000_099, 4, 0.075_680_861_460_424_26),
            (10.5, 1.7, 23, 2, -0.585_389_478_940_032_3),
        ] {
            let got = fbm(x, y, seed, oct);
            assert!((got - want).abs() < 1e-12, "fbm({x},{y},{seed},{oct}) = {got}, want {want}");
        }
    }

    #[test]
    fn fbm_is_deterministic_and_bounded() {
        for i in 0..200 {
            let v = fbm(i as f64 * 0.37, i as f64 * 0.11, 7, 4);
            assert!((-1.0..=1.0).contains(&v));
            assert_eq!(v, fbm(i as f64 * 0.37, i as f64 * 0.11, 7, 4));
        }
    }

    #[test]
    fn substreams_differ_by_tag() {
        let (mut a, mut b) = (substream(5, 1), substream(5, 2));
        assert_ne!(a.random(), b.random());
    }
}
