"""Shareable URL generation for frame designs.

This module handles the creation of compact shareable URLs that encode
all frame settings in a binary format.
"""

import struct
import base64
from ui_helpers import get_form_values_as_inches


def generate_shareable_url(document, current_unit: str, include_mat: bool) -> str | None:
    """Generate a compact shareable URL encoding all frame settings.

    Binary format (28 bytes → ~38 chars base64 → 81 char URL):
        5 × uint24: h, w, mw, fw, fd (×10000 for 4 decimal precision)
        6 × uint16: gt, mt, at, bt, rd, bw (×10000 for 4 decimal precision)
        1 × byte: flags (bit 0 = mat, bit 1 = unit_mm)

    All values stored in inches internally.

    Args:
        document: PyScript document object
        current_unit: Current unit ("inches" or "mm")
        include_mat: Whether matboard is included

    Returns:
        Shareable URL string or None if validation fails
    """
    values = get_form_values_as_inches(document, current_unit)
    if values is None:
        return None

    unit_mm = (current_unit == "mm")

    def pack_uint24(val):
        """Pack a value as big-endian uint24 (3 bytes)."""
        v = int(val * 10000)
        return bytes([(v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF])

    # Build binary data
    packed = b''

    # uint24 fields: h, w, mw, fw, fd
    packed += pack_uint24(values["artwork_height"])
    packed += pack_uint24(values["artwork_width"])
    packed += pack_uint24(values["mat_width"])
    packed += pack_uint24(values["frame_width"])
    packed += pack_uint24(values["frame_depth"])

    # uint16 fields: gt, mt, at, bt, rd, bw
    for key in ["glazing_thickness", "matboard_thickness", "artwork_thickness",
                "backing_thickness", "rabbet_depth", "blade_width"]:
        v = int(values[key] * 10000)
        packed += struct.pack('>H', v)

    # Flags byte
    flags = (1 if include_mat else 0) | ((1 if unit_mm else 0) << 1)
    packed += bytes([flags])

    # Base64 encode (URL-safe, no padding)
    b64 = base64.urlsafe_b64encode(packed).decode().rstrip('=')

    # Build full URL
    base_url = "https://glarue.github.io/ReferenceFrame/"
    return f"{base_url}?d={b64}"
