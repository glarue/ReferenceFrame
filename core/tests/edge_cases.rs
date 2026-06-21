//! Edge-case regression tests ported from the archived PyScript suite.
//!
//! These cover the three gaps the 2026-06-10 codebase audit (AUDIT_REPORT.md, H4)
//! flagged as worth porting to Rust when the Python tests were archived:
//!   1. mm <-> inches toggle misinterpretation (the "frame depth treated as mm" bug)
//!   2. round-half-up behavior at the 0.5 boundary
//!   3. rejection of zero/negative artwork dimensions
//!
//! Scope is deliberately limited to these three. Other scenarios (rabbet > frame,
//! extreme aspect ratios, overlap > half artwork, etc.) already have dedicated tests
//! in core/src/validation.rs and core/src/frame.rs and are NOT duplicated here.

use referenceframe_core::conversions::{convert_to_tape_measure, Fraction, DEFAULT_DENOMS};
use referenceframe_core::{inches_to_mm, mm_to_inches, validate_design, FrameDesign, ValidationConfig};

// ============================================================================
// 1. mm <-> inches toggle misinterpretation
//
// Regression for the bug where toggling units changed the label but not the
// stored value, so an inch value (e.g. 0.75") was later interpreted as mm
// (0.75mm -> 0.0295"), collapsing frame depth / blade width to nothing.
// Source: legacy/pyscript/tests/test_unit_conversion_bugs.py
// ============================================================================

/// Every convertible field must survive an inches -> mm -> inches round trip.
#[test]
fn unit_roundtrip_is_lossless_for_all_fields() {
    // (label, inch value) for every field the unit toggle converts.
    let fields = [
        ("artwork_height", 12.5),
        ("artwork_width", 18.75),
        ("mat_width", 2.0),
        ("frame_width", 0.75),
        ("frame_depth", 0.75), // bug was here
        ("blade_width", 0.125), // bug was here
        ("glazing_thickness", 0.093),
        ("matboard_thickness", 0.055),
        ("artwork_thickness", 0.008),
        ("backing_thickness", 0.125),
        ("rabbet_depth", 0.375),
    ];

    for (name, inches) in fields {
        let round_tripped = mm_to_inches(inches_to_mm(inches));
        assert!(
            (round_tripped - inches).abs() < 1e-9,
            "field {name} failed round trip: {inches} -> {} -> {round_tripped}",
            inches_to_mm(inches)
        );
        // And the mm representation must be sane (not collapsed to ~0).
        assert!(inches_to_mm(inches) > 0.1, "field {name} mm value too small");
    }
}

/// Specific conversions used by the unit toggle, exact to the published values.
#[test]
fn unit_conversions_match_known_values() {
    assert!((inches_to_mm(1.0) - 25.4).abs() < 1e-9);
    assert!((inches_to_mm(0.75) - 19.05).abs() < 1e-9); // default frame depth
    assert!((inches_to_mm(0.125) - 3.175).abs() < 1e-9); // default blade width
    assert!((mm_to_inches(25.4) - 1.0).abs() < 1e-9);
    assert!((mm_to_inches(19.05) - 0.75).abs() < 1e-9);
}

/// Demonstrates the original bug's signature: treating an inch value AS mm
/// produces an absurdly small inch value. This guards against any future
/// regression that double-converts or skips conversion on the unit toggle.
#[test]
fn treating_inches_as_mm_collapses_value() {
    // 0.75 (an inch value) wrongly fed through mm_to_inches:
    let wrongly_converted = mm_to_inches(0.75);
    assert!(
        (wrongly_converted - 0.0295).abs() < 0.001,
        "expected ~0.0295, got {wrongly_converted}"
    );
    assert!(wrongly_converted < 0.1, "clearly insufficient depth (the bug)");
}

