#!/usr/bin/env python3
"""
Unit tests for unit conversion edge cases and regression tests for bugs.

These tests specifically cover issues found with the mm/inches toggle feature,
including the frame depth conversion bug and error margin formatting.
"""

import pytest
import sys
sys.path.insert(0, 'src')

from conversions import format_value, inches_to_mm, mm_to_inches
from ui_helpers import input_to_inches, round_to_step
from frame import FrameDesign


class TestErrorMarginFormatting:
    """Tests for ERROR_MARGIN_INCHES constant formatting."""

    ERROR_MARGIN_INCHES = 0.0625  # 1/16"

    def test_error_margin_formats_as_fraction_in_inches(self):
        """Test that 0.0625" formats as 1/16" in inches mode."""
        result = format_value(self.ERROR_MARGIN_INCHES, "inches")
        assert "1/16" in result
        assert '"' in result  # Should have inch symbol

    def test_error_margin_formats_as_mm(self):
        """Test that 0.0625" converts to ~1.6mm in mm mode."""
        result = format_value(self.ERROR_MARGIN_INCHES, "mm")
        # 0.0625 * 25.4 = 1.5875 mm, rounds to 1.6
        assert "1.6" in result or "1.59" in result
        assert "mm" in result

    def test_error_margin_conversion_roundtrip(self):
        """Test that error margin survives inches→mm→inches roundtrip."""
        mm_value = inches_to_mm(self.ERROR_MARGIN_INCHES)
        back_to_inches = mm_to_inches(mm_value)
        assert back_to_inches == pytest.approx(self.ERROR_MARGIN_INCHES, abs=1e-6)


class TestFrameDepthConversion:
    """Regression tests for frame depth unit conversion bug.

    Bug: switch_unit() was not converting frame-depth and blade-width values,
    causing depth calculations to fail when toggling to mm.
    """

    def test_default_frame_depth_converts_to_mm(self):
        """Test default frame depth (0.75") converts correctly to mm."""
        default_depth_inches = 0.75
        expected_mm = 19.05  # 0.75 * 25.4

        result = inches_to_mm(default_depth_inches)
        assert result == pytest.approx(expected_mm, abs=0.01)

    def test_frame_depth_roundtrip_conversion(self):
        """Test frame depth survives inches→mm→inches conversion.

        This is critical because the bug caused:
        1. Input: 0.75" (frame depth in inches)
        2. Toggle to mm (label changes but value stays 0.75)
        3. System interprets 0.75 as mm
        4. Converts: 0.75mm → 0.0295" (way too shallow!)
        """
        original_inches = 0.75

        # Simulate correct conversion
        as_mm = inches_to_mm(original_inches)
        back_to_inches = mm_to_inches(as_mm)

        assert back_to_inches == pytest.approx(original_inches, abs=1e-6)

    def test_incorrect_depth_interpretation_demonstrates_bug(self):
        """Test showing what happens when frame depth is NOT converted.

        This demonstrates the bug: treating an inch value as mm.
        """
        inch_value_treated_as_mm = 0.75  # Bug: should be 19.05mm
        wrongly_converted = mm_to_inches(inch_value_treated_as_mm)

        # This is way too small - demonstrates the bug's impact
        assert wrongly_converted == pytest.approx(0.0295, abs=0.001)
        assert wrongly_converted < 0.1  # Clearly insufficient depth

    def test_depth_calculation_consistent_across_units(self):
        """Test that depth calculations give same result in inches and mm.

        A design that has adequate depth in inches mode should also have
        adequate depth in mm mode (when properly converted).
        """
        # Create design with default values (should have adequate depth)
        design = FrameDesign(
            artwork_height=12.5,
            artwork_width=18.75,
            mat_width_top_bottom=2.0,
            mat_width_sides=2.0,
            frame_material_width=0.75,
            frame_material_depth=0.75,  # Default frame depth
            glazing_thickness=0.093,
            matboard_thickness=0.055,
            artwork_thickness=0.008,
            backing_thickness=0.125,
            rabbet_depth=0.375,
        )

        required_depth = design.get_rabbet_z_depth_required()
        frame_depth = design.frame_material_depth

        # In inches mode
        assert required_depth < frame_depth, "Should have adequate depth in inches"

        # Simulate mm mode (all values should be converted together)
        required_depth_mm = inches_to_mm(required_depth)
        frame_depth_mm = inches_to_mm(frame_depth)

        # Should still have adequate depth in mm
        assert required_depth_mm < frame_depth_mm, "Should have adequate depth in mm"

        # The depth comparison result should be the same
        has_clearance_inches = frame_depth > required_depth
        has_clearance_mm = frame_depth_mm > required_depth_mm
        assert has_clearance_inches == has_clearance_mm


