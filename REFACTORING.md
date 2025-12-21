# ReferenceFrame Code Refactoring Plan

## Overview
Main.py is currently 2629 lines. This document outlines the modularization plan to improve maintainability.

## Completed Modules

### 1. shareable_url.py (Created ✓)
**Purpose**: Generate compact shareable URLs encoding frame settings
**Functions**:
- `generate_shareable_url(document, current_unit, include_mat)` - Binary URL encoding

**Dependencies**: ui_helpers.get_form_values_as_inches

**Lines saved**: ~53 lines

### 2. export_text.py (Created ✓)
**Purpose**: Text/markdown export functionality
**Functions**:
- `generate_text_summary(document, current_unit)` - Generate formatted text summary
- `download_file(content, filename, mime_type)` - Trigger browser download
- `handle_export_text(document, current_unit)` - Event handler wrapper

**Dependencies**: ui_helpers, frame_design, conversions.format_value

**Lines saved**: ~178 lines

### 3. export_pdf.py (Created ✓)
**Purpose**: PDF export with vector diagrams and QR codes
**Functions**:
- `generate_pdf_content(pdf, document, current_unit, start_y)` - Add formatted content to PDF
- `add_qr_code_to_pdf(pdf, url, status_div, console)` - Add QR code to PDF corner
- `handle_export_pdf(document, current_unit, shareable_url, console)` - Main PDF export handler

**Dependencies**: ui_helpers, frame_design, conversions.format_value, shareable_url

**Lines saved**: ~350 lines

### 4. data_backup.py (Created ✓)
**Purpose**: Data backup and import functionality
**Functions**:
- `export_all_data(...)` - Export all localStorage to JSON
- `import_data(...)` - Import from JSON with merge/replace
- `handle_file_upload(event, console, show_import_dialog_fn)` - File upload handler
- `show_import_dialog(file_content, document, import_data_fn)` - Merge/replace dialog

**Dependencies**: Depends on config_manager and custom_sizes functions (passed as parameters)

**Lines saved**: ~188 lines

## Remaining Modules to Create

### 5. config_manager.py (TODO)
**Purpose**: Named configuration save/load/delete
**Functions to extract from main.py** (lines 1394-1541):
- `get_current_config(document)`
- `load_saved_configs(localStorage, console)`
- `save_config_to_storage(name, config, localStorage, console, load_saved_configs_fn)`
- `delete_config(name, localStorage, console, load_saved_configs_fn, render_saved_configs_fn)`
- `load_config(config, document, save_current_settings_fn, render_visualization_fn, console)`
- `render_saved_configs(document, load_saved_configs_fn, load_config_fn, delete_config_fn)`
- `handle_save_config(event, document, console, get_current_config_fn, save_config_to_storage_fn, render_saved_configs_fn)` - @when wrapper

**Lines to save**: ~148 lines

### 6. custom_sizes.py (TODO)
**Purpose**: Custom artwork size management
**Functions to extract from main.py** (lines 214-388, 1732-1820):
- `load_custom_sizes(localStorage, console)`
- `save_custom_sizes(sizes, localStorage, console)`
- `render_custom_sizes(document, load_custom_sizes_fn, apply_custom_size_fn, delete_custom_size_fn)`
- `apply_custom_size(index, document, localStorage, console, ...)`
- `delete_custom_size(index, localStorage, console, load_custom_sizes_fn, render_custom_sizes_fn)`
- `handle_add_custom_size(event, document, localStorage, console, ...)` - @when wrapper
- `handle_apply_saved_size(event, ...)` - @when wrapper
- `handle_delete_saved_size(event, ...)` - @when wrapper

**Lines to save**: ~265 lines

## Total Reduction
- **Created modules**: ~769 lines
- **Remaining modules**: ~413 lines
- **Total potential reduction**: ~1182 lines (45% of main.py)
- **New main.py size**: ~1447 lines

## Integration Plan

### Step 1: Create remaining modules
1. Create `config_manager.py` with named configuration functions
2. Create `custom_sizes.py` with custom size management

### Step 2: Update main.py
1. Add imports for all new modules
2. Replace function definitions with imports
3. Update @when event handlers to call module functions
4. Pass necessary dependencies (document, localStorage, console) to module functions

### Step 3: Test all functionality
1. Export text - verify text summary generation
2. Export PDF - verify PDF with diagram and QR code
3. Generate shareable URL - verify URL encoding/decoding
4. Data backup - export/import with merge/replace
5. Named configurations - save/load/delete
6. Custom sizes - add/apply/delete

### Step 4: Update documentation
1. Update technical documentation
2. Add module documentation
3. Update README if needed

## Example Integration Pattern

**Before** (main.py):
```python
@when("click", "#export-text")
def handle_export_text(event):
    try:
        summary = generate_text_summary()
        download_file(summary, "frame_design_summary.txt", "text/plain")
        # ...
```

**After** (main.py):
```python
from export_text import handle_export_text as export_text_impl

@when("click", "#export-text")
def handle_export_text(event):
    current_unit = get_current_unit()
    success, message = export_text_impl(document, current_unit)
    status_div = document.getElementById("export-status")
    status_div.innerHTML = f'<span class="{"success" if success else "warning"}">{message}</span>'
```

## Module Dependencies Graph

```
main.py
├── shareable_url.py
│   └── ui_helpers.py
├── export_text.py
│   ├── ui_helpers.py
│   ├── frame_design.py
│   └── conversions.py
├── export_pdf.py
│   ├── ui_helpers.py
│   ├── frame_design.py
│   ├── conversions.py
│   └── shareable_url.py
├── data_backup.py
│   ├── config_manager.py (functions passed as params)
│   └── custom_sizes.py (functions passed as params)
├── config_manager.py
│   └── (no module dependencies, uses passed params)
└── custom_sizes.py
    └── (no module dependencies, uses passed params)
```

## Benefits

1. **Maintainability**: Easier to find and modify specific functionality
2. **Testability**: Each module can be tested independently
3. **Readability**: Smaller files are easier to understand
4. **Reusability**: Modules can be imported where needed
5. **Separation of Concerns**: Clear boundaries between different features

## Notes

- All modules use dependency injection pattern to avoid circular imports
- PyScript document, localStorage, and console objects passed as parameters
- Event handlers remain in main.py for PyScript @when decorator compatibility
- Existing helper modules (ui_helpers, frame_design, conversions) already well-organized
