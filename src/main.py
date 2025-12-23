"""
ReferenceFrame - Main PyScript Application

This module contains the main application logic for the ReferenceFrame
picture frame calculator web app.
"""

from pyscript import document, when
from js import localStorage, console
from pyodide.ffi import create_proxy
import json

# Import our existing code
from frame import FrameSize
from conversions import format_value, inches_to_mm, mm_to_inches
from ui_helpers import (
    round_to_step,
    get_form_values_as_inches,
    create_frame_design_from_values,
)
from aspect_ratio import (
    get_aspect_ratio_display,
    get_aspect_ratio_display_from_ratio,
)
from defaults import (
    DEFAULT_ARTWORK_HEIGHT,
    DEFAULT_ARTWORK_WIDTH,
    DEFAULT_INCLUDE_MAT,
    DEFAULT_MAT_WIDTH,
    DEFAULT_FRAME_MATERIAL_WIDTH,
    DEFAULT_FRAME_THICKNESS,
    DEFAULT_RABBET_DEPTH,
    DEFAULT_GLAZING_THICKNESS,
    DEFAULT_MATBOARD_THICKNESS,
    DEFAULT_ARTWORK_THICKNESS,
    DEFAULT_BACKING_THICKNESS,
    DEFAULT_BLADE_WIDTH,
)

# Import refactored modules
from shareable_url import generate_shareable_url as generate_url
from export_text import generate_text_summary, download_file
from export_pdf import (
    generate_pdf_content as generate_pdf_content_impl,
    add_qr_code_to_pdf as add_qr_code_to_pdf_impl,
    handle_export_pdf as export_pdf_impl
)
from data_backup import (
    export_all_data as export_data_impl,
    import_data as import_data_impl,
    handle_file_upload as file_upload_impl,
    show_import_dialog as show_dialog_impl
)
from config_manager import (
    get_current_config as get_config,
    load_saved_configs as load_configs,
    save_config_to_storage as save_config,
    delete_config as del_config,
    load_config as apply_config,
    render_saved_configs as render_configs,
    handle_save_config as save_config_handler_impl
)

# Application state
app_state = {
    "current_unit": "inches",  # Current display unit
    "custom_sizes": [],  # List of FrameSize objects
}

# Load unit preference from localStorage
try:
    saved_unit = localStorage.getItem("frame_designer_unit")
    if saved_unit and saved_unit in ["inches", "mm"]:
        app_state["current_unit"] = saved_unit
        # Update toggle button active state
        if saved_unit == "mm":
            document.getElementById("unit-mm").classList.add("active")
            document.getElementById("unit-inches").classList.remove("active")
            # Update labels
            document.getElementById("label-artwork-unit").textContent = "(mm)"
            document.getElementById("label-mat-frame-unit").textContent = "(mm)"
        console.log(f"Restored unit preference: {saved_unit}")
except Exception as e:
    console.log(f"Could not load unit preference: {e}")

# Test matplotlib loading
try:
    import matplotlib
    import matplotlib.font_manager as fm
    # Use 'none' fonttype - required for svg2pdf.js (fonttype 42 causes missing chars)
    # Trade-off: text rendered by browser/PDF viewer, not matplotlib's bundled font
    matplotlib.rcParams['svg.fonttype'] = 'none'
    # DejaVu Sans is included in Pyodide, so matplotlib uses it for bbox calculations.
    # Put it first so PDF rendering uses the same font as matplotlib's measurements.
    # This ensures bounding boxes around text fit properly.
    matplotlib.rcParams['font.family'] = 'sans-serif'
    matplotlib.rcParams['font.sans-serif'] = ['DejaVu Sans', 'Verdana', 'Roboto', 'Helvetica', 'Arial', 'sans-serif']
    matplotlib.rcParams['font.size'] = 10
    # Debug: show which font was actually selected
    try:
        prop = fm.FontProperties()
        prop.set_family('sans-serif')
        selected_font = fm.findfont(prop)
        console.log(f"matplotlib: font selected = {selected_font}")
    except Exception as e:
        console.log(f"matplotlib: could not determine font: {e}")
    console.log("matplotlib: configured with DejaVu Sans priority (matches Pyodide metrics)")
    console.log("✅ matplotlib loaded successfully")
except Exception as e:
    console.log(f'⏳ matplotlib loading (may take 30s): {e}')

# ===== Helper Functions (must be defined before custom sizes) =====

def get_current_unit():
    """Get the currently selected unit from toggle buttons."""
    if "active" in document.getElementById("unit-inches").classList:
        return "inches"
    else:
        return "mm"

# ===== Aspect Ratio Lock State =====
aspect_lock_state: dict[str, bool | float | None] = {
    "locked": False,
    "ratio": None,
}

def update_aspect_ratio_display():
    """Update the displayed aspect ratio."""
    try:
        # If locked, always show the locked ratio
        if aspect_lock_state["locked"] and aspect_lock_state["ratio"]:
            ratio_display = get_aspect_ratio_display_from_ratio(aspect_lock_state["ratio"])
            console.log(f"Locked ratio display: {ratio_display} (ratio={aspect_lock_state['ratio']:.3f})")
        else:
            # Show current ratio from field values
            height_val = document.getElementById("artwork-height").value
            width_val = document.getElementById("artwork-width").value
            if not height_val or not width_val:
                ratio_display = "—"
            else:
                height = float(height_val)
                width = float(width_val)
                console.log(f"Ratio calc: h={height}, w={width}")
                ratio_display = get_aspect_ratio_display(height, width)
                console.log(f"Ratio display: {ratio_display}")
    except (ValueError, TypeError) as e:
        console.log(f"Error in ratio display: {e}")
        ratio_display = "—"

    ratio_elem = document.getElementById("aspect-ratio")
    ratio_elem.textContent = ratio_display
    if aspect_lock_state["locked"]:
        ratio_elem.classList.add("locked")
    else:
        ratio_elem.classList.remove("locked")

# ===== Standard Sizes =====

# Common photo and artwork sizes (Height × Width convention)
# "4×6" means height=4", width=6"
# FrameSize stores (name, height, width)
STANDARD_SIZES = [
    FrameSize("4×6", 4.0, 6.0),      # 4" tall × 6" wide
    FrameSize("5×7", 5.0, 7.0),      # 5" tall × 7" wide
    FrameSize("8×10", 8.0, 10.0),    # 8" tall × 10" wide
    FrameSize("11×14", 11.0, 14.0),  # 11" tall × 14" wide
    FrameSize("12×18", 12.0, 18.0),  # 12" tall × 18" wide
    FrameSize("16×20", 16.0, 20.0),  # 16" tall × 20" wide
    FrameSize("18×24", 18.0, 24.0),  # 18" tall × 24" wide
    FrameSize("20×30", 20.0, 30.0),  # 20" tall × 30" wide
]

def render_standard_sizes():
    """Render standard size dropdown options."""
    select_elem = document.getElementById("standard-sizes-select")
    if not select_elem:
        return

    # Clear existing options except the first placeholder
    select_elem.innerHTML = '<option value="">-- Select a size --</option>'

    current_unit = get_current_unit()

    for idx, size in enumerate(STANDARD_SIZES):
        option = document.createElement("option")
        option.value = str(idx)

        # Format dimensions in current unit
        if current_unit == "inches":
            # For standard sizes (whole numbers), don't show decimal equivalent
            # Use use_tape_conversion=False to get simple whole number format
            if size.height == int(size.height) and size.width == int(size.width):
                height_str = f'{int(size.height)}"'
                width_str = f'{int(size.width)}"'
            else:
                height_str = format_value(size.height, "inches", 2)
                width_str = format_value(size.width, "inches", 2)
            option.textContent = f"{size.name} ({height_str} × {width_str})"
        else:
            height_mm = inches_to_mm(size.height)
            width_mm = inches_to_mm(size.width)
            option.textContent = f"{size.name} ({height_mm:.0f} × {width_mm:.0f}mm)"

        select_elem.appendChild(option)

def apply_standard_size(height, width):
    """Apply a standard size to the form inputs."""
    current_unit = get_current_unit()

    if current_unit == "mm":
        # Convert to mm for display
        document.getElementById("artwork-height").value = str(inches_to_mm(height))
        document.getElementById("artwork-width").value = str(inches_to_mm(width))
    else:
        document.getElementById("artwork-height").value = str(height)
        document.getElementById("artwork-width").value = str(width)

    # Update orientation icon to match new dimensions
    update_orientation_icon()

    # Auto-render visualization
    render_visualization()

# Event handler for apply standard size button
@when("click", "#apply-standard-size")
def handle_apply_standard_size(event):
    """Handle apply standard size button click."""
    select_elem = document.getElementById("standard-sizes-select")
    selected_value = select_elem.value

    if selected_value and selected_value != "":
        idx = int(selected_value)
        size = STANDARD_SIZES[idx]
        apply_standard_size(size.height, size.width)

# ===== Custom Sizes Management =====

def load_custom_sizes():
    """Load custom sizes from localStorage."""
    try:
        sizes_data = localStorage.getItem("frame_designer_custom_sizes")
        if sizes_data and sizes_data != "null":
            sizes_list = json.loads(sizes_data)
            app_state["custom_sizes"] = [
                FrameSize(name=s["name"], height=s["height"], width=s["width"])
                for s in sizes_list
            ]
            console.log(f"Loaded {len(app_state['custom_sizes'])} custom sizes")
        else:
            app_state["custom_sizes"] = []
    except Exception as e:
        console.log(f"Error loading custom sizes: {e}")
        app_state["custom_sizes"] = []

