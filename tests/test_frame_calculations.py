#!/usr/bin/env python3
"""
Unit tests for core frame calculation logic.

These tests verify the mathematical correctness of FrameDesign calculations,
which are the heart of the application.
"""

import pytest
import sys
sys.path.insert(0, '.')

from frame import FrameDesign, FrameSize, calculate_visual_mat_width


class TestFrameDesign:
    """Tests for FrameDesign calculation methods."""

    def test_basic_initialization(self):
        """Test that FrameDesign initializes with correct defaults."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0)
        assert design.artwork_height == 10.0
        assert design.artwork_width == 8.0
        assert design.has_mat  # Default mat width > 0

    def test_initialization_rejects_invalid_dimensions(self):
        """Test that zero or negative dimensions raise ValueError."""
        with pytest.raises(ValueError, match="Artwork dimensions must be positive"):
            FrameDesign(artwork_height=0, artwork_width=8.0)

        with pytest.raises(ValueError, match="Artwork dimensions must be positive"):
            FrameDesign(artwork_height=10.0, artwork_width=-5.0)

    def test_has_mat_property(self):
        """Test has_mat correctly identifies presence of matting."""
        with_mat = FrameDesign(artwork_height=10.0, artwork_width=8.0,
                               mat_width_top_bottom=2.0, mat_width_sides=2.0)
        assert with_mat.has_mat is True

        without_mat = FrameDesign(artwork_height=10.0, artwork_width=8.0,
                                  mat_width_top_bottom=0.0, mat_width_sides=0.0)
        assert without_mat.has_mat is False

    def test_symmetrical_mat_enforcement(self):
        """Test that symmetrical_mat flag forces equal mat widths."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=3.0,
            symmetrical_mat=True
        )
        # When symmetrical, sides should be forced to match top/bottom
        assert design.mat_width_sides == design.mat_width_top_bottom


class TestMatOpeningDimensions:
    """Tests for mat opening calculations."""

    def test_mat_opening_with_overlap(self):
        """Test mat opening is reduced by overlap on all sides."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_overlap=0.125  # 1/8" overlap
        )
        h, w = design.get_mat_opening_dimensions()
        # Opening = artwork - 2*overlap
        assert h == pytest.approx(10.0 - 2 * 0.125)  # 9.75
        assert w == pytest.approx(8.0 - 2 * 0.125)   # 7.75

    def test_mat_opening_no_artwork_margin(self):
        """Test that no_artwork_margin flag gives full artwork dimensions."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_overlap=0.125,
            no_artwork_margin=True
        )
        h, w = design.get_mat_opening_dimensions()
        # With no_artwork_margin, opening equals artwork size
        assert h == 10.0
        assert w == 8.0


class TestVisibleDimensions:
    """Tests for visible (face) dimension calculations."""

    def test_visible_dimensions_with_mat(self):
        """Test visible dimensions include mat borders."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=2.0,
            mat_overlap=0.125
        )
        h, w = design.get_visible_dimensions()
        # Visible = mat_opening + 2*mat_width
        # mat_opening = artwork - 2*overlap = (9.75, 7.75)
        # visible = mat_opening + 2*mat_width = (9.75+4, 7.75+4) = (13.75, 11.75)
        assert h == pytest.approx(13.75)
        assert w == pytest.approx(11.75)

    def test_visible_dimensions_without_mat(self):
        """Test visible dimensions equal artwork when no mat."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=0.0, mat_width_sides=0.0
        )
        h, w = design.get_visible_dimensions()
        assert h == 10.0
        assert w == 8.0


