"""
Aspect ratio utilities for ReferenceFrame.

This module provides functions for calculating and displaying aspect ratios,
as well as managing the aspect ratio lock state.
"""

from __future__ import annotations

# Common aspect ratios with their display names
COMMON_RATIOS = [
    (1, 1, "1:1"),
    (4, 3, "4:3"), (3, 4, "3:4"),
    (3, 2, "3:2"), (2, 3, "2:3"),
    (5, 4, "5:4"), (4, 5, "4:5"),
    (16, 9, "16:9"), (9, 16, "9:16"),
    (5, 7, "5:7"), (7, 5, "7:5"),
    (8, 10, "4:5"), (10, 8, "5:4"),  # Same as 4:5, 5:4
    (11, 14, "11:14"), (14, 11, "14:11"),
]


def get_aspect_ratio_display_from_ratio(ratio: float) -> str:
    """
    Get a nice display string from a ratio value (height/width).

    Args:
        ratio: The aspect ratio as height/width

    Returns:
        A formatted string like "3:2" or "1.5:1"
    """
    if ratio == 0:
        return "—"

    # Check against common ratios
    for h, w, name in COMMON_RATIOS:
        if abs(ratio - h / w) < 0.01:
            return name

    # Fall back to decimal ratio
    # If ratio < 1, show as 1:x instead of 0.xx:1 for readability
    if ratio < 1:
        inv_ratio = 1 / ratio
        # Use integer if it's a whole number, otherwise 2 decimals
        if abs(inv_ratio - round(inv_ratio)) < 0.01:
            return f"1:{int(round(inv_ratio))}"
        else:
            return f"1:{inv_ratio:.2f}"
    else:
        # Use integer if it's a whole number, otherwise 2 decimals
        if abs(ratio - round(ratio)) < 0.01:
            return f"{int(round(ratio))}:1"
        else:
            return f"{ratio:.2f}:1"


def get_aspect_ratio_display(height: float, width: float) -> str:
    """
    Get a nice display string for the aspect ratio.

    Args:
        height: The height dimension
        width: The width dimension

    Returns:
        A formatted aspect ratio string
    """
    if width == 0 or height == 0:
        return "—"
    ratio = height / width
    return get_aspect_ratio_display_from_ratio(ratio)


def calculate_dimension_from_ratio(
    known_value: float,
    ratio: float,
    known_is_height: bool
) -> float:
    """
    Calculate the unknown dimension given one dimension and the aspect ratio.

    Args:
        known_value: The known dimension value
        ratio: The aspect ratio (height/width)
        known_is_height: True if known_value is the height, False if width

    Returns:
        The calculated dimension
    """
    if known_is_height:
        # height = ratio * width, so width = height / ratio
        return known_value / ratio
    else:
        # height = ratio * width
        return known_value * ratio


def invert_ratio(ratio: float) -> float:
    """
    Invert an aspect ratio (for when orientation is swapped).

    Args:
        ratio: The original ratio (height/width)

    Returns:
        The inverted ratio (1/ratio)
    """
    if ratio == 0:
        return 0
    return 1 / ratio


class AspectLockState:
    """
    Manages the aspect ratio lock state.

    This class encapsulates the locked/unlocked state and the locked ratio,
    providing methods to lock, unlock, and query the state.
    """

    def __init__(self):
        self._locked = False
        self._ratio = None  # height / width when locked

    @property
    def locked(self) -> bool:
        """Whether the aspect ratio is currently locked."""
        return self._locked

    @property
    def ratio(self) -> Optional[float]:
        """The locked ratio, or None if not locked."""
        return self._ratio

    def lock(self, height: float, width: float) -> bool:
        """
        Lock the aspect ratio to the given dimensions.

        Args:
            height: Current height value
            width: Current width value

        Returns:
            True if successfully locked, False if width is zero
        """
        if width <= 0:
            return False
        self._locked = True
        self._ratio = height / width
        return True

    def unlock(self) -> None:
        """Unlock the aspect ratio."""
        self._locked = False
        self._ratio = None

    def toggle(self, height: float, width: float) -> bool:
        """
        Toggle the lock state.

        Args:
            height: Current height value (used if locking)
            width: Current width value (used if locking)

        Returns:
            The new locked state
        """
        if self._locked:
            self.unlock()
        else:
            self.lock(height, width)
        return self._locked

    def invert(self) -> None:
        """Invert the locked ratio (for orientation swap)."""
        if self._ratio is not None:
            self._ratio = invert_ratio(self._ratio)

    def get_width_for_height(self, height: float, step: float = 0.25) -> float:
        """
        Calculate width for a given height, rounded to step.

        Args:
            height: The height value
            step: The step size for rounding

        Returns:
            The calculated width, or 0 if not locked
        """
        if not self._locked or self._ratio is None or self._ratio == 0:
            return 0
        width = height / self._ratio
        return round(width / step) * step

    def get_height_for_width(self, width: float, step: float = 0.25) -> float:
        """
        Calculate height for a given width, rounded to step.

        Args:
            width: The width value
            step: The step size for rounding

        Returns:
            The calculated height, or 0 if not locked
        """
        if not self._locked or self._ratio is None:
            return 0
        height = width * self._ratio
        return round(height / step) * step
