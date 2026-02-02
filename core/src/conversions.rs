// Unit conversion and formatting utilities
//
// Ported from Python conversions.py to Rust with identical behavior

use serde::{Deserialize, Serialize};

// Constants
const INCHES_TO_MM: f64 = 25.4;

/// Unit system for displaying measurements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    Inches,
    Millimeters,
}

/// Convert inches to millimeters
pub fn inches_to_mm(value: f64) -> f64 {
    value * INCHES_TO_MM
}

/// Convert millimeters to inches
pub fn mm_to_inches(value: f64) -> f64 {
    value / INCHES_TO_MM
}

/// Calculate greatest common divisor using Euclid's algorithm
pub fn gcd(a: i32, b: i32) -> i32 {
    let a = a.abs();
    let b = b.abs();
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// A simple fraction representation for tape measure calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fraction {
    pub numerator: i32,
    pub denominator: i32,
}

impl Fraction {
    /// Create a new fraction, automatically reduced to lowest terms
    pub fn new(numerator: i32, denominator: i32) -> Self {
        if denominator == 0 {
            panic!("Denominator cannot be zero");
        }
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    /// Convert fraction to f64
    pub fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Check if fraction is zero
    pub fn is_zero(self) -> bool {
        self.numerator == 0
    }
}

impl std::fmt::Display for Fraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

/// Result of tape measure conversion
#[derive(Debug, Clone, PartialEq)]
pub struct TapeMeasureResult {
    /// Whole inches
    pub whole: i32,
    /// Base fraction (e.g., 3/4), None if whole number
    pub fraction: Option<Fraction>,
    /// Fine adjustment (e.g., +1/32 or -1/32), None if exact match
    pub adjustment: Option<Fraction>,
}

impl TapeMeasureResult {
    /// Format as a tape measure string (e.g., "4 3/4 - 1/32\"")
    pub fn format(&self) -> String {
        match (&self.fraction, &self.adjustment) {
            (None, None) => format!("{}\"", self.whole),
            (Some(frac), None) => {
                if self.whole == 0 {
                    format!("{}\"", frac)
                } else {
                    format!("{} {}\"", self.whole, frac)
                }
            }
            (Some(frac), Some(adj)) => {
                let sign = if adj.numerator > 0 { "+" } else { "-" };
                let abs_adj = Fraction::new(adj.numerator.abs(), adj.denominator);
                if self.whole == 0 {
                    format!("{} {} {}\"", frac, sign, abs_adj)
                } else {
                    format!("{} {} {} {}\"", self.whole, frac, sign, abs_adj)
                }
            }
            (None, Some(adj)) => {
                // Edge case: adjustment without base fraction (very small values)
                let sign = if adj.numerator > 0 { "+" } else { "-" };
                let abs_adj = Fraction::new(adj.numerator.abs(), adj.denominator);
                if self.whole == 0 {
                    format!("{} {}\"", sign, abs_adj)
                } else {
                    format!("{} {} {}\"", self.whole, sign, abs_adj)
                }
            }
        }
    }
}

/// Convert a decimal inch value to tape-measure friendly representation.
///
/// When `use_segments` is true, returns a base fraction plus a fine adjustment
/// (e.g., "3/4 - 1/32" instead of "23/32"). This matches how woodworkers read
/// tape measures - find the nearest major mark and adjust by a small amount.
///
/// # Arguments
/// * `value` - Measurement in decimal inches (must be non-negative)
/// * `use_segments` - If true, return base + adjustment; if false, return best single fraction
/// * `allowed_denoms` - Allowed denominators, must be in ascending order (e.g., [2, 4, 8, 16, 32])
///
/// # Returns
/// A `TapeMeasureResult` with whole inches, optional base fraction, and optional adjustment
///
/// # Examples
/// ```
/// use referenceframe_core::conversions::convert_to_tape_measure;
///
/// let result = convert_to_tape_measure(4.72, true, &[2, 4, 8, 16, 32]);
/// assert_eq!(result.whole, 4);
/// assert_eq!(result.format(), "4 3/4 - 1/32\"");
/// ```
pub fn convert_to_tape_measure(
    value: f64,
    use_segments: bool,
    allowed_denoms: &[i32],
) -> TapeMeasureResult {
    debug_assert!(value >= 0.0, "Measurement values must be non-negative");

    // Separate whole and fractional parts
    let whole = value.floor() as i32;
    let frac_val = value - whole as f64;

    // If fractional part is essentially zero, return whole number only
    if frac_val < 1e-9 {
        return TapeMeasureResult {
            whole,
            fraction: None,
            adjustment: None,
        };
    }

    // Get the finest denominator (last in sorted list)
    let finest_denom = *allowed_denoms.last().unwrap_or(&32);

    // Handle very small fractions - force to smallest increment
    let threshold = 0.5 / finest_denom as f64;
    if frac_val < threshold {
        return TapeMeasureResult {
            whole,
            fraction: Some(Fraction::new(1, finest_denom)),
            adjustment: None,
        };
    }

    // Find the best fraction using the finest denominator for maximum precision
    let fine_numerator = (frac_val * finest_denom as f64).round() as i32;

    // Check if it rounds up to a whole
    if fine_numerator >= finest_denom {
        return TapeMeasureResult {
            whole: whole + 1,
            fraction: None,
            adjustment: None,
        };
    }

    let best_fraction = Fraction::new(fine_numerator, finest_denom);

    // If not using segments, just return the best fraction
    if !use_segments {
        if best_fraction.is_zero() {
            return TapeMeasureResult {
                whole,
                fraction: None,
                adjustment: None,
            };
        }
        return TapeMeasureResult {
            whole,
            fraction: Some(best_fraction),
            adjustment: None,
        };
    }

    // Segmentation: find the best coarse fraction and calculate adjustment
    // Use all denominators except the finest for the base
    let coarse_denoms: Vec<i32> = allowed_denoms.iter()
        .filter(|&&d| d < finest_denom)
        .copied()
        .collect();

    if coarse_denoms.is_empty() {
        // No coarser denominators available, return as-is
        return TapeMeasureResult {
            whole,
            fraction: Some(best_fraction),
            adjustment: None,
        };
    }

    // Find the coarse fraction that's closest to our fine fraction
    let fine_value = best_fraction.to_f64();
    let mut best_base: Option<Fraction> = None;
    let mut best_error = f64::INFINITY;

    for &denom in &coarse_denoms {
        let numerator = (fine_value * denom as f64).round() as i32;
        let candidate = if numerator >= denom {
            Fraction::new(1, 1) // Would round up to 1
        } else {
            Fraction::new(numerator, denom)
        };

        let error = (candidate.to_f64() - fine_value).abs();

        // Prefer lower error, or same error with smaller denominator
        if error < best_error - 1e-9 ||
           (error < best_error + 1e-9 && best_base.map_or(true, |b| denom < b.denominator)) {
            best_error = error;
            best_base = Some(candidate);
        }
    }

    let base = best_base.unwrap_or(best_fraction);

    // Calculate adjustment as difference in 32nds (or finest denom)
    let base_in_finest = (base.to_f64() * finest_denom as f64).round() as i32;
    let adjustment_numerator = fine_numerator - base_in_finest;

    let adjustment = if adjustment_numerator == 0 {
        None
    } else {
        Some(Fraction::new(adjustment_numerator, finest_denom))
    };

    // Handle case where base rounds to 1
    if base.numerator >= base.denominator {
        return TapeMeasureResult {
            whole: whole + 1,
            fraction: None,
            adjustment,
        };
    }

    TapeMeasureResult {
        whole,
        fraction: if base.is_zero() { None } else { Some(base) },
        adjustment,
    }
}

/// Format a decimal inch value as a fractional measurement
///
/// Converts decimal inches to tape-measure friendly fractions (1/2, 1/4, 1/8, 1/16, 1/32)
/// with proper reduction (e.g., 2/4 becomes 1/2).
///
/// # Arguments
/// * `value` - Measurement in decimal inches
///
/// # Returns
/// Formatted string like "12 3/4\"" or "1/2\"" or "5\""
pub fn format_inches_as_fraction(value: f64) -> String {
    let whole = value.floor() as i32;
    let decimal = value - whole as f64;

    // If no fractional part, return whole number
    if decimal.abs() < 0.001 {
        return format!("{}\"", whole);
    }

    // Try common denominators (halves, quarters, eighths, sixteenths, thirty-seconds)
    for denom in [2, 4, 8, 16, 32] {
        let numerator = (decimal * denom as f64).round() as i32;

        // Check if this denominator gives a close match
        if ((numerator as f64 / denom as f64) - decimal).abs() < 0.001 {
            if numerator == 0 {
                return format!("{}\"", whole);
            }

            // Reduce the fraction
            let divisor = gcd(numerator, denom);
            let num = numerator / divisor;
            let den = denom / divisor;

            if whole > 0 {
                return format!("{} {}/{}\"", whole, num, den);
            } else {
                return format!("{}/{}\"", num, den);
            }
        }
    }

    // Fallback to decimal if no common fraction matches
    format!("{:.2}\"", value)
}

/// Format a measurement value with the appropriate unit
///
/// # Arguments
/// * `value` - Measurement in inches (always stored internally as inches)
/// * `unit` - Display unit (Inches or Millimeters)
///
/// # Returns
/// Formatted string with unit symbol
///
/// # Examples
/// ```
/// use rust_core::conversions::{format_value, Unit};
///
/// assert_eq!(format_value(12.75, Unit::Inches), "12 3/4\"");
/// assert_eq!(format_value(1.0, Unit::Millimeters), "25.4 mm");
/// ```
pub fn format_value(value: f64, unit: Unit) -> String {
    match unit {
        Unit::Inches => format_inches_as_fraction(value),
        Unit::Millimeters => {
            let mm_value = inches_to_mm(value);
            // Strip trailing zeros and decimal point if not needed
            let formatted = format!("{:.1}", mm_value);
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            format!("{} mm", trimmed)
        }
    }
}

/// Format a measurement value with decimal in parentheses (e.g., "12 3/4" (12.75)")
/// Only shows decimal if different from fraction representation
///
/// # Arguments
/// * `value` - Measurement in inches (always stored internally as inches)
/// * `unit` - Display unit (Inches or Millimeters)
///
/// # Examples
/// ```
/// assert_eq!(format_value_with_decimal(12.75, Unit::Inches), "12 3/4\" (12.75\")");
/// assert_eq!(format_value_with_decimal(10.0, Unit::Inches), "10\"");  // No decimal for whole numbers
/// ```
pub fn format_value_with_decimal(value: f64, unit: Unit) -> String {
    match unit {
        Unit::Inches => {
            let fraction = format_inches_as_fraction(value);
            // Check if this is a whole number (no fractional part)
            let whole = value.floor();
            let decimal_part = value - whole;

            if decimal_part.abs() < 0.001 {
                // Whole number - no need for decimal
                fraction
            } else {
                // Has fractional part - show decimal
                let fraction_no_quote = fraction.trim_end_matches('"');
                let decimal = format!("{:.2}", value).trim_end_matches('0').trim_end_matches('.').to_string();
                format!("{} ({})\"", fraction_no_quote, decimal)
            }
        }
        Unit::Millimeters => {
            let mm_value = inches_to_mm(value);
            let formatted = format!("{:.1}", mm_value);
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

            // Check if decimal would be redundant
            let decimal = format!("{:.2}", mm_value).trim_end_matches('0').trim_end_matches('.').to_string();
            if trimmed == decimal {
                // Same representation - no need for parenthetical
                format!("{} mm", trimmed)
            } else {
                format!("{} ({}) mm", trimmed, decimal)
            }
        }
    }
}

/// Default denominators for tape measure conversion
pub const DEFAULT_DENOMS: &[i32] = &[2, 4, 8, 16, 32];

/// Format a measurement value with configurable tape measure segmentation
///
/// # Arguments
/// * `value` - Measurement in inches
/// * `unit` - Display unit (Inches or Millimeters)
/// * `use_segments` - If true and unit is Inches, use segmented format (e.g., "3/4 - 1/32")
///
/// This is the primary formatting function for dimension labels.
pub fn format_dimension(value: f64, unit: Unit, use_segments: bool) -> String {
    match unit {
        Unit::Inches if use_segments => format_value_tape_measure(value, unit),
        _ => format_value(value, unit),
    }
}

/// Format a measurement value with tape measure segmentation
///
/// For inches, shows base fraction + adjustment (e.g., "4 3/4 - 1/32\"")
/// with decimal in parentheses if different.
///
/// # Arguments
/// * `value` - Measurement in inches
/// * `unit` - Display unit
///
/// # Examples
/// ```
/// use referenceframe_core::conversions::{format_value_tape_measure, Unit};
///
/// assert_eq!(format_value_tape_measure(4.72, Unit::Inches), "4 3/4 - 1/32\" (4.72\")");
/// assert_eq!(format_value_tape_measure(4.5, Unit::Inches), "4 1/2\"");
/// ```
pub fn format_value_tape_measure(value: f64, unit: Unit) -> String {
    match unit {
        Unit::Inches => {
            let result = convert_to_tape_measure(value, true, DEFAULT_DENOMS);
            let tape_str = result.format();

            // Only show decimal if there's an adjustment (meaning the value
            // didn't land exactly on a standard fraction)
            if result.adjustment.is_some() {
                // Format decimal, strip trailing zeros
                let decimal = format!("{:.3}", value);
                let decimal = decimal.trim_end_matches('0').trim_end_matches('.');
                let tape_no_quote = tape_str.trim_end_matches('"');
                format!("{} ({}\")", tape_no_quote, decimal)
            } else {
                tape_str
            }
        }
        Unit::Millimeters => {
            let mm_value = inches_to_mm(value);
            let formatted = format!("{:.1}", mm_value);
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            format!("{} mm", trimmed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Unit Conversion Tests ====================

    #[test]
    fn test_unit_conversions() {
        assert!((inches_to_mm(1.0) - 25.4).abs() < 0.001);
        assert!((mm_to_inches(25.4) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(8, 12), 4);
        assert_eq!(gcd(7, 13), 1);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
        assert_eq!(gcd(-8, 12), 4); // handles negative
    }

    // ==================== Fraction Tests ====================

    #[test]
    fn test_fraction_reduction() {
        let f = Fraction::new(4, 8);
        assert_eq!(f.numerator, 1);
        assert_eq!(f.denominator, 2);

        let f = Fraction::new(6, 32);
        assert_eq!(f.numerator, 3);
        assert_eq!(f.denominator, 16);
    }

    #[test]
    fn test_fraction_display() {
        assert_eq!(format!("{}", Fraction::new(3, 4)), "3/4");
        assert_eq!(format!("{}", Fraction::new(1, 32)), "1/32");
    }

    // ==================== Tape Measure Conversion Tests ====================

    #[test]
    fn test_tape_measure_whole_numbers() {
        let result = convert_to_tape_measure(4.0, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert!(result.fraction.is_none());
        assert!(result.adjustment.is_none());
        assert_eq!(result.format(), "4\"");
    }

    #[test]
    fn test_tape_measure_exact_fractions() {
        // Exact 1/2
        let result = convert_to_tape_measure(4.5, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert_eq!(result.fraction, Some(Fraction::new(1, 2)));
        assert!(result.adjustment.is_none());
        assert_eq!(result.format(), "4 1/2\"");

        // Exact 3/4
        let result = convert_to_tape_measure(4.75, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert_eq!(result.fraction, Some(Fraction::new(3, 4)));
        assert!(result.adjustment.is_none());
        assert_eq!(result.format(), "4 3/4\"");

        // Exact 1/8
        let result = convert_to_tape_measure(0.125, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 0);
        assert_eq!(result.fraction, Some(Fraction::new(1, 8)));
        assert!(result.adjustment.is_none());
        assert_eq!(result.format(), "1/8\"");
    }

    #[test]
    fn test_tape_measure_with_adjustment() {
        // 4.72 ≈ 23/32 = 0.71875
        // Best coarse: 3/4 = 24/32 = 0.75
        // Adjustment: 23/32 - 24/32 = -1/32
        let result = convert_to_tape_measure(4.72, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert_eq!(result.fraction, Some(Fraction::new(3, 4)));
        assert_eq!(result.adjustment, Some(Fraction::new(-1, 32)));
        assert_eq!(result.format(), "4 3/4 - 1/32\"");

        // 4.78 ≈ 25/32 = 0.78125
        // Best coarse: 3/4 = 24/32 = 0.75
        // Adjustment: 25/32 - 24/32 = +1/32
        let result = convert_to_tape_measure(4.78, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert_eq!(result.fraction, Some(Fraction::new(3, 4)));
        assert_eq!(result.adjustment, Some(Fraction::new(1, 32)));
        assert_eq!(result.format(), "4 3/4 + 1/32\"");
    }

    #[test]
    fn test_tape_measure_non_segmented() {
        // Without segmentation, should just return best fraction
        let result = convert_to_tape_measure(4.72, false, DEFAULT_DENOMS);
        assert_eq!(result.whole, 4);
        assert_eq!(result.fraction, Some(Fraction::new(23, 32)));
        assert!(result.adjustment.is_none());
        assert_eq!(result.format(), "4 23/32\"");
    }

    #[test]
    fn test_tape_measure_small_fractions() {
        // Very small value should round to 1/32
        let result = convert_to_tape_measure(0.01, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 0);
        assert_eq!(result.fraction, Some(Fraction::new(1, 32)));
        assert!(result.adjustment.is_none());
    }

    #[test]
    fn test_tape_measure_near_whole() {
        // 4.97 ≈ 31/32 ≈ 1 - 1/32
        let result = convert_to_tape_measure(4.97, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 5);
        assert!(result.fraction.is_none());
        // Could have -1/32 adjustment depending on rounding
    }

    #[test]
    fn test_tape_measure_fractional_only() {
        // No whole part
        let result = convert_to_tape_measure(0.72, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 0);
        assert_eq!(result.fraction, Some(Fraction::new(3, 4)));
        assert_eq!(result.adjustment, Some(Fraction::new(-1, 32)));
        assert_eq!(result.format(), "3/4 - 1/32\"");
    }

    // ==================== Format Value Tests ====================

    #[test]
    fn test_format_basic() {
        assert_eq!(format_inches_as_fraction(1.0), "1\"");
        assert_eq!(format_inches_as_fraction(0.5), "1/2\"");
        assert_eq!(format_inches_as_fraction(12.75), "12 3/4\"");
    }

    #[test]
    fn test_format_value_tape_measure_inches() {
        // Whole number
        assert_eq!(format_value_tape_measure(4.0, Unit::Inches), "4\"");

        // Exact fraction
        assert_eq!(format_value_tape_measure(4.5, Unit::Inches), "4 1/2\"");

        // With adjustment - shows decimal in parens
        let result = format_value_tape_measure(4.72, Unit::Inches);
        assert!(result.contains("3/4"));
        assert!(result.contains("1/32"));
        assert!(result.contains("4.72"));
    }

    #[test]
    fn test_format_value_tape_measure_mm() {
        assert_eq!(format_value_tape_measure(1.0, Unit::Millimeters), "25.4 mm");
        assert_eq!(format_value_tape_measure(0.5, Unit::Millimeters), "12.7 mm");
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_edge_cases() {
        // Zero
        let result = convert_to_tape_measure(0.0, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 0);
        assert!(result.fraction.is_none());
        assert_eq!(result.format(), "0\"");

        // Very small (below threshold)
        let result = convert_to_tape_measure(0.001, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 0);
        // Should round to 1/32

        // Large value
        let result = convert_to_tape_measure(100.5, true, DEFAULT_DENOMS);
        assert_eq!(result.whole, 100);
        assert_eq!(result.fraction, Some(Fraction::new(1, 2)));
    }
}
