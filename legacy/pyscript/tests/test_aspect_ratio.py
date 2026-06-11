#!/usr/bin/env python3
"""
Unit tests for aspect ratio utilities.
"""

import pytest
import sys
sys.path.insert(0, 'src')

from aspect_ratio import (
    get_aspect_ratio_display_from_ratio,
    get_aspect_ratio_display,
    calculate_dimension_from_ratio,
    invert_ratio,
    AspectLockState,
    COMMON_RATIOS,
)


class TestGetAspectRatioDisplayFromRatio:
    """Tests for get_aspect_ratio_display_from_ratio function."""

    def test_zero_ratio(self):
        """Test that zero ratio returns em dash."""
        assert get_aspect_ratio_display_from_ratio(0) == "—"

    def test_square_ratio(self):
        """Test 1:1 square ratio."""
        assert get_aspect_ratio_display_from_ratio(1.0) == "1:1"

    def test_common_ratios(self):
        """Test common aspect ratios are recognized."""
        assert get_aspect_ratio_display_from_ratio(3/2) == "3:2"
        assert get_aspect_ratio_display_from_ratio(2/3) == "2:3"
        assert get_aspect_ratio_display_from_ratio(4/3) == "4:3"
        assert get_aspect_ratio_display_from_ratio(16/9) == "16:9"

    def test_ratio_less_than_one(self):
        """Test ratios < 1 show as 1:x format."""
        # 0.5 = 1:2
        result = get_aspect_ratio_display_from_ratio(0.5)
        assert "1:" in result
        assert "2" in result

    def test_ratio_greater_than_one(self):
        """Test ratios > 1 show as x:1 format."""
        # 2.0 = 2:1
        result = get_aspect_ratio_display_from_ratio(2.0)
        assert "2" in result
        assert ":1" in result


class TestGetAspectRatioDisplay:
    """Tests for get_aspect_ratio_display function."""

    def test_zero_dimensions(self):
        """Test that zero dimensions return em dash."""
        assert get_aspect_ratio_display(0, 10) == "—"
        assert get_aspect_ratio_display(10, 0) == "—"
        assert get_aspect_ratio_display(0, 0) == "—"

    def test_square(self):
        """Test square dimensions."""
        assert get_aspect_ratio_display(10, 10) == "1:1"

    def test_landscape(self):
        """Test landscape aspect ratios."""
        # 8x10 is 4:5 ratio
        result = get_aspect_ratio_display(8, 10)
        assert result == "4:5"

    def test_portrait(self):
        """Test portrait aspect ratios."""
        # 10x8 is 5:4 ratio
        result = get_aspect_ratio_display(10, 8)
        assert result == "5:4"


class TestCalculateDimensionFromRatio:
    """Tests for calculate_dimension_from_ratio function."""

    def test_calculate_width_from_height(self):
        """Test calculating width when height is known."""
        # ratio = 2 (h/w), height = 10, so width = 5
        width = calculate_dimension_from_ratio(10, 2.0, known_is_height=True)
        assert width == pytest.approx(5.0)

    def test_calculate_height_from_width(self):
        """Test calculating height when width is known."""
        # ratio = 2 (h/w), width = 5, so height = 10
        height = calculate_dimension_from_ratio(5, 2.0, known_is_height=False)
        assert height == pytest.approx(10.0)

    def test_square_ratio(self):
        """Test with 1:1 ratio."""
        width = calculate_dimension_from_ratio(10, 1.0, known_is_height=True)
        assert width == pytest.approx(10.0)


class TestInvertRatio:
    """Tests for invert_ratio function."""

    def test_invert_two(self):
        """Test inverting 2 gives 0.5."""
        assert invert_ratio(2.0) == pytest.approx(0.5)

    def test_invert_half(self):
        """Test inverting 0.5 gives 2."""
        assert invert_ratio(0.5) == pytest.approx(2.0)

    def test_invert_one(self):
        """Test inverting 1 gives 1."""
        assert invert_ratio(1.0) == pytest.approx(1.0)

    def test_invert_zero(self):
        """Test inverting 0 gives 0 (edge case)."""
        assert invert_ratio(0) == 0


class TestAspectLockState:
    """Tests for AspectLockState class."""

    def test_initial_state(self):
        """Test initial state is unlocked."""
        state = AspectLockState()
        assert state.locked is False
        assert state.ratio is None

    def test_lock(self):
        """Test locking with valid dimensions."""
        state = AspectLockState()
        result = state.lock(10, 8)
        assert result is True
        assert state.locked is True
        assert state.ratio == pytest.approx(10/8)

    def test_lock_zero_width(self):
        """Test locking with zero width fails."""
        state = AspectLockState()
        result = state.lock(10, 0)
        assert result is False
        assert state.locked is False

    def test_unlock(self):
        """Test unlocking."""
        state = AspectLockState()
        state.lock(10, 8)
        state.unlock()
        assert state.locked is False
        assert state.ratio is None

    def test_toggle(self):
        """Test toggle behavior."""
        state = AspectLockState()
        # Toggle on
        result = state.toggle(10, 8)
        assert result is True
        assert state.locked is True
        # Toggle off
        result = state.toggle(10, 8)
        assert result is False
        assert state.locked is False

    def test_invert(self):
        """Test ratio inversion."""
        state = AspectLockState()
        state.lock(10, 5)  # ratio = 2
        state.invert()
        assert state.ratio == pytest.approx(0.5)

    def test_get_width_for_height(self):
        """Test calculating width from height."""
        state = AspectLockState()
        state.lock(10, 8)  # ratio = 1.25
        width = state.get_width_for_height(12.5, step=0.25)
        assert width == pytest.approx(10.0)

    def test_get_height_for_width(self):
        """Test calculating height from width."""
        state = AspectLockState()
        state.lock(10, 8)  # ratio = 1.25
        height = state.get_height_for_width(8.0, step=0.25)
        assert height == pytest.approx(10.0)

    def test_get_dimension_when_unlocked(self):
        """Test that dimension calculations return 0 when unlocked."""
        state = AspectLockState()
        assert state.get_width_for_height(10) == 0
        assert state.get_height_for_width(8) == 0


class TestCommonRatios:
    """Tests for COMMON_RATIOS constant."""

    def test_has_common_photo_ratios(self):
        """Test that common photography ratios are included."""
        ratio_names = [name for _, _, name in COMMON_RATIOS]
        assert "3:2" in ratio_names
        assert "4:3" in ratio_names
        assert "16:9" in ratio_names
        assert "1:1" in ratio_names


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