// ============================================================================
// 2. round-half-up at the 0.5 boundary
//
// The fraction/tape conversion relies on round-half-away-from-zero (Rust's
// f64::round), which equals round-half-UP for the non-negative measurement
// domain. Banker's rounding (round-half-to-even) would silently break fraction
// snapping (e.g. 2.5 -> 2 instead of 3), so we pin the behavior both at the
// primitive level and through the real conversion path.
// Source: legacy/pyscript/tests/test_conversions.py::TestRoundHalfUp
//
// NOTE: round-half-away-from-zero and Python's round_half_up DIVERGE for
// negative inputs (-2.5 -> -3 vs -2), but measurements are always >= 0, so the
// distinction never reaches this code. We do not test negative inputs because
// convert_to_tape_measure debug-asserts on them.
// ============================================================================

/// The rounding primitive the conversion code depends on rounds .5 up, not
/// to-even. If this ever changes, fraction snapping is wrong.
#[test]
fn round_half_up_primitive_assumption() {
    assert_eq!(2.5_f64.round() as i32, 3);
    assert_eq!(3.5_f64.round() as i32, 4); // to-even would give 4 here (coincidence)
    assert_eq!(0.5_f64.round() as i32, 1);
    assert_eq!(2.4_f64.round() as i32, 2);
    assert_eq!(2.6_f64.round() as i32, 3);
}

/// Real-code path: a value exactly halfway between two 1/32" marks
/// (2.5/32" = 0.078125") must round UP to 3/32", not down to 2/32".
/// This is where the round-half-up assumption actually bites.
#[test]
fn tape_measure_rounds_half_up_at_finest_denominator() {
    let result = convert_to_tape_measure(0.078125, false, DEFAULT_DENOMS);
    assert_eq!(result.whole, 0);
    assert_eq!(
        result.fraction,
        Some(Fraction::new(3, 32)),
        "2.5/32 must round up to 3/32 (banker's rounding would give 2/32)"
    );
}

// ============================================================================
// 3. rejection of zero/negative artwork dimensions
//
// There is no direct "artwork > 0" guard; rejection happens indirectly because
// a non-positive artwork dimension drives the frame opening below min_opening
// (no mat) or makes the mat overlap exceed the artwork (with mat). We therefore
// assert on validity/field/severity rather than exact message strings, which
// are not contractual. Source: behavior gap noted in AUDIT_REPORT.md H4.
// ============================================================================

/// Sanity baseline: a normal design with default materials IS valid, so the
/// rejection tests below are meaningful (not just "everything is invalid").
#[test]
fn baseline_default_design_is_valid() {
    let mut design = FrameDesign::new(11.0, 14.0);
    design.enforce_constraints();
    let result = validate_design(&design, &ValidationConfig::default());
    assert!(result.is_valid(), "default 11x14 should be valid: {:?}", result.issues);
}

#[test]
fn zero_artwork_is_rejected() {
    let design = FrameDesign::new(0.0, 0.0);
    let result = validate_design(&design, &ValidationConfig::default());
    assert!(!result.is_valid(), "zero artwork must produce a validation error");
}

#[test]
fn negative_artwork_is_rejected() {
    let design = FrameDesign::new(-5.0, -8.0);
    let result = validate_design(&design, &ValidationConfig::default());
    assert!(!result.is_valid(), "negative artwork must produce a validation error");
}

/// With no mat, the rejection surfaces directly on the artwork dimension fields
/// (the frame opening = artwork - 2*rabbet falls below min_opening).
#[test]
fn zero_artwork_no_mat_errors_on_artwork_field() {
    let mut design = FrameDesign::new(0.0, 0.0);
    design.mat_width_sides = 0.0;
    design.mat_width_top_bottom = 0.0;
    assert!(!design.has_mat(), "test setup: expected no mat");

    let result = validate_design(&design, &ValidationConfig::default());
    assert!(!result.is_valid());
    assert!(
        result
            .errors()
            .iter()
            .any(|e| e.field == "artwork_width" || e.field == "artwork_height"),
        "expected an artwork dimension error, got: {:?}",
        result.errors()
    );
}
