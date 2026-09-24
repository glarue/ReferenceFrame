//! Aspect-ratio display and locking.
//!
//! Ratios are height / width throughout. Provides display names for common
//! ratios and `AspectLockState` for keeping proportions while editing.

use serde::{Deserialize, Serialize};

/// Tolerance for matching common aspect ratios (~1% of ratio value).
/// At 0.01, a 4:5 ratio (0.8) matches anything in [0.79, 0.81].
const ASPECT_RATIO_MATCH_TOLERANCE: f64 = 0.01;

/// Common aspect ratios with their display names
/// Format: (height, width, display_name)
const COMMON_RATIOS: &[(i32, i32, &str)] = &[
    (1, 1, "1:1"),
    (4, 3, "4:3"), (3, 4, "3:4"),
    (3, 2, "3:2"), (2, 3, "2:3"),
    (5, 4, "5:4"), (4, 5, "4:5"),
    (16, 9, "16:9"), (9, 16, "9:16"),
    (5, 7, "5:7"), (7, 5, "7:5"),
    (11, 14, "11:14"), (14, 11, "14:11"),
];

/// Get a nice display string from a ratio value (height/width)
pub fn get_aspect_ratio_display_from_ratio(ratio: f64) -> String {
    if ratio == 0.0 {
        return "—".to_string();
    }

    // Check against common ratios
    for &(h, w, name) in COMMON_RATIOS {
        if (ratio - h as f64 / w as f64).abs() < ASPECT_RATIO_MATCH_TOLERANCE {
            return name.to_string();
        }
    }

    // Fall back to decimal ratio
    // If ratio < 1, show as 1:x instead of 0.xx:1 for readability
    if ratio < 1.0 {
        let inv_ratio = 1.0 / ratio;
        // Use integer if it's a whole number, otherwise 2 decimals
        if (inv_ratio - inv_ratio.round()).abs() < ASPECT_RATIO_MATCH_TOLERANCE {
            format!("1:{}", inv_ratio.round() as i32)
        } else {
            format!("1:{:.2}", inv_ratio)
        }
    } else {
        // Use integer if it's a whole number, otherwise 2 decimals
        if (ratio - ratio.round()).abs() < ASPECT_RATIO_MATCH_TOLERANCE {
            format!("{}:1", ratio.round() as i32)
        } else {
            format!("{:.2}:1", ratio)
        }
    }
}

/// Get a nice display string for the aspect ratio
pub fn get_aspect_ratio_display(height: f64, width: f64) -> String {
    if width == 0.0 || height == 0.0 {
        return "—".to_string();
    }
    let ratio = height / width;
    get_aspect_ratio_display_from_ratio(ratio)
}

/// Invert an aspect ratio (for when orientation is swapped)
pub fn invert_ratio(ratio: f64) -> f64 {
    if ratio == 0.0 {
        return 0.0;
    }
    1.0 / ratio
}

/// Manages the aspect ratio lock state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectLockState {
    locked: bool,
    ratio: Option<f64>,  // height / width when locked
}

impl Default for AspectLockState {
    fn default() -> Self {
        Self::new()
    }
}

impl AspectLockState {
    /// Create a new unlocked aspect lock state
    pub fn new() -> Self {
        Self {
            locked: false,
            ratio: None,
        }
    }

    /// Whether the aspect ratio is currently locked
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// The locked ratio, or None if not locked
    pub fn ratio(&self) -> Option<f64> {
        self.ratio
    }

    /// Lock the aspect ratio to the given dimensions
    ///
    /// Returns true if successfully locked, false if width is zero
    pub fn lock(&mut self, height: f64, width: f64) -> bool {
        if width <= 0.0 {
            return false;
        }
        self.locked = true;
        self.ratio = Some(height / width);
        true
    }

    /// Unlock the aspect ratio
    pub fn unlock(&mut self) {
        self.locked = false;
        self.ratio = None;
    }

    /// Toggle the lock state
    ///
    /// Returns the new locked state
    pub fn toggle(&mut self, height: f64, width: f64) -> bool {
        if self.locked {
            self.unlock();
        } else {
            self.lock(height, width);
        }
        self.locked
    }

    /// Invert the locked ratio (for orientation swap)
    pub fn invert(&mut self) {
        if let Some(ratio) = self.ratio {
            self.ratio = Some(invert_ratio(ratio));
        }
    }

    /// Calculate width for a given height, rounded to step
    ///
    /// Returns 0.0 when unlocked or the locked ratio is zero; a step that
    /// isn't a positive finite number means "don't round" (never NaN).
    pub fn get_width_for_height(&self, height: f64, step: f64) -> f64 {
        if !self.locked {
            return 0.0;
        }

        match self.ratio {
            Some(ratio) if ratio != 0.0 => round_to_step(height / ratio, step),
            _ => 0.0,
        }
    }

    /// Calculate height for a given width, rounded to step
    ///
    /// Same guards as [`AspectLockState::get_width_for_height`].
    pub fn get_height_for_width(&self, width: f64, step: f64) -> f64 {
        if !self.locked {
            return 0.0;
        }

        match self.ratio {
            Some(ratio) if ratio != 0.0 => round_to_step(width * ratio, step),
            _ => 0.0,
        }
    }
}

