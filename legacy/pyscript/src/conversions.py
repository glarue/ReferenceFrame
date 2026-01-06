"""
Utility functions for unit conversion and text formatting.

This module provides helper functions for converting between different measurement units
and formatting measurement values for display.
"""

from __future__ import annotations

from fractions import Fraction

# Constants
INCHES_TO_MM = 25.4  # Conversion factor from inches to millimeters


def inches_to_mm(value: float) -> float:
    """
    Convert a measurement from inches to millimeters.

    Args:
        value: Measurement in inches

    Returns:
        Equivalent measurement in millimeters
    """
    return value * INCHES_TO_MM


def mm_to_inches(value: float) -> float:
    """
    Convert a measurement from millimeters to inches.

    Args:
        value: Measurement in millimeters

    Returns:
        Equivalent measurement in inches
    """
    return value / INCHES_TO_MM


def round_half_up(x: float) -> int:
    """
    Rounds x using the "round half up" strategy.
    For positive numbers this is implemented by adding 0.5 and taking the floor.
    """
    return int(x + 0.5)


def convert_decimal_to_tape_measure(
    value: float,
    allowed_denoms: tuple[int, ...] = (2, 4, 8, 16, 32),
    segments: bool = False,
) -> tuple[int, Fraction | None, Fraction | None]:
    """
    Convert a decimal inch measurement to a tape-measure friendly mixed number.

    Args:
      value: float
         The measurement in inches.
      allowed_denoms: tuple[int, ...]
         Allowed denominators (e.g., (2, 4, 8, 16, 32)).
      segments: bool
         If False: returns a tuple (whole, fraction, None) where 'fraction' is a Fraction
         representing the best candidate for the fractional part.
         If True: returns a tuple (whole, base_fraction, adjustment_fraction), where
                 measurement = whole inches + base_fraction + adjustment_fraction.
                 If no adjustment is needed, adjustment_fraction will be None.

    Returns:
      A three-element tuple:
         - whole (int): whole-inch part.
         - fraction (Fraction | None): the main fractional component, or None if not needed.
         - adjustment (Fraction | None): the fine adjustment when segmentation is enabled;
           otherwise, None.

    Examples:
      • convert_decimal_to_tape_measure(4.72, segments=True)
          returns (4, Fraction(3, 4), Fraction(-1, 32))  # "4 3/4 – 1/32 inches"
      • convert_decimal_to_tape_measure(4.5, segments=True)
          returns (4, Fraction(1, 2), None)
      • convert_decimal_to_tape_measure(4.0)
          returns (4, None, None)
      • convert_decimal_to_tape_measure(0.015, segments=True)
          returns (0, Fraction(1, 32), None)
    """
    # --- Step 1: Separate whole and fractional parts ---
    whole = int(value)
    frac_val = value - whole

    # If the fractional part is (almost) zero, return immediately.
    if abs(frac_val) < 1e-9:
        return (whole, None, None)

    # --- NEW: Handle very small fractional parts ---
    # Determine the threshold below which we force the smallest increment.
    finest = max(allowed_denoms)
    threshold = 0.5 / finest  # half-step size for the finest resolution
    if 0 < frac_val < threshold:
        forced_candidate = Fraction(1, finest)
        return (whole, forced_candidate, None)

    # --- Step 2: Determine the best candidate fraction from allowed_denoms ---
    best_candidate: Fraction | None = None
    best_error = float("inf")

    for d in allowed_denoms:
        candidate_num = round_half_up(frac_val * d)
        # If the candidate rounds to or above the denominator, treat it as a full inch.
        if candidate_num >= d:
            candidate = Fraction(1, 1)
        else:
            candidate = Fraction(candidate_num, d)
        error = abs(float(candidate) - frac_val)
        # Tie-breaker: prefer the candidate with the lower denominator.
        if error < best_error or (
            error == best_error
            and best_candidate is not None
            and d < best_candidate.denominator
        ):
            best_error = error
            best_candidate = candidate

    if best_candidate is None:
        raise RuntimeError("Unable to compute a candidate fraction.")

    if best_candidate >= 1:
        return (whole + 1, None, None)

    # --- Step 3: No segmentation requested: return best candidate ---
    if not segments:
        return (whole, best_candidate, None)

    # --- Step 4: Create segmented output (base + adjustment) ---
    # Use coarser denominators (those less than the maximum allowed) for the base.
    max_allowed = max(allowed_denoms)
    coarser_denoms = [d for d in allowed_denoms if d < max_allowed]
    if not coarser_denoms:
        return (whole, best_candidate, None)

    best_base: Fraction | None = None
    best_base_error = float("inf")
    candidate_float = float(best_candidate)

    for d in coarser_denoms:
        base_num = round_half_up(candidate_float * d)
        base_candidate = Fraction(base_num, d)
        base_error = abs(float(base_candidate) - candidate_float)
        if base_error < best_base_error or (
            base_error == best_base_error
            and best_base is not None
            and d < best_base.denominator
        ):
            best_base_error = base_error
            best_base = base_candidate

    if best_base is None:
        return (whole, best_candidate, None)

    adjustment = best_candidate - best_base
    if adjustment == 0:
        adjustment = None

    return (whole, best_base, adjustment)