class TestFrameDimensions:
    """Tests for frame inside/outside dimension calculations."""

    def test_inside_dimensions_match_visible(self):
        """Test that inside dimensions equal visible dimensions."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=2.0
        )
        visible = design.get_visible_dimensions()
        inside = design.get_frame_inside_dimensions()
        assert inside == visible

    def test_outside_dimensions_add_frame_width(self):
        """Test outside dimensions add frame material on all sides."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=2.0,
            frame_material_width=1.5,
            mat_overlap=0.125
        )
        inside_h, inside_w = design.get_frame_inside_dimensions()
        outside_h, outside_w = design.get_frame_outside_dimensions()

        # Outside = inside + 2*frame_width
        assert outside_h == pytest.approx(inside_h + 2 * 1.5)
        assert outside_w == pytest.approx(inside_w + 2 * 1.5)

    def test_outside_always_larger_than_inside(self):
        """Invariant: outside dimensions always exceed inside dimensions."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            frame_material_width=0.75
        )
        inside_h, inside_w = design.get_frame_inside_dimensions()
        outside_h, outside_w = design.get_frame_outside_dimensions()

        assert outside_h > inside_h
        assert outside_w > inside_w


class TestMatboardDimensions:
    """Tests for matboard physical dimensions."""

    def test_matboard_dimensions_include_rabbet(self):
        """Test matboard extends into rabbet area."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=2.0,
            rabbet_depth=0.375,
            mat_overlap=0.125
        )
        inside_h, inside_w = design.get_frame_inside_dimensions()
        mat_h, mat_w = design.get_matboard_dimensions()

        # Matboard = inside + 2*rabbet
        assert mat_h == pytest.approx(inside_h + 2 * 0.375)
        assert mat_w == pytest.approx(inside_w + 2 * 0.375)

    def test_matboard_cut_dimensions(self):
        """Test mat border cut width includes rabbet on outer edge only."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0, mat_width_sides=2.5,
            rabbet_depth=0.375,
            symmetrical_mat=False  # Allow different side widths
        )
        top_bottom_cut, side_cut = design.get_matboard_cut_dimensions()

        # Cut width = visual mat width + rabbet (one side only)
        assert top_bottom_cut == pytest.approx(2.0 + 0.375)
        assert side_cut == pytest.approx(2.5 + 0.375)


class TestWoodCalculations:
    """Tests for wood/material length calculations."""

    def test_total_wood_length_basic(self):
        """Test total wood length includes perimeter plus margins."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=0.0, mat_width_sides=0.0,
            frame_material_width=1.0,
            mat_overlap=0.0
        )
        # Inside = artwork = (10, 8)
        # Outside = inside + 2*frame = (12, 10)
        # Perimeter = 2*(12+10) = 44
        # Default margins: 4 * (0.125 + 0.0625) = 0.75
        total = design.get_total_wood_length()
        assert total == pytest.approx(44.0 + 0.75)

    def test_total_wood_length_custom_margins(self):
        """Test custom saw and error margins."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=0.0, mat_width_sides=0.0,
            frame_material_width=1.0,
            mat_overlap=0.0
        )
        # Perimeter = 44
        total = design.get_total_wood_length(saw_margin=0.25, error_margin=0.0)
        # Margins = 4 * 0.25 = 1.0
        assert total == pytest.approx(44.0 + 1.0)


class TestCutList:
    """Tests for cut list generation."""

    def test_cut_list_structure(self):
        """Test cut list returns correct structure."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0)
        cut_list = design.get_cut_list()

        assert "horizontal_pieces" in cut_list
        assert "vertical_pieces" in cut_list
        assert len(cut_list["horizontal_pieces"]) == 1
        assert len(cut_list["vertical_pieces"]) == 1

    def test_cut_list_quantities(self):
        """Test cut list specifies quantity of 2 for each orientation."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0)
        cut_list = design.get_cut_list()

        assert cut_list["horizontal_pieces"][0]["quantity"] == 2
        assert cut_list["vertical_pieces"][0]["quantity"] == 2

    def test_cut_list_dimensions(self):
        """Test cut list dimensions match frame calculations."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=0.0, mat_width_sides=0.0,
            frame_material_width=1.5,
            mat_overlap=0.0
        )
        inside_h, inside_w = design.get_frame_inside_dimensions()
        outside_h, outside_w = design.get_frame_outside_dimensions()
        cut_list = design.get_cut_list()

        # Horizontal pieces span the width
        horiz = cut_list["horizontal_pieces"][0]
        assert horiz["inside_length"] == inside_w
        assert horiz["outside_length"] == outside_w

        # Vertical pieces span the height
        vert = cut_list["vertical_pieces"][0]
        assert vert["inside_length"] == inside_h
        assert vert["outside_length"] == outside_h


