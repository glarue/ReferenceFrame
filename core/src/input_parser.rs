//! Fractional input parser for dimension values
//!
//! Provides `DimensionInput` - a type that handles fractional dimension parsing.
//!
//! # Supported Input Formats
//! - Integer: "12" → 12.0
//! - Decimal: "12.5" → 12.5
//! - Fraction only: "3/4" → 0.75
//! - Mixed (space): "1 3/4" → 1.75
//! - Mixed (hyphen): "1-3/4" → 1.75
//! - Mixed (plus): "1+3/4" → 1.75
//! - Unicode fractions: "½", "¼", "⅜" → 0.5, 0.25, 0.375
//!
//! # Example
//! ```
//! use referenceframe_core::input_parser::DimensionInput;
//! 
//! let dim = DimensionInput::new("1 3/4");
//! assert!(dim.is_valid());
//! assert_eq!(dim.value(), 1.75);
//! assert_eq!(dim.as_fraction(16), "1 3/4");
//! ```

use serde::{Deserialize, Serialize};

/// A dimension input that handles fractional parsing and formatting.
///
/// Use this type for any numeric input field that should accept
/// fractional values like "1 3/4", "2-1/2", or "½".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionInput {
    /// The decimal value
    value: f64,
    /// Original input string
    original: String,
    /// Whether the input contained a fraction
    was_fractional: bool,
    /// Error message if parsing failed
    error: Option<String>,
}

impl DimensionInput {
    /// Create a new DimensionInput from a string
    pub fn new(input: &str) -> DimensionInput {
        parse_input(input)
    }

    /// Create from a decimal value
    pub fn from_decimal(value: f64) -> DimensionInput {
        DimensionInput {
            value,
            original: format_decimal(value),
            was_fractional: false,
            error: None,
        }
    }

    /// Get the decimal value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the decimal value directly
    pub fn set_value(&mut self, value: f64) {
        self.value = value;
        self.original = format_decimal(value);
        self.was_fractional = false;
        self.error = None;
    }

    /// Get the original input string
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Parse a new input string
    pub fn parse(&mut self, input: &str) {
        let parsed = parse_input(input);
        self.value = parsed.value;
        self.original = parsed.original;
        self.was_fractional = parsed.was_fractional;
        self.error = parsed.error;
    }

    /// Check if the input was valid
    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }

    /// Get error message if invalid
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether the input was fractional
    pub fn was_fractional(&self) -> bool {
        self.was_fractional
    }

    /// Format as a fraction with the given max denominator
    pub fn as_fraction(&self, max_denominator: u32) -> String {
        decimal_to_fraction_impl(self.value, max_denominator)
    }

    /// Format as a decimal string
    pub fn as_decimal(&self) -> String {
        format_decimal(self.value)
    }

    /// Format based on unit system (fraction for inches, decimal for mm)
    pub fn format(&self, use_fractions: bool, max_denominator: u32) -> String {
        if use_fractions {
            self.as_fraction(max_denominator)
        } else {
            self.as_decimal()
        }
    }

    /// Add another dimension
    pub fn add(&self, other: &DimensionInput) -> DimensionInput {
        DimensionInput::from_decimal(self.value + other.value)
    }

    /// Subtract another dimension
    pub fn subtract(&self, other: &DimensionInput) -> DimensionInput {
        DimensionInput::from_decimal(self.value - other.value)
    }

    /// Multiply by a scalar
    pub fn multiply(&self, scalar: f64) -> DimensionInput {
        DimensionInput::from_decimal(self.value * scalar)
    }

    /// Divide by a scalar
    ///
    /// Dividing by zero is a no-op returning the value unchanged
    pub fn divide(&self, scalar: f64) -> DimensionInput {
        if scalar == 0.0 {
            return self.clone();
        }
        DimensionInput::from_decimal(self.value / scalar)
    }
}

// ============================================================================
// Legacy API (for backwards compatibility)
// ============================================================================