def save_custom_sizes():
    """Save custom sizes to localStorage."""
    try:
        sizes_data = [
            {"name": s.name, "height": s.height, "width": s.width}
            for s in app_state["custom_sizes"]
        ]
        localStorage.setItem("frame_designer_custom_sizes", json.dumps(sizes_data))
        console.log(f"Saved {len(sizes_data)} custom sizes")
    except Exception as e:
        console.log(f"Error saving custom sizes: {e}")

def render_custom_sizes():
    """Render the dropdown of custom sizes."""
    select_elem = document.getElementById("saved-sizes-select")

    # Check if element exists (DOM might not be ready yet)
    if not select_elem:
        console.log("Custom sizes dropdown not ready yet, skipping render")
        return

    # Clear existing options
    select_elem.innerHTML = ""

    if len(app_state["custom_sizes"]) == 0:
        # Show "no sizes" option
        option = document.createElement("option")
        option.value = ""
        option.textContent = "-- No saved sizes yet --"
        select_elem.appendChild(option)
    else:
        # Add placeholder option
        placeholder = document.createElement("option")
        placeholder.value = ""
        placeholder.textContent = "-- Select a size --"
        select_elem.appendChild(placeholder)

        # Add each saved size as an option
        current_unit = get_current_unit()
        for idx, size in enumerate(app_state["custom_sizes"]):
            # Format dimensions in current unit
            height_display = format_value(size.height, current_unit)
            width_display = format_value(size.width, current_unit)

            option = document.createElement("option")
            option.value = str(idx)
            option.textContent = f"{size.name} ({height_display} × {width_display})"
            select_elem.appendChild(option)

def apply_custom_size(index):
    """Apply a saved size to the form inputs."""
    try:
        if 0 <= index < len(app_state["custom_sizes"]):
            size = app_state["custom_sizes"][index]
            current_unit = get_current_unit()

            # Convert from inches (storage format) to current display unit
            if current_unit == "mm":
                height_display = inches_to_mm(size.height)
                width_display = inches_to_mm(size.width)
            else:
                height_display = size.height
                width_display = size.width

            # Update form inputs
            document.getElementById("artwork-height").value = str(round(height_display, 2))
            document.getElementById("artwork-width").value = str(round(width_display, 2))

            # Clear any warning message
            msg_div = document.getElementById("add-size-message")
            msg_div.innerHTML = ""

            # Update orientation icon to match new dimensions
            update_orientation_icon()

            console.log(f"Applied custom size: {size.name}")

            # Automatically render visualization with new dimensions
            render_visualization()
    except Exception as e:
        console.log(f"Error applying custom size: {e}")
        results_div = document.getElementById("results")
        results_div.innerHTML = f'<span class="warning">❌ Error: {e}</span>'

def delete_custom_size(index):
    """Delete a saved size."""
    try:
        if 0 <= index < len(app_state["custom_sizes"]):
            deleted_size = app_state["custom_sizes"][index]
            app_state["custom_sizes"].pop(index)
            save_custom_sizes()
            render_custom_sizes()
            console.log(f"Deleted custom size: {deleted_size.name}")

            # Show success message
            msg_div = document.getElementById("add-size-message")
            msg_div.innerHTML = f'<div class="success">✅ Deleted: {deleted_size.name}</div>'
            # Clear message after 3 seconds
            import asyncio
            async def clear_message():
                await asyncio.sleep(3)
                msg_div.innerHTML = ""
            asyncio.create_task(clear_message())
    except Exception as e:
        console.log(f"Error deleting custom size: {e}")

# Load custom sizes on startup
load_custom_sizes()

# Render with retry to ensure DOM is ready
import asyncio
async def init_sizes():
    """Initialize standard and custom sizes display with retry for DOM readiness."""
    for attempt in range(5):
        custom_select = document.getElementById("saved-sizes-select")
        standard_select = document.getElementById("standard-sizes-select")
        if custom_select and standard_select:
            render_standard_sizes()
            render_custom_sizes()
            console.log("Standard and custom sizes rendered successfully")
            break
        else:
            console.log(f"DOM not ready (attempt {attempt + 1}), waiting...")
            await asyncio.sleep(0.2)

asyncio.create_task(init_sizes())

# Event handler: Apply saved size button
@when("click", "#apply-saved-size")
def handle_apply_saved_size(event):
    """Apply the selected saved size to the form."""
    select_elem = document.getElementById("saved-sizes-select")
    selected_value = select_elem.value

    if selected_value and selected_value != "":
        idx = int(selected_value)
        apply_custom_size(idx)
    else:
        msg_div = document.getElementById("add-size-message")
        msg_div.innerHTML = '<div class="warning">⚠️ Please select a size first</div>'

# Event handler: Delete saved size button
@when("click", "#delete-saved-size")
def handle_delete_saved_size(event):
    """Delete the selected saved size."""
    select_elem = document.getElementById("saved-sizes-select")
    selected_value = select_elem.value

    if selected_value and selected_value != "":
        idx = int(selected_value)
        delete_custom_size(idx)
        # Reset dropdown selection after delete
        select_elem.value = ""
    else:
        msg_div = document.getElementById("add-size-message")
        msg_div.innerHTML = '<div class="warning">⚠️ Please select a size first</div>'

# ===== End Custom Sizes Management =====

# Event handler: Unit toggle button clicks
def switch_unit(new_unit):
    """Handle unit toggle - convert all form values."""
    old_unit = app_state["current_unit"]

    if old_unit == new_unit:
        return  # No change

    # Update button active states
    inches_btn = document.getElementById("unit-inches")
    mm_btn = document.getElementById("unit-mm")
    if new_unit == "mm":
        inches_btn.classList.remove("active")
        mm_btn.classList.add("active")
    else:
        mm_btn.classList.remove("active")
        inches_btn.classList.add("active")

    # Get all input fields (main and advanced)
    height_input = document.getElementById("artwork-height")
    width_input = document.getElementById("artwork-width")
    mat_input = document.getElementById("mat-width")
    frame_input = document.getElementById("frame-width")
    glazing_input = document.getElementById("glazing-thickness")
    matboard_input = document.getElementById("matboard-thickness")
    artwork_t_input = document.getElementById("artwork-thickness")
    backing_input = document.getElementById("backing-thickness")
    rabbet_input = document.getElementById("rabbet-depth")

    # Convert values
    if new_unit == "mm":
        # Convert inches → mm (main fields)
        height_input.value = str(round(inches_to_mm(float(height_input.value)), 1))
        width_input.value = str(round(inches_to_mm(float(width_input.value)), 1))
        mat_input.value = str(round(inches_to_mm(float(mat_input.value)), 1))
        frame_input.value = str(round(inches_to_mm(float(frame_input.value)), 1))

        # Convert advanced fields
        glazing_input.value = str(round(inches_to_mm(float(glazing_input.value)), 2))
        matboard_input.value = str(round(inches_to_mm(float(matboard_input.value)), 2))
        artwork_t_input.value = str(round(inches_to_mm(float(artwork_t_input.value)), 2))
        backing_input.value = str(round(inches_to_mm(float(backing_input.value)), 2))
        rabbet_input.value = str(round(inches_to_mm(float(rabbet_input.value)), 2))

        # Update step sizes for mm
        height_input.step = "1"
        width_input.step = "1"
        mat_input.step = "1"
        frame_input.step = "1"
        glazing_input.step = "0.1"
        matboard_input.step = "0.1"
        artwork_t_input.step = "0.1"
        backing_input.step = "0.1"
        rabbet_input.step = "1"

        # Update labels
        document.getElementById("label-artwork-unit").textContent = "(mm)"
        document.getElementById("label-mat-frame-unit").textContent = "(mm)"
        document.getElementById("label-glazing-unit").textContent = "(mm)"
        document.getElementById("label-matboard-unit").textContent = "(mm)"
        document.getElementById("label-artwork-thickness-unit").textContent = "(mm)"
        document.getElementById("label-backing-unit").textContent = "(mm)"
        document.getElementById("label-rabbet-unit").textContent = "(mm)"
        document.getElementById("label-frame-depth-unit").textContent = "(mm)"
        document.getElementById("label-blade-unit").textContent = "(mm)"
    else:
        # Convert mm → inches (main fields) - round to step to avoid drift
        height_input.value = str(round_to_step(mm_to_inches(float(height_input.value)), 0.25))
        width_input.value = str(round_to_step(mm_to_inches(float(width_input.value)), 0.25))
        mat_input.value = str(round_to_step(mm_to_inches(float(mat_input.value)), 0.125))
        frame_input.value = str(round_to_step(mm_to_inches(float(frame_input.value)), 0.125))

        # Convert advanced fields
        glazing_input.value = str(round_to_step(mm_to_inches(float(glazing_input.value)), 0.01))
        matboard_input.value = str(round_to_step(mm_to_inches(float(matboard_input.value)), 0.01))
        artwork_t_input.value = str(round_to_step(mm_to_inches(float(artwork_t_input.value)), 0.01))
        backing_input.value = str(round_to_step(mm_to_inches(float(backing_input.value)), 0.01))
        rabbet_input.value = str(round_to_step(mm_to_inches(float(rabbet_input.value)), 0.125))

        # Update step sizes for inches
        height_input.step = "0.25"
        width_input.step = "0.25"
        mat_input.step = "0.125"
        frame_input.step = "0.125"
        glazing_input.step = "0.01"
        matboard_input.step = "0.01"
        artwork_t_input.step = "0.01"
        backing_input.step = "0.01"
        rabbet_input.step = "0.125"

        # Update labels
        document.getElementById("label-artwork-unit").textContent = "(inches)"
        document.getElementById("label-mat-frame-unit").textContent = "(inches)"
        document.getElementById("label-glazing-unit").textContent = "(in)"
        document.getElementById("label-matboard-unit").textContent = "(in)"
        document.getElementById("label-artwork-thickness-unit").textContent = "(in)"
        document.getElementById("label-backing-unit").textContent = "(in)"
        document.getElementById("label-rabbet-unit").textContent = "(in)"
        document.getElementById("label-frame-depth-unit").textContent = "(in)"
        document.getElementById("label-blade-unit").textContent = "(in)"

    # Update app state
    app_state["current_unit"] = new_unit

    # Save preference to localStorage
    localStorage.setItem("frame_designer_unit", new_unit)

    console.log(f"Unit changed from {old_unit} to {new_unit}")

    # Also update custom size form labels
    if new_unit == "mm":
        document.getElementById("label-custom-height-unit").textContent = "(millimeters)"
        document.getElementById("label-custom-width-unit").textContent = "(millimeters)"
        document.getElementById("custom-size-height").step = "1"
        document.getElementById("custom-size-width").step = "1"
    else:
        document.getElementById("label-custom-height-unit").textContent = "(inches)"
        document.getElementById("label-custom-width-unit").textContent = "(inches)"
        document.getElementById("custom-size-height").step = "0.25"
        document.getElementById("custom-size-width").step = "0.25"

    # Re-render standard and custom sizes to show in new unit
    render_standard_sizes()
    render_custom_sizes()

