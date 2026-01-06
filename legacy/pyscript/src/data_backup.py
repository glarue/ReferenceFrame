"""Data backup and import functionality.

This module handles exporting and importing all localStorage data
(configurations, custom sizes, settings) as JSON files.
"""

from datetime import datetime
import json


def export_all_data(document, localStorage, console, get_current_config_fn, load_saved_configs_fn):
    """Export all localStorage data as JSON file.

    Args:
        document: PyScript document object
        localStorage: JS localStorage object
        console: JS console object
        get_current_config_fn: Function to get current form configuration
        load_saved_configs_fn: Function to load saved configurations
    """
    from js import Blob, URL

    try:
        # Get current form state (use get_current_config which reads from form directly)
        current_settings = get_current_config_fn()

        # Gather all localStorage data
        export_data = {
            "version": "1.0",  # For future compatibility
            "exported_at": datetime.now().isoformat(),
            "saved_configs": load_saved_configs_fn(),
            "custom_sizes": json.loads(localStorage.getItem("frame_designer_custom_sizes") or "[]"),
            "current_settings": current_settings,  # Current form state, not localStorage
            "unit": localStorage.getItem("frame_designer_unit") or "inches"
        }

        # Create JSON blob and download
        json_str = json.dumps(export_data, indent=2)
        blob = Blob.new([json_str], {"type": "application/json"})
        url = URL.createObjectURL(blob)

        # Create download link with timestamp
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        a = document.createElement("a")
        a.href = url
        a.download = f"referenceframe_backup_{timestamp}.json"
        a.click()
        URL.revokeObjectURL(url)

        console.log("Exported all data successfully")
    except Exception as e:
        console.error(f"Error exporting data: {e}")


def import_data(file_content, merge_mode, localStorage, console,
                load_saved_configs_fn, render_saved_configs_fn,
                render_custom_sizes_fn, restore_settings_fn,
                calculate_frame_fn, render_visualization_fn):
    """Import localStorage data from JSON.

    Args:
        file_content: JSON string from uploaded file
        merge_mode: "merge" or "replace"
        localStorage: JS localStorage object
        console: JS console object
        load_saved_configs_fn: Function to load saved configurations
        render_saved_configs_fn: Function to render saved configs UI
        render_custom_sizes_fn: Function to render custom sizes UI
        restore_settings_fn: Function to restore settings to form
        calculate_frame_fn: Function to recalculate frame design
        render_visualization_fn: Function to update visualization

    Returns:
        True if successful, False otherwise
    """
    try:
        data = json.loads(file_content)

        # Validate structure
        if "version" not in data:
            console.error("Invalid backup file format - missing version")
            return False

        if merge_mode == "replace":
            # Clear existing data
            localStorage.removeItem("frame_designer_saved_configs")
            localStorage.removeItem("frame_designer_custom_sizes")
            localStorage.removeItem("frame_designer_settings")
            localStorage.removeItem("frame_designer_unit")

        # Import saved configs
        if "saved_configs" in data:
            if merge_mode == "merge":
                existing = load_saved_configs_fn()
                existing_names = {c["name"] for c in existing}
                for config in data["saved_configs"]:
                    if config["name"] not in existing_names:
                        existing.append(config)
                    else:
                        # Update existing config with same name
                        for i, c in enumerate(existing):
                            if c["name"] == config["name"]:
                                existing[i] = config
                                break
                localStorage.setItem("frame_designer_saved_configs", json.dumps(existing))
            else:
                localStorage.setItem("frame_designer_saved_configs", json.dumps(data["saved_configs"]))

        # Import custom sizes
        if "custom_sizes" in data:
            if merge_mode == "merge":
                existing = json.loads(localStorage.getItem("frame_designer_custom_sizes") or "[]")
                # Merge unique sizes (avoid duplicates by height×width)
                existing_sizes = {(s["height"], s["width"]) for s in existing}
                for size in data["custom_sizes"]:
                    if (size["height"], size["width"]) not in existing_sizes:
                        existing.append(size)
                localStorage.setItem("frame_designer_custom_sizes", json.dumps(existing))
            else:
                localStorage.setItem("frame_designer_custom_sizes", json.dumps(data["custom_sizes"]))

        # Import current settings
        if "current_settings" in data:
            localStorage.setItem("frame_designer_settings", json.dumps(data["current_settings"]))

        # Import unit preference
        if "unit" in data:
            localStorage.setItem("frame_designer_unit", data["unit"])

        # Refresh UI and apply imported settings
        render_saved_configs_fn()
        render_custom_sizes_fn()
        restore_settings_fn()  # Apply current_settings to form fields

        # Update visualization with imported settings
        try:
            calculate_frame_fn()
            render_visualization_fn()
        except Exception as e:
            console.error(f"Error updating visualization after import: {e}")

        console.log(f"Imported data successfully (mode: {merge_mode})")
        return True

    except Exception as e:
        console.error(f"Import failed: {e}")
        import traceback
        console.error(traceback.format_exc())
        return False


def handle_file_upload(event, console, show_import_dialog_fn):
    """Handle file input change event.

    Args:
        event: File input change event
        console: JS console object
        show_import_dialog_fn: Function to show import dialog
    """
    from js import FileReader
    from pyodide.ffi import create_proxy

    # Access the first file using .item() method for JsProxy objects
    file = event.target.files.item(0)
    if not file:
        return

    reader = FileReader.new()

    def on_load(e):
        content = e.target.result
        # Show merge/replace dialog
        show_import_dialog_fn(content)

    reader.onload = create_proxy(on_load)
    reader.readAsText(file)

    console.log(f"Reading file: {file.name}")


def show_import_dialog(file_content, document, import_data_fn):
    """Show dialog asking user to merge or replace.

    Args:
        file_content: JSON file content
        document: PyScript document object
        import_data_fn: Import function (already has dependencies bound)
    """
    from pyodide.ffi import create_proxy

    # Update status message with buttons
    status_div = document.getElementById("import-status")
    status_div.innerHTML = '''
        <div style="background: #2a2d35; padding: 10px; border-radius: 4px;">
            <p style="margin: 0 0 10px 0;">Import mode:</p>
            <button id="import-merge" style="margin-right: 8px; background-color: #2a7d2e;">
                Merge (keep existing + add new)
            </button>
            <button id="import-replace" style="background-color: #dc3545;">
                Replace (clear existing)
            </button>
        </div>
    '''

    # Attach handlers
    def do_merge(e):
        if import_data_fn(file_content, "merge"):
            status_div.innerHTML = '<span style="color: #4caf50;">✓ Data imported (merged)</span>'
        else:
            status_div.innerHTML = '<span style="color: #f44336;">✗ Import failed - check console</span>'

    def do_replace(e):
        if import_data_fn(file_content, "replace"):
            status_div.innerHTML = '<span style="color: #4caf50;">✓ Data imported (replaced)</span>'
        else:
            status_div.innerHTML = '<span style="color: #f44336;">✗ Import failed - check console</span>'

    document.getElementById("import-merge").onclick = create_proxy(do_merge)
    document.getElementById("import-replace").onclick = create_proxy(do_replace)
