"""PDF export functionality for frame designs.

This module handles generation and export of PDF summaries with
vector diagrams and QR codes for design recreation.
"""

from ui_helpers import get_form_values_as_inches
from frame_design import create_frame_design_from_values
from conversions import format_value


def generate_pdf_content(pdf, document, current_unit: str, start_y: float = 20) -> float:
    """Add formatted content to PDF.

    Args:
        pdf: jsPDF instance
        document: PyScript document object
        current_unit: Current unit ("inches" or "mm")
        start_y: Starting Y position for content

    Returns:
        Final Y position after content
    """
    values = get_form_values_as_inches(document, current_unit)
    if values is None:
        return start_y

    design = create_frame_design_from_values(values)

    # Extract commonly used values
    height = values["artwork_height"]
    width = values["artwork_width"]
    mat_width = values["mat_width"]
    frame_width = values["frame_width"]
    rabbet_depth = values["rabbet_depth"]
    frame_depth = values["frame_depth"]
    blade_width = values["blade_width"]

    # Colors from UI theme (RGB values)
    COLOR_PRIMARY = (39, 125, 161)      # cerulean #277da1
    COLOR_SUCCESS = (144, 190, 109)     # willow-green #90be6d
    COLOR_ERROR = (249, 65, 68)         # strawberry-red #f94144
    COLOR_HIGHLIGHT = (255, 165, 0)     # orange for cut list
    COLOR_BLACK = (0, 0, 0)
    COLOR_GRAY = (100, 100, 100)

    frame_inside = design.get_frame_inside_dimensions()
    frame_outside = design.get_frame_outside_dimensions()
    cut_list = design.get_cut_list()
    required_depth = design.get_rabbet_z_depth_required()
    total_wood_length = design.get_total_wood_length(saw_margin=blade_width)

    y = start_y
    left_col = 15
    right_col = 110  # Right column starts here
    col_width = 90  # Maximum width for each column (prevents overlap)

    def add_section(text, x=left_col):
        nonlocal y
        pdf.setFontSize(12)
        pdf.setFont("helvetica", "bold")
        pdf.setTextColor(*COLOR_PRIMARY)
        pdf.text(text, x, y)
        pdf.setTextColor(*COLOR_BLACK)
        y += 6

    def add_line(text, x=left_col, indent=0, color=None, max_width=None):
        nonlocal y
        pdf.setFontSize(11)
        pdf.setFont("helvetica", "normal")
        if color:
            pdf.setTextColor(*color)

        # Use column width if max_width not specified
        if max_width is None:
            max_width = col_width

        # Check if text fits within max_width
        text_width = pdf.getTextWidth(text)
        if text_width > max_width:
            # Split text into multiple lines
            words = text.split(' ')
            lines = []
            current_line = ''

            for word in words:
                test_line = current_line + (' ' if current_line else '') + word
                if pdf.getTextWidth(test_line) <= max_width:
                    current_line = test_line
                else:
                    if current_line:
                        lines.append(current_line)
                    current_line = word

            if current_line:
                lines.append(current_line)

            # Print each line
            for line in lines:
                pdf.text(line, x + indent, y)
                y += 5
        else:
            pdf.text(text, x + indent, y)
            y += 5

        if color:
            pdf.setTextColor(*COLOR_BLACK)

    def add_spacer(height=3):
        nonlocal y
        y += height

    # Two-column layout for compact display
    col_start_y = y

    # LEFT COLUMN
    # Artwork
    add_section("Artwork")
    add_line(f"{format_value(height, current_unit)} H  x  {format_value(width, current_unit)} W")
    add_spacer()

    # Cut List
    add_section("Cut List")
    for category, pieces in cut_list.items():
        category_name = "Top/Bottom" if "horizontal" in category else "Sides"
        for piece_spec in pieces:
            qty = piece_spec.get('quantity', 1)
            outside = piece_spec.get('outside_length', 0)
            # Format: "Top/Bottom: 17 3/4" (17.75") (×2)"
            # Note: format_value already includes decimal in parens for fractional values
            formatted_val = format_value(outside, current_unit)
            pdf.setFontSize(11)
            pdf.setFont("helvetica", "normal")
            pdf.text(f"{category_name}: ", left_col, y)
            x_pos = left_col + pdf.getTextWidth(f"{category_name}: ")
            pdf.setFont("helvetica", "bold")
            pdf.setTextColor(*COLOR_HIGHLIGHT)
            pdf.text(formatted_val, x_pos, y)
            x_pos += pdf.getTextWidth(formatted_val)
            pdf.setFont("helvetica", "normal")
            pdf.setTextColor(*COLOR_GRAY)
            pdf.text(f" (×{qty})", x_pos, y)
            pdf.setTextColor(*COLOR_BLACK)
            y += 5
    add_spacer()

    # Material
    add_section("Material")
    pdf.setFontSize(11)
    pdf.setFont("helvetica", "normal")
    pdf.text("Total length: ", left_col, y)
    x_pos = left_col + pdf.getTextWidth("Total length: ")
    pdf.setFont("helvetica", "bold")
    pdf.setTextColor(*COLOR_PRIMARY)
    pdf.text(format_value(total_wood_length, current_unit), x_pos, y)
    pdf.setTextColor(*COLOR_BLACK)
    y += 5
    add_spacer()

    # Frame dims (outside first)
    add_section("Frame")
    add_line(f"Outside: {format_value(frame_outside[0], current_unit)} x {format_value(frame_outside[1], current_unit)}")
    add_line(f"Inside: {format_value(frame_inside[0], current_unit)} x {format_value(frame_inside[1], current_unit)}")

    left_col_end_y = y

    # RIGHT COLUMN
    y = col_start_y

    # Matboard
    if design.has_mat:
        matboard_dims = design.get_matboard_dimensions()
        mat_opening = design.get_mat_opening_dimensions()
        mat_cut_width = mat_width + rabbet_depth

        add_section("Matboard", right_col)
        add_line(f"Size: {format_value(matboard_dims[0], current_unit)} x {format_value(matboard_dims[1], current_unit)}", right_col)
        add_line(f"Opening: {format_value(mat_opening[0], current_unit)} x {format_value(mat_opening[1], current_unit)}", right_col)
        add_line(f"Visual width: {format_value(mat_width, current_unit)}", right_col)
        add_line(f"Cut width: {format_value(mat_cut_width, current_unit)}", right_col)
        add_spacer()

    # Depth
    add_section("Depth (Z)", right_col)
    add_line(f"Required: {format_value(required_depth, current_unit)}", right_col)
    add_line(f"Available: {format_value(frame_depth, current_unit)}", right_col)
    if required_depth > frame_depth:
        shortfall = required_depth - frame_depth
        add_line(f"SHORT BY {format_value(shortfall, current_unit)}!", right_col, 0, COLOR_ERROR)
    else:
        clearance = frame_depth - required_depth
        add_line(f"Margin: {format_value(clearance, current_unit)}", right_col, 0, COLOR_SUCCESS)
    add_spacer()

    # Specs
    add_section("Specs", right_col)
    add_line(f"Frame width: {format_value(frame_width, current_unit)}", right_col)
    add_line(f"Rabbet: {format_value(rabbet_depth, current_unit)}", right_col)
    if design.has_mat:
        add_line(f"Mat overlap: {format_value(design.mat_overlap, current_unit)}", right_col)

    return max(y, left_col_end_y)