@when("click", "#unit-inches")
def handle_unit_inches(event):
    switch_unit("inches")

@when("click", "#unit-mm")
def handle_unit_mm(event):
    switch_unit("mm")

# ===== Aspect Ratio Event Handlers =====

# SVG icons for lock button (Material Design icons)
# Unlocked: open shackle (lock_open)
LOCK_ICON_UNLOCKED = '<svg viewBox="0 0 24 24"><path d="M12 17c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm6-9h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6h1.9c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm0 12H6V10h12v10z"/></svg>'
# Locked: closed shackle (lock)
LOCK_ICON_LOCKED = '<svg viewBox="0 0 24 24"><path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/></svg>'

@when("click", "#aspect-lock")
def handle_aspect_lock(event):
    """Toggle aspect ratio lock."""
    lock_btn = document.getElementById("aspect-lock")
    if aspect_lock_state["locked"]:
        # Unlock
        aspect_lock_state["locked"] = False
        aspect_lock_state["ratio"] = None
        lock_btn.innerHTML = LOCK_ICON_UNLOCKED
        lock_btn.className = "aspect-lock-btn unlocked"
        lock_btn.title = "Lock aspect ratio"
    else:
        # Lock current ratio
        height = float(document.getElementById("artwork-height").value)
        width = float(document.getElementById("artwork-width").value)
        if width > 0:
            aspect_lock_state["locked"] = True
            aspect_lock_state["ratio"] = height / width
            lock_btn.innerHTML = LOCK_ICON_LOCKED
            lock_btn.className = "aspect-lock-btn locked"
            lock_btn.title = "Unlock aspect ratio"
    lock_btn.blur()  # Remove focus to clear highlight on mobile
    update_aspect_ratio_display()

@when("input", "#artwork-height")
def handle_height_change(event):
    """Handle height input change - update width if locked."""
    update_aspect_ratio_display()
    # Only update orientation if NOT locked (locked orientation is user-controlled via toggle button)
    if not aspect_lock_state["locked"]:
        update_orientation_icon()
    if aspect_lock_state["locked"] and aspect_lock_state["ratio"]:
        height_val = document.getElementById("artwork-height").value
        console.log(f"Height changed: '{height_val}', type: {type(height_val)}, bool: {bool(height_val)}")
        if not height_val or not str(height_val).strip():
            # If height is cleared, clear width too
            console.log("Height cleared, clearing width")
            document.getElementById("artwork-width").value = ""
        else:
            try:
                height = float(height_val)
                console.log(f"Parsed height: {height}, ratio: {aspect_lock_state['ratio']}")
                if height > 0:  # Only update for positive values
                    new_width = height / aspect_lock_state["ratio"]
                    # Round to step
                    step = float(document.getElementById("artwork-width").step)
                    new_width = round_to_step(new_width, step)
                    console.log(f"Setting width to: {new_width}")
                    # Format: use int if whole number, float otherwise
                    if abs(new_width - round(new_width)) < 0.001:
                        document.getElementById("artwork-width").value = str(int(round(new_width)))
                    else:
                        document.getElementById("artwork-width").value = str(new_width)
            except (ValueError, TypeError) as e:
                console.log(f"Error parsing height: {e}")
                pass  # Ignore invalid input

@when("input", "#artwork-width")
def handle_width_change(event):
    """Handle width input change - update height if locked."""
    update_aspect_ratio_display()
    # Only update orientation if NOT locked (locked orientation is user-controlled via toggle button)
    if not aspect_lock_state["locked"]:
        update_orientation_icon()
    if aspect_lock_state["locked"] and aspect_lock_state["ratio"]:
        width_val = document.getElementById("artwork-width").value
        if not width_val or not str(width_val).strip():
            # If width is cleared, clear height too
            console.log("Width cleared, clearing height")
            document.getElementById("artwork-height").value = ""
        else:
            try:
                width = float(width_val)
                if width > 0:  # Only update for positive values
                    new_height = width * aspect_lock_state["ratio"]
                    # Round to step
                    step = float(document.getElementById("artwork-height").step)
                    new_height = round_to_step(new_height, step)
                    # Format: use int if whole number, float otherwise
                    if abs(new_height - round(new_height)) < 0.001:
                        document.getElementById("artwork-height").value = str(int(round(new_height)))
                    else:
                        document.getElementById("artwork-height").value = str(new_height)
            except (ValueError, TypeError):
                pass  # Ignore invalid input

# ===== Orientation Toggle =====

def update_orientation_icon():
    """Update the orientation icon based on current dimensions."""
    try:
        height_val = document.getElementById("artwork-height").value
        width_val = document.getElementById("artwork-width").value
        if not height_val or not width_val:
            return  # Skip update if either field is empty
        height = float(height_val)
        width = float(width_val)
        btn = document.getElementById("orientation-toggle")
        if width > height:
            btn.className = "orientation-btn landscape"
        else:
            btn.className = "orientation-btn portrait"
    except (ValueError, TypeError):
        pass  # Ignore invalid input

@when("click", "#orientation-toggle")
def handle_orientation_toggle(event):
    """Swap height and width values."""
    height_input = document.getElementById("artwork-height")
    width_input = document.getElementById("artwork-width")
    # Swap values
    old_height = height_input.value
    height_input.value = width_input.value
    width_input.value = old_height

    # If ratio is locked, invert the locked ratio to match new orientation
    if aspect_lock_state["locked"] and aspect_lock_state["ratio"]:
        aspect_lock_state["ratio"] = 1 / aspect_lock_state["ratio"]
        console.log(f"Inverted locked ratio to: {aspect_lock_state['ratio']:.3f}")

    # Update icon
    update_orientation_icon()
    # Update aspect ratio display
    update_aspect_ratio_display()
    # Trigger save and re-render
    save_current_settings()
    try:
        calculate_frame()
        render_visualization()
    except Exception as e:
        console.log(f"Auto-update after orientation swap: {e}")

# ===== Mat Toggle =====

@when("change", "#include-mat")
def handle_mat_toggle(event):
    """Toggle mat inclusion - enable/disable mat width input."""
    include_mat = document.getElementById("include-mat").checked
    mat_input = document.getElementById("mat-width")
    if include_mat:
        mat_input.disabled = False
        # Restore a sensible default if it was 0
        if float(mat_input.value) == 0:
            mat_input.value = "2.0"
    else:
        mat_input.disabled = True

# ===== Settings Management (localStorage) =====

def get_default_settings():
    """Return default settings for all form fields (from defaults.py)."""
    return {
        "artwork_height": str(DEFAULT_ARTWORK_HEIGHT),
        "artwork_width": str(DEFAULT_ARTWORK_WIDTH),
        "include_mat": DEFAULT_INCLUDE_MAT,
        "mat_width": str(DEFAULT_MAT_WIDTH),
        "frame_width": str(DEFAULT_FRAME_MATERIAL_WIDTH),
        "glazing_thickness": str(DEFAULT_GLAZING_THICKNESS),
        "matboard_thickness": str(DEFAULT_MATBOARD_THICKNESS),
        "artwork_thickness": str(DEFAULT_ARTWORK_THICKNESS),
        "backing_thickness": str(DEFAULT_BACKING_THICKNESS),
        "rabbet_depth": str(DEFAULT_RABBET_DEPTH),
        "frame_depth": str(DEFAULT_FRAME_THICKNESS),
        "blade_width": str(DEFAULT_BLADE_WIDTH),
    }

