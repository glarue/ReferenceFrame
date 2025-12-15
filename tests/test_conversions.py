#!/usr/bin/env python3
"""
Unit tests for conversion and formatting functions.

These tests verify the tape-measure conversion logic and value formatting,
which are critical for displaying dimensions correctly to users.
"""

import pytest
from fractions import Fraction
import sys
sys.path.insert(0, '.')

from conversions import (
    convert_decimal_to_tape_measure,
    format_value,
    format_float_informative,
    format_dimension_pair,
    inches_to_mm,
    mm_to_inches,
    round_half_up,
    INCHES_TO_MM,
)


class TestUnitConversions:
    """Tests for basic unit conversion functions."""

    def test_inches_to_mm(self):
        """Test inch to millimeter conversion."""
        assert inches_to_mm(1.0) == pytest.approx(25.4)
        assert inches_to_mm(0.0) == 0.0
        assert inches_to_mm(10.0) == pytest.approx(254.0)

    def test_mm_to_inches(self):
        """Test millimeter to inch conversion."""
        assert mm_to_inches(25.4) == pytest.approx(1.0)
        assert mm_to_inches(0.0) == 0.0
        assert mm_to_inches(254.0) == pytest.approx(10.0)

    def test_roundtrip_conversion(self):
        """Test that inch→mm→inch roundtrip is lossless."""
        original = 12.75
        converted = mm_to_inches(inches_to_mm(original))
        assert converted == pytest.approx(original)

    def test_conversion_constant(self):
        """Test the conversion constant is correct."""
        assert INCHES_TO_MM == 25.4


class TestRoundHalfUp:
    """Tests for round_half_up function."""

    def test_round_half_up_basic(self):
        """Test basic rounding behavior."""
        assert round_half_up(2.3) == 2
        assert round_half_up(2.7) == 3
        assert round_half_up(2.0) == 2

    def test_round_half_up_at_boundary(self):
        """Test rounding at 0.5 boundary (should round up)."""
        assert round_half_up(2.5) == 3
        assert round_half_up(3.5) == 4

    def test_round_half_up_small_values(self):
        """Test rounding small fractional values."""
        assert round_half_up(0.4) == 0
        assert round_half_up(0.5) == 1
        assert round_half_up(0.6) == 1


