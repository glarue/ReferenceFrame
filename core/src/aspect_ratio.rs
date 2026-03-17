// Aspect ratio utilities
//
// Ported from Python aspect_ratio.py with identical behavior

use serde::{Deserialize, Serialize};

/// Common aspect ratios with their display names
/// Format: (height, width, display_name)
const COMMON_RATIOS: &[(i32, i32, &str)] = &[
    (1, 1, "1:1"),
    (4, 3, "4:3"), (3, 4, "3:4"),
    (3, 2, "3:2"), (2, 3, "2:3"),
    (5, 4, "5:4"), (4, 5, "4:5"),
    (16, 9, "16:9"), (9, 16, "9:16"),
    (5, 7, "5:7"), (7, 5, "7:5"),
    (8, 10, "4:5"), (10, 8, "5:4"),  // Same as 4:5, 5:4
    (11, 14, "11:14"), (14, 11, "14:11"),
];

/// Get a nice display string from a ratio value (height/width)
pub fn get_aspect_ratio_display_from_ratio(ratio: f64) -> String {
    if ratio == 0.0 {
        return "—".to_string();
    }

    // Check against common ratios
    for &(h, w, name) in COMMON_RATIOS {
        if (ratio - h as f64 / w as f64).abs() < 0.01 {
            return name.to_string();
        }
    }

    // Fall back to decimal ratio
    // If ratio < 1, show as 1:x instead of 0.xx:1 for readability
    if ratio < 1.0 {
        let inv_ratio = 1.0 / ratio;
        // Use integer if it's a whole number, otherwise 2 decimals
        if (inv_ratio - inv_ratio.round()).abs() < 0.01 {
            format!("1:{}", inv_ratio.round() as i32)
        } else {
            format!("1:{:.2}", inv_ratio)
        }
    } else {
        // Use integer if it's a whole number, otherwise 2 decimals
        if (ratio - ratio.round()).abs() < 0.01 {
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

/// Calculate the unknown dimension given one dimension and the aspect ratio
///
/// # Arguments
/// * `known_value` - The known dimension value
/// * `ratio` - The aspect ratio (height/width)
/// * `known_is_height` - True if known_value is the height, False if width
pub fn calculate_dimension_from_ratio(
    known_value: f64,
    ratio: f64,
    known_is_height: bool,
) -> f64 {
    if known_is_height {
        // height = ratio * width, so width = height / ratio
        known_value / ratio
    } else {
        // height = ratio * width
        known_value * ratio
    }
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
    pub fn get_width_for_height(&self, height: f64, step: f64) -> f64 {
        if !self.locked {
            return 0.0;
        }

        match self.ratio {
            Some(ratio) if ratio != 0.0 => {
                let width = height / ratio;
                (width / step).round() * step
            }
            _ => 0.0,
        }
    }

    /// Calculate height for a given width, rounded to step
    pub fn get_height_for_width(&self, width: f64, step: f64) -> f64 {
        if !self.locked {
            return 0.0;
        }

        match self.ratio {
            Some(ratio) => {
                let height = width * ratio;
                (height / step).round() * step
            }
            None => 0.0,
        }
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
    fn test_calculate_dimension_from_ratio() {
        // Known height, find width: width = height / ratio
        let width = calculate_dimension_from_ratio(12.0, 1.5, true);
        assert!((width - 8.0).abs() < 0.001);

        // Known width, find height: height = width * ratio
        let height = calculate_dimension_from_ratio(8.0, 1.5, false);
        assert!((height - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_dimension_from_ratio_zero_produces_infinity() {
        // Division by zero ratio produces infinity — callers must guard
        let result = calculate_dimension_from_ratio(10.0, 0.0, true);
        assert!(result.is_infinite());
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
}