def save_current_settings():
    """Save all current form field values to localStorage."""
    try:
        settings = {
            "artwork_height": document.getElementById("artwork-height").value,
            "artwork_width": document.getElementById("artwork-width").value,
            "include_mat": document.getElementById("include-mat").checked,
            "mat_width": document.getElementById("mat-width").value,
            "frame_width": document.getElementById("frame-width").value,
            "glazing_thickness": document.getElementById("glazing-thickness").value,
            "matboard_thickness": document.getElementById("matboard-thickness").value,
            "artwork_thickness": document.getElementById("artwork-thickness").value,
            "backing_thickness": document.getElementById("backing-thickness").value,
            "rabbet_depth": document.getElementById("rabbet-depth").value,
            "frame_depth": document.getElementById("frame-depth").value,
            "blade_width": document.getElementById("blade-width").value
        }
        settings_json = json.dumps(settings)
        localStorage.setItem("frame_designer_settings", settings_json)
        console.log(f"💾 Settings saved to localStorage: {settings_json}")
    except Exception as e:
        console.error(f"Error saving settings: {e}")

def restore_settings():
    """Restore form field values from localStorage."""
    try:
        settings_json = localStorage.getItem("frame_designer_settings")
        console.log(f"Attempting to restore settings: {settings_json}")
        if settings_json:
            settings = json.loads(settings_json)
            console.log(f"Parsed settings: {settings}")
            document.getElementById("artwork-height").value = settings.get("artwork_height", "10")
            document.getElementById("artwork-width").value = settings.get("artwork_width", "8")
            # Restore mat toggle state
            include_mat = settings.get("include_mat", True)
            document.getElementById("include-mat").checked = include_mat
            mat_input = document.getElementById("mat-width")
            mat_input.value = settings.get("mat_width", "2")
            mat_input.disabled = not include_mat
            document.getElementById("frame-width").value = settings.get("frame_width", "1.5")
            document.getElementById("glazing-thickness").value = settings.get("glazing_thickness", "0.093")
            document.getElementById("matboard-thickness").value = settings.get("matboard_thickness", "0.055")
            document.getElementById("artwork-thickness").value = settings.get("artwork_thickness", "0.008")
            document.getElementById("backing-thickness").value = settings.get("backing_thickness", "0.125")
            document.getElementById("rabbet-depth").value = settings.get("rabbet_depth", "0.375")
            document.getElementById("frame-depth").value = settings.get("frame_depth", "0.75")
            document.getElementById("blade-width").value = settings.get("blade_width", "0.125")
            console.log("✅ Settings restored from localStorage")
            return True
        else:
            console.log("No saved settings found in localStorage")
        return False
    except Exception as e:
        console.error(f"Error restoring settings: {e}")
        return False

def reset_to_defaults():
    """Reset all form fields to default values, respecting current unit mode."""
    defaults = get_default_settings()
    current_unit = get_current_unit()

    # Convert defaults to display unit if in mm mode
    # Defaults are stored in inches
    def to_display(val_str):
        val = float(val_str)
        if current_unit == "mm":
            return str(round(inches_to_mm(val), 2))
        return val_str

    document.getElementById("artwork-height").value = to_display(defaults["artwork_height"])
    document.getElementById("artwork-width").value = to_display(defaults["artwork_width"])
    # Reset mat toggle
    document.getElementById("include-mat").checked = defaults["include_mat"]
    mat_input = document.getElementById("mat-width")
    mat_input.value = to_display(defaults["mat_width"])
    mat_input.disabled = not defaults["include_mat"]
    document.getElementById("frame-width").value = to_display(defaults["frame_width"])
    document.getElementById("glazing-thickness").value = to_display(defaults["glazing_thickness"])
    document.getElementById("matboard-thickness").value = to_display(defaults["matboard_thickness"])
    document.getElementById("artwork-thickness").value = to_display(defaults["artwork_thickness"])
    document.getElementById("backing-thickness").value = to_display(defaults["backing_thickness"])
    document.getElementById("rabbet-depth").value = to_display(defaults["rabbet_depth"])
    document.getElementById("frame-depth").value = to_display(defaults["frame_depth"])
    document.getElementById("blade-width").value = to_display(defaults["blade_width"])
    # Clear from localStorage
    localStorage.removeItem("frame_designer_settings")
    console.log("Reset to default settings")

# Auto-save settings when any form field changes
def setup_auto_save():
    """Attach change listeners to all form fields to auto-save settings and auto-update visualization."""
    from pyodide.ffi import create_proxy

    def save_and_update_handler(event):
        """Handler that saves settings, updates calculations, and re-renders visualization."""
        save_current_settings()
        # Auto-update calculations (Streamlit-like behavior)
        try:
            calculate_frame()
        except Exception as e:
            console.log(f"Auto-calculate skipped: {e}")
        # Auto-update visualization
        try:
            render_visualization()
        except Exception as e:
            console.log(f"Auto-render skipped: {e}")

    field_ids = [
        "artwork-height", "artwork-width", "include-mat", "mat-width", "frame-width",
        "glazing-thickness", "matboard-thickness", "artwork-thickness",
        "backing-thickness", "rabbet-depth", "frame-depth", "blade-width"
    ]

    # Create a proxy that persists beyond the function call
    handler_proxy = create_proxy(save_and_update_handler)
    for field_id in field_ids:
        element = document.getElementById(field_id)
        if element:
            element.addEventListener("change", handler_proxy)
            console.log(f"Added auto-save and auto-render listener to {field_id}")

# Event handler: Reset to Defaults button
@when("click", "#reset-settings")
def reset_settings_handler(event):
    """Reset all settings to defaults."""
    reset_to_defaults()
    console.log("Settings reset to defaults")
    # Update UI indicators
    update_orientation_icon()
    update_aspect_ratio_display()
    # Auto-update calculations and visualization with default values
    try:
        calculate_frame()
    except Exception as e:
        console.log(f"Auto-calculate after reset skipped: {e}")
    try:
        render_visualization()
    except Exception as e:
        console.log(f"Auto-render after reset skipped: {e}")

# ===== Export Functions =====

def generate_shareable_url():
    """Generate a compact shareable URL encoding all frame settings.

    Wrapper for shareable_url module function.
    """
    current_unit = get_current_unit()
    include_mat = document.getElementById("include-mat").checked
    return generate_url(document, current_unit, include_mat)


@when("click", "#export-text")
def handle_export_text(event):
    """Export frame design as text file."""
    try:
        current_unit = get_current_unit()
        summary = generate_text_summary(document, current_unit)
        download_file(summary, "frame_design_summary.txt", "text/plain")
        status_div = document.getElementById("export-status")
        status_div.innerHTML = '<span class="success">✅ Text summary downloaded!</span>'
    except Exception as e:
        status_div = document.getElementById("export-status")
        status_div.innerHTML = f'<span class="warning">❌ Export error: {e}</span>'
        console.log(f"Export error: {e}")

def generate_pdf_content(pdf, start_y=20):
    """Add formatted content to PDF. Wrapper for export_pdf module function."""
    current_unit = get_current_unit()
    return generate_pdf_content_impl(pdf, document, current_unit, start_y)

def add_qr_code_to_pdf(pdf, url, status_div):
    """Add QR code to bottom-right corner of PDF. Wrapper for export_pdf module function."""
    add_qr_code_to_pdf_impl(pdf, url, status_div, console)

@when("click", "#export-pdf")
def handle_export_pdf(event):
    """Export frame design as PDF with vector diagram, details, and QR code."""
    current_unit = get_current_unit()
    shareable_url = generate_shareable_url()
    export_pdf_impl(document, current_unit, shareable_url, console)

# ===== Named Configurations Management =====

def get_current_config():
    """Get current form values as a configuration object. Wrapper for config_manager module."""
    return get_config(document)

def load_saved_configs():
    """Load saved configurations from localStorage. Wrapper for config_manager module."""
    return load_configs(localStorage, console)

def save_config_to_storage(name, config):
    """Save a named configuration to localStorage. Wrapper for config_manager module."""
    return save_config(name, config, localStorage, console, load_saved_configs)

def delete_config(name):
    """Delete a named configuration from localStorage. Wrapper for config_manager module."""
    del_config(name, localStorage, console, load_saved_configs, render_saved_configs)

def load_config(config):
    """Load a configuration into the form fields. Wrapper for config_manager module."""
    apply_config(config, document, save_current_settings, render_visualization, console)

def render_saved_configs():
    """Render the list of saved configurations. Wrapper for config_manager module."""
    render_configs(document, load_saved_configs, load_config, delete_config)

# Event handler: Save Configuration button
@when("click", "#save-config")
def save_config_handler(event):
    """Save the current configuration with a name."""
    save_config_handler_impl(event, document, console, get_current_config,
                             save_config_to_storage, render_saved_configs)

# ===== Data Export/Import Functions =====

def export_all_data():
    """Export all localStorage data as JSON file. Wrapper for data_backup module."""
    export_data_impl(document, localStorage, console, get_current_config, load_saved_configs)

def import_data(file_content, merge_mode):
    """Import localStorage data from JSON. Wrapper for data_backup module."""
    return import_data_impl(
        file_content, merge_mode, localStorage, console,
        load_saved_configs, render_saved_configs,
        render_custom_sizes, restore_settings,
        calculate_frame, render_visualization
    )

def handle_file_upload(event):
    """Handle file input change event. Wrapper for data_backup module."""
    file_upload_impl(event, console, show_import_dialog)

def show_import_dialog(file_content):
    """Show dialog asking user to merge or replace. Wrapper for data_backup module."""
    show_dialog_impl(file_content, document, import_data)