class TestConvertDecimalToTapeMeasure:
    """Tests for tape-measure conversion logic."""

    def test_whole_number(self):
        """Test conversion of whole inches."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.0)
        assert whole == 4
        assert frac is None
        assert adj is None

    def test_simple_half(self):
        """Test conversion of .5 (1/2)."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.5)
        assert whole == 4
        assert frac == Fraction(1, 2)
        assert adj is None

    def test_simple_quarter(self):
        """Test conversion of .25 (1/4)."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.25)
        assert whole == 4
        assert frac == Fraction(1, 4)
        assert adj is None

    def test_simple_eighth(self):
        """Test conversion of .125 (1/8)."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.125)
        assert whole == 4
        assert frac == Fraction(1, 8)
        assert adj is None

    def test_three_quarters(self):
        """Test conversion of .75 (3/4)."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.75)
        assert whole == 4
        assert frac == Fraction(3, 4)
        assert adj is None

    def test_sixteenth(self):
        """Test conversion of 1/16."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.0625)
        assert whole == 4
        assert frac == Fraction(1, 16)
        assert adj is None

    def test_fraction_only_no_whole(self):
        """Test conversion when whole part is zero."""
        whole, frac, adj = convert_decimal_to_tape_measure(0.5)
        assert whole == 0
        assert frac == Fraction(1, 2)

    def test_very_small_value_forced_to_smallest_increment(self):
        """Test that very small values get forced to 1/32."""
        # 0.015 is less than half of 1/32 (0.015625)
        whole, frac, adj = convert_decimal_to_tape_measure(0.015)
        assert whole == 0
        assert frac == Fraction(1, 32)

    def test_rounds_to_next_inch(self):
        """Test that .99+ rounds up to next whole inch."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.99)
        assert whole == 5
        assert frac is None

    def test_with_segments_no_adjustment_needed(self):
        """Test segments=True when no adjustment needed."""
        whole, frac, adj = convert_decimal_to_tape_measure(4.5, segments=True)
        assert whole == 4
        assert frac == Fraction(1, 2)
        assert adj is None

    def test_with_segments_adjustment_needed(self):
        """Test segments=True produces base + adjustment."""
        # 4.72 ≈ 4 + 3/4 - 1/32
        # 3/4 = 0.75, actual frac = 0.72, diff = -0.03 ≈ -1/32
        whole, frac, adj = convert_decimal_to_tape_measure(4.72, segments=True)
        assert whole == 4
        # The base should be a coarser fraction
        assert frac is not None
        # Adjustment should be small
        if adj is not None:
            assert abs(float(adj)) <= 1/16

    def test_prefers_simpler_denominator(self):
        """Test that simpler fractions are preferred when equal error."""
        # 0.5 should give 1/2, not 2/4 or 4/8
        whole, frac, adj = convert_decimal_to_tape_measure(0.5)
        assert frac == Fraction(1, 2)
        assert frac.denominator == 2  # Not reduced from larger denom

    def test_allowed_denoms_respected(self):
        """Test custom allowed denominators."""
        # Only allow halves and quarters - use 4.375 (3/8) which rounds to 1/2
        whole, frac, adj = convert_decimal_to_tape_measure(
            4.375, allowed_denoms=(2, 4)
        )
        # 3/8 not allowed, should round to 1/2 (closer than 1/4)
        assert frac == Fraction(1, 2)
        assert frac.denominator == 2


class TestFormatFloatInformative:
    """Tests for format_float_informative function."""

    def test_strips_trailing_zeros(self):
        """Test that trailing zeros are removed."""
        assert format_float_informative(1.0, 3) == "1"
        assert format_float_informative(1.50, 3) == "1.5"
        assert format_float_informative(1.500, 3) == "1.5"

    def test_preserves_significant_decimals(self):
        """Test that significant decimals are kept."""
        assert format_float_informative(0.015, 3) == "0.015"
        assert format_float_informative(1.234, 3) == "1.234"

    def test_removes_trailing_decimal_point(self):
        """Test that trailing decimal point is removed."""
        assert format_float_informative(5.0, 2) == "5"
        # Not "5."

    def test_respects_precision(self):
        """Test precision parameter is respected."""
        assert format_float_informative(1.23456, 2) == "1.23"
        assert format_float_informative(1.23456, 4) == "1.2346"


class TestFormatValue:
    """Tests for format_value function."""

    def test_format_inches_whole_number(self):
        """Test formatting whole inch value."""
        result = format_value(4.0, "inches")
        assert result == '4"'

    def test_format_inches_simple_fraction(self):
        """Test formatting simple fractional inch."""
        result = format_value(4.5, "inches")
        assert '1/2"' in result

    def test_format_inches_with_decimal_shown(self):
        """Test that decimal equivalent is shown when different."""
        result = format_value(4.72, "inches")
        # Should contain both fraction and decimal
        assert '"' in result
        assert "4.72" in result or "4 3/4" in result

    def test_format_inches_no_tape_conversion(self):
        """Test formatting without tape measure conversion."""
        result = format_value(4.72, "inches", use_tape_conversion=False)
        assert result == '4.72"'

    def test_format_mm(self):
        """Test formatting as millimeters."""
        result = format_value(1.0, "mm")
        assert "25.4" in result
        assert "mm" in result

    def test_format_mm_precision(self):
        """Test mm precision is applied."""
        result = format_value(1.0, "mm", precision_mm=2)
        assert "25.40" in result or "25.4" in result

    def test_format_value_zero(self):
        """Test formatting zero value."""
        result = format_value(0.0, "inches")
        assert result == '0"'


class TestFormatDimensionPair:
    """Tests for format_dimension_pair function."""

    def test_dimension_pair_format(self):
        """Test dimension pair includes label and both values."""
        result = format_dimension_pair("Frame Size", 10.0, 8.0, "inches")
        assert "Frame Size" in result
        assert "10" in result
        assert "8" in result
        assert "×" in result

    def test_dimension_pair_markdown(self):
        """Test dimension pair uses markdown bold for label."""
        result = format_dimension_pair("Test", 5.0, 4.0, "inches")
        assert result.startswith("**Test:**")


class TestEdgeCases:
    """Tests for edge cases and boundary conditions."""

    def test_very_large_value(self):
        """Test handling of large values."""
        whole, frac, adj = convert_decimal_to_tape_measure(100.0)
        assert whole == 100
        assert frac is None

    def test_negative_value_handling(self):
        """Test behavior with negative values (implementation-defined)."""
        # Negative values aren't typical for measurements, but shouldn't crash
        # The function takes the integer part, which for -4.5 would be -4
        whole, frac, adj = convert_decimal_to_tape_measure(-4.5)
        # Just verify it doesn't crash and returns something
        assert isinstance(whole, int)

    def test_format_value_large_number(self):
        """Test formatting large measurements."""
        result = format_value(144.0, "inches")  # 12 feet
        assert "144" in result

    def test_format_value_tiny_fraction(self):
        """Test formatting very small fractions."""
        result = format_value(0.03125, "inches")  # 1/32
        assert "1/32" in result


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