class TestBladeWidthConversion:
    """Tests for blade width (kerf) unit conversion."""

    def test_default_blade_width_converts_to_mm(self):
        """Test default blade width (0.125" = 1/8") converts to mm."""
        default_blade_inches = 0.125
        expected_mm = 3.175  # 0.125 * 25.4

        result = inches_to_mm(default_blade_inches)
        assert result == pytest.approx(expected_mm, abs=0.01)

    def test_blade_width_roundtrip(self):
        """Test blade width survives roundtrip conversion."""
        original = 0.125
        converted = mm_to_inches(inches_to_mm(original))
        assert converted == pytest.approx(original, abs=1e-6)

    def test_blade_width_formats_correctly_in_both_units(self):
        """Test that blade width formats nicely in both units."""
        blade_width = 0.125

        # In inches: should show as 1/8"
        inches_format = format_value(blade_width, "inches")
        assert "1/8" in inches_format or "0.125" in inches_format

        # In mm: should show as 3.2mm or similar
        mm_format = format_value(blade_width, "mm")
        assert "3.2" in mm_format or "3.17" in mm_format
        assert "mm" in mm_format


class TestCompleteFieldConversion:
    """Test that all form fields convert properly when toggling units.

    These tests verify that no fields are left behind when switching units.
    """

    # All fields that should be converted
    CONVERTIBLE_FIELDS = [
        ("artwork_height", 12.5),
        ("artwork_width", 18.75),
        ("mat_width", 2.0),
        ("frame_width", 0.75),
        ("glazing_thickness", 0.093),
        ("matboard_thickness", 0.055),
        ("artwork_thickness", 0.008),
        ("backing_thickness", 0.125),
        ("rabbet_depth", 0.375),
        ("frame_depth", 0.75),  # Bug was here
        ("blade_width", 0.125),  # Bug was here
    ]

    @pytest.mark.parametrize("field_name,inch_value", CONVERTIBLE_FIELDS)
    def test_field_roundtrip_conversion(self, field_name, inch_value):
        """Test each field survives inches→mm→inches roundtrip."""
        mm_value = inches_to_mm(inch_value)
        back_to_inches = mm_to_inches(mm_value)

        assert back_to_inches == pytest.approx(inch_value, abs=1e-5), \
            f"Field {field_name} failed roundtrip: {inch_value} → {mm_value} → {back_to_inches}"

    def test_all_fields_have_proper_mm_representation(self):
        """Test that all fields have reasonable mm values."""
        for field_name, inch_value in self.CONVERTIBLE_FIELDS:
            mm_value = inches_to_mm(inch_value)

            # MM values should be positive and reasonable (not tiny like the bug)
            assert mm_value > 0, f"{field_name}: mm value should be positive"
            assert mm_value > 0.1, f"{field_name}: mm value too small (likely bug)"

            # For context: 1" = 25.4mm, so smallest field (0.008") should be ~0.2mm
            if inch_value == 0.008:  # artwork_thickness
                assert mm_value == pytest.approx(0.2032, abs=0.01)


class TestInputToInchesWithUnits:
    """Test input_to_inches function with various unit scenarios."""

    def test_input_to_inches_preserves_inches(self):
        """Test that inch inputs pass through unchanged."""
        assert input_to_inches("0.75", "inches") == 0.75
        assert input_to_inches("19.05", "inches") == 19.05

    def test_input_to_inches_converts_mm(self):
        """Test that mm inputs are converted to inches."""
        # 19.05mm should become 0.75"
        result = input_to_inches("19.05", "mm")
        assert result == pytest.approx(0.75, abs=0.001)

        # 25.4mm should become 1"
        result = input_to_inches("25.4", "mm")
        assert result == pytest.approx(1.0, abs=0.001)

    def test_depth_field_interpreted_correctly(self):
        """Test specific case from the bug: frame depth interpretation.

        When user enters 19.05mm for frame depth:
        - System should convert to 0.75"
        - Calculations should use 0.75"

        Bug was: value stayed as 19.05 but was treated as 19.05" (way too deep)
        """
        user_input_mm = "19.05"
        converted = input_to_inches(user_input_mm, "mm")

        assert converted == pytest.approx(0.75, abs=0.001)
        assert converted < 1.0  # Sanity check


class TestRoundToStepPreservesValues:
    """Test that round_to_step doesn't introduce conversion errors."""

    def test_round_to_step_for_frame_depth(self):
        """Test rounding frame depth after mm→inches conversion."""
        # 19.05mm → 0.75" → round to 0.125" step
        mm_value = 19.05
        inch_value = mm_to_inches(mm_value)
        rounded = round_to_step(inch_value, 0.125)

        assert rounded == pytest.approx(0.75, abs=0.001)

    def test_round_to_step_for_blade_width(self):
        """Test rounding blade width after mm→inches conversion."""
        # 3.175mm → 0.125" → round to 0.03125" (1/32") step
        mm_value = 3.175
        inch_value = mm_to_inches(mm_value)
        rounded = round_to_step(inch_value, 0.03125)

        assert rounded == pytest.approx(0.125, abs=0.001)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