class TestRabbetDepth:
    """Tests for rabbet z-depth calculations."""

    def test_rabbet_depth_with_mat(self):
        """Test required rabbet depth includes all materials plus mat."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=2.0,  # Has mat
            glazing_thickness=0.093,
            matboard_thickness=0.055,
            artwork_thickness=0.008,
            backing_thickness=0.125,
            assembly_margin=0.125
        )
        depth = design.get_rabbet_z_depth_required()
        expected = 0.093 + 0.055 + 0.008 + 0.125 + 0.125  # includes matboard
        assert depth == pytest.approx(expected)

    def test_rabbet_depth_without_mat(self):
        """Test required rabbet depth excludes matboard when no mat."""
        design = FrameDesign(
            artwork_height=10.0, artwork_width=8.0,
            mat_width_top_bottom=0.0, mat_width_sides=0.0,  # No mat
            glazing_thickness=0.093,
            matboard_thickness=0.055,
            artwork_thickness=0.008,
            backing_thickness=0.125,
            assembly_margin=0.125
        )
        depth = design.get_rabbet_z_depth_required()
        expected = 0.093 + 0.008 + 0.125 + 0.125  # NO matboard
        assert depth == pytest.approx(expected)


class TestFrameSize:
    """Tests for FrameSize dataclass."""

    def test_framesize_creation(self):
        """Test FrameSize stores dimensions correctly."""
        size = FrameSize(name="8×10", height=8.0, width=10.0)
        assert size.name == "8×10"
        assert size.height == 8.0
        assert size.width == 10.0

    def test_framesize_str_representation(self):
        """Test FrameSize string representation."""
        size = FrameSize(name="8×10", height=8.0, width=10.0)
        assert str(size) == '8×10 (8.0" × 10.0")'


class TestCalculateVisualMatWidth:
    """Tests for the visual mat width calculation helper."""

    def test_visual_mat_width_returns_tuple(self):
        """Test function returns (width, source) tuple."""
        result = calculate_visual_mat_width(10.0, 8.0)
        assert isinstance(result, tuple)
        assert len(result) == 2
        assert isinstance(result[0], float)
        assert result[1] in ("artwork", "frame")

    def test_visual_mat_width_respects_min(self):
        """Test mat width respects minimum."""
        # Very small artwork should still get minimum mat
        width, _ = calculate_visual_mat_width(2.0, 1.0, min_mat=0.5)
        assert width >= 0.5

    def test_visual_mat_width_respects_max(self):
        """Test mat width respects maximum."""
        # Very large artwork should be capped
        width, _ = calculate_visual_mat_width(100.0, 80.0, max_mat=10.0)
        assert width <= 10.0

    def test_visual_mat_width_precision(self):
        """Test result is rounded to precision."""
        width, _ = calculate_visual_mat_width(10.0, 8.0, precision=0.25)
        # Result should be a multiple of 0.25
        assert (width * 4) == pytest.approx(int(width * 4))

    def test_visual_mat_width_accepts_framedesign(self):
        """Test function accepts FrameDesign object."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0)
        width, source = calculate_visual_mat_width(design)
        assert width > 0
        assert source in ("artwork", "frame")


class TestDimensionsInMM:
    """Tests for millimeter conversion output."""

    def test_dimensions_in_mm_structure(self):
        """Test get_dimensions_in_mm returns all expected keys."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0,
                            mat_width_top_bottom=2.0)
        result = design.get_dimensions_in_mm()

        assert "artwork" in result
        assert "frame_inside" in result
        assert "frame_outside" in result
        assert "matboard" in result
        assert "mat_opening" in result

    def test_dimensions_in_mm_conversion(self):
        """Test mm conversion is correct (1 inch = 25.4mm)."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0,
                            mat_width_top_bottom=0.0, mat_width_sides=0.0)
        result = design.get_dimensions_in_mm()

        # Artwork should be 10*25.4 x 8*25.4
        assert result["artwork"][0] == pytest.approx(254.0)
        assert result["artwork"][1] == pytest.approx(203.2)

    def test_dimensions_in_mm_no_mat(self):
        """Test matboard dimensions are None when no mat."""
        design = FrameDesign(artwork_height=10.0, artwork_width=8.0,
                            mat_width_top_bottom=0.0, mat_width_sides=0.0)
        result = design.get_dimensions_in_mm()

        assert result["matboard"] is None
        assert result["mat_opening"] is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
