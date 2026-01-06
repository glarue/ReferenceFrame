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
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
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
                format!("{} ({}\")", fraction_no_quote, decimal)
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
                format!("{} mm ({} mm)", trimmed, decimal)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversions() {
        assert!((inches_to_mm(1.0) - 25.4).abs() < 0.001);
        assert!((mm_to_inches(25.4) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(8, 12), 4);
        assert_eq!(gcd(7, 13), 1);
    }

    #[test]
    fn test_format_basic() {
        assert_eq!(format_inches_as_fraction(1.0), "1\"");
        assert_eq!(format_inches_as_fraction(0.5), "1/2\"");
        assert_eq!(format_inches_as_fraction(12.75), "12 3/4\"");
    }
}