/// Result of parsing a dimension input.
/// Legacy API -- prefer `DimensionInput` for all new code. This struct is retained
/// only for backwards compatibility with the WASM bindings layer.
#[deprecated(since = "1.5.0", note = "Use DimensionInput instead")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDimension {
    /// The decimal value
    decimal: f64,
    /// Normalized display string
    display: String,
    /// Whether the input contained a fraction
    was_fractional: bool,
    /// Error message if parsing failed
    error: Option<String>,
}

impl ParsedDimension {
    pub fn decimal(&self) -> f64 {
        self.decimal
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn was_fractional(&self) -> bool {
        self.was_fractional
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }
}

/// Common fractions lookup table (denominator up to 64)
const COMMON_FRACTIONS: &[(f64, u32, u32)] = &[
    // 64ths
    (0.015625, 1, 64),
    (0.03125, 1, 32),
    (0.046875, 3, 64),
    (0.0625, 1, 16),
    (0.078125, 5, 64),
    (0.09375, 3, 32),
    (0.109375, 7, 64),
    (0.125, 1, 8),
    (0.140625, 9, 64),
    (0.15625, 5, 32),
    (0.171875, 11, 64),
    (0.1875, 3, 16),
    (0.203125, 13, 64),
    (0.21875, 7, 32),
    (0.234375, 15, 64),
    (0.25, 1, 4),
    (0.265625, 17, 64),
    (0.28125, 9, 32),
    (0.296875, 19, 64),
    (0.3125, 5, 16),
    (0.328125, 21, 64),
    (0.34375, 11, 32),
    (0.359375, 23, 64),
    (0.375, 3, 8),
    (0.390625, 25, 64),
    (0.40625, 13, 32),
    (0.421875, 27, 64),
    (0.4375, 7, 16),
    (0.453125, 29, 64),
    (0.46875, 15, 32),
    (0.484375, 31, 64),
    (0.5, 1, 2),
    (0.515625, 33, 64),
    (0.53125, 17, 32),
    (0.546875, 35, 64),
    (0.5625, 9, 16),
    (0.578125, 37, 64),
    (0.59375, 19, 32),
    (0.609375, 39, 64),
    (0.625, 5, 8),
    (0.640625, 41, 64),
    (0.65625, 21, 32),
    (0.671875, 43, 64),
    (0.6875, 11, 16),
    (0.703125, 45, 64),
    (0.71875, 23, 32),
    (0.734375, 47, 64),
    (0.75, 3, 4),
    (0.765625, 49, 64),
    (0.78125, 25, 32),
    (0.796875, 51, 64),
    (0.8125, 13, 16),
    (0.828125, 53, 64),
    (0.84375, 27, 32),
    (0.859375, 55, 64),
    (0.875, 7, 8),
    (0.890625, 57, 64),
    (0.90625, 29, 32),
    (0.921875, 59, 64),
    (0.9375, 15, 16),
    (0.953125, 61, 64),
    (0.96875, 31, 32),
    (0.984375, 63, 64),
];

/// Unicode fraction characters mapping
fn unicode_to_ascii(c: char) -> Option<&'static str> {
    match c {
        '½' => Some("1/2"),
        '⅓' => Some("1/3"),
        '⅔' => Some("2/3"),
        '¼' => Some("1/4"),
        '¾' => Some("3/4"),
        '⅕' => Some("1/5"),
        '⅖' => Some("2/5"),
        '⅗' => Some("3/5"),
        '⅘' => Some("4/5"),
        '⅙' => Some("1/6"),
        '⅚' => Some("5/6"),
        '⅛' => Some("1/8"),
        '⅜' => Some("3/8"),
        '⅝' => Some("5/8"),
        '⅞' => Some("7/8"),
        '⅐' => Some("1/7"),
        '⅑' => Some("1/9"),
        '⅒' => Some("1/10"),
        _ => None,
    }
}