# Event handler: Add Custom Size button
@when("click", "#add-custom-size")
def add_custom_size_handler(event):
    """Add a new custom size to localStorage."""
    try:
        # Get form values
        name_input = document.getElementById("custom-size-name")
        height_input = document.getElementById("custom-size-height")
        width_input = document.getElementById("custom-size-width")
        msg_div = document.getElementById("add-size-message")

        name = name_input.value.strip()
        if not name:
            msg_div.innerHTML = '<div class="warning">⚠️ Please enter a name for the size</div>'
            return

        current_unit = get_current_unit()

        # Get values and convert to inches for storage
        height_value = float(height_input.value)
        width_value = float(width_input.value)

        # Convert to inches (storage format)
        if current_unit == "mm":
            height_inches = mm_to_inches(height_value)
            width_inches = mm_to_inches(width_value)
        else:
            height_inches = height_value
            width_inches = width_value

        # Check for duplicate name (prompt to override)
        existing_size_index = None
        for idx, size in enumerate(app_state["custom_sizes"]):
            if size.name.lower() == name.lower():
                existing_size_index = idx
                break

        if existing_size_index is not None:
            # Ask user if they want to override
            if not confirm(f'A size named "{name}" already exists. Do you want to override it with the new dimensions?'):
                msg_div.innerHTML = f'<div class="warning">⚠️ Cancelled - size "{name}" was not modified.</div>'
                return
            # Remove the old one (will be replaced below)
            app_state["custom_sizes"].pop(existing_size_index)

        # Check for duplicate dimensions (warning but allow)
        duplicate_dims = None
        for size in app_state["custom_sizes"]:
            if abs(size.height - height_inches) < 0.01 and abs(size.width - width_inches) < 0.01:
                duplicate_dims = size.name
                break

        # Add new size
        new_size = FrameSize(name=name, height=height_inches, width=width_inches)
        app_state["custom_sizes"].append(new_size)

        # Save to localStorage
        save_custom_sizes()

        # Re-render the list
        render_custom_sizes()

        # Show success message (with warning if duplicate dimensions or override)
        if existing_size_index is not None:
            msg_div.innerHTML = f'<div class="success">✅ Updated: {name} (overridden with new dimensions)</div>'
        elif duplicate_dims:
            msg_div.innerHTML = f'<div class="warning">✅ Saved: {name}<br>⚠️ Note: These dimensions match existing size "{duplicate_dims}"</div>'
        else:
            msg_div.innerHTML = f'<div class="success">✅ Saved: {name}</div>'

        # Clear form
        name_input.value = ""
        height_input.value = "18.0"
        width_input.value = "24.0"

        console.log(f"Added custom size: {name}")

        # Clear message after 3 seconds
        import asyncio
        async def clear_message():
            await asyncio.sleep(3)
            msg_div.innerHTML = ""
        asyncio.create_task(clear_message())

    except ValueError:
        msg_div = document.getElementById("add-size-message")
        msg_div.innerHTML = '<div class="warning">⚠️ Please enter valid numbers for dimensions</div>'
    except Exception as e:
        msg_div = document.getElementById("add-size-message")
        msg_div.innerHTML = f'<div class="warning">❌ Error: {e}</div>'
        console.log(f"Error adding custom size: {e}")

def calculate_frame(event=None):
    """Calculate frame dimensions and display results."""
    try:
        current_unit = get_current_unit()
        values = get_form_values_as_inches(document, current_unit)
        if values is None:
            console.log("Skipping calculation - required fields empty")
            return

        design = create_frame_design_from_values(values)

        # Extract values for display
        height = values["artwork_height"]
        width = values["artwork_width"]
        mat_width = values["mat_width"]
        rabbet_depth = values["rabbet_depth"]
        frame_depth = values["frame_depth"]
        blade_width = values["blade_width"]

        # Get calculations
        frame_inside = design.get_frame_inside_dimensions()
        frame_outside = design.get_frame_outside_dimensions()
        cut_list = design.get_cut_list()
        required_depth = design.get_rabbet_z_depth_required()
        total_wood_length = design.get_total_wood_length(saw_margin=blade_width)

        # Matboard dimensions (if using mat)
        matboard_dims = design.get_matboard_dimensions() if design.has_mat else None
        mat_opening = design.get_mat_opening_dimensions() if design.has_mat else None

        # Format results using selected unit
        results_html = ''

        # === Artwork Dimensions ===
        results_html += '<strong>📐 Artwork Dimensions:</strong>'
        results_html += f'<ul style="margin: 4px 0 12px 0; padding-left: 20px;"><li>{format_value(height, current_unit)} H × {format_value(width, current_unit)} W</li></ul>'

        # === Cut List ===
        results_html += '<strong>🪚 Cut List:</strong>'
        results_html += '<ul style="margin: 4px 0 12px 0; padding-left: 20px;">'
        for category, pieces in cut_list.items():
            category_name = "Top & Bottom" if "horizontal" in category else "Left & Right"
            for piece_spec in pieces:
                qty = piece_spec.get('quantity', 1)
                inside = piece_spec.get('inside_length', 0)
                outside = piece_spec.get('outside_length', 0)
                results_html += f'<li>{category_name}: {qty}× '
                results_html += f'<strong style="color: #ffa500;">{format_value(outside, current_unit)}</strong> '
                results_html += f'<span style="color: #888;">(inside edge: {format_value(inside, current_unit)})</span></li>'
        results_html += '</ul>'

        # === Total Wood Length ===
        results_html += '<strong>📦 Material Requirements:</strong>'
        results_html += '<ul style="margin: 4px 0 12px 0; padding-left: 20px;">'
        results_html += f'<li>Total Wood Length: <strong style="color: #3b9cff;">{format_value(total_wood_length, current_unit)}</strong>'
        results_html += f'<br><small style="color: #888;">Includes saw blade kerf ({format_value(blade_width, current_unit)}) and error margin (1/16") per piece</small></li>'
        results_html += '</ul>'

        # === Frame Dimensions ===
        results_html += '<strong>🖼️ Frame Dimensions:</strong>'
        results_html += '<ul style="margin: 4px 0 12px 0; padding-left: 20px;">'
        results_html += f'<li>Inside: {format_value(frame_inside[0], current_unit)} H × {format_value(frame_inside[1], current_unit)} W</li>'
        results_html += f'<li>Outside: {format_value(frame_outside[0], current_unit)} H × {format_value(frame_outside[1], current_unit)} W</li>'
        results_html += '</ul>'

        # === Matboard Dimensions (if applicable) ===
        if design.has_mat and matboard_dims and mat_opening:
            mat_cut_width = mat_width + rabbet_depth
            results_html += '<strong>🎨 Matboard Details:</strong>'
            results_html += '<ul style="margin: 4px 0 12px 0; padding-left: 20px;">'
            results_html += f'<li>Matboard Size: {format_value(matboard_dims[0], current_unit)} H × {format_value(matboard_dims[1], current_unit)} W</li>'
            results_html += f'<li>Mat Opening: {format_value(mat_opening[0], current_unit)} H × {format_value(mat_opening[1], current_unit)} W</li>'
            results_html += f'<li>Visual Mat Width: {format_value(mat_width, current_unit)}</li>'
            results_html += f'<li>Mat Border Cut Width: {format_value(mat_cut_width, current_unit)} <small style="color: #888;">(visual + {format_value(rabbet_depth, current_unit)} rabbet)</small></li>'
            results_html += '</ul>'

        # === Depth Requirements with Warning ===
        results_html += '<strong>📏 Depth Requirements (Z-axis):</strong>'
        results_html += '<ul style="margin: 4px 0 12px 0; padding-left: 20px;">'
        results_html += f'<li>Required: {format_value(required_depth, current_unit)}'
        results_html += '<br><small style="color: #888;">(glazing + matboard + artwork + backing + assembly margin)</small></li>'
        results_html += f'<li>Available: {format_value(frame_depth, current_unit)}</li>'
        # Depth warning/success
        if required_depth > frame_depth:
            shortfall = required_depth - frame_depth
            results_html += f'<li><span style="color: var(--rf-error-red); font-weight: bold;">⚠️ WARNING: Frame is {format_value(shortfall, current_unit)} too shallow!</span>'
            results_html += '<br><small style="color: var(--rf-error-red);">Materials will not fit in the rabbet. Use deeper frame stock.</small></li>'
        else:
            clearance = frame_depth - required_depth
            results_html += f'<li><span style="color: var(--rf-success-green);">✅ Adequate depth ({format_value(clearance, current_unit)} clearance)</span></li>'
        results_html += '</ul>'

        # Display results
        results_div = document.getElementById("results")
        results_div.innerHTML = results_html

    except Exception as e:
        results_div = document.getElementById("results")
        results_div.innerHTML = f'<span class="warning">❌ Error: {e}</span>'

# Button click handler for manual calculate (if button exists)
@when("click", "#calculate")
def calculate_frame_click(event):
    """Manual calculate button handler."""
    calculate_frame()