def format_float_informative(value: float, precision: int) -> str:
    """
    Format a float using fixed-point notation with the given precision,
    but strip off uninformative trailing zeros and a trailing decimal point.

    For example:
      format_float_informative(1.000, 3) returns "1"
      format_float_informative(0.015, 3) returns "0.015"
    """
    s = f"{value:.{precision}f}"
    if "." in s:
        s = s.rstrip("0").rstrip(".")
    return s


def format_value(
    value: float,
    unit: str,
    precision_in: int = 3,
    precision_mm: int = 1,
    use_tape_conversion: bool = True,
    segments: bool = True,
    allowed_denoms: tuple[int, ...] = (2, 4, 8, 16, 32),
) -> str:
    """
    Format a measurement value with the appropriate unit symbol.

    For "inches":
        When use_tape_conversion is True, applies tape-measure conversion to yield a friendly
        mixed-number representation. The conversion function returns a three-element tuple
        (whole, fraction, adjustment) where components that do not apply are None.
        When use_tape_conversion is False, the value is simply formatted as a float
        using precision_in.

    For other units (e.g., "mm"):
        The value is converted (here, inches_to_mm) and formatted with the given precision.

    Args:
        value: float
            Measurement value in inches.
        unit: str
            The display unit system ("inches" or "mm").
        precision_in: int
            Number of decimal places for a plain inch value (used when use_tape_conversion is False).
        precision_mm: int
            Number of decimal places for millimeter values.
        use_tape_conversion: bool
            If True, use the tape-measure conversion for inch values.
        segments: bool
            If True, use segmented conversion (base + adjustment) when tape conversion is enabled.
        allowed_denoms: tuple[int, ...]
            Allowed denominators passed to the conversion function.

    Returns:
        A formatted string representing the measurement with unit.

    Examples:
        • format_value(4.72, "inches", use_tape_conversion=True)
              -> '4 3/4 - 1/32"'
        • format_value(4.72, "inches", use_tape_conversion=False)
              -> '4.72"'
        • format_value(4.72, "mm")
              -> '120.1 mm'
    """
    formatted_val = None
    if unit == "inches":
        if use_tape_conversion:
            # unformatted_val = f"{value:.{precision_in}f}\""
            unformatted_val = f'{format_float_informative(value, precision_in)}"'
            whole, fraction, adjustment = convert_decimal_to_tape_measure(
                value, allowed_denoms=allowed_denoms, segments=segments
            )
            if fraction is None:
                formatted_val = f'{whole}"'
            else:
                if whole == 0:
                    # No whole part.
                    whole = ""
                else:
                    whole = str(whole) + " "
                # Basic fraction string.
                fraction_str = f"{fraction.numerator}/{fraction.denominator}"
                if adjustment is None:
                    formatted_val = f'{whole}{fraction_str}"'
                else:
                    sign = "+" if adjustment > 0 else "-"
                    abs_adjustment = abs(adjustment)
                    adjustment_str = (
                        f"{abs_adjustment.numerator}/{abs_adjustment.denominator}"
                    )
                    formatted_val = f'{whole}{fraction_str} {sign} {adjustment_str}"'
        else:
            # Use plain floating point formatting.
            # formatted_val = f"{value:.{precision_in}f}\""
            formatted_val = f'{format_float_informative(value, precision_in)}"'
            unformatted_val = None
    else:
        mm_value = inches_to_mm(value)
        # formatted_val = f"{mm_value:.{precision_mm}f} mm"
        formatted_val = f"{format_float_informative(mm_value, precision_mm)} mm"
        unformatted_val = None

    # Only append decimal form if it's different from the formatted version
    if unformatted_val and unformatted_val != formatted_val:
        formatted_val = f"{formatted_val} ({unformatted_val})"
    return formatted_val


def format_dimension_pair(label: str, value1: float, value2: float, unit: str) -> str:
    """
    Format a pair of dimension values (height x width) with label.

    Args:
        label: Descriptive label for the dimension pair
        value1: First dimension (typically height) in inches
        value2: Second dimension (typically width) in inches
        unit: The display unit system ("inches" or "mm")

    Returns:
        Formatted string with label and dimensions
    """
    return f"**{label}:** {format_value(value1, unit)} × {format_value(value2, unit)}"