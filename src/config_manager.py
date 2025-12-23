"""Named configuration management.

This module handles saving, loading, and managing named frame configurations
that users can store and recall later.
"""

import json


def get_current_config(document) -> dict:
    """Get current form values as a configuration object.

    Args:
        document: PyScript document object

    Returns:
        Dictionary of current form values
    """
    return {
        "artwork_height": document.getElementById("artwork-height").value,
        "artwork_width": document.getElementById("artwork-width").value,
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


def load_saved_configs(localStorage, console) -> list:
    """Load saved configurations from localStorage.

    Args:
        localStorage: JS localStorage object
        console: JS console object

    Returns:
        List of saved configuration objects
    """
    try:
        configs_json = localStorage.getItem("frame_designer_saved_configs")
        if configs_json:
            return json.loads(configs_json)
        return []
    except Exception as e:
        console.error(f"Error loading saved configs: {e}")
        return []


def save_config_to_storage(name: str, config: dict, localStorage, console,
                           load_saved_configs_fn) -> bool:
    """Save a named configuration to localStorage.

    Args:
        name: Configuration name
        config: Configuration dictionary
        localStorage: JS localStorage object
        console: JS console object
        load_saved_configs_fn: Function to load existing configs

    Returns:
        True if successful, False otherwise
    """
    try:
        configs = load_saved_configs_fn()
        # Check for duplicate name
        existing = [c for c in configs if c["name"] == name]
        if existing:
            console.log(f"Configuration '{name}' already exists, updating...")
            configs = [c for c in configs if c["name"] != name]

        configs.append({"name": name, "config": config})
        localStorage.setItem("frame_designer_saved_configs", json.dumps(configs))
        console.log(f"Saved configuration: {name}")
        return True
    except Exception as e:
        console.error(f"Error saving config: {e}")
        return False


def delete_config(name: str, localStorage, console,
                 load_saved_configs_fn, render_saved_configs_fn):
    """Delete a named configuration from localStorage.

    Args:
        name: Configuration name to delete
        localStorage: JS localStorage object
        console: JS console object
        load_saved_configs_fn: Function to load existing configs
        render_saved_configs_fn: Function to re-render UI
    """
    try:
        configs = load_saved_configs_fn()
        configs = [c for c in configs if c["name"] != name]
        localStorage.setItem("frame_designer_saved_configs", json.dumps(configs))
        console.log(f"Deleted configuration: {name}")
        render_saved_configs_fn()
    except Exception as e:
        console.error(f"Error deleting config: {e}")


def load_config(config: dict, document, save_current_settings_fn,
                calculate_frame_fn, render_visualization_fn, console):
    """Load a configuration into the form fields.

    Args:
        config: Configuration dictionary
        document: PyScript document object
        save_current_settings_fn: Function to save current settings
        calculate_frame_fn: Function to recalculate frame dimensions
        render_visualization_fn: Function to update visualization
        console: JS console object
    """
    try:
        document.getElementById("artwork-height").value = config.get("artwork_height", "10")
        document.getElementById("artwork-width").value = config.get("artwork_width", "8")
        document.getElementById("mat-width").value = config.get("mat_width", "2")
        document.getElementById("frame-width").value = config.get("frame_width", "1.5")
        document.getElementById("glazing-thickness").value = config.get("glazing_thickness", "0.093")
        document.getElementById("matboard-thickness").value = config.get("matboard_thickness", "0.055")
        document.getElementById("artwork-thickness").value = config.get("artwork_thickness", "0.008")
        document.getElementById("backing-thickness").value = config.get("backing_thickness", "0.125")
        document.getElementById("rabbet-depth").value = config.get("rabbet_depth", "0.375")
        document.getElementById("frame-depth").value = config.get("frame_depth", "0.75")
        document.getElementById("blade-width").value = config.get("blade_width", "0.125")
        # Also save to current settings
        save_current_settings_fn()
        # Recalculate frame dimensions and update visualization
        try:
            calculate_frame_fn()
            render_visualization_fn()
        except Exception as e:
            console.log(f"Auto-update after config load skipped: {e}")
    except Exception as e:
        console.error(f"Error loading config: {e}")


def render_saved_configs(document, load_saved_configs_fn, load_config_fn, delete_config_fn):
    """Render the list of saved configurations.

    Args:
        document: PyScript document object
        load_saved_configs_fn: Function to load existing configs
        load_config_fn: Function to load a config into form
        delete_config_fn: Function to delete a config
    """
    from pyodide.ffi import create_proxy

    configs = load_saved_configs_fn()
    container = document.getElementById("saved-configs-list")

    if not configs:
        container.innerHTML = '<p style="color: #888; font-style: italic;">No saved configurations yet.</p>'
        return

    html = '<div style="display: grid; gap: 10px;">'
    for config_data in configs:
        name = config_data["name"]
        config = config_data["config"]
        # Create a card for each configuration
        html += f'''
        <div style="background: #2a2d35; padding: 10px; border-radius: 4px; display: flex; justify-content: space-between; align-items: center;">
            <span style="font-weight: bold;">{name}</span>
            <div style="display: flex; gap: 5px;">
                <button class="load-config-btn" data-name="{name}" style="background-color: #4a7c8a; padding: 5px 10px; font-size: 12px;">Load</button>
                <button class="delete-config-btn" data-name="{name}" style="background-color: #8a4a4a; padding: 5px 10px; font-size: 12px;">Delete</button>
            </div>
        </div>
        '''
    html += '</div>'
    container.innerHTML = html

    # Attach event listeners to buttons using create_proxy for proper callback handling
    def make_load_handler(cfg):
        def handler(e):
            load_config_fn(cfg)
        return create_proxy(handler)

    def make_delete_handler(config_name):
        def handler(e):
            delete_config_fn(config_name)
        return create_proxy(handler)

    for btn in document.querySelectorAll(".load-config-btn"):
        config_name = btn.getAttribute("data-name")
        config_data = [c for c in configs if c["name"] == config_name][0]
        btn.addEventListener("click", make_load_handler(config_data["config"]))

    for btn in document.querySelectorAll(".delete-config-btn"):
        config_name = btn.getAttribute("data-name")
        btn.addEventListener("click", make_delete_handler(config_name))


def handle_save_config(event, document, console, get_current_config_fn,
                      save_config_to_storage_fn, render_saved_configs_fn):
    """Handle save configuration button click.

    Args:
        event: Click event
        document: PyScript document object
        console: JS console object
        get_current_config_fn: Function to get current form values
        save_config_to_storage_fn: Function to save config to storage
        render_saved_configs_fn: Function to re-render UI
    """
    try:
        name_input = document.getElementById("config-name")
        name = name_input.value.strip()

        if not name:
            console.log("Please enter a configuration name")
            return

        config = get_current_config_fn()
        if save_config_to_storage_fn(name, config):
            name_input.value = ""  # Clear input
            render_saved_configs_fn()
            console.log(f"Configuration '{name}' saved successfully")
    except Exception as e:
        console.error(f"Error in save_config_handler: {e}")