# Event handler: Test localStorage
@when("click", "#test-storage")
def test_local_storage(event):
    """Demonstrate NATIVE localStorage access (no workarounds!)."""
    try:
        # Save custom size to localStorage
        custom_sizes = [
            {"name": "Test Canvas", "height": 12.5, "width": 18.75},
            {"name": "Test Print", "height": 8.0, "width": 10.0}
        ]

        # Save to localStorage (DIRECT ACCESS!)
        localStorage.setItem(
            "frame_designer_custom_sizes",
            json.dumps(custom_sizes)
        )

        # Load back from localStorage
        loaded_data = localStorage.getItem("frame_designer_custom_sizes")
        loaded_sizes = json.loads(loaded_data)

        results_html = f"""
        <div class="success">✅ localStorage Test Successful!</div>
        <br>
        <strong>Saved Data:</strong><br>
        {json.dumps(custom_sizes, indent=2)}<br>
        <br>
        <strong>Loaded Data:</strong><br>
        {json.dumps(loaded_sizes, indent=2)}<br>
        <br>
        <span class="success">⭐ NO WORKAROUNDS NEEDED - Native browser API!</span>
        """

        results_div = document.getElementById("results")
        results_div.innerHTML = f'<pre>{results_html}</pre>'

    except Exception as e:
        results_div = document.getElementById("results")
        results_div.innerHTML = f'<span class="warning">❌ Error: {e}</span>'