def add_qr_code_to_pdf(pdf, url: str, status_div, console):
    """Add QR code to bottom-right corner of PDF with label and URL, then save.

    The QR code is positioned in the bottom-right corner (A4 page) with a small
    label and the shortened URL underneath. After adding the QR code, the PDF
    is saved automatically.

    Args:
        pdf: jsPDF instance
        url: The shareable URL to encode in the QR code
        status_div: DOM element to update with status messages
        console: JS console object for logging
    """
    from js import window
    from pyodide.ffi import create_proxy

    console.log(f"add_qr_code_to_pdf called with URL: {url}")

    # QR code dimensions and position (bottom-right corner)
    # A4 page: 210mm x 297mm, with 10mm margins
    qr_size = 25  # 25mm square - large enough to scan reliably
    margin = 10
    page_width = 210
    x_pos = page_width - margin - qr_size  # QR positioned at right margin
    y_pos = 297 - margin - qr_size - 3  # Bottom edge minus margin, QR size, and small gap

    def on_qr_generated(data_url):
        console.log(f"on_qr_generated callback fired, data_url exists: {bool(data_url)}")
        try:
            if data_url:
                # Add text above QR code
                # Calculate text widths and position to right-align within margins

                # Label text (above QR)
                label_text = "Scan to recreate this design"
                pdf.setFontSize(7)
                pdf.setTextColor(128, 128, 128)  # Gray
                label_width = pdf.getTextWidth(label_text)
                label_x = page_width - margin - label_width
                label_y = y_pos - 3  # 3mm above QR code
                pdf.text(label_text, label_x, label_y)

                console.log(f"Adding QR image at ({x_pos}, {y_pos}), size {qr_size}")
                # Add QR code image
                pdf.addImage(data_url, 'PNG', x_pos, y_pos, qr_size, qr_size)

                # URL text (below QR)
                pdf.setFontSize(5)
                pdf.setTextColor(128, 128, 128)  # Same gray as label
                url_width = pdf.getTextWidth(url)
                url_x = page_width - margin - url_width
                url_y = y_pos + qr_size + 3
                pdf.text(url, url_x, url_y)

                console.log("QR code added to PDF successfully")
            else:
                console.log("QR code generation returned null, saving PDF without QR")

            # Save PDF (with or without QR code)
            console.log("Saving PDF...")
            pdf.save("frame_design_summary.pdf")
            status_div.innerHTML = '<span class="success">✅ PDF downloaded!</span>'

        except Exception as e:
            console.log(f"Error adding QR code to PDF: {e}")
            import traceback
            console.log(traceback.format_exc())
            # Still try to save the PDF
            pdf.save("frame_design_summary.pdf")
            status_div.innerHTML = '<span class="success">✅ PDF downloaded!</span>'

    # Generate QR code asynchronously
    promise = window.generateQrCodeDataUrl(url)
    promise.then(create_proxy(on_qr_generated))


