#!/usr/bin/env python3
"""
Unit tests for UI helper functions.
"""

import pytest
import sys
sys.path.insert(0, '.')

from ui_helpers import (
    input_to_inches,
    round_to_step,
    format_integer_if_whole,
    FIELD_IDS,
)


class TestInputToInches:
    """Tests for input_to_inches conversion."""

    def test_inches_passthrough(self):
        """Test that inches values pass through unchanged."""
        assert input_to_inches("10.5", "inches") == 10.5
        assert input_to_inches("0", "inches") == 0.0

    def test_mm_conversion(self):
        """Test mm to inches conversion."""
        # 25.4mm = 1 inch
        assert input_to_inches("25.4", "mm") == pytest.approx(1.0)
        assert input_to_inches("254", "mm") == pytest.approx(10.0)

    def test_float_string_parsing(self):
        """Test that float strings are parsed correctly."""
        assert input_to_inches("12.75", "inches") == 12.75


class TestRoundToStep:
    """Tests for round_to_step function."""

    def test_quarter_inch_steps(self):
        """Test rounding to quarter inch."""
        assert round_to_step(10.1, 0.25) == pytest.approx(10.0)
        assert round_to_step(10.2, 0.25) == pytest.approx(10.25)
        assert round_to_step(10.4, 0.25) == pytest.approx(10.5)

    def test_eighth_inch_steps(self):
        """Test rounding to eighth inch."""
        assert round_to_step(10.05, 0.125) == pytest.approx(10.0)
        assert round_to_step(10.1, 0.125) == pytest.approx(10.125)

    def test_exact_step_values(self):
        """Test that exact step values remain unchanged."""
        assert round_to_step(10.25, 0.25) == pytest.approx(10.25)
        assert round_to_step(10.5, 0.25) == pytest.approx(10.5)


class TestFormatIntegerIfWhole:
    """Tests for format_integer_if_whole function."""

    def test_whole_numbers(self):
        """Test that whole numbers are formatted as integers."""
        assert format_integer_if_whole(10.0) == "10"
        assert format_integer_if_whole(10.0001) == "10"  # Within tolerance

    def test_fractional_numbers(self):
        """Test that fractional numbers keep decimal."""
        assert format_integer_if_whole(10.5) == "10.5"
        assert format_integer_if_whole(10.25) == "10.25"


class TestFieldIds:
    """Tests for FIELD_IDS constant."""

    def test_all_expected_fields_present(self):
        """Test that all expected form fields are defined."""
        expected_fields = [
            "height", "width", "mat_width", "frame_width",
            "glazing", "matboard", "artwork_thickness", "backing",
            "rabbet", "frame_depth", "blade_width", "include_mat"
        ]
        for field in expected_fields:
            assert field in FIELD_IDS, f"Missing field: {field}"

    def test_field_ids_are_strings(self):
        """Test that all field IDs are non-empty strings."""
        for key, value in FIELD_IDS.items():
            assert isinstance(value, str), f"Field {key} has non-string ID"
            assert len(value) > 0, f"Field {key} has empty ID"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