def render_visualization():
    """Render the frame visualization with dimension annotations and transparency overlaps."""
    try:
        import matplotlib.pyplot as plt
        import matplotlib.patches as mpatches

        current_unit = get_current_unit()
        values = get_form_values_as_inches(document, current_unit)
        if values is None:
            return

        design = create_frame_design_from_values(values)

        # Extract values for visualization
        artwork_height = values["artwork_height"]
        artwork_width = values["artwork_width"]
        mat_width = values["mat_width"]
        frame_width = values["frame_width"]
        rabbet = values["rabbet_depth"]

        # ==== 1. Get all dimensions ====
        frame_outer_h, frame_outer_w = design.get_frame_outside_dimensions()
        inside_h, inside_w = design.get_frame_inside_dimensions()
        matboard_h, matboard_w = design.get_matboard_dimensions()
        mat_inner_h, mat_inner_w = design.get_mat_opening_dimensions()
        mat_outer_h, mat_outer_w = design.get_visible_dimensions()

        # Debug: log dimensions to verify proportions
        console.log(f"=== Visualization Debug ===")
        console.log(f"Artwork: {artwork_height} x {artwork_width}")
        console.log(f"Frame width (input): {frame_width}")
        console.log(f"Mat width: {mat_width}")
        console.log(f"Frame outer: {frame_outer_h} x {frame_outer_w}")
        console.log(f"Visible/inside: {inside_h} x {inside_w}")
        console.log(f"Expected frame_outer = inside + 2*frame_width: {inside_h + 2*frame_width} x {inside_w + 2*frame_width}")
        console.log(f"Ratio frame_width/frame_outer_w: {frame_width/frame_outer_w:.4f} (should show as this fraction of total width)")

        # ==== 2. Create figure ====
        # For SVG output, figsize determines aspect ratio; DPI is ignored
        figsize = (6, 6)
        fig, ax = plt.subplots(figsize=figsize)

        # ==== 3. Draw Frame (outermost rectangle) ====
        frame_x, frame_y = 0, 0
        ax.add_patch(mpatches.Rectangle(
            (frame_x, frame_y),
            frame_outer_w,
            frame_outer_h,
            facecolor='#8B6F47',
            edgecolor=None,
            linewidth=2,
            label='Frame'
        ))

        # ==== 4. Draw Matboard/Artwork (content inside frame) ====
        #
        # VISUALIZATION GEOMETRY (important for correct proportions):
        # ─────────────────────────────────────────────────────────────
        # The frame_width measurement INCLUDES the rabbet depth. For example:
        #   - frame_width = 1" (total frame material width)
        #   - rabbet_depth = 0.375" (the lip that overlaps content)
        #   - visible frame face = 0.625" (the part you see from the front)
        #
        # Key positions (from frame outer edge):
        #   - frame outer edge: 0
        #   - content_x/y: frame_width - rabbet (where content starts, UNDER frame lip)
        #   - visible_x/y: frame_width (the visible opening - where you see through)
        #
        # The content (matboard/artwork) is drawn starting at content_x/y, extending
        # UNDER the frame's rabbet lip. The rabbet overlay shows the frame covering
        # the content edges - this overlay is WITHIN the frame_width area, not beyond it.
        #
        # This ensures the visual frame width exactly matches the specified frame_width,
        # with the rabbet shown as a semi-transparent overlap within that width.
        #
        visible_x = frame_x + frame_width
        visible_y = frame_y + frame_width
        content_x = visible_x - rabbet  # Content starts under the frame lip
        content_y = visible_y - rabbet

        if mat_width > 0:
            # Matboard extends under the frame's rabbet lip
            ax.add_patch(mpatches.Rectangle(
                (content_x, content_y),
                mat_outer_w + 2 * rabbet,  # Extended to go under frame on both sides
                mat_outer_h + 2 * rabbet,
                facecolor='#FEFEFE',
                edgecolor=None,
                linewidth=1,
                label='Matboard'
            ))

            # === Frame-Matboard overlap (rabbet) - show frame lip over matboard ===
            # Draw semi-transparent overlay from content edge to visible opening
            # This is WITHIN the frame_width area, not extending beyond it
            overlap_alpha = 0.7

            # Bottom rabbet overlap (frame lip covering bottom edge of matboard)
            ax.add_patch(mpatches.Rectangle(
                (content_x, content_y),
                mat_outer_w + 2 * rabbet,
                rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Top rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (content_x, visible_y + mat_outer_h),
                mat_outer_w + 2 * rabbet,
                rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Left rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (content_x, content_y + rabbet),
                rabbet,
                mat_outer_h,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Right rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (visible_x + mat_outer_w, content_y + rabbet),
                rabbet,
                mat_outer_h,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))

            # === Matboard window (white opening) ===
            mat_window_x = visible_x + mat_width
            mat_window_y = visible_y + mat_width
            ax.add_patch(mpatches.Rectangle(
                (mat_window_x, mat_window_y),
                mat_inner_w,
                mat_inner_h,
                facecolor='white',
                edgecolor='#DDDDDD',
                linewidth=0.75
            ))

            # === Artwork (sits under mat window) ===
            art_x = mat_window_x - design.mat_overlap
            art_y = mat_window_y - design.mat_overlap
            ax.add_patch(mpatches.Rectangle(
                (art_x, art_y),
                artwork_width,
                artwork_height,
                facecolor='#5B9BD5',
                edgecolor=None,
                linewidth=1,
                label='Artwork'
            ))

            # === Mat-Artwork overlap - TRANSPARENCY ===
            if design.mat_overlap > 0:
                # Top overlap
                ax.add_patch(mpatches.Rectangle(
                    (art_x, art_y),
                    artwork_width,
                    design.mat_overlap,
                    facecolor='#FEFEFE',
                    edgecolor=None,
                    alpha=0.8
                ))
                # Bottom overlap
                ax.add_patch(mpatches.Rectangle(
                    (art_x, art_y + artwork_height - design.mat_overlap),
                    artwork_width,
                    design.mat_overlap,
                    facecolor='#FEFEFE',
                    edgecolor=None,
                    alpha=0.8
                ))
                # Left overlap
                ax.add_patch(mpatches.Rectangle(
                    (art_x, art_y + design.mat_overlap),
                    design.mat_overlap,
                    artwork_height - 2 * design.mat_overlap,
                    facecolor='#FEFEFE',
                    edgecolor=None,
                    alpha=0.8
                ))
                # Right overlap
                ax.add_patch(mpatches.Rectangle(
                    (art_x + artwork_width - design.mat_overlap, art_y + design.mat_overlap),
                    design.mat_overlap,
                    artwork_height - 2 * design.mat_overlap,
                    facecolor='#FEFEFE',
                    edgecolor=None,
                    alpha=0.8
                ))
        else:
            # No mat - draw artwork directly inside frame
            # Artwork is positioned to extend under the frame's rabbet lip on all sides
            # The frame's inside opening is smaller than the artwork by rabbet on all sides
            art_x = visible_x - rabbet  # Artwork extends beyond visible opening
            art_y = visible_y - rabbet
            ax.add_patch(mpatches.Rectangle(
                (art_x, art_y),
                artwork_width,  # Actual artwork dimensions
                artwork_height,
                facecolor='#5B9BD5',
                edgecolor=None,
                linewidth=1,
                label='Artwork'
            ))

            # === Frame-Artwork overlap (rabbet) - show frame lip over artwork ===
            overlap_alpha = 0.7

            # Bottom rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (art_x, art_y),
                artwork_width,
                rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Top rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (art_x, art_y + artwork_height - rabbet),
                artwork_width,
                rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Left rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (art_x, art_y + rabbet),
                rabbet,
                artwork_height - 2 * rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))
            # Right rabbet overlap
            ax.add_patch(mpatches.Rectangle(
                (art_x + artwork_width - rabbet, art_y + rabbet),
                rabbet,
                artwork_height - 2 * rabbet,
                facecolor='#8B6F47',
                edgecolor=None,
                alpha=overlap_alpha
            ))

        # ==== 5. Set axes limits with margin ====
        # Responsive margin calculation based on figure size (matching Streamlit approach)
        fig_width_inches = figsize[0]
        if fig_width_inches <= 6:
            margin_factor = 0.15  # Larger margins for small figures
        elif fig_width_inches <= 8:
            margin_factor = 0.12
        else:
            margin_factor = 0.10

        margin = max(frame_outer_w, frame_outer_h) * margin_factor

        # Center the frame by making margins equal on all sides
        ax.set_xlim(-margin, frame_outer_w + margin)
        ax.set_ylim(-margin, frame_outer_h + margin)
        ax.set_aspect('equal')

        # ==== 6. Add dimension annotations (blueprint style) ====
        # Font sizing based on figure size (not data dimensions) for consistency
        # This keeps text size constant regardless of frame thickness
        base_font_size = min(figsize[0], figsize[1])  # figsize is (6, 6)
        font_size = max(8, int(base_font_size * 1.1))

        # Define shared y-positions for consolidated labels at fixed distances from frame
        # This ensures consistent spacing regardless of frame dimensions
        top_label_y = frame_outer_h + margin * 0.35  # Shared y for Outside and Frame
        bottom_label_y = frame_y - margin * 0.35      # Shared y for Inside and Mat

        # Prepare label texts
        outside_text = f"Outside: {format_value(frame_outer_h, current_unit, 2)} × {format_value(frame_outer_w, current_unit, 2)}"
        frame_width_text = f"Frame: {format_value(frame_width, current_unit, 2)}"

        # Approximate character width in data units for label width estimation
        char_width_approx = frame_outer_w * 0.012

        # Horizontal offset from label center for arrow positioning
        ARROW_HORIZONTAL_OFFSET_FACTOR = 0.08
        horizontal_offset = frame_outer_w * ARROW_HORIZONTAL_OFFSET_FACTOR

        # Frame width annotation (top section)
        # Drafting-style dimension with arrows and callout
        # Position dimension line at consistent height relative to frame top
        frame_bracket_y = frame_outer_h + margin * 0.05  # Just above frame top edge

        # Draw dimension line with bidirectional arrow: |<--  -->|
        # Frame width measured from inner edge (visible opening) to outer edge
        #
        # TICK ALIGNMENT CALCULATION:
        # ─────────────────────────────────────────────────────────────
        # Matplotlib draws lines CENTERED on the specified coordinate.
        # To align a tick's EDGE with a boundary, offset by half the line width.
        #
        # Formula: tick_line_width = (lw_points / 72) * (data_range / figure_inches)
        #   - lw_points: line width in points (1 point = 1/72 inch)
        #   - data_range: total data units across the figure
        #   - figure_inches: physical figure width in inches
        #
        # For frame width tick (measuring from visible opening to outer edge):
        #   - LEFT tick: align LEFT edge with visible opening → offset by -tick_line_width/2
        #   - RIGHT tick: at frame outer edge (no offset needed, extends outward)
        #
        # For mat width tick (measuring from content edge to mat window):
        #   - BOTTOM tick: align TOP edge with content edge → offset by -tick_line_width/2
        #   - TOP tick: align BOTTOM edge with mat window → offset by +tick_line_width/2
        #
        tick_lw_points = 0.8
        data_width = frame_outer_w + 2 * margin
        physical_width = 6  # figsize width in inches
        tick_line_width = (tick_lw_points / 72) * (data_width / physical_width)
        # Position tick so its LEFT edge aligns with the visible opening
        # (line is centered on coordinate, so offset by half line width)
        frame_dim_left = visible_x + mat_outer_w - tick_line_width / 2  # Inner edge of frame
        frame_dim_right = frame_outer_w  # Outer edge of frame
        frame_dim_center = (frame_dim_left + frame_dim_right) / 2

        # Single continuous bidirectional arrow spanning full distance
        ax.annotate(
            '',
            xy=(frame_dim_right, frame_bracket_y),  # Right endpoint
            xytext=(frame_dim_left, frame_bracket_y),  # Left endpoint
            arrowprops=dict(arrowstyle='<->,head_length=0.3,head_width=0.15', lw=0.6, color='#D2691E', shrinkA=0, shrinkB=0, mutation_scale=5)
        )

        # Perpendicular tick marks at ends (asymmetric - shorter on top to make room for label)
        # Use frame-relative tick length (3% of frame height) instead of font_size
        tick_len = frame_outer_h * 0.03
        ax.plot([frame_dim_left, frame_dim_left], [frame_bracket_y - tick_len * 3, frame_bracket_y + tick_len],
               color='#D2691E', lw=0.8, linestyle='--', dashes=[3, 2], alpha=0.8)
        ax.plot([frame_dim_right, frame_dim_right], [frame_bracket_y - tick_len * 3, frame_bracket_y + tick_len],
               color='#D2691E', lw=0.8, linestyle='--', dashes=[3, 2], alpha=0.8)

        # Label positioning algorithm:
        # Default: Outside left-aligned, Frame right-aligned with frame edges
        # If they would overlap, shift apart to maintain minimum gap (may extend past frame)

        # Estimate label widths in data coordinates (inches)
        char_width_fraction = 0.005  # Each character is roughly 0.5% of frame width
        frame_label_half_width = len(frame_width_text) * char_width_fraction * frame_outer_w
        outside_label_half_width = len(outside_text) * char_width_fraction * frame_outer_w
        min_gap = 0.15  # minimum gap between labels in inches

        # Default positions: left-aligned and right-aligned with frame edges
        outside_x_default = outside_label_half_width + margin * 0.1
        frame_x_default = frame_outer_w - frame_label_half_width - margin * 0.1

        # Check for overlap
        outside_right_edge = outside_x_default + outside_label_half_width
        frame_left_edge = frame_x_default - frame_label_half_width
        overlap = outside_right_edge + min_gap - frame_left_edge

        if overlap > 0:
            # Labels would overlap - shift each outward by half the overlap
            shift = overlap / 2
            outside_x = outside_x_default - shift
            frame_label_x = frame_x_default + shift
        else:
            outside_x = outside_x_default
            frame_label_x = frame_x_default

        # Outside label - arrow points to top edge of frame
        outside_arrow_x = max(0, min(frame_outer_w, outside_x + horizontal_offset))
        ax.annotate(
            outside_text,
            xy=(outside_arrow_x, frame_outer_h),  # Arrow tip AT frame top edge
            xytext=(outside_x, top_label_y),  # Label position
            arrowprops=dict(arrowstyle='->', lw=0.7, color='black'),
            ha='center', va='center',
            fontsize=font_size,
            bbox=dict(boxstyle='round,pad=0.25', fc='#FFF3E0', ec='gray', alpha=0.85)
        )

        # Frame label
        ax.text(
            frame_label_x,
            top_label_y,  # Same Y as Outside label
            frame_width_text,
            ha='center', va='center',
            fontsize=font_size - 1,
            bbox=dict(boxstyle='round,pad=0.2', fc='#FFF3E0', ec='#D2691E', alpha=0.9, linewidth=1.5)
        )

        # Inside dimension (bottom, shifted right to make room for rabbet callout)
        inside_text = f"Inside: {format_value(inside_h, current_unit, 2)} × {format_value(inside_w, current_unit, 2)}"
        if mat_width > 0:
            # Mat cut width = visible width + rabbet (part hidden under frame)
            mat_cut_width = mat_width + rabbet
            mat_width_text = f"Mat: {format_value(mat_cut_width, current_unit, 2)} ({format_value(mat_width, current_unit, 2)} visible)"
        else:
            mat_width_text = "Mat: None"

        # Rabbet depth callout (bottom left, tick-style like frame width)
        # Use teal color (#009688) to distinguish from frame (orange) and mat (gray)
        rabbet_color = '#009688'
        rabbet_text = f"Rabbet: {format_value(rabbet, current_unit, 2)}"
        rabbet_bracket_y = frame_y - margin * 0.05  # Just below frame bottom edge

        # Rabbet dimension from content edge to visible opening
        rabbet_dim_left = content_x - tick_line_width / 2  # Left edge (content starts here)
        rabbet_dim_right = visible_x + tick_line_width / 2  # Right edge (visible opening)

        # Single bidirectional arrow (smaller head for narrow dimension)
        ax.annotate(
            '',
            xy=(rabbet_dim_right, rabbet_bracket_y),
            xytext=(rabbet_dim_left, rabbet_bracket_y),
            arrowprops=dict(arrowstyle='<->,head_length=0.3,head_width=0.15', lw=0.6, color=rabbet_color,
                           shrinkA=0, shrinkB=0, mutation_scale=5)
        )

        # Perpendicular tick marks at ends (shorter for compact appearance)
        # Use frame-relative tick height (2% of frame height)
        rabbet_tick_height = frame_outer_h * 0.02
        ax.plot([rabbet_dim_left, rabbet_dim_left], [rabbet_bracket_y - rabbet_tick_height * 0.3, rabbet_bracket_y + rabbet_tick_height],
               color=rabbet_color, lw=0.6, linestyle='--', dashes=[2, 1.5], alpha=0.8)
        ax.plot([rabbet_dim_right, rabbet_dim_right], [rabbet_bracket_y - rabbet_tick_height * 0.3, rabbet_bracket_y + rabbet_tick_height],
               color=rabbet_color, lw=0.6, linestyle='--', dashes=[2, 1.5], alpha=0.8)

        # Estimate label widths
        inside_label_width = len(inside_text) * char_width_approx
        mat_label_width = len(mat_width_text) * char_width_approx
        rabbet_label_width = len(rabbet_text) * char_width_approx
        rabbet_label_half_width = rabbet_label_width / 2
        inside_label_half_width = inside_label_width / 2

        # Bottom label positioning: same algorithm as top labels
        # Default: Rabbet left-aligned, Inside right-aligned with frame edges
        # If they would overlap, shift apart to maintain minimum gap (may extend past frame)
        rabbet_x_default = rabbet_label_half_width + margin * 0.1
        inside_x_default = frame_outer_w - inside_label_half_width - margin * 0.1

        # Check for overlap
        rabbet_right_edge = rabbet_x_default + rabbet_label_half_width
        inside_left_edge = inside_x_default - inside_label_half_width
        bottom_overlap = rabbet_right_edge + min_gap - inside_left_edge

        if bottom_overlap > 0:
            # Labels would overlap - shift each outward by half the overlap
            bottom_shift = bottom_overlap / 2
            rabbet_label_x = rabbet_x_default - bottom_shift
            inside_x = inside_x_default + bottom_shift
        else:
            rabbet_label_x = rabbet_x_default
            inside_x = inside_x_default

        # Rabbet label
        ax.text(
            rabbet_label_x,
            bottom_label_y,  # Same Y as other bottom labels
            rabbet_text,
            ha='center', va='center',
            fontsize=font_size - 1,
            bbox=dict(boxstyle='round,pad=0.2', fc='#E0F2F1', ec=rabbet_color, alpha=0.9, linewidth=1.5)
        )

        # Inside label - arrow points to visible opening edge (at visible_y)
        inside_edge_y = visible_y  # The visible opening is at frame_width from the edge
        inside_arrow_x = max(0, min(frame_outer_w, inside_x + horizontal_offset))  # Arrow points RIGHT
        ax.annotate(
            inside_text,
            xy=(inside_arrow_x, inside_edge_y),  # Arrow tip AT visible opening edge
            xytext=(inside_x, bottom_label_y),  # Label position
            arrowprops=dict(arrowstyle='->', lw=0.7, color='black'),
            ha='center', va='center',
            fontsize=font_size,
            bbox=dict(boxstyle='round,pad=0.25', fc='#E3F2FD', ec='gray', alpha=0.85)
        )

        # Mat width annotation (on right side, opposite from Inside label)
        if mat_width > 0:
            # Drafting-style dimension with arrows and callout (vertical)
            # Position dimension line on the RIGHT side of the mat
            mat_bracket_x = visible_x + mat_outer_w - mat_cut_width * 0.3  # Position bracket partway into mat from right

            # Draw vertical dimension line with bidirectional arrow
            # Measure full mat cut width (from content edge under rabbet to mat window)
            # Position tick so its TOP edge aligns with content edge (under rabbet)
            mat_dim_bottom = content_y - tick_line_width / 2  # Outer edge (under frame rabbet)
            # Position tick so its BOTTOM edge aligns with mat window opening
            mat_dim_top = visible_y + mat_width + tick_line_width / 2  # Inner edge (mat window)
            mat_dim_center = (mat_dim_bottom + mat_dim_top) / 2

            # Single continuous bidirectional arrow spanning full distance (vertical)
            ax.annotate(
                '',
                xy=(mat_bracket_x, mat_dim_top),  # Top endpoint
                xytext=(mat_bracket_x, mat_dim_bottom),  # Bottom endpoint
                arrowprops=dict(arrowstyle='<->,head_length=0.3,head_width=0.15', lw=0.6, color='#666666', shrinkA=0, shrinkB=0, mutation_scale=5)
            )

            # Perpendicular tick marks at ends (horizontal for vertical dimension)
            # Use frame-relative tick length (3% of frame width)
            mat_tick_len = frame_outer_w * 0.03
            ax.plot([mat_bracket_x - mat_tick_len, mat_bracket_x + mat_tick_len], [mat_dim_bottom, mat_dim_bottom],
                   color='#666666', lw=0.8, linestyle='--', dashes=[3, 2], alpha=0.8)
            ax.plot([mat_bracket_x - mat_tick_len, mat_bracket_x + mat_tick_len], [mat_dim_top, mat_dim_top],
                   color='#666666', lw=0.8, linestyle='--', dashes=[3, 2], alpha=0.8)

            # Label positioned to the left of dimension line with gap (no callout line)
            ax.text(
                mat_bracket_x - mat_cut_width * 0.4,  # To the left of dimension line
                mat_dim_center,  # Centered vertically on dimension line
                mat_width_text,
                ha='right', va='center',
                fontsize=font_size - 1,
                bbox=dict(boxstyle='round,pad=0.2', fc='#F1F8E9', ec='#666666', alpha=0.9, linewidth=1.5)
            )

        # Matboard dimension (on top mat band)
        if mat_width > 0:
            mat_center_y = (visible_y + mat_outer_h) - (mat_width / 2)
            matboard_text = f"Matboard: {format_value(matboard_h, current_unit, 2)} × {format_value(matboard_w, current_unit, 2)}"
            ax.text(
                visible_x + mat_outer_w / 2,
                mat_center_y,
                matboard_text,
                ha='center', va='center',
                fontsize=font_size,
                bbox=dict(boxstyle='round,pad=0.2', fc='#F1F8E9', alpha=0.85)
            )

        # Center annotations - Artwork & Mat opening
        center_x = frame_outer_w / 2
        center_y = frame_outer_h / 2
        line_gap = 0.06 * frame_outer_h

        # Artwork size
        artwork_text = f"Artwork: {format_value(artwork_height, current_unit, 2)} × {format_value(artwork_width, current_unit, 2)}"
        ax.text(
            center_x,
            center_y + (line_gap / 2),
            artwork_text,
            ha='center', va='center',
            fontsize=font_size,
            bbox=dict(boxstyle='round,pad=0.2', fc='white', alpha=0.85)
        )

        # Mat opening
        if mat_width > 0:
            mat_open_text = f"Mat opening: {format_value(mat_inner_h, current_unit, 2)} × {format_value(mat_inner_w, current_unit, 2)}"
            ax.text(
                center_x,
                center_y - (line_gap / 2),
                mat_open_text,
                ha='center', va='center',
                fontsize=font_size,
                bbox=dict(boxstyle='round,pad=0.2', fc='white', alpha=0.85)
            )

        # ==== 7. Add legend (unique entries) ====
        handles, labels = ax.get_legend_handles_labels()
        unique_labels = []
        unique_handles = []
        for handle, label in zip(handles, labels):
            if label not in unique_labels:
                unique_labels.append(label)
                # Create legend handle with black outline
                legend_handle = mpatches.Patch(
                    facecolor=handle.get_facecolor(),
                    edgecolor='black',
                    linewidth=0.5
                )
                unique_handles.append(legend_handle)

        ax.legend(
            unique_handles,
            unique_labels,
            loc='lower center',
            bbox_to_anchor=(0.5, -0.02),  # Even closer to plot
            ncol=len(unique_labels),
            fontsize=8,
            columnspacing=1.0,
            handlelength=1.5
        )

        # ==== 8. Final touches ====
        ax.axis('off')  # Remove title - diagram speaks for itself

        # Tight layout with space for legend at bottom
        # rect=[left, bottom, right, top] in figure coordinates (0-1)
        # Leave 8% space at bottom for legend
        plt.tight_layout(pad=0.1, rect=[0, 0.08, 1, 1])

        # Render to SVG for crisp vector output (perfect zoom quality)
        from io import BytesIO
        svg_buffer = BytesIO()
        fig.savefig(svg_buffer, format='svg', bbox_inches='tight', transparent=True)
        svg_buffer.seek(0)
        svg_content = svg_buffer.read().decode('utf-8')

        # Clean up figure (use clf instead of close to avoid PyScript canvas issues)
        plt.clf()

        # Insert SVG directly into DOM (CSS handles responsive sizing via !important)
        canvas_div = document.getElementById("matplotlib-canvas")
        canvas_div.innerHTML = svg_content

    except Exception as e:
        import traceback
        error_details = traceback.format_exc()
        results_div = document.getElementById("results")
        results_div.innerHTML = f'<span class="warning">❌ matplotlib error: {e}<br><br><pre style="font-size:10px">{error_details}</pre><br>Note: matplotlib may take 30-60s to load on first run.</span>'

