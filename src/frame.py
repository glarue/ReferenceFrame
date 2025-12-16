"""
Frame design models for the ReferenceFrame application.

This module contains the core data models for frame sizes and designs,
providing the foundation for calculations and visualizations.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from defaults import (
    DEFAULT_ARTWORK_HEIGHT,
    DEFAULT_ARTWORK_THICKNESS,
    DEFAULT_ARTWORK_WIDTH,
    DEFAULT_BACKING_THICKNESS,
    DEFAULT_FRAME_MATERIAL_WIDTH,
    DEFAULT_FRAME_THICKNESS,
    DEFAULT_GLAZING_THICKNESS,
    DEFAULT_MAT_OVERLAP,
    DEFAULT_MAT_WIDTH,
    DEFAULT_MATBOARD_THICKNESS,
    DEFAULT_RABBET_DEPTH,
)


@dataclass
class FrameSize:
    """
    Represents a standard or custom frame size.

    Attributes:
        name: Display name for the size (e.g., "4×6")
        height: Height measurement in inches
        width: Width measurement in inches
    """

    name: str
    height: float  # inches
    width: float  # inches

    def __str__(self) -> str:
        """Return a user-friendly string representation of the frame size."""
        return f'{self.name} ({self.height}" × {self.width}")'


@dataclass
class FrameDesign:
    """
    It provides methods to calculate various derived dimensions.

    Attributes:
        artwork_width: Width of the artwork in inches
        artwork_height: Height of the artwork in inches
        mat_width_top_bottom: Visual mat border thickness for top/bottom in inches
        mat_width_sides: Visual mat border thickness for sides in inches
        rabbet_depth: The rabbet extension on each side (x/y plane) in inches
        mat_overlap: How much the mat overlaps the artwork in inches
        frame_material_width: Width of frame material used around the cut opening in inches
        matboard_thickness: Thickness of the matboard in inches
        artwork_thickness: Thickness of the artwork in inches
        backing_thickness: Thickness of the backing in inches
        glazing_thickness: Thickness of the glass/acrylic in inches
        frame_material_depth: Depth of frame stock (z-axis) in inches
        symmetrical_mat: Whether the mat has equal borders on all sides
        no_artwork_margin: When True, the matboard opening equals the artwork dimensions
    """

    artwork_width: float = DEFAULT_ARTWORK_WIDTH
    artwork_height: float = DEFAULT_ARTWORK_HEIGHT
    mat_width_top_bottom: float = DEFAULT_MAT_WIDTH
    mat_width_sides: float = DEFAULT_MAT_WIDTH
    rabbet_depth: float = DEFAULT_RABBET_DEPTH
    mat_overlap: float = DEFAULT_MAT_OVERLAP
    frame_material_width: float | None = DEFAULT_FRAME_MATERIAL_WIDTH

    # Material thicknesses (z-axis)
    matboard_thickness: float = DEFAULT_MATBOARD_THICKNESS
    artwork_thickness: float = DEFAULT_ARTWORK_THICKNESS
    backing_thickness: float = DEFAULT_BACKING_THICKNESS
    glazing_thickness: float = DEFAULT_GLAZING_THICKNESS
    frame_material_depth: float = DEFAULT_FRAME_THICKNESS
    assembly_margin: float = 0.125  # Margin for assembly clearance

    # Flag for symmetrical matting
    symmetrical_mat: bool = True

    # Flag for no margin around the artwork
    no_artwork_margin: bool = False

    def __post_init__(self) -> None:
        """Initialize derived attributes after instance creation."""
        # If frame_material_width is not provided, default to frame_thickness
        if self.frame_material_width is None:
            self.frame_material_width = self.frame_material_depth

        # Validate dimensions
        if self.artwork_width <= 0 or self.artwork_height <= 0:
            raise ValueError("Artwork dimensions must be positive")

        # Enforce symmetrical mat if the flag is set
        if self.symmetrical_mat and self.mat_width_sides != self.mat_width_top_bottom:
            self.mat_width_sides = self.mat_width_top_bottom

        # When no_artwork_margin is set, the mat opening equals artwork size
        if self.no_artwork_margin:
            self.mat_overlap = 0.0

    @property
    def has_mat(self) -> bool:
        """Whether this design includes matting."""
        return self.mat_width_sides > 0 or self.mat_width_top_bottom > 0

    def _add_border(
        self, height: float, width: float, border: float
    ) -> tuple[float, float]:
        """Add a border of specified width to both dimensions."""
        return height + (2 * border), width + (2 * border)

    def get_visible_dimensions(self) -> tuple[float, float]:
        """
        Calculate the visible (face) dimensions of the frame opening.

        With mat, this is the mat opening plus the visible mat borders.
        Without mat, this is just the artwork dimensions.

        Returns:
            Tuple of (height, width) in inches.
        """
        if self.has_mat:
            # Get mat opening dimensions (in standardized order: height, width)
            mat_opening_height, mat_opening_width = self.get_mat_opening_dimensions()
            visible_height = mat_opening_height + (2 * self.mat_width_top_bottom)
            visible_width = mat_opening_width + (2 * self.mat_width_sides)
        else:
            visible_height = self.artwork_height
            visible_width = self.artwork_width
        return visible_height, visible_width

    @property
    def inside_dimensions(self) -> tuple[float, float]:
        """The inside (cut) dimensions of the frame (height, width)."""
        return self.get_visible_dimensions()

    def get_frame_inside_dimensions(self) -> tuple[float, float]:
        """
        Calculate the inside (cut) dimensions of the frame.

        These dimensions should match the visible dimensions.

        Returns:
            Tuple of (height, width) in inches.
        """
        return self.inside_dimensions

    def get_frame_outside_dimensions(self) -> tuple[float, float]:
        """
        Calculate the outside dimensions of the frame.

        These dimensions include the inside (cut) opening plus the frame
        material border on each side.

        Returns:
            Tuple of (height, width) in inches.
        """
        return self._add_border(*self.inside_dimensions, self.frame_material_width)

    def get_total_wood_length(
        self, saw_margin: float = 0.125, error_margin: float = 0.0625
    ) -> float:
        """
        Calculate the total wood length required to build the frame.

        It computes the outer perimeter of the frame plus an additional margin
        for each of the four pieces to account for the saw blade width and an extra error margin.

        Args:
            saw_margin: The width of the saw blade margin (in inches), default is 0.125.
            error_margin: Additional error margin per piece (in inches), default is 0.0625.

        Returns:
            Total wood length in inches.
        """
        outside_height, outside_width = self.get_frame_outside_dimensions()
        # For the top and bottom pieces, length is the width dimension; for left/right, length is the height.
        base_length = 2 * (outside_width + outside_height)
        total_margin = 4 * (saw_margin + error_margin)
        return base_length + total_margin

    def get_matboard_dimensions(self) -> tuple[float, float]:
        """
        Calculate the physical dimensions of the matboard.

        The matboard needs to extend into the rabbet area on all sides.

        Returns:
            Tuple of (height, width) in inches.
        """
        return self._add_border(*self.inside_dimensions, self.rabbet_depth)

    def get_mat_opening_dimensions(self) -> tuple[float, float]:
        """
        Calculate the dimensions of the opening cut in the matboard.

        If no_artwork_margin is True, returns the full artwork dimensions.
        Otherwise, subtracts twice the mat_overlap from both dimensions.
        """
        if self.no_artwork_margin:
            return self.artwork_height, self.artwork_width
        else:
            mat_opening_height = self.artwork_height - (2 * self.mat_overlap)
            mat_opening_width = self.artwork_width - (2 * self.mat_overlap)
            return mat_opening_height, mat_opening_width

    def get_matboard_cut_dimensions(self) -> tuple[float, float]:
        """
        Calculate the mat border cut width for top/bottom and sides.

        This is the actual width of the mat border strip when cutting:
        visual mat width + rabbet depth (the portion hidden under the frame).

        The mat border only extends into the rabbet on ONE side (the outer edge
        where it meets the frame), not both sides.

        Returns:
            Tuple (top_bottom_border_width, side_border_width) in inches.
        """
        top_bottom_cut = self.mat_width_top_bottom + self.rabbet_depth
        side_cut = self.mat_width_sides + self.rabbet_depth
        return top_bottom_cut, side_cut

    def get_rabbet_z_depth_required(self) -> float:
        """
        Calculate the required depth (z-axis) of the rabbet based on material thicknesses.

        Args:
            assembly_margin: Additional clearance to allow for assembly in inches.

        Returns:
            Required rabbet depth in inches.
        """
        # Start with required materials
        materials = [
            self.glazing_thickness,
            self.artwork_thickness,
            self.backing_thickness,
        ]

        # Add matboard if used
        if self.has_mat:
            materials.append(self.matboard_thickness)

        # Calculate total with margin
        return sum(materials) + self.assembly_margin

    def get_cut_list(self) -> dict[str, list[dict[str, float]]]:
        """
        Generate a cut list with dimensions for each frame piece.

        Returns:
            Dictionary with horizontal and vertical piece specifications.
        """
        # Retrieve inside and outside dimensions in (height, width) order.
        inside_height, inside_width = self.get_frame_inside_dimensions()
        outside_height, outside_width = self.get_frame_outside_dimensions()
        return {
            "horizontal_pieces": [
                {
                    "quantity": 2,
                    "inside_length": inside_width,
                    "outside_length": outside_width,
                    "width": self.frame_material_width,
                }
            ],
            "vertical_pieces": [
                {
                    "quantity": 2,
                    "inside_length": inside_height,
                    "outside_length": outside_height,
                    "width": self.frame_material_width,
                }
            ],
        }

    def get_dimensions_in_mm(self) -> dict[str, tuple[float, float]]:
        """Get all key dimensions in millimeters."""
        mm = 25.4  # conversion factor

        def to_mm(dims: tuple[float, float]) -> tuple[float, float]:
            return (dims[0] * mm, dims[1] * mm)

        # Compute each dimension once
        inside = self.inside_dimensions
        outside = self.get_frame_outside_dimensions()
        matboard = self.get_matboard_dimensions() if self.has_mat else None
        mat_opening = self.get_mat_opening_dimensions() if self.has_mat else None

        return {
            "artwork": (self.artwork_height * mm, self.artwork_width * mm),
            "frame_inside": to_mm(inside),
            "frame_outside": to_mm(outside),
            "matboard": to_mm(matboard) if matboard else None,
            "mat_opening": to_mm(mat_opening) if mat_opening else None,
        }

    @staticmethod
    def initialize_standard_sizes(
        aspect_h: float = 4.0,
        aspect_w: float = 6.0,
        min_long_edge: float = 6.0,
        max_long_edge: float = 20.0,
        increment: float = 0.5,
    ) -> list[FrameSize]:
        """
        Generate a list of standard frame sizes based on an aspect ratio.
        The sizes are expressed with height first then width.

        Args:
            aspect_h: Aspect ratio height part (default 4.0)
            aspect_w: Aspect ratio width part (default 6.0)
            min_long_edge: Minimum allowed long edge size in inches (default 6)
            max_long_edge: Maximum allowed long edge size in inches (default 20)
            increment: Scale increment factor (default 0.5)

        Returns:
            List of auto-generated FrameSize objects.
        """
        sizes = []
        base = max(aspect_h, aspect_w)
        # Determine the starting scale factor so that the long edge is at least min_long_edge.
        factor = max(1.0, min_long_edge / base)

        def format_dimension(n: float) -> str:
            return f"{int(n)}" if n.is_integer() else f"{n:.1f}"

        while base * factor <= max_long_edge:
            height = aspect_h * factor
            width = aspect_w * factor
            height_str = format_dimension(height)
            width_str = format_dimension(width)
            sizes.append(FrameSize(f"{height_str}×{width_str}", height, width))
            factor += increment
        return sizes


def calculate_visual_mat_width(
    design_or_height: FrameDesign | float,
    width: float | None = None,
    artwork_ratio: float = 2 / 15,
    frame_ratio: float = 2.5 / 20,
    min_mat: float = 0.5,
    max_mat: float = 10,
    precision: float = 0.25,
) -> tuple[float, str]:
    """
    Calculate a visual mat width based on both frame and artwork dimensions.
    Chooses whichever calculation results in the larger mat width, and rounds
    the returned value to the nearest multiple of the specified precision.

    Args:
        design_or_height: Either a FrameDesign object or the height value as a float.
        width: Width value, only used if design_or_height is a float.
        artwork_ratio: Ratio to multiply the artwork's longest edge by (default: 2/15).
        frame_ratio: Ratio to multiply the frame's longest edge by (default: 2.5/20).
        min_mat: Minimum mat width in inches (default: 0.5).
        max_mat: Maximum mat width in inches (default: 10).
        precision: Round to the nearest multiple of this value (default: 0.25).

    Returns:
        Tuple of (calculated mat width, source description)
    """
    # Determine if we're working with a FrameDesign or raw dimensions
    if isinstance(design_or_height, FrameDesign):
        design = design_or_height
        outside_height, outside_width = design.get_frame_outside_dimensions()
        frame_long_edge = max(outside_height, outside_width)
        artwork_long_edge = max(design.artwork_height, design.artwork_width)
    else:
        height = design_or_height
        artwork_long_edge = max(height, width)
        temp_design = FrameDesign(artwork_width=width, artwork_height=height)
        outside_height, outside_width = temp_design.get_frame_outside_dimensions()
        frame_long_edge = max(outside_height, outside_width)

    # Calculate based on artwork using artwork-specific ratio
    artwork_based_mat = artwork_long_edge * artwork_ratio
    artwork_based_mat = max(min_mat, min(artwork_based_mat, max_mat))

    # Calculate based on frame using frame-specific ratio
    frame_based_mat = frame_long_edge * frame_ratio
    frame_based_mat = max(min_mat, min(frame_based_mat, max_mat))

    # Round to the nearest multiple of precision using "round half up"
    def round_to_precision(value: float, precision: float) -> float:
        return math.floor(value / precision + 0.5) * precision

    artwork_rounded = round_to_precision(artwork_based_mat, precision)
    frame_rounded = round_to_precision(frame_based_mat, precision)

    # Choose the larger value between the two rounded calculations
    if frame_rounded >= artwork_rounded:
        return frame_rounded, "frame"
    else:
        return artwork_rounded, "artwork"
