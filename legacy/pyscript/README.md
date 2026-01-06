# PyScript Version (Legacy)

**Status:** ✅ Archived - Functional reference implementation

**Previous URL:** https://glarue.github.io/ReferenceFrame (now serves WASM version)

## Overview

This is the original PyScript/Pyodide implementation of ReferenceFrame. It served as the production version from December 2024 through January 2026.

## Why Archived

The WASM version (`platforms/web/`) has replaced this as the primary web deployment because:
- **50-100× smaller payload** (~220 KB vs ~30 MB)
- **Faster load times** (<100ms vs 10-30s first load)
- **Better performance** (no Python interpreter overhead)
- **Same functionality** (all features ported)
- **Shared core with mobile** (Rust core enables iOS/Android apps)

## Files Preserved

- `index.html` - Main HTML structure
- `styles.css` - PyScript-specific styles
- `app.js` - JavaScript helpers
- `manifest.json` - PWA manifest
- `sw.js` - Service worker for offline functionality
- `src/` - Python source modules
  - `main.py` - PyScript event handlers
  - `frame.py` - FrameDesign class & calculations
  - `conversions.py` - Unit conversion & formatting
  - `defaults.py` - Default values & constants
  - `export_text.py` - Text export functionality
  - `export_pdf.py` - PDF export with vector diagrams
  - `aspect_ratio.py` - Aspect ratio lock logic
  - `shareable_url.py` - URL parameter encoding/decoding
  - `config_manager.py` - Saved configurations
  - `data_backup.py` - Import/export all user data
  - `ui_helpers.py` - DOM manipulation helpers

## Technical Details

**Technology Stack:**
- Runtime: PyScript/Pyodide (Python compiled to WebAssembly)
- Visualization: matplotlib → SVG
- UI: HTML/CSS/JavaScript + Python event handlers
- Storage: Browser localStorage
- Deployment: GitHub Pages (static site)

**Key Features:**
- ✅ Full frame calculations
- ✅ Interactive visualizations with dimension callouts
- ✅ Vector PDF export (SVG embedded via svg2pdf.js)
- ✅ Text export
- ✅ Shareable URLs
- ✅ Saved custom sizes (localStorage)
- ✅ Aspect ratio locking
- ✅ Unit conversion (inches/mm)
- ✅ Progressive Web App (installable)

## Running Locally

```bash
# Must use HTTP server (CORS restrictions prevent file:// URLs)
python3 -m http.server 8000
# Open http://localhost:8000
```

## Value as Reference

This implementation remains valuable as:
- **Algorithm reference** - Python calculations are easier to read than Rust
- **UI patterns** - Proven UX that worked well
- **PDF export example** - Hybrid JS/Python approach with svg2pdf.js
- **Feature completeness** - All features successfully implemented
- **Test cases** - Python tests validate Rust implementation

## Migration Notes

All core functionality was successfully ported to the Rust/WASM version:
- Calculations: Validated via 26 regression tests against Python behavior
- Visualization: Rust SVG generation produces identical output
- PDF export: Ported to pure JavaScript with same layout/features
- Saved configs: Same localStorage keys, compatible data format
- Shareable URLs: Same binary encoding format (cross-compatible)

## Last Production Deployment

- **Date:** January 2026
- **Commit:** [To be filled when archiving]
- **Status:** Fully functional, all features working

---

**Archived:** 2026-01-06
**Reason:** Replaced by superior WASM implementation at same URL
**Preservation:** Reference implementation, algorithm validation