# Event handler: Draw visualization button
@when("click", "#draw-viz")
def draw_visualization(event):
    """Event handler for Draw Visualization button."""
    render_visualization()

# Initialize auto-save and restore settings
setup_auto_save()

# Only restore from localStorage if NOT loaded from URL
# (JS sets form values from URL params, we don't want to overwrite them)
from js import window
if not (hasattr(window, 'rfLoadedFromUrl') and window.rfLoadedFromUrl):
    restore_settings()
else:
    console.log("Skipping localStorage restore - using URL parameters instead")

render_saved_configs()
update_aspect_ratio_display()
update_orientation_icon()

# Setup export/import event handlers
export_btn = document.getElementById("export-all-data")
if export_btn:
    export_btn.onclick = create_proxy(lambda e: export_all_data())

import_input = document.getElementById("import-data-file")
if import_input:
    import_input.onchange = create_proxy(handle_file_upload)

# Auto-calculate and render on page load
try:
    calculate_frame()
except Exception as e:
    console.log(f"Initial calculate skipped: {e}")
try:
    render_visualization()
except Exception as e:
    console.log(f"Initial render skipped: {e}")

# Mark app as ready
app_status = document.getElementById("app-status")
app_status.textContent = "✓"
app_status.className = "app-status ready"
app_status.title = "Application ready"

# Hide the ready checkmark after 2 seconds
import asyncio
async def hide_ready_status():
    await asyncio.sleep(2)
    app_status.style.opacity = "0"
    app_status.style.transition = "opacity 0.5s ease"
    await asyncio.sleep(0.5)  # Wait for fade out
    app_status.style.display = "none"

asyncio.create_task(hide_ready_status())

console.log("✅ PyScript initialized successfully!")
console.log("🖼️ ReferenceFrame core logic loaded with ZERO changes!")