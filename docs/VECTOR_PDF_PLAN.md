# Vector PDF Export Implementation Plan

## Current State
- PDF export uses 6x rasterized PNG of the SVG diagram
- Works well, but not infinitely scalable like true vector
- svg2pdf.js failed due to Pyodide JavaScript interop limitations

## Goal
Embed the frame diagram as native PDF vector paths for perfect quality at any zoom level.

---

## Option 1: Pure JavaScript PDF Export (Recommended)

**Approach**: Move PDF generation entirely to JavaScript, bypassing Pyodide interop issues.

### Implementation Steps

1. **Create JavaScript PDF export function** in index.html:
```javascript
async function exportPdfWithVector() {
    const { jsPDF } = window.jspdf;
    const pdf = new jsPDF();

    // Get SVG element
    const svg = document.querySelector("#matplotlib-canvas svg");

    // Use svg2pdf.js (works natively in JS)
    await pdf.svg(svg, {
        x: 10,
        y: 5,
        width: 190,
        height: 140
    });

    // Add text content (call back to Python or duplicate logic)
    // ... text generation ...

    pdf.save("frame_design_summary.pdf");
}
```

2. **Expose to Python** via window object:
```javascript
window.exportPdfWithVector = exportPdfWithVector;
```

3. **Call from Python**:
```python
from js import window
window.exportPdfWithVector()
```

### Challenges
- Text content generation is currently in Python (`generate_pdf_content()`)
- Would need to either:
  - a) Duplicate text logic in JavaScript
  - b) Pass calculated values from Python to JS function
  - c) Have Python generate text-only PDF, JS add diagram, then merge (complex)

### Estimated Effort: 2-3 hours

---

## Option 2: Matplotlib PDF Backend

**Approach**: Use matplotlib's native PDF output instead of SVG.

### Implementation Steps

1. **Change matplotlib output to PDF**:
```python
from io import BytesIO
pdf_buffer = BytesIO()
fig.savefig(pdf_buffer, format='pdf', bbox_inches='tight')
pdf_bytes = pdf_buffer.getvalue()
```

2. **Embed PDF page into jsPDF** - This is the hard part. jsPDF doesn't natively support embedding PDF pages.

### Challenges
- jsPDF cannot embed existing PDF content
- Would need pdf-lib.js or similar to merge PDFs
- Complex multi-library coordination
- Matplotlib PDF backend may not be available in Pyodide

### Estimated Effort: 4-6 hours (high uncertainty)

---

## Option 3: Hybrid Approach

**Approach**: Generate diagram in JS, text in Python, combine.

### Implementation Steps

1. **Python generates text-only PDF**:
```python
pdf = jspdf.jsPDF.new()
generate_pdf_content(pdf, start_y=155)  # Leave room at top
# Don't save yet - store reference
```

2. **JavaScript adds vector diagram to existing PDF**:
```javascript
async function addVectorDiagram(pdf) {
    const svg = document.querySelector("#matplotlib-canvas svg");
    await pdf.svg(svg, { x: 10, y: 5, width: 190, height: 140 });
    pdf.save("frame_design_summary.pdf");
}
```

3. **Coordinate between Python and JS**:
```python
# Python creates PDF with text
pdf = jspdf.jsPDF.new()
generate_pdf_content(pdf, start_y=155)

# Pass to JS for diagram
from js import window
window.addVectorDiagram(pdf)
```

### Challenges
- Passing jsPDF instance between Python and JS contexts
- Async coordination
- May still hit interop issues

### Estimated Effort: 2-4 hours

---

## Option 4: Full JavaScript Rewrite of PDF Export

**Approach**: Move all PDF logic to JavaScript, read values from DOM.

### Implementation Steps

1. **Create comprehensive JS function** that:
   - Reads all input values from DOM
   - Performs calculations (or reads from result divs)
   - Generates complete PDF with vector diagram and text

2. **Single button triggers JS directly**:
```html
<button onclick="generateFullPdf()">Export PDF</button>
```

### Advantages
- No Pyodide interop issues
- svg2pdf.js works natively
- Faster execution

### Disadvantages
- Duplicates calculation/formatting logic
- Two codebases to maintain
- Loses Python's nice string formatting

### Estimated Effort: 3-4 hours

---

## Recommendation

**Option 1 (Pure JS PDF Export)** or **Option 3 (Hybrid)** are most practical.

### Suggested Implementation Order

1. **Quick Win**: Try Option 3 first - keep Python text generation, add JS vector diagram
2. **If that fails**: Fall back to Option 1 - full JS implementation
3. **Skip**: Option 2 (too complex, uncertain)

### Prerequisites
- Add svg2pdf.js back to HTML head:
```html
<script src="https://cdnjs.cloudflare.com/ajax/libs/svg2pdf.js/2.2.3/svg2pdf.umd.min.js"></script>
```

---

## Testing Vector Output

To verify PDF contains vectors (not raster):
1. Open PDF in viewer
2. Zoom to 400-800%
3. Vector: Lines stay crisp
4. Raster: Lines become pixelated

Or check file size - vector PDFs are typically smaller than high-res raster.

---

## Notes

- Current 6x raster approach produces ~300 DPI at print size, which is acceptable for most uses
- Vector is "nice to have" not "must have"
- Consider effort vs. benefit before implementing

---

## Implementation Status: ✅ COMPLETED

**Solution Used**: Option 3 (Hybrid Approach) with modifications

### What Was Implemented

1. **JavaScript helper function** (`window.createPdfWithVectorSvg`):
   - Caches `_jsPDF` constructor at page load (before Pyodide interference)
   - Uses `window.svg2pdf.svg2pdf()` fallback when `pdf.svg()` unavailable
   - Returns jsPDF instance to Python via Promise

2. **Python PDF handler**:
   - Calls JS helper with diagram dimensions
   - Receives PDF with vector diagram
   - Adds text content using `generate_pdf_content()`
   - Saves final PDF

3. **Matplotlib configuration**:
   ```python
   matplotlib.rcParams['svg.fonttype'] = 'none'  # Keep text as SVG text elements
   matplotlib.rcParams['font.sans-serif'] = ['DejaVu Sans', ...]  # Match Pyodide metrics
   ```

### Key Learnings

- Pyodide-wrapped JS objects don't expose prototype extensions (like svg2pdf's methods)
- Must cache jsPDF constructor before Pyodide loads
- `svg.fonttype = 'none'` required for svg2pdf.js compatibility (fonttype 42 causes missing chars)
- Font rendering differs between matplotlib (DejaVu Sans) and browser/PDF viewer
- Reduced bbox padding (0.2-0.25) compensates for font metric differences

### Known Limitation / Future Improvement

The vector PDF uses `svg.fonttype = 'none'` which outputs `<text>` elements with CSS font-family. This means:
- matplotlib calculates bboxes using its bundled DejaVu Sans
- Browser/PDF viewer renders using system fonts (may differ visually)
- `svg.fonttype = 42` (embedded fonts) looks correct but svg2pdf.js can't handle it (missing characters)

Potential future solutions:
- Find a font that svg2pdf.js handles correctly with fonttype 42
- Use a different PDF library that supports embedded fonts
- Accept the visual difference (current approach)

### CDN Scripts Required
```html
<script src="https://cdnjs.cloudflare.com/ajax/libs/jspdf/2.5.1/jspdf.umd.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/svg2pdf.js@2.2.3/dist/svg2pdf.umd.min.js"></script>
```

---

**Created**: 2025-12-11
**Implemented**: 2025-12-11
**Status**: ✅ Complete - Vector PDF export working with hybrid JS/Python approach