/// Replace unicode fractions with ASCII equivalents
/// Adds space before fraction if preceded by a digit
fn normalize_unicode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    let mut last_was_digit = false;
    
    for c in input.chars() {
        if let Some(ascii) = unicode_to_ascii(c) {
            // Add space if previous char was a digit (e.g., "2½" → "2 1/2")
            if last_was_digit {
                result.push(' ');
            }
            result.push_str(ascii);
            last_was_digit = false;
        } else {
            result.push(c);
            last_was_digit = c.is_ascii_digit();
        }
    }
    result
}

/// Reduce a fraction to lowest terms
fn reduce_fraction(num: u32, den: u32) -> (u32, u32) {
    if den == 0 {
        return (num, den);
    }
    let g = crate::conversions::gcd(num as i32, den as i32) as u32;
    (num / g, den / g)
}

/// Parse a fraction string like "3/4" into (numerator, denominator)
fn parse_fraction_part(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let num: u32 = parts[0].trim().parse().ok()?;
    let den: u32 = parts[1].trim().parse().ok()?;
    if den == 0 {
        return None;
    }
    Some((num, den))
}

/// Parse a dimension input string (internal implementation)
fn parse_input(input: &str) -> DimensionInput {
    // Handle empty input
    let input_trimmed = input.trim();
    if input_trimmed.is_empty() {
        return DimensionInput {
            value: 0.0,
            original: String::new(),
            was_fractional: false,
            error: None, // Empty is valid, just zero
        };
    }

    // Normalize unicode fractions
    let normalized = normalize_unicode(input_trimmed);
    let normalized = normalized.trim();

    // Check for negative sign
    let (is_negative, work_str) = if normalized.starts_with('-') {
        (true, normalized[1..].trim())
    } else {
        (false, normalized)
    };

    // Clean up separators: replace + with space, handle multiple spaces
    let cleaned: String = work_str
        .replace('+', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Try to parse as pure decimal/integer first
    if let Ok(val) = cleaned.parse::<f64>() {
        let val = if is_negative { -val } else { val };
        return DimensionInput {
            value: val,
            original: input.to_string(),
            was_fractional: false,
            error: None,
        };
    }

    // Check for fraction
    if cleaned.contains('/') {
        // Could be pure fraction "3/4" or mixed "1 3/4" or "1-3/4"
        
        // Split on space first (handles "1 3/4")
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        
        match parts.len() {
            1 => {
                // Could be "3/4" or "1-3/4"
                let part = parts[0];
                
                // Check for hyphen-separated mixed: "1-3/4"
                if let Some(hyphen_pos) = part.find('-') {
                    // Make sure hyphen is not in the fraction part
                    if let Some(slash_pos) = part.find('/') {
                        if hyphen_pos < slash_pos {
                            // "1-3/4" format
                            let whole_str = &part[..hyphen_pos];
                            let frac_str = &part[hyphen_pos + 1..];
                            
                            if let (Ok(whole), Some((num, den))) = 
                                (whole_str.parse::<u32>(), parse_fraction_part(frac_str)) 
                            {
                                let (num, den) = reduce_fraction(num, den);
                                let val = whole as f64 + (num as f64 / den as f64);
                                let val = if is_negative { -val } else { val };
                                return DimensionInput {
                                    value: val,
                                    original: input.to_string(),
                                    was_fractional: true,
                                    error: None,
                                };
                            }
                        }
                    }
                }
                
                // Pure fraction "3/4"
                if let Some((num, den)) = parse_fraction_part(part) {
                    let (num, den) = reduce_fraction(num, den);
                    let val = num as f64 / den as f64;
                    let val = if is_negative { -val } else { val };
                    return DimensionInput {
                        value: val,
                        original: input.to_string(),
                        was_fractional: true,
                        error: None,
                    };
                }
            }
            2 => {
                // "1 3/4" format
                let whole_str = parts[0];
                let frac_str = parts[1];
                
                if let (Ok(whole), Some((num, den))) = 
                    (whole_str.parse::<u32>(), parse_fraction_part(frac_str)) 
                {
                    let (num, den) = reduce_fraction(num, den);
                    let val = whole as f64 + (num as f64 / den as f64);
                    let val = if is_negative { -val } else { val };
                    return DimensionInput {
                        value: val,
                        original: input.to_string(),
                        was_fractional: true,
                        error: None,
                    };
                }
            }
            _ => {
                // Too many parts
                return DimensionInput {
                    value: 0.0,
                    original: input.to_string(),
                    was_fractional: false,
                    error: Some("Invalid format: too many parts".to_string()),
                };
            }
        }
    }

    // If we get here, couldn't parse
    DimensionInput {
        value: 0.0,
        original: input.to_string(),
        was_fractional: false,
        error: Some(format!("Could not parse: {}", input)),
    }
}

/// Parse a dimension input string (legacy API)
pub fn parse_dimension(input: &str) -> ParsedDimension {
    let dim = parse_input(input);
    ParsedDimension {
        decimal: dim.value,
        display: if dim.was_fractional {
            format_mixed(dim.value)
        } else {
            format_decimal(dim.value)
        },
        was_fractional: dim.was_fractional,
        error: dim.error,
    }
}

/// Format a decimal value as a simple decimal string
fn format_decimal(val: f64) -> String {
    if val == val.floor() {
        format!("{}", val as i64)
    } else {
        // Remove trailing zeros
        let s = format!("{:.6}", val);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Format a decimal as a mixed fraction if possible
fn format_mixed(val: f64) -> String {
    decimal_to_fraction_impl(val, 64)
}

/// Internal implementation of decimal to fraction conversion
fn decimal_to_fraction_impl(val: f64, max_denominator: u32) -> String {
    let is_negative = val < 0.0;
    let abs_val = val.abs();
    let whole = abs_val.floor() as i64;
    let frac_part = abs_val - whole as f64;
    
    if frac_part < 0.0001 {
        let result = if whole == 0 { "0".to_string() } else { whole.to_string() };
        return if is_negative { format!("-{}", result) } else { result };
    }
    
    // Find best matching fraction within max_denominator
    let mut best_num = 0u32;
    let mut best_den = 1u32;
    let mut best_diff = f64::MAX;
    
    for &(decimal, num, den) in COMMON_FRACTIONS {
        if den <= max_denominator {
            let diff = (frac_part - decimal).abs();
            if diff < best_diff {
                best_diff = diff;
                best_num = num;
                best_den = den;
            }
        }
    }
    
    let prefix = if is_negative { "-" } else { "" };
    if best_diff < 0.001 {
        if whole == 0 {
            format!("{}{}/{}", prefix, best_num, best_den)
        } else {
            format!("{}{} {}/{}", prefix, whole, best_num, best_den)
        }
    } else {
        // No good fraction match
        format_decimal(val)
    }
}

/// Convert a decimal to the nearest common fraction (legacy API)
pub fn decimal_to_fraction(val: f64, max_denominator: u32) -> String {
    decimal_to_fraction_impl(val, max_denominator)
}

/// Check if input is a valid dimension string
pub fn is_valid_dimension_input(input: &str) -> bool {
    let result = parse_dimension(input);
    result.is_valid()
}

/// Get common fractions for a picker UI (returns JSON array)
pub fn get_common_fractions(max_denominator: u32) -> String {
    let fractions: Vec<serde_json::Value> = COMMON_FRACTIONS
        .iter()
        .filter(|(_, _, den)| *den <= max_denominator)
        .map(|(decimal, num, den)| {
            serde_json::json!({
                "decimal": decimal,
                "display": format!("{}/{}", num, den),
                "numerator": num,
                "denominator": den
            })
        })
        .collect();
    
    serde_json::to_string(&fractions).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        let r = parse_dimension("12");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 12.0);
        assert!(!r.was_fractional);
    }

    #[test]
    fn test_decimal() {
        let r = parse_dimension("12.5");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 12.5);
        assert!(!r.was_fractional);
    }

    #[test]
    fn test_fraction_only() {
        let r = parse_dimension("3/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 0.75);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_mixed_space() {
        let r = parse_dimension("1 3/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 1.75);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_mixed_hyphen() {
        let r = parse_dimension("1-3/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 1.75);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_mixed_plus() {
        let r = parse_dimension("1+3/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 1.75);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_unicode_half() {
        let r = parse_dimension("½");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 0.5);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_unicode_mixed() {
        let r = parse_dimension("2½");
        assert!(r.is_valid());
        assert!((r.decimal - 2.5).abs() < 0.001);
        assert!(r.was_fractional);
    }

    #[test]
    fn test_negative_mixed() {
        let r = parse_dimension("-1 3/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, -1.75);
    }

    #[test]
    fn test_whitespace() {
        let r = parse_dimension("  1   3/4  ");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 1.75);
    }

    #[test]
    fn test_reduce_fraction() {
        let r = parse_dimension("2/4");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 0.5);
        assert_eq!(r.display, "1/2");
    }

    #[test]
    fn test_division_by_zero() {
        let r = parse_dimension("1/0");
        assert!(!r.is_valid());
    }

    #[test]
    fn test_empty() {
        let r = parse_dimension("");
        assert!(r.is_valid());
        assert_eq!(r.decimal, 0.0);
    }

    #[test]
    fn test_decimal_to_fraction() {
        assert_eq!(decimal_to_fraction(1.75, 16), "1 3/4");
        assert_eq!(decimal_to_fraction(0.125, 16), "1/8");
        assert_eq!(decimal_to_fraction(2.0, 16), "2");
    }

    #[test]
    fn test_format_mixed() {
        assert_eq!(format_mixed(1.75), "1 3/4");
        assert_eq!(format_mixed(0.5), "1/2");
        assert_eq!(format_mixed(2.0), "2");
        assert_eq!(format_mixed(-1.25), "-1 1/4");
    }

    // ==========================================
    // Tests for new DimensionInput class
    // ==========================================

    #[test]
    fn test_dimension_input_new() {
        let dim = DimensionInput::new("1 3/4");
        assert!(dim.is_valid());
        assert_eq!(dim.value(), 1.75);
        assert!(dim.was_fractional());
    }

    #[test]
    fn test_dimension_input_from_decimal() {
        let dim = DimensionInput::from_decimal(2.5);
        assert!(dim.is_valid());
        assert_eq!(dim.value(), 2.5);
        assert!(!dim.was_fractional());
    }

    #[test]
    fn test_dimension_input_as_fraction() {
        let dim = DimensionInput::new("1.75");
        assert_eq!(dim.as_fraction(16), "1 3/4");
    }

    #[test]
    fn test_dimension_input_as_decimal() {
        let dim = DimensionInput::new("1 3/4");
        assert_eq!(dim.as_decimal(), "1.75");
    }

    #[test]
    fn test_dimension_input_format() {
        let dim = DimensionInput::new("1 3/4");
        assert_eq!(dim.format(true, 16), "1 3/4");
        assert_eq!(dim.format(false, 16), "1.75");
    }

    #[test]
    fn test_dimension_input_parse() {
        let mut dim = DimensionInput::new("1");
        assert_eq!(dim.value(), 1.0);
        dim.parse("2 1/2");
        assert_eq!(dim.value(), 2.5);
    }

    #[test]
    fn test_dimension_input_arithmetic() {
        let a = DimensionInput::new("1 1/2");
        let b = DimensionInput::new("3/4");
        
        let sum = a.add(&b);
        assert_eq!(sum.value(), 2.25);
        
        let diff = a.subtract(&b);
        assert_eq!(diff.value(), 0.75);
        
        let product = a.multiply(2.0);
        assert_eq!(product.value(), 3.0);
        
        let quotient = a.divide(2.0);
        assert_eq!(quotient.value(), 0.75);
    }

    #[test]
    fn test_dimension_input_divide_by_zero_is_noop() {
        let a = DimensionInput::new("1 1/2");
        let result = a.divide(0.0);
        assert!(result.is_valid());
        assert_eq!(result.value(), 1.5); // Unchanged, not infinity/NaN
        assert_eq!(result.original(), a.original());
    }

    #[test]
    fn test_dimension_input_set_value() {
        let mut dim = DimensionInput::new("1");
        dim.set_value(3.5);
        assert_eq!(dim.value(), 3.5);
        assert_eq!(dim.as_fraction(16), "3 1/2");
    }

    // ==========================================
    // Unicode fraction coverage
    // ==========================================

    #[test]
    fn test_all_unicode_fractions() {
        // Every unicode fraction in unicode_to_ascii
        let cases: &[(&str, f64)] = &[
            // Halves
            ("½", 0.5),
            // Thirds
            ("⅓", 1.0 / 3.0),
            ("⅔", 2.0 / 3.0),
            // Quarters
            ("¼", 0.25),
            ("¾", 0.75),
            // Fifths
            ("⅕", 0.2),
            ("⅖", 0.4),
            ("⅗", 0.6),
            ("⅘", 0.8),
            // Sixths
            ("⅙", 1.0 / 6.0),
            ("⅚", 5.0 / 6.0),
            // Eighths
            ("⅛", 0.125),
            ("⅜", 0.375),
            ("⅝", 0.625),
            ("⅞", 0.875),
            // Sevenths, ninths, tenths
            ("⅐", 1.0 / 7.0),
            ("⅑", 1.0 / 9.0),
            ("⅒", 0.1),
        ];
        for (input, expected) in cases {
            let dim = DimensionInput::new(input);
            assert!(dim.is_valid(), "Unicode '{}' should be valid", input);
            assert!(
                (dim.value() - expected).abs() < 0.001,
                "Unicode '{}': expected {}, got {}",
                input,
                expected,
                dim.value()
            );
            assert!(
                dim.was_fractional(),
                "Unicode '{}' should be fractional",
                input
            );
        }
    }

    #[test]
    fn test_unicode_mixed_with_whole_numbers() {
        let cases: &[(&str, f64)] = &[
            ("3½", 3.5),
            ("2¼", 2.25),
            ("5⅛", 5.125),
            ("1⅓", 1.0 + 1.0 / 3.0),
            ("4¾", 4.75),
            ("10⅜", 10.375),
            ("7⅝", 7.625),
            ("2⅞", 2.875),
            ("6⅕", 6.2),
            ("1⅐", 1.0 + 1.0 / 7.0),
            ("3⅑", 3.0 + 1.0 / 9.0),
            ("2⅒", 2.1),
        ];
        for (input, expected) in cases {
            let dim = DimensionInput::new(input);
            assert!(dim.is_valid(), "Mixed '{}' should be valid", input);
            assert!(
                (dim.value() - expected).abs() < 0.001,
                "Mixed '{}': expected {}, got {}",
                input,
                expected,
                dim.value()
            );
            assert!(dim.was_fractional(), "Mixed '{}' should be fractional", input);
        }
    }

    #[test]
    fn test_unicode_with_whitespace() {
        // Leading/trailing whitespace
        let cases: &[(&str, f64)] = &[
            (" ½ ", 0.5),
            ("  ¾", 0.75),
            ("⅜  ", 0.375),
            ("  ⅝  ", 0.625),
        ];
        for (input, expected) in cases {
            let dim = DimensionInput::new(input);
            assert!(dim.is_valid(), "Whitespace '{}' should be valid", input);
            assert!(
                (dim.value() - expected).abs() < 0.001,
                "Whitespace '{}': expected {}, got {}",
                input,
                expected,
                dim.value()
            );
        }
    }

    #[test]
    fn test_unicode_adjacent_vs_spaced() {
        // "2½" (no space) and "2 ½" (with space) should both produce 2.5
        let adjacent = DimensionInput::new("2½");
        let spaced = DimensionInput::new("2 ½");
        assert!(adjacent.is_valid());
        assert!(spaced.is_valid());
        assert!((adjacent.value() - 2.5).abs() < 0.001);
        assert!((spaced.value() - 2.5).abs() < 0.001);
        assert!(adjacent.was_fractional());
        assert!(spaced.was_fractional());
    }
}
