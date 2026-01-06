# Plan: Saved Configurations & Data Management for WASM Version

## Overview
Implement configuration saving/loading and data backup/restore functionality with maximum cross-platform reusability (Web, iOS, Android).

## PyScript Version Feature Analysis

### Storage Keys (localStorage)
1. `frame_designer_saved_configs` - Array of named configurations
2. `frame_designer_custom_sizes` - Array of custom artwork sizes
3. `frame_designer_unit` - Current unit preference (inches/mm)
4. `frame_designer_settings` - Last used settings (auto-saved)

### Features
1. **Named Configurations**
   - Save current form state with a user-defined name
   - Load saved configuration (populates all form fields)
   - Delete saved configuration
   - UI: Collapsible section with name input, save button, list of saved configs

2. **Custom Artwork Sizes**
   - Save frequently-used artwork dimensions
   - Quick-load buttons for standard sizes
   - (Not yet implemented in PyScript version fully)

3. **Data Backup/Restore**
   - Export all data as JSON file with timestamp
   - Import data with merge or replace mode
   - Includes: saved configs, custom sizes, current settings, unit preference

## Cross-Platform Architecture Strategy

### Option A: Rust Core + Platform Bindings (RECOMMENDED)
**Pros:**
- Single source of truth for storage logic
- Type safety via serde serialization
- Reusable across Web (WASM), iOS (FFI), Android (JNI/FFI)
- Business logic stays in Rust

**Cons:**
- Cannot directly access localStorage from Rust (needs JS bridge)
- More initial complexity

**Implementation:**
```
Rust Core (storage logic)
    ├── Web: WASM bindings → JS localStorage
    ├── iOS: FFI → UserDefaults
    └── Android: JNI → SharedPreferences
```

### Option B: Platform-Specific Storage (Simpler for now)
**Pros:**
- Simpler initial implementation
- Direct localStorage access from JavaScript
- Can copy PyScript patterns directly

**Cons:**
- Need to reimplement for iOS/Android later
- But: Can use same JSON schema for compatibility

**Recommendation:** Start with Option B, design JSON schema for future portability.

## Implementation Plan (Web First, Cross-Platform Ready)

### Phase 1: Data Structures & Storage Module

**1.1 Define Storage Schema (JSON)**
Create storage format compatible across platforms:

```json
{
  "version": "1.0",
  "saved_configs": [
    {
      "name": "Standard Print",
      "config": {
        "artwork_height": "12.5",
        "artwork_width": "18.75",
        "mat_width": "2.0",
        "frame_width": "0.75",
        "frame_depth": "0.75",
        "glazing_thickness": "0.093",
        "matboard_thickness": "0.055",
        "artwork_thickness": "0.008",
        "backing_thickness": "0.125",
        "rabbet_width": "0.375",
        "rabbet_depth": "0.375",
        "assembly_margin": "0.03125",
        "blade_width": "0.125",
        "mat_overlap": "0.25",
        "include_mat": true
      }
    }
  ],
  "custom_sizes": [
    {
      "name": "8×10 Print",
      "height": "10",
      "width": "8"
    }
  ],
  "current_settings": { /* same as config */ },
  "unit": "inches"
}
```

**1.2 Create JavaScript Storage Module**
File: `platforms/web/storage.js`

```javascript
// Storage keys (match PyScript for compatibility)
const STORAGE_KEYS = {
    CONFIGS: 'frame_designer_saved_configs',
    CUSTOM_SIZES: 'frame_designer_custom_sizes',
    UNIT: 'frame_designer_unit',
    SETTINGS: 'frame_designer_settings'
};

// Load saved configurations
function loadSavedConfigs() { ... }

// Save configuration
function saveConfig(name, config) { ... }

// Delete configuration
function deleteConfig(name) { ... }

// Export all data
function exportAllData() { ... }

// Import data (merge or replace)
function importData(jsonData, mode = 'merge') { ... }
```

### Phase 2: UI Components

**2.1 Saved Configurations Section**
Add to `index.html` (similar structure to PyScript version):

```html
<details class="settings-section">
    <summary>💾 Saved Configurations</summary>
    <div class="saved-configs-content">
        <div class="form-group">
            <label for="config-name">Configuration Name:</label>
            <input type="text" id="config-name"
                   placeholder="e.g., Standard Print, Large Canvas">
        </div>
        <button id="save-config" class="btn-primary">
            Save Current Configuration
        </button>
        <div id="saved-configs-list" class="configs-list">
            <!-- Dynamically populated -->
        </div>

        <!-- Backup & Restore -->
        <div class="backup-section">
            <h4>Backup & Restore</h4>
            <div class="backup-buttons">
                <button id="export-data" class="btn-secondary">
                    📥 Export All Data
                </button>
                <label for="import-file" class="btn-secondary">
                    📤 Import Data
                    <input type="file" id="import-file"
                           accept=".json" style="display: none;">
                </label>
            </div>
        </div>
    </div>
</details>
```

