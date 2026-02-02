# Platform Parity Plan: WASM Web ↔ iOS Mobile

## Executive Summary

This document outlines the feature gaps between the WASM web platform and iOS mobile platform, with a prioritized implementation plan to bring them into parity.

**Current State (January 2026):**
- iOS (Flutter): Full-featured with recent color customization and validation settings additions
- WASM (Web): Production-ready with theme, color customization, and display format support

---

## Feature Comparison Matrix

| Feature | iOS | WASM | Priority | Status |
|---------|-----|------|----------|--------|
| Core frame calculations | ✅ | ✅ | - | Complete |
| SVG visualization (Plan/Section/Combined) | ✅ | ✅ | - | Complete |
| PDF export with vector diagrams | ✅ | ✅ | - | Complete |
| Saved configurations | ✅ | ✅ | - | Complete |
| Custom artwork sizes | ✅ | ✅ | - | Complete |
| Aspect ratio lock | ✅ | ✅ | - | Complete |
| Unit switching (in/mm) | ✅ | ✅ | - | Complete |
| Color customization | ✅ | ✅ | High | ✅ Complete |
| Theme selection (light/dark/system) | ✅ | ✅ | High | ✅ Complete |
| Display format (fractions/decimal/tape) | ✅ | ✅ | Medium | ✅ Complete |
| Custom defaults management | ✅ | ❌ | Medium | Pending |
| Custom PDF title | ✅ | ✅ | Low | ✅ Complete |
| Shareable URL (in PDF) | ✅ | ✅ | - | Complete |
| Validation settings modal | ✅ | ✅ | - | Complete |

---

## Completed Work (2026-01-26)

### Phase 1: Color Customization ✅

**Files modified:**
- `platforms/web/styles.css` - Expanded to 10-color palette, updated semantic mappings
- `platforms/web/storage.js` - Added color customization storage functions
- `platforms/web/index.html` - Added color settings UI in Appearance tab

**Changes:**
1. **Expanded palette from 7 to 10 colors:**
   - Added `--rf-flag-red: #d52023` (critical errors, delete)
   - Added `--rf-dark-cyan: #478583` (muted/disabled states)
   - Added `--rf-air-force-blue: #7890a5` (hover/backgrounds)
   - Added light and dark variants for all new colors

2. **Updated semantic color mappings to match iOS:**
   ```css
   --rf-primary: var(--rf-blue);
   --rf-secondary: var(--rf-teal);
   --rf-success: var(--rf-green);
   --rf-warning: var(--rf-orange);
   --rf-error: var(--rf-flag-red);      /* Was: --rf-red */
   --rf-modified: var(--rf-yellow);      /* Was: --rf-red-orange */
   --rf-cut-dimension: var(--rf-red-orange);  /* Was: --rf-orange */
   --rf-incidental: var(--rf-dark-cyan);      /* Was: --rf-teal */
   --rf-material-property: var(--rf-air-force-blue);  /* Was: --rf-green */
   ```

3. **Color customization UI:**
   - 9 semantic categories displayed with swatches
   - Click swatch to open 10-color picker popup
   - Checkmark badge indicates customized colors
   - Reset individual or all colors
   - Persists to `frame_designer_custom_colors` in localStorage

### Phase 2: Theme Selection ✅

**Changes:**
1. **CSS theme support:**
   - Dark theme is default (existing behavior)
   - Added `[data-theme="light"]` CSS block with light surface colors
   - Added `@media (prefers-color-scheme: light)` for system auto-detection
   - Surface containers, text colors, borders all adapt to theme

2. **Theme toggle button in header:**
   - Cycles through: System (🔄) → Light (☀️) → Dark (🌙)
   - Persists preference to `frame_designer_theme`
   - Applies immediately without page reload

3. **Theme selector in Settings → Appearance tab:**
   - Three-button selector for System/Light/Dark
   - Shows active state on current selection

### Phase 3: Display Format UI ✅

**Changes:**
1. **Format selector in Settings → Appearance tab:**
   - Three options: Fractions (4 3/4"), Decimal (4.75"), Tape (4 - 3/4)
   - Persists to `frame_designer_display_format`
   - **Note:** Format selection is wired up but full integration with `formatValue()` calls needs completion

### Settings Modal Expansion

