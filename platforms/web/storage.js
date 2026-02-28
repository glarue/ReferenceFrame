/**
 * Storage module for ReferenceFrame
 * Handles saved configurations, custom sizes, and data backup/restore
 *
 * Compatible with PyScript version localStorage schema
 */

// Storage keys (match PyScript for compatibility)
const STORAGE_KEYS = {
    CONFIGS: 'frame_designer_saved_configs',
    CUSTOM_SIZES: 'frame_designer_custom_sizes',
    UNIT: 'frame_designer_unit',
    SETTINGS: 'frame_designer_settings',
    THEME: 'frame_designer_theme',
    CUSTOM_COLORS: 'frame_designer_custom_colors',
    CUSTOM_DEFAULTS: 'frame_designer_custom_defaults',
    DISPLAY_FORMAT: 'frame_designer_display_format'
};

/**
 * Load saved configurations from localStorage
 * @returns {Array} Array of {name, config} objects
 */
function loadSavedConfigs() {
    try {
        const json = localStorage.getItem(STORAGE_KEYS.CONFIGS);
        if (!json) return [];
        return JSON.parse(json);
    } catch (e) {
        console.error('Error loading saved configs:', e);
        return [];
    }
}

/**
 * Save a named configuration to localStorage
 * @param {string} name - Configuration name
 * @param {object} config - Configuration object
 * @returns {boolean} Success status
 */
function saveConfig(name, config) {
    try {
        const configs = loadSavedConfigs();

        // Remove existing config with same name (if any)
        const filtered = configs.filter(c => c.name !== name);

        // Add new/updated config
        filtered.push({ name, config });

        localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(filtered));
        console.log(`Saved configuration: ${name}`);
        return true;
    } catch (e) {
        console.error('Error saving config:', e);
        return false;
    }
}

/**
 * Delete a saved configuration
 * @param {string} name - Configuration name to delete
 * @returns {boolean} Success status
 */
function deleteConfig(name) {
    try {
        const configs = loadSavedConfigs();
        const filtered = configs.filter(c => c.name !== name);
        localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(filtered));
        console.log(`Deleted configuration: ${name}`);
        return true;
    } catch (e) {
        console.error('Error deleting config:', e);
        return false;
    }
}

/**
 * Get configuration by name
 * @param {string} name - Configuration name
 * @returns {object|null} Configuration object or null if not found
 */
function getConfigByName(name) {
    const configs = loadSavedConfigs();
    const found = configs.find(c => c.name === name);
    return found ? found.config : null;
}

/**
 * Load custom artwork sizes from localStorage
 * @returns {Array} Array of {name, height, width} objects
 */
function loadCustomSizes() {
    try {
        const json = localStorage.getItem(STORAGE_KEYS.CUSTOM_SIZES);
        if (!json) return [];
        return JSON.parse(json);
    } catch (e) {
        console.error('Error loading custom sizes:', e);
        return [];
    }
}

/**
 * Save a custom artwork size
 * @param {string} name - Size name
 * @param {number} height - Height in inches
 * @param {number} width - Width in inches
 * @returns {boolean} Success status
 */
function saveCustomSize(name, height, width) {
    try {
        const sizes = loadCustomSizes();
        const filtered = sizes.filter(s => s.name !== name);
        filtered.push({ name, height, width });
        localStorage.setItem(STORAGE_KEYS.CUSTOM_SIZES, JSON.stringify(filtered));
        return true;
    } catch (e) {
        console.error('Error saving custom size:', e);
        return false;
    }
}

/**
 * Delete a custom size
 * @param {string} name - Size name to delete
 * @returns {boolean} Success status
 */
function deleteCustomSize(name) {
    try {
        const sizes = loadCustomSizes();
        const filtered = sizes.filter(s => s.name !== name);
        localStorage.setItem(STORAGE_KEYS.CUSTOM_SIZES, JSON.stringify(filtered));
        return true;
    } catch (e) {
        console.error('Error deleting custom size:', e);
        return false;
    }
}

/**
 * Save current settings (auto-save)
 * @param {object} settings - Current form settings
 */
function saveCurrentSettings(settings) {
    try {
        localStorage.setItem(STORAGE_KEYS.SETTINGS, JSON.stringify(settings));
    } catch (e) {
        console.error('Error saving current settings:', e);
    }
}

/**
 * Load last saved settings
 * @returns {object|null} Settings object or null
 */
function loadCurrentSettings() {
    try {
        const json = localStorage.getItem(STORAGE_KEYS.SETTINGS);
        if (!json) return null;
        return JSON.parse(json);
    } catch (e) {
        console.error('Error loading current settings:', e);
        return null;
    }
}

/**
 * Save unit preference
 * @param {string} unit - 'inches' or 'mm'
 */
function saveUnitPreference(unit) {
    try {
        localStorage.setItem(STORAGE_KEYS.UNIT, unit);
    } catch (e) {
        console.error('Error saving unit preference:', e);
    }
}

/**
 * Load unit preference
 * @returns {string} 'inches' or 'mm'
 */