**2.2 Config Card Template**
For each saved configuration:

```html
<div class="config-card">
    <span class="config-name">{name}</span>
    <div class="config-actions">
        <button class="btn-load" data-name="{name}">Load</button>
        <button class="btn-delete" data-name="{name}">Delete</button>
    </div>
</div>
```

**2.3 Styling**
Add to `styles.css`:
- `.settings-section` - Collapsible section styling
- `.config-card` - Card layout for saved configs
- `.backup-section` - Backup/restore UI styling

### Phase 3: Core Functionality

**3.1 Get Current Configuration**
```javascript
function getCurrentConfig() {
    return {
        artwork_height: getDimensionValue('artwork-height'),
        artwork_width: getDimensionValue('artwork-width'),
        mat_width: getDimensionValue('mat-width'),
        frame_width: getDimensionValue('frame-width'),
        frame_depth: getDimensionValue('frame-depth'),
        glazing_thickness: getDimensionValue('glazing-thickness'),
        matboard_thickness: getDimensionValue('matboard-thickness'),
        artwork_thickness: getDimensionValue('artwork-thickness'),
        backing_thickness: getDimensionValue('backing-thickness'),
        rabbet_width: getDimensionValue('rabbet-width'),
        rabbet_depth: getDimensionValue('rabbet-depth'),
        assembly_margin: getDimensionValue('assembly-margin'),
        blade_width: getDimensionValue('blade-width'),
        mat_overlap: getDimensionValue('mat-overlap'),
        include_mat: document.getElementById('include-mat').checked
    };
}
```

**3.2 Load Configuration**
```javascript
function loadConfig(config) {
    // Populate form fields
    setDimensionValue('artwork-height', config.artwork_height);
    setDimensionValue('artwork-width', config.artwork_width);
    // ... all other fields ...

    // Trigger recalculation
    window.calculate();
}
```

**3.3 Save Configuration**
```javascript
function saveConfiguration(name) {
    const config = getCurrentConfig();
    const configs = loadSavedConfigs();

    // Check for duplicate names
    const existing = configs.find(c => c.name === name);
    if (existing) {
        if (!confirm(`Configuration "${name}" exists. Overwrite?`)) {
            return;
        }
        configs = configs.filter(c => c.name !== name);
    }

    configs.push({ name, config });
    localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(configs));
    renderSavedConfigs();
}
```

**3.4 Render Saved Configurations**
```javascript
function renderSavedConfigs() {
    const configs = loadSavedConfigs();
    const container = document.getElementById('saved-configs-list');

    if (configs.length === 0) {
        container.innerHTML = '<p class="empty-state">No saved configurations yet.</p>';
        return;
    }

    container.innerHTML = configs.map(({ name, config }) => `
        <div class="config-card">
            <span class="config-name">${escapeHtml(name)}</span>
            <div class="config-actions">
                <button class="btn-load" onclick="loadConfigByName('${escapeJs(name)}')">Load</button>
                <button class="btn-delete" onclick="deleteConfigByName('${escapeJs(name)}')">Delete</button>
            </div>
        </div>
    `).join('');
}
```

### Phase 4: Data Backup/Restore

**4.1 Export All Data**
```javascript
function exportAllData() {
    const exportData = {
        version: '1.0',
        exported_at: new Date().toISOString(),
        saved_configs: loadSavedConfigs(),
        custom_sizes: loadCustomSizes(),
        current_settings: getCurrentConfig(),
        unit: currentUnit
    };

    const json = JSON.stringify(exportData, null, 2);
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const timestamp = new Date().toISOString().slice(0, 10).replace(/-/g, '');
    const a = document.createElement('a');
    a.href = url;
    a.download = `referenceframe_backup_${timestamp}.json`;
    a.click();
    URL.revokeObjectURL(url);
}
```