The existing "Validation Settings" modal was expanded to a general "Settings" modal:
- Renamed header to "Settings"
- Added "Appearance" tab alongside validation tabs (Structural, Dimensions, Materials, Warnings)
- Unit toggle (in/mm) hidden when Appearance tab is active
- Description text updates based on active tab

---

## Remaining Work

### Phase 4: Custom Defaults Management (Medium Priority)

**Status:** Not started

**Features needed:**
- Each default shows current value with "Modified" indicator if changed
- Edit button opens input dialog with validation
- Reset button reverts to factory default
- All customizations persist to localStorage

**Storage Key:** `frame_designer_custom_defaults` (already added to storage.js)

**UI Location:** New "Defaults" tab in settings, organized by category:
- Frame Defaults
- Mat Defaults
- Material Thicknesses

### Phase 5: Display Format Integration (Medium Priority) ✅

**Status:** Complete (2026-01-27)

**Implementation:**
- Added `formatValueTapeMeasure` WASM binding for tape measure format
- Created `formatDisplay()` wrapper function that selects formatter based on `currentDisplayFormat`
- Replaced all `formatValueWithDecimal()` calls with `formatDisplay()`
- Format options: fractions (`formatValue`), decimal (`formatValueWithDecimal`), tape (`formatValueTapeMeasure`)

### Phase 6: Custom PDF Title (Low Priority) ✅

**Status:** Complete (2026-01-27)

**Implementation:**
- Added text input field in Results header (placeholder: "PDF title (optional)")
- Updated `exportPdf()` to read custom title and pass to `generateCombinedViewSvgForPdf()`
- If empty, defaults to "Frame Design"
- Added CSS styling for `.pdf-title-input` and `.results-actions` container

---

## Technical Notes

### Storage Keys (Updated)

| Purpose | iOS Key | WASM Key | Status |
|---------|---------|----------|--------|
| Unit preference | `frame_designer_unit` | `frame_designer_unit` | ✅ |
| Saved configs | `frame_designer_saved_configs` | `frame_designer_saved_configs` | ✅ |
| Custom sizes | `frame_designer_custom_sizes` | `frame_designer_custom_sizes` | ✅ |
| Custom colors | `frame_designer_custom_colors` | `frame_designer_custom_colors` | ✅ |
| Custom defaults | `frame_designer_custom_defaults` | `frame_designer_custom_defaults` | Added |
| Theme | (system ThemeMode) | `frame_designer_theme` | ✅ |
| Display format | `frame_designer_decimal_display` | `frame_designer_display_format` | ✅ |

### CSS Variable Naming Convention

```css
/* Palette colors: --rf-{color-name} */
--rf-flag-red
--rf-red
--rf-red-orange
--rf-orange
--rf-yellow
--rf-green
--rf-teal
--rf-dark-cyan
--rf-blue
--rf-air-force-blue

/* Semantic colors: --rf-{purpose} */
--rf-primary
--rf-secondary
--rf-success
--rf-warning
--rf-error
--rf-modified
--rf-cut-dimension
--rf-incidental
--rf-material-property

/* Variants: --rf-{name}-light, --rf-{name}-dark */
```

---

## Verification Checklist

After implementation, verify:

- [x] Color picker shows all 10 palette colors
- [x] Selecting a color updates UI immediately
- [x] Custom colors persist after page refresh
- [x] Theme toggle works correctly
- [x] System theme detection works
- [ ] Custom defaults can be edited and reset
- [ ] "Modified" badges appear for customized values
- [x] Display format selector UI works
- [x] Display format actually changes output formatting
- [x] PDF export uses custom title when provided
- [ ] All features work in Safari, Chrome, Firefox

---

## iOS Updates (Same Session)

Also completed during this session:

### Validation Settings Screen
- Added validation limits configuration to iOS Settings screen
- 6 expandable categories: Structural, Frame, Opening, Rabbet, Materials, Warnings
- Edit dialog with fraction input support
- Reset individual / Reset all functionality
- Persists to `frame_designer_validation` in storage

### New Validation Config Fields
Added to Rust core `ValidationConfig`:
- `min_visible_opening` (0.125") - Minimum artwork visible through mat per side
- `warn_min_mat_opening` (1.0") - Warn if mat opening is smaller than this

---

*Last Updated: 2026-01-27*
