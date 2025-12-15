# ReferenceFrame

A browser-based picture frame calculator that runs entirely client-side using PyScript/Pyodide. Calculate frame dimensions, mat sizes, material requirements, and generate detailed diagrams - all with zero server costs and complete privacy.

**Status**: ✅ Production Ready - Fully functional web application

## Features

✅ **Core frame calculations**
- Calculate frame dimensions for any artwork size
- Support for custom mat widths (including asymmetric mats)
- Material thickness accounting (glazing, matboard, backing)
- Rabbet depth validation

✅ **Native browser storage**
- Settings persist automatically via localStorage
- Custom sizes saved locally
- No server required - 100% private

✅ **Interactive visualizations**
- Client-side matplotlib rendering
- Frame/matboard/artwork display
- Dimension annotations
- Vector PDF export

✅ **Progressive Web App**
- Installable on mobile/desktop
- Offline support via service worker
- Fast subsequent loads (cached assets)

## How to Run

### Option 1: Local Development Server

```bash
python3 -m http.server 8000

# Open browser to:
http://localhost:8000/
```

### Option 2: Deploy to GitHub Pages

See [DEPLOYMENT.md](docs/DEPLOYMENT.md) for deployment instructions.

## Performance

**Expected load times**:
- First visit: 10-15 seconds (downloads Pyodide runtime)
- Subsequent visits: <1 second (cached)
- matplotlib first render: 30-60 seconds (downloads packages)
- matplotlib cached: instant

## File Structure

```
ReferenceFrame/
├── index.html          # Main application
├── manifest.json       # PWA manifest
├── sw.js              # Service worker (offline support)
├── src/               # Python modules
│   ├── frame.py           # Core calculations
│   ├── conversions.py     # Unit formatting
│   ├── defaults.py        # Default values
│   ├── ui_helpers.py      # UI utilities
│   └── aspect_ratio.py    # Aspect ratio logic
├── tests/             # Unit tests
└── docs/              # Documentation
```

## Browser Console Output

Check DevTools → Console for initialization status:
```
✅ PyScript initialized successfully!
✅ ReferenceFrame modules loaded
✅ Settings restored from localStorage
```

## Unit Support

- **Inches** (default)
- **Millimeters** (toggle in UI)
- Auto-conversion between units
- Precision formatting for both systems

## Technical Stack

- **PyScript 2024.6.2** - Python in the browser
- **Pyodide 0.26.2** - Python runtime via WebAssembly
- **matplotlib** - Client-side visualization
- **jsPDF + svg2pdf.js** - Vector PDF export
- **Service Worker** - Offline support and caching

## Browser Compatibility

Tested and working on:
- Chrome/Edge (recommended)
- Firefox
- Safari (iOS and macOS)

Requires modern browser with WebAssembly support.

## Privacy & Offline Use

- **100% client-side** - No data sent to servers
- **Offline capable** - Works without internet after first load
- **Local storage only** - All settings stored in browser
- **No tracking** - Zero analytics or telemetry

## Documentation

- [DEPLOYMENT.md](docs/DEPLOYMENT.md) - Deployment guide (GitHub Pages, Cloudflare, etc.)
- [VALIDATION_STATUS.md](docs/VALIDATION_STATUS.md) - Feature status and testing
- [SETUP_NOTES.md](docs/SETUP_NOTES.md) - Technical implementation details
- [PERFORMANCE_OPTIMIZATION_PLAN.md](docs/PERFORMANCE_OPTIMIZATION_PLAN.md) - Load time optimizations

## Development

### Running Tests

```bash
# Python tests
pytest tests/

# Browser-based testing
python3 -m http.server 8000
# Open browser DevTools and check console output
```

### Project Structure

The application is built as a single-page app with modular Python code:
- Core logic in `src/` is imported by PyScript
- UI state managed via vanilla JavaScript + Python
- No build process required - deploy static files directly

## Troubleshooting

**"Failed to fetch" errors**:
- Must serve via HTTP server (not `file://`)
- Use `python3 -m http.server 8000`

**Long load times**:
- First load downloads Pyodide (~10-15 seconds)
- matplotlib adds another 30-60 seconds first time
- This is normal - subsequent loads are instant

**Calculations don't work**:
- Check browser console for Python errors
- Verify HTTP server is running
- Try hard refresh (Cmd+Shift+R / Ctrl+Shift+F5)

**matplotlib doesn't render**:
- Wait 60+ seconds on first load
- Check console for "matplotlib loaded!" message
- Subsequent renders are instant after packages download

## License

[Add your license here]