def handle_export_pdf(document, current_unit: str, shareable_url: str | None, console):
    """Export frame design as PDF with vector diagram at top, details below, QR code in corner.

    The PDF includes:
    - Vector diagram of the frame design (top)
    - Detailed specifications in two columns (middle)
    - QR code in bottom-right corner linking to shareable URL

    The QR code encodes a compact binary URL (~81 chars) that captures all frame
    settings, allowing the design to be recreated by scanning or visiting the URL.

    Args:
        document: PyScript document object
        current_unit: Current unit ("inches" or "mm")
        shareable_url: Shareable URL for QR code (or None)
        console: JS console object for logging
    """
    from js import jspdf, window
    from pyodide.ffi import create_proxy

    status_div = document.getElementById("export-status")

    try:
        status_div.innerHTML = '<span style="color: #888;">Generating PDF...</span>'

        # Check if SVG exists
        svg_elem = document.querySelector("#matplotlib-canvas svg")
        if svg_elem:
            # Get SVG dimensions to calculate aspect ratio
            bbox = svg_elem.getBoundingClientRect()
            svg_width = float(bbox.width) or 800
            svg_height = float(bbox.height) or 600

            # Calculate diagram dimensions for top portion of page
            max_diagram_height = 145
            page_width = 195
            aspect = svg_height / svg_width
            img_width = page_width
            img_height = page_width * aspect

            if img_height > max_diagram_height:
                img_height = max_diagram_height
                img_width = img_height / aspect

            x_offset = (210 - img_width) / 2

            # JS creates PDF with vector SVG, returns it to Python for text
            def on_pdf_created(pdf):
                try:
                    if pdf:
                        # Add text content below the diagram
                        text_start_y = 5 + img_height + 6
                        generate_pdf_content(pdf, document, current_unit, start_y=text_start_y)

                        # Add QR code to bottom-right corner, then save
                        if shareable_url:
                            add_qr_code_to_pdf(pdf, shareable_url, status_div, console)
                        else:
                            pdf.save("frame_design_summary.pdf")
                            status_div.innerHTML = '<span class="success">✅ PDF downloaded!</span>'
                    else:
                        # Fallback to Python-only PDF (no diagram)
                        console.log("JS PDF creation failed, falling back to text-only")
                        fallback_pdf = jspdf.jsPDF.new()
                        generate_pdf_content(fallback_pdf, document, current_unit, start_y=20)

                        if shareable_url:
                            add_qr_code_to_pdf(fallback_pdf, shareable_url, status_div, console)
                        else:
                            fallback_pdf.save("frame_design_summary.pdf")
                            status_div.innerHTML = '<span class="success">✅ PDF downloaded (no diagram)</span>'
                except Exception as e:
                    console.log(f"Error in PDF completion: {e}")
                    import traceback
                    console.log(traceback.format_exc())
                    status_div.innerHTML = f'<span class="warning">❌ PDF error: {e}</span>'

            # Call JS to create PDF with vector SVG
            promise = window.createPdfWithVectorSvg(x_offset, 5, img_width, img_height)
            promise.then(create_proxy(on_pdf_created))

        else:
            # No SVG, just save the text PDF
            pdf = jspdf.jsPDF.new()
            generate_pdf_content(pdf, document, current_unit, start_y=20)

            if shareable_url:
                add_qr_code_to_pdf(pdf, shareable_url, status_div, console)
            else:
                pdf.save("frame_design_summary.pdf")
                status_div.innerHTML = '<span class="success">✅ PDF downloaded!</span>'

    except Exception as e:
        import traceback
        console.log(f"PDF export error: {e}")
        console.log(traceback.format_exc())
        status_div = document.getElementById("export-status")
        status_div.innerHTML = f'<span class="warning">❌ PDF export error: {e}</span>'