**4.2 Import Data**
```javascript
async function importData(file) {
    const text = await file.text();
    const data = JSON.parse(text);

    // Validate version
    if (data.version !== '1.0') {
        alert('Unsupported backup version');
        return;
    }

    // Prompt for merge or replace
    const mode = confirm(
        'Merge with existing data? (Cancel to replace all data)'
    ) ? 'merge' : 'replace';

    if (mode === 'replace') {
        localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(data.saved_configs || []));
        localStorage.setItem(STORAGE_KEYS.CUSTOM_SIZES, JSON.stringify(data.custom_sizes || []));
    } else {
        // Merge configurations (skip duplicates)
        const existing = loadSavedConfigs();
        const merged = [...existing];
        for (const config of (data.saved_configs || [])) {
            if (!merged.find(c => c.name === config.name)) {
                merged.push(config);
            }
        }
        localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(merged));
    }

    // Update unit preference
    if (data.unit) {
        localStorage.setItem(STORAGE_KEYS.UNIT, data.unit);
    }

    // Render updated UI
    renderSavedConfigs();
    renderCustomSizes();
}
```

### Phase 5: Auto-Save Current Settings

**5.1 Save on Every Change**
Hook into existing `calculate()` function:

```javascript
function saveCurrentSettings() {
    const settings = getCurrentConfig();
    localStorage.setItem(STORAGE_KEYS.SETTINGS, JSON.stringify(settings));
}

// Call after successful calculation
window.calculate = function() {
    // ... existing calculation code ...

    // Auto-save settings
    saveCurrentSettings();
}
```

**5.2 Restore on Page Load**
```javascript
function restoreLastSettings() {
    const settings = localStorage.getItem(STORAGE_KEYS.SETTINGS);
    if (settings) {
        try {
            const config = JSON.parse(settings);
            loadConfig(config);
        } catch (e) {
            console.warn('Failed to restore last settings:', e);
        }
    }
}

// Call after WASM loads
init().then(() => {
    wasmLoaded = true;
    restoreLastSettings();
});
```

### Phase 6: Custom Artwork Sizes (Future Enhancement)

**6.1 Custom Size Schema**
```json
{
  "name": "8×10 Print",
  "height": "10",
  "width": "8"
}
```

**6.2 UI Location**
Add quick-select buttons above artwork dimensions:

```html
<div class="quick-sizes">
    <label>Quick Sizes:</label>
    <div id="custom-sizes-buttons">
        <!-- Dynamically populated -->
    </div>
    <button id="add-custom-size" class="btn-small">+ Add Current Size</button>
</div>
```

## Cross-Platform Migration Path

### When Building iOS/Android Apps

**Option 1: Rust Storage Manager (Recommended Long-Term)**
```rust
// core/src/storage.rs
pub struct ConfigManager {
    configs: Vec<SavedConfig>,
}

impl ConfigManager {
    pub fn load_configs(storage_json: &str) -> Result<Self, Error> { ... }
    pub fn save_config(&mut self, name: String, config: FrameConfig) { ... }
    pub fn delete_config(&mut self, name: &str) { ... }
    pub fn export_json(&self) -> String { ... }
}

// Platform bindings handle persistence:
// Web: localStorage
// iOS: UserDefaults
// Android: SharedPreferences
```

**Option 2: Reuse JSON Schema**
- Copy JavaScript logic to Dart/Swift/Kotlin
- Use same JSON schema for compatibility
- Import/export works across platforms

## File Changes Summary

### New Files
- `platforms/web/storage.js` - Storage management module
- `platforms/web/SAVED_CONFIGS_PLAN.md` - This document

### Modified Files
- `platforms/web/index.html` - Add UI sections, wire up event handlers
- `platforms/web/styles.css` - Add styling for new UI components
- `platforms/web/index.html` (calculate function) - Add auto-save call

### Optional (Future)
- `core/src/storage.rs` - Rust storage manager for cross-platform use
- `platforms/web/wasm_bindings/src/lib.rs` - Export storage functions

## Testing Checklist

- [ ] Save configuration with name
- [ ] Load configuration populates all fields correctly
- [ ] Delete configuration removes from list
- [ ] Duplicate name prompts for overwrite
- [ ] Export creates JSON file with timestamp
- [ ] Import (merge mode) preserves existing configs
- [ ] Import (replace mode) overwrites all data
- [ ] Auto-save persists settings between page reloads
- [ ] Works with unit toggle (inches/mm)
- [ ] Configuration includes mat_overlap and all advanced settings

## Notes

- **localStorage limits:** ~5-10MB per domain (plenty for this use case)
- **Privacy:** All data stays local, nothing sent to server
- **Compatibility:** JSON schema designed for future cross-platform use
- **Migration:** PyScript users can export and import into WASM version seamlessly
