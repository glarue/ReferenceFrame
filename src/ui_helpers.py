"""
UI helper functions for the PyScript PoC.

This module provides utilities for DOM manipulation, form value handling,
and common UI patterns. It helps keep the main index.html cleaner by
extracting reusable code.
"""

from typing import Dict, Optional, Tuple, Any
from conversions import mm_to_inches

# Field IDs for form inputs (single source of truth)
FIELD_IDS = {
    "height": "artwork-height",
    "width": "artwork-width",
    "mat_width": "mat-width",
    "frame_width": "frame-width",
    "glazing": "glazing-thickness",
    "matboard": "matboard-thickness",
    "artwork_thickness": "artwork-thickness",
    "backing": "backing-thickness",
    "rabbet": "rabbet-depth",
    "frame_depth": "frame-depth",
    "blade_width": "blade-width",
    "include_mat": "include-mat",
}


def get_field_value(document, field_key: str) -> str:
    """
    Get the value of a form field by its logical key.

    Args:
        document: The PyScript document object
        field_key: Logical name (e.g., "height", "mat_width")

    Returns:
        The field's value as a string
    """
    field_id = FIELD_IDS.get(field_key, field_key)
    element = document.getElementById(field_id)
    if element is None:
        return ""
    return element.value


def set_field_value(document, field_key: str, value: Any) -> None:
    """
    Set the value of a form field by its logical key.

    Args:
        document: The PyScript document object
        field_key: Logical name (e.g., "height", "mat_width")
        value: The value to set
    """
    field_id = FIELD_IDS.get(field_key, field_key)
    element = document.getElementById(field_id)
    if element is not None:
        element.value = str(value)


def get_checkbox_state(document, field_key: str) -> bool:
    """
    Get the checked state of a checkbox by its logical key.

    Args:
        document: The PyScript document object
        field_key: Logical name (e.g., "include_mat")

    Returns:
        True if checked, False otherwise
    """
    field_id = FIELD_IDS.get(field_key, field_key)
    element = document.getElementById(field_id)
    if element is None:
        return False
    return element.checked


def input_to_inches(value_str: str, from_unit: str) -> float:
    """
    Convert an input field value to inches for calculations.

    Args:
        value_str: The string value from the input field
        from_unit: The current display unit ("inches" or "mm")

    Returns:
        The value converted to inches
    """
    value = float(value_str)
    if from_unit == "mm":
        return mm_to_inches(value)
    return value


def round_to_step(value: float, step: float) -> float:
    """
    Round a value to the nearest step increment.

    This helps avoid floating-point drift when converting between units.

    Args:
        value: The value to round
        step: The step size to round to

    Returns:
        The value rounded to the nearest step
    """
    return round(value / step) * step


def get_form_values_as_inches(document, current_unit: str) -> Optional[Dict[str, float]]:
    """
    Get all form values converted to inches for calculations.

    Args:
        document: The PyScript document object
        current_unit: The current display unit ("inches" or "mm")

    Returns:
        Dictionary of field values in inches, or None if required fields are empty
    """
    # Get primary fields
    height_input = get_field_value(document, "height")
    width_input = get_field_value(document, "width")
    mat_width_input = get_field_value(document, "mat_width")
    frame_width_input = get_field_value(document, "frame_width")

    # Skip if required fields are empty
    if not height_input or not width_input or not frame_width_input:
        return None

    # Convert primary values
    height = input_to_inches(height_input, current_unit)
    width = input_to_inches(width_input, current_unit)
    frame_width = input_to_inches(frame_width_input, current_unit)

    # Check mat toggle
    include_mat = get_checkbox_state(document, "include_mat")
    if include_mat and mat_width_input:
        mat_width = input_to_inches(mat_width_input, current_unit)
    else:
        mat_width = 0.0

    # Get advanced options
    glazing = input_to_inches(get_field_value(document, "glazing"), current_unit)
    matboard = input_to_inches(get_field_value(document, "matboard"), current_unit)
    artwork_thickness = input_to_inches(get_field_value(document, "artwork_thickness"), current_unit)
    backing = input_to_inches(get_field_value(document, "backing"), current_unit)
    rabbet = input_to_inches(get_field_value(document, "rabbet"), current_unit)
    frame_depth = input_to_inches(get_field_value(document, "frame_depth"), current_unit)
    blade_width = input_to_inches(get_field_value(document, "blade_width"), current_unit)

    return {
        "artwork_height": height,
        "artwork_width": width,
        "mat_width": mat_width,
        "frame_width": frame_width,
        "glazing_thickness": glazing,
        "matboard_thickness": matboard,
        "artwork_thickness": artwork_thickness,
        "backing_thickness": backing,
        "rabbet_depth": rabbet,
        "frame_depth": frame_depth,
        "blade_width": blade_width,
        "include_mat": include_mat,
    }


def format_integer_if_whole(value: float) -> str:
    """
    Format a number as an integer if it's a whole number, otherwise as float.

    Args:
        value: The number to format

    Returns:
        String representation (e.g., "10" or "10.5")
    """
    if abs(value - round(value)) < 0.001:
        return str(int(round(value)))
    return str(value)