/// Round `value` to the nearest multiple of `step`; a step that isn't a
/// positive finite number leaves the value unrounded (avoids `x / 0` → NaN).
fn round_to_step(value: f64, step: f64) -> f64 {
    if step > 0.0 && step.is_finite() {
        (value / step).round() * step
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_ratio_recognition() {
        assert_eq!(get_aspect_ratio_display(4.0, 3.0), "4:3");
        assert_eq!(get_aspect_ratio_display(16.0, 9.0), "16:9");
    }

    #[test]
    fn test_aspect_lock_basic() {
        let mut state = AspectLockState::new();
        assert!(!state.locked());

        state.lock(12.0, 8.0);
        assert!(state.locked());
        assert!(state.ratio().is_some());
    }

    #[test]
    fn test_display_zero_inputs() {
        assert_eq!(get_aspect_ratio_display(0.0, 5.0), "—");
        assert_eq!(get_aspect_ratio_display(5.0, 0.0), "—");
        assert_eq!(get_aspect_ratio_display(0.0, 0.0), "—");
    }

    #[test]
    fn test_display_from_ratio_zero() {
        assert_eq!(get_aspect_ratio_display_from_ratio(0.0), "—");
    }

    #[test]
    fn test_display_fallback_ratio_gt_1() {
        // 2.5:1 — not a common ratio
        assert_eq!(get_aspect_ratio_display_from_ratio(2.5), "2.50:1");
        // Whole number ratio
        assert_eq!(get_aspect_ratio_display_from_ratio(3.0), "3:1");
    }

    #[test]
    fn test_display_fallback_ratio_lt_1() {
        // 0.4 → 1:2.50
        assert_eq!(get_aspect_ratio_display_from_ratio(0.4), "1:2.50");
        // 0.25 → 1:4
        assert_eq!(get_aspect_ratio_display_from_ratio(0.25), "1:4");
    }

    #[test]
    fn test_invert_ratio() {
        assert!((invert_ratio(2.0) - 0.5).abs() < 0.001);
        assert_eq!(invert_ratio(0.0), 0.0);
        assert!((invert_ratio(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_lock_with_zero_width_returns_false() {
        let mut state = AspectLockState::new();
        assert!(!state.lock(10.0, 0.0));
        assert!(!state.locked());
    }

    #[test]
    fn test_toggle() {
        let mut state = AspectLockState::new();
        let locked = state.toggle(12.0, 8.0);
        assert!(locked);
        assert!(state.locked());

        let locked = state.toggle(12.0, 8.0);
        assert!(!locked);
        assert!(!state.locked());
    }

    #[test]
    fn test_invert_locked_ratio() {
        let mut state = AspectLockState::new();
        state.lock(12.0, 8.0); // ratio = 1.5
        state.invert();
        let ratio = state.ratio().unwrap();
        assert!((ratio - 1.0 / 1.5).abs() < 0.001);
    }

    #[test]
    fn test_get_width_for_height_unlocked_returns_zero() {
        let state = AspectLockState::new();
        assert_eq!(state.get_width_for_height(10.0, 0.125), 0.0);
    }

    #[test]
    fn test_get_width_for_height_rounds_to_step() {
        let mut state = AspectLockState::new();
        state.lock(12.0, 8.0); // ratio = 1.5
        // width = 10.0 / 1.5 = 6.666..., rounded to nearest 0.125 = 6.625
        let w = state.get_width_for_height(10.0, 0.125);
        assert!((w - 6.625).abs() < 0.001);
    }

    #[test]
    fn test_get_height_for_width_rounds_to_step() {
        let mut state = AspectLockState::new();
        state.lock(12.0, 8.0); // ratio = 1.5
        // height = 5.0 * 1.5 = 7.5, rounded to nearest 0.125 = 7.5
        let h = state.get_height_for_width(5.0, 0.125);
        assert!((h - 7.5).abs() < 0.001);
    }

    #[test]
    fn test_zero_or_invalid_step_never_nan() {
        let mut state = AspectLockState::new();
        state.lock(12.0, 8.0); // ratio = 1.5
        for step in [0.0, -0.125, f64::NAN, f64::INFINITY] {
            let w = state.get_width_for_height(10.0, step);
            let h = state.get_height_for_width(5.0, step);
            assert!((w - 10.0 / 1.5).abs() < 1e-9, "step {step}: width unrounded, got {w}");
            assert!((h - 7.5).abs() < 1e-9, "step {step}: height unrounded, got {h}");
        }
    }

    #[test]
    fn test_zero_ratio_returns_zero() {
        let mut state = AspectLockState::new();
        state.lock(0.0, 8.0); // ratio = 0
        assert_eq!(state.get_width_for_height(10.0, 0.125), 0.0);
        assert_eq!(state.get_height_for_width(10.0, 0.125), 0.0);
        assert_eq!(state.get_width_for_height(10.0, 0.0), 0.0);
    }

    #[test]
    fn test_common_ratios_have_no_duplicates() {
        // Each ratio value appears once, so every entry is reachable
        for (i, &(h1, w1, n1)) in COMMON_RATIOS.iter().enumerate() {
            for &(h2, w2, n2) in &COMMON_RATIOS[i + 1..] {
                let (r1, r2) = (h1 as f64 / w1 as f64, h2 as f64 / w2 as f64);
                assert!((r1 - r2).abs() >= ASPECT_RATIO_MATCH_TOLERANCE,
                    "{n1} and {n2} match the same ratios");
            }
        }
        // 8x10 still displays via the 4:5 entry
        assert_eq!(get_aspect_ratio_display(8.0, 10.0), "4:5");
        assert_eq!(get_aspect_ratio_display(10.0, 8.0), "5:4");
    }
}
