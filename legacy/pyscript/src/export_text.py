"""Text/markdown export functionality for frame designs.

This module handles generation and export of text-based summaries
of frame designs.
"""

from ui_helpers import get_form_values_as_inches, create_frame_design_from_values
from conversions import format_value

# Constants
# Error margin for wood cutting (1/16" per piece)
ERROR_MARGIN_INCHES = 0.0625


def generate_text_summary(document, current_unit: str) -> str:
    """Generate a text/markdown summary of the current frame design.

    Args:
        document: PyScript document object
        current_unit: Current unit ("inches" or "mm")

    Returns:
        Formatted text summary
    """
    values = get_form_values_as_inches(document, current_unit)
    if values is None:
        return "Error: Required fields are empty"

    design = create_frame_design_from_values(values)

    # Extract commonly used values for readability
    height = values["artwork_height"]
    width = values["artwork_width"]
    mat_width = values["mat_width"]
    frame_width = values["frame_width"]
    rabbet_depth = values["rabbet_depth"]
    frame_depth = values["frame_depth"]
    blade_width = values["blade_width"]
    glazing_thick = values["glazing_thickness"]
    matboard_thick = values["matboard_thickness"]
    artwork_thick = values["artwork_thickness"]
    backing_thick = values["backing_thickness"]

    # Get calculations
    frame_inside = design.get_frame_inside_dimensions()
    frame_outside = design.get_frame_outside_dimensions()
    cut_list = design.get_cut_list()
    required_depth = design.get_rabbet_z_depth_required()
    total_wood_length = design.get_total_wood_length(saw_margin=blade_width)

    # Build summary
    lines = []
    lines.append("=" * 50)
    lines.append("FRAME DESIGN SUMMARY")
    lines.append("=" * 50)
    lines.append("")

    # Artwork
    lines.append("ARTWORK DIMENSIONS")
    lines.append("-" * 30)
    lines.append(f"  Height: {format_value(height, current_unit)}")
    lines.append(f"  Width:  {format_value(width, current_unit)}")
    lines.append("")

    # Cut List
    lines.append("CUT LIST")
    lines.append("-" * 30)
    for category, pieces in cut_list.items():
        category_name = "Top & Bottom" if "horizontal" in category else "Left & Right"
        for piece_spec in pieces:
            qty = piece_spec.get('quantity', 1)
            inside = piece_spec.get('inside_length', 0)
            outside = piece_spec.get('outside_length', 0)
            lines.append(f"  {category_name}: {qty}x {format_value(outside, current_unit)} (inside: {format_value(inside, current_unit)})")
    lines.append("")

    # Material Requirements
    lines.append("MATERIAL REQUIREMENTS")
    lines.append("-" * 30)
    lines.append(f"  Total Wood Length: {format_value(total_wood_length, current_unit)}")
    lines.append(f"    (includes {format_value(blade_width, current_unit)} blade kerf + {format_value(ERROR_MARGIN_INCHES, current_unit)} error margin per piece)")
    lines.append("")

    # Frame Dimensions
    lines.append("FRAME DIMENSIONS")
    lines.append("-" * 30)
    lines.append(f"  Inside:  {format_value(frame_inside[0], current_unit)} H x {format_value(frame_inside[1], current_unit)} W")
    lines.append(f"  Outside: {format_value(frame_outside[0], current_unit)} H x {format_value(frame_outside[1], current_unit)} W")
    lines.append("")

    # Matboard (if applicable)
    if design.has_mat:
        matboard_dims = design.get_matboard_dimensions()
        mat_opening = design.get_mat_opening_dimensions()
        mat_cut_width = mat_width + rabbet_depth

        lines.append("MATBOARD DETAILS")
        lines.append("-" * 30)
        lines.append(f"  Matboard Size: {format_value(matboard_dims[0], current_unit)} H x {format_value(matboard_dims[1], current_unit)} W")
        lines.append(f"  Mat Opening:   {format_value(mat_opening[0], current_unit)} H x {format_value(mat_opening[1], current_unit)} W")
        lines.append(f"  Visual Mat Width: {format_value(mat_width, current_unit)}")
        lines.append(f"  Mat Border Cut Width: {format_value(mat_cut_width, current_unit)} (visual + {format_value(rabbet_depth, current_unit)} rabbet)")
        lines.append("")

    # Depth Requirements
    lines.append("DEPTH REQUIREMENTS (Z-AXIS)")
    lines.append("-" * 30)
    lines.append(f"  Required Depth: {format_value(required_depth, current_unit)}")
    lines.append(f"  Available Depth: {format_value(frame_depth, current_unit)}")
    if required_depth > frame_depth:
        shortfall = required_depth - frame_depth
        lines.append(f"  *** WARNING: Frame is {format_value(shortfall, current_unit)} too shallow! ***")
    else:
        clearance = frame_depth - required_depth
        lines.append(f"  Clearance: {format_value(clearance, current_unit)}")
    lines.append("")

    # Specifications
    lines.append("SPECIFICATIONS")
    lines.append("-" * 30)
    lines.append(f"  Frame Material Width: {format_value(frame_width, current_unit)}")
    lines.append(f"  Frame Material Depth: {format_value(frame_depth, current_unit)}")
    lines.append(f"  Rabbet Depth (x/y): {format_value(rabbet_depth, current_unit)}")
    lines.append(f"  Mat Overlap: {format_value(design.mat_overlap, current_unit)}")
    lines.append(f"  Assembly Margin: {format_value(design.assembly_margin, current_unit)}")
    lines.append(f"  Blade Width (kerf): {format_value(blade_width, current_unit)}")
    lines.append("")
    lines.append("  Material Thicknesses:")
    lines.append(f"    Glazing:  {format_value(glazing_thick, current_unit)}")
    lines.append(f"    Matboard: {format_value(matboard_thick, current_unit)}")
    lines.append(f"    Artwork:  {format_value(artwork_thick, current_unit)}")
    lines.append(f"    Backing:  {format_value(backing_thick, current_unit)}")
    lines.append("")
    lines.append("=" * 50)

    return "\n".join(lines)


def download_file(content: str, filename: str, mime_type: str):
    """Trigger a file download in the browser.

    Args:
        content: File content as string
        filename: Name for downloaded file
        mime_type: MIME type for the file
    """
    from js import Blob, URL, document as js_document

    # Create blob and download link
    blob = Blob.new([content], {"type": mime_type})
    url = URL.createObjectURL(blob)

    # Create temporary link and click it
    link = js_document.createElement("a")
    link.href = url
    link.download = filename
    link.click()

    # Clean up
    URL.revokeObjectURL(url)


def handle_export_text(document, current_unit: str) -> tuple[bool, str]:
    """Export frame design as text file.

    Args:
        document: PyScript document object
        current_unit: Current unit ("inches" or "mm")

    Returns:
        Tuple of (success: bool, message: str)
    """
    try:
        summary = generate_text_summary(document, current_unit)
        download_file(summary, "frame_design_summary.txt", "text/plain")
        return (True, "✅ Text summary downloaded!")
    except Exception as e:
        import traceback
        traceback.print_exc()
        return (False, f"❌ Text export error: {e}")