function loadUnitPreference() {
    try {
        return localStorage.getItem(STORAGE_KEYS.UNIT) || 'inches';
    } catch (e) {
        console.error('Error loading unit preference:', e);
        return 'inches';
    }
}

// ============================================================================
// Theme Management
// ============================================================================

/**
 * Save theme preference
 * @param {string} theme - 'system', 'light', or 'dark'
 */
function saveThemePreference(theme) {
    try {
        localStorage.setItem(STORAGE_KEYS.THEME, theme);
    } catch (e) {
        console.error('Error saving theme preference:', e);
    }
}

/**
 * Load theme preference
 * @returns {string} 'system', 'light', or 'dark'
 */
function loadThemePreference() {
    try {
        return localStorage.getItem(STORAGE_KEYS.THEME) || 'system';
    } catch (e) {
        console.error('Error loading theme preference:', e);
        return 'system';
    }
}

/**
 * Apply theme to document
 * @param {string} theme - 'system', 'light', or 'dark'
 */
function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    saveThemePreference(theme);
}

/**
 * Get the effective theme (resolves 'system' to actual theme)
 * @returns {string} 'light' or 'dark'
 */
function getEffectiveTheme() {
    const saved = loadThemePreference();
    if (saved === 'system') {
        return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    }
    return saved;
}

// ============================================================================
// Custom Colors Management
// ============================================================================

/**
 * Load custom color overrides
 * @returns {Object} Map of category -> colorName
 */
function loadCustomColors() {
    try {
        const json = localStorage.getItem(STORAGE_KEYS.CUSTOM_COLORS);
        if (!json) return {};
        return JSON.parse(json);
    } catch (e) {
        console.error('Error loading custom colors:', e);
        return {};
    }
}

/**
 * Save custom color for a semantic category
 * @param {string} category - e.g., 'primary', 'error', 'cutDimension'
 * @param {string} colorName - e.g., 'blue', 'flagRed', 'teal'
 */
function saveCustomColor(category, colorName) {
    try {
        const colors = loadCustomColors();
        colors[category] = colorName;
        localStorage.setItem(STORAGE_KEYS.CUSTOM_COLORS, JSON.stringify(colors));
        return true;
    } catch (e) {
        console.error('Error saving custom color:', e);
        return false;
    }
}

/**
 * Reset a custom color to factory default
 * @param {string} category - Category to reset
 */
function resetCustomColor(category) {
    try {
        const colors = loadCustomColors();
        delete colors[category];
        localStorage.setItem(STORAGE_KEYS.CUSTOM_COLORS, JSON.stringify(colors));
        return true;
    } catch (e) {
        console.error('Error resetting custom color:', e);
        return false;
    }
}

/**
 * Reset all custom colors
 */
function resetAllCustomColors() {
    try {
        localStorage.removeItem(STORAGE_KEYS.CUSTOM_COLORS);
        return true;
    } catch (e) {
        console.error('Error resetting all custom colors:', e);
        return false;
    }
}

// ============================================================================
// Custom Defaults Management
// ============================================================================

/**
 * Load custom default values
 * @returns {Object} Map of field -> value
 */
function loadCustomDefaults() {
    try {
        const json = localStorage.getItem(STORAGE_KEYS.CUSTOM_DEFAULTS);
        if (!json) return {};
        return JSON.parse(json);
    } catch (e) {
        console.error('Error loading custom defaults:', e);
        return {};
    }
}

/**
 * Save a custom default value
 * @param {string} field - Field name
 * @param {number} value - Value in inches
 */
function saveCustomDefault(field, value) {
    try {
        const defaults = loadCustomDefaults();
        defaults[field] = value;
        localStorage.setItem(STORAGE_KEYS.CUSTOM_DEFAULTS, JSON.stringify(defaults));
        return true;
    } catch (e) {
        console.error('Error saving custom default:', e);
        return false;
    }
}

/**
 * Reset a custom default to factory value
 * @param {string} field - Field to reset
 */
function resetCustomDefault(field) {
    try {
        const defaults = loadCustomDefaults();
        delete defaults[field];
        localStorage.setItem(STORAGE_KEYS.CUSTOM_DEFAULTS, JSON.stringify(defaults));
        return true;
    } catch (e) {
        console.error('Error resetting custom default:', e);
        return false;
    }
}

/**
 * Reset all custom defaults
 */
function resetAllCustomDefaults() {
    try {
        localStorage.removeItem(STORAGE_KEYS.CUSTOM_DEFAULTS);
        return true;
    } catch (e) {
        console.error('Error resetting all custom defaults:', e);
        return false;
    }
}

// ============================================================================
// Display Format Management
// ============================================================================

/**
 * Save display format preference
 * @param {string} format - 'fractions', 'decimal', or 'tape'
 */
function saveDisplayFormat(format) {
    try {
        localStorage.setItem(STORAGE_KEYS.DISPLAY_FORMAT, format);
    } catch (e) {
        console.error('Error saving display format:', e);
    }
}

/**
 * Load display format preference
 * @returns {string} 'fractions', 'decimal', or 'tape'
 */
function loadDisplayFormat() {
    try {
        return localStorage.getItem(STORAGE_KEYS.DISPLAY_FORMAT) || 'fractions';
    } catch (e) {
        console.error('Error loading display format:', e);
        return 'fractions';
    }
}

/**
 * Export all localStorage data as JSON
 * @param {object} currentSettings - Current form state
 * @param {string} currentUnit - Current unit preference
 * @returns {string} JSON string
 */
function exportAllData(currentSettings, currentUnit) {
    const exportData = {
        version: '1.0',
        exported_at: new Date().toISOString(),
        saved_configs: loadSavedConfigs(),
        custom_sizes: loadCustomSizes(),
        custom_colors: loadCustomColors(),
        custom_defaults: loadCustomDefaults(),
        current_settings: currentSettings,
        unit: currentUnit
    };
    return JSON.stringify(exportData, null, 2);
}

/**
 * Import data from JSON
 * @param {string} jsonData - JSON string to import
 * @param {string} mode - 'merge' or 'replace'
 * @returns {object} Import result {success: boolean, message: string}
 */
function importData(jsonData, mode = 'merge') {
    try {
        const data = JSON.parse(jsonData);

        // Validate version
        if (!data.version || data.version !== '1.0') {
            return { success: false, message: 'Unsupported backup version' };
        }

        if (mode === 'replace') {
            // Replace all data
            localStorage.setItem(STORAGE_KEYS.CONFIGS,
                JSON.stringify(data.saved_configs || []));
            localStorage.setItem(STORAGE_KEYS.CUSTOM_SIZES,
                JSON.stringify(data.custom_sizes || []));
            if (data.custom_colors) {
                localStorage.setItem(STORAGE_KEYS.CUSTOM_COLORS,
                    JSON.stringify(data.custom_colors));
            }
            if (data.custom_defaults) {
                localStorage.setItem(STORAGE_KEYS.CUSTOM_DEFAULTS,
                    JSON.stringify(data.custom_defaults));
            }
        } else {
            // Merge configurations (skip duplicates by name)
            const existingConfigs = loadSavedConfigs();
            const existingNames = new Set(existingConfigs.map(c => c.name));
            const newConfigs = (data.saved_configs || []).filter(
                c => !existingNames.has(c.name)
            );
            const merged = [...existingConfigs, ...newConfigs];
            localStorage.setItem(STORAGE_KEYS.CONFIGS, JSON.stringify(merged));

            // Merge custom sizes
            const existingSizes = loadCustomSizes();
            const existingSizeNames = new Set(existingSizes.map(s => s.name));
            const newSizes = (data.custom_sizes || []).filter(
                s => !existingSizeNames.has(s.name)
            );
            const mergedSizes = [...existingSizes, ...newSizes];
            localStorage.setItem(STORAGE_KEYS.CUSTOM_SIZES, JSON.stringify(mergedSizes));

            // Merge custom colors (imported values fill gaps, don't overwrite)
            if (data.custom_colors) {
                const existingColors = loadCustomColors();
                const mergedColors = { ...data.custom_colors, ...existingColors };
                localStorage.setItem(STORAGE_KEYS.CUSTOM_COLORS,
                    JSON.stringify(mergedColors));
            }

            // Merge custom defaults (imported values fill gaps, don't overwrite)
            if (data.custom_defaults) {
                const existingDefaults = loadCustomDefaults();
                const mergedDefaults = { ...data.custom_defaults, ...existingDefaults };
                localStorage.setItem(STORAGE_KEYS.CUSTOM_DEFAULTS,
                    JSON.stringify(mergedDefaults));
            }
        }

        // Update unit preference
        if (data.unit) {
            localStorage.setItem(STORAGE_KEYS.UNIT, data.unit);
        }

        const importedConfigs = (data.saved_configs || []).length;
        const importedSizes = (data.custom_sizes || []).length;
        const message = mode === 'merge'
            ? `Merged ${importedConfigs} configurations and ${importedSizes} custom sizes`
            : `Imported ${importedConfigs} configurations and ${importedSizes} custom sizes`;

        return { success: true, message };
    } catch (e) {
        console.error('Error importing data:', e);
        return { success: false, message: `Import failed: ${e.message}` };
    }
}

/**
 * Clear all stored data (dangerous!)
 */
function clearAllData() {
    try {
        localStorage.removeItem(STORAGE_KEYS.CONFIGS);
        localStorage.removeItem(STORAGE_KEYS.CUSTOM_SIZES);
        localStorage.removeItem(STORAGE_KEYS.SETTINGS);
        localStorage.removeItem(STORAGE_KEYS.UNIT);
        console.log('Cleared all stored data');
        return true;
    } catch (e) {
        console.error('Error clearing data:', e);
        return false;
    }
}

// Utility functions for HTML escaping (security)
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function escapeJs(text) {
    return text.replace(/'/g, "\\'").replace(/"/g, '\\"');
}
