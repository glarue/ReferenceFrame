# ReferenceFrame UI Refresh Plan (Incremental)

> **Status (2026-06-10):** Largely implemented. Input steppers (`stepper-btn`) and the depth analysis visualization (`depth-bar`, `depth-fill`, capacity marker) exist in `platforms/web/index.html`/`styles.css`, though with different markup than sketched here. Historical reference only.

**Status:** Planning Phase
**Approach:** Small, focused improvements maintaining all existing functionality
**Inspiration:** Stitch UI examples (technical/blueprint aesthetic)

---

## Design Philosophy

**Keep:**
- Current two-column layout
- All existing features and functionality
- Responsive behavior
- Color palette (cerulean, seagrass, orange)
- Collapsible sections
- Current JavaScript logic

**Enhance:**
- Visual hierarchy with cards/elevation
- Input control styling (add steppers)
- Results presentation clarity
- Technical aesthetic (grid backgrounds, better labels)
- Warning/error messaging
- Depth analysis visualization

---

## Phase 1: Results Section Polish (Low Risk, High Impact)

### 1.1 Depth Analysis Visualization (2-3 hours)
**File:** `platforms/web/index.html` (JavaScript section), `styles.css`

**Current State:**
- Text-only depth comparison: "Clearance: +0.03"" or "Shortfall: -0.12""
- Simple color-coded text (green/red)

**Proposed Change:**
```html
<!-- Instead of just text, add visual progress bars like Stitch example 4 -->
<div class="depth-comparison">
  <div class="depth-metric">
    <label>Package Depth</label>
    <span class="value">0.875"</span>
    <div class="progress-bar">
      <div class="progress-fill" style="width: 100%"></div>
    </div>
  </div>
  <div class="depth-metric">
    <label>Rabbet Depth</label>
    <span class="value">0.750"</span>
    <div class="progress-bar warning">
      <div class="progress-fill" style="width: 85%"></div>
    </div>
  </div>
</div>
```

**CSS Addition:**
```css
.depth-comparison {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin: 12px 0;
}

.depth-metric {
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.depth-metric label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--rf-gray);
}

.depth-metric .value {
    font-size: 18px;
    font-weight: 700;
    font-family: var(--font-mono);
}

.progress-bar {
    width: 100%;
    height: 6px;
    background: var(--md-surface-container-high);
    border-radius: 3px;
    overflow: hidden;
}

.progress-fill {
    height: 100%;
    background: var(--rf-gray);
    transition: width var(--md-motion-duration-medium) var(--md-easing-emphasized);
}

.progress-bar.warning .progress-fill {
    background: var(--rf-warning-orange);
}
```

**Impact:** Better visual understanding of depth constraints at a glance

---

### 1.2 Enhanced Warning/Error Cards (1-2 hours)
**File:** `styles.css` (update existing `.validation-error`, `.validation-warning`)

**Current State:**
- Simple colored backgrounds with text
- Small icons

**Proposed Enhancement:**
```css
/* Update existing validation styles */
.validation-warning {
    background: rgba(249, 115, 22, 0.1); /* Orange tint */
    border-left: 4px solid var(--rf-warning-orange); /* Accent stripe */
    border-radius: 8px;
    padding: 12px 16px;
    margin: 8px 0;
    display: flex;
    gap: 12px;
    align-items: flex-start;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.validation-warning .icon {
    color: var(--rf-warning-orange);
    font-size: 20px;
    margin-top: 2px;
    flex-shrink: 0;
}

.validation-warning .content {
    flex: 1;
}

.validation-warning .title {
    font-weight: 700;
    font-size: 13px;
    color: var(--rf-warning-orange);
    margin-bottom: 4px;
}

.validation-warning .message {
    font-size: 12px;
    line-height: 1.5;
    color: var(--rf-text);
}

/* Same pattern for .validation-error with red colors */
```

**Impact:** Clearer visual hierarchy for validation feedback

---

### 1.3 Cut List Table Styling (1 hour)
**File:** `styles.css`, `index.html` (results section)

**Current State:**
- Bulleted list format
- Text-only

**Proposed Enhancement:**
```html
<div class="cut-list-table">
  <div class="cut-list-header">
    <span class="col-qty">QTY</span>
    <span class="col-component">COMPONENT</span>
    <span class="col-length">LENGTH</span>
  </div>
  <div class="cut-list-row">
    <span class="col-qty">2</span>
    <span class="col-component">Long Rails</span>
    <span class="col-length">22 1/8"</span>
  </div>
  <!-- ... more rows -->
  <div class="cut-list-footer">
    <span>Total Wood Length</span>
    <span class="total">~ 7 ft</span>
  </div>
</div>
```

**CSS:**
```css
.cut-list-table {
    background: var(--md-surface-container);
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--rf-border);
}

.cut-list-header {
    display: grid;
    grid-template-columns: 50px 1fr 100px;
    padding: 8px 16px;
    background: var(--md-surface-container-high);
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--rf-gray);
    border-bottom: 1px solid var(--rf-border);
}

.cut-list-row {
    display: grid;
    grid-template-columns: 50px 1fr 100px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--rf-line);
    transition: background var(--md-motion-duration-short) ease;
}

.cut-list-row:hover {
    background: var(--md-surface-container-high);
}

.cut-list-row .col-qty {
    color: var(--rf-gray);
    font-family: var(--font-mono);
    font-weight: 700;
}

.cut-list-row .col-length {
    font-family: var(--font-mono);
    font-weight: 700;
    color: var(--rf-primary);
    text-align: right;
}

.cut-list-footer {
    display: flex;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--md-surface-container-high);
    font-size: 12px;
}

.cut-list-footer .total {
    font-family: var(--font-mono);
    font-weight: 700;
}
```

**Impact:** More scannable, professional cut list presentation

---

## Phase 2: Input Controls Enhancement (Medium Risk)

### 2.0 Asymmetric Mat Borders (NEW FEATURE, 3-4 hours)
**File:** `index.html`, `styles.css`, WASM bindings

**Current State:**
- Mat width has two inputs: Top/Bottom (shared value), Sides (shared value)
- Symmetry toggle switches between "all sides same" and "TB different from Sides"

**Proposed Enhancement:**
```html
<!-- Replace current mat width inputs -->
<div class="mat-borders-group">
  <div class="input-row">
    <div class="field">
      <label>Top</label>
      <div class="input-stepper">
        <button class="stepper-btn minus">−</button>
        <input type="text" id="mat-width-top" value="2.0">
        <button class="stepper-btn plus">+</button>
      </div>
    </div>
    <div class="field">
      <label>Bottom</label>
      <div class="input-stepper">
        <button class="stepper-btn minus">−</button>
        <input type="text" id="mat-width-bottom" value="2.5">
        <button class="stepper-btn plus">+</button>
      </div>
    </div>
  </div>

  <div class="input-row">
    <div class="field">
      <label>Left</label>
      <div class="input-stepper">
        <button class="stepper-btn minus">−</button>
        <input type="text" id="mat-width-left" value="2.0">
        <button class="stepper-btn plus">+</button>
      </div>
    </div>
    <div class="field">
      <label>Right</label>
      <div class="input-stepper">
        <button class="stepper-btn minus">−</button>
        <input type="text" id="mat-width-right" value="2.0">
        <button class="stepper-btn plus">+</button>
      </div>
    </div>
  </div>

  <!-- Symmetry helpers -->
  <div class="mat-symmetry-controls">
    <button class="btn-secondary-compact" onclick="makeMatSymmetric('all')">
      All Equal
    </button>
    <button class="btn-secondary-compact" onclick="makeMatSymmetric('tb-lr')">
      TB = LR
    </button>
    <button class="btn-secondary-compact" onclick="makeMatSymmetric('tb')">
      T = B
    </button>
    <button class="btn-secondary-compact" onclick="makeMatSymmetric('lr')">
      L = R
    </button>
  </div>
</div>
```

**JavaScript Logic:**
```javascript
function makeMatSymmetric(mode) {
    const top = parseFloat(document.getElementById('mat-width-top').value) || 2.0;
    const bottom = parseFloat(document.getElementById('mat-width-bottom').value) || 2.0;
    const left = parseFloat(document.getElementById('mat-width-left').value) || 2.0;
    const right = parseFloat(document.getElementById('mat-width-right').value) || 2.0;

    switch(mode) {
        case 'all':
            // Use top value for all
            document.getElementById('mat-width-bottom').value = top;
            document.getElementById('mat-width-left').value = top;
            document.getElementById('mat-width-right').value = top;
            break;
        case 'tb-lr':
            // Top=Bottom, Left=Right (current default behavior)
            document.getElementById('mat-width-bottom').value = top;
            document.getElementById('mat-width-right').value = left;
            break;
        case 'tb':
            // Top = Bottom only
            document.getElementById('mat-width-bottom').value = top;
            break;
        case 'lr':
            // Left = Right only
            document.getElementById('mat-width-right').value = left;
            break;
    }

    // Trigger recalculation
    calculate();
}
```

**WASM Changes Required:**
- Update `FrameDesign` struct to accept 4 mat values instead of 2
- Or keep 2-value internal structure and calculate visible dimensions accordingly
- Minimal Rust changes - calculations already support asymmetric mats

**Default Behavior:**
- Initially all 4 fields show same value (e.g., 2.0)
- User can edit any field independently
- "TB = LR" button restores current default behavior (top/bottom same, left/right same)
- "All Equal" button makes all 4 values match top value

**Impact:** Enables traditional weighting (larger bottom mat) or custom artistic layouts

### 2.1 Numeric Stepper Controls (3-4 hours)
**File:** `index.html`, `styles.css`

**Current State:**
- Plain number inputs
- No increment/decrement buttons

**Proposed Addition:**
```html
<!-- Wrap numeric inputs with stepper controls -->
<div class="input-stepper">
  <button class="stepper-btn minus" onclick="decrementValue('mat-width-tb')">−</button>
  <input type="text" id="mat-width-tb" value="2.0">
  <button class="stepper-btn plus" onclick="incrementValue('mat-width-tb')">+</button>
</div>
```

**CSS:**
```css
.input-stepper {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--md-surface-container);
    border: 1px solid var(--rf-border);
    border-radius: 8px;
    padding: 4px;
    transition: border-color var(--md-motion-duration-short) ease,
                box-shadow var(--md-motion-duration-short) ease;
}

.input-stepper:focus-within {
    border-color: var(--rf-primary);
    box-shadow: 0 0 0 1px var(--rf-primary);
}

.input-stepper input {
    flex: 1;
    border: none;
    background: transparent;
    text-align: center;
    font-size: 16px;
    font-weight: 700;
    font-family: var(--font-mono);
    padding: 6px;
}

.input-stepper input:focus {
    outline: none;
    box-shadow: none;
}

.stepper-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--md-surface-container-high);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    color: var(--rf-text);
    cursor: pointer;
    transition: all var(--md-motion-duration-short) ease;
}

.stepper-btn:hover {
    background: var(--rf-primary);
    color: white;
}

.stepper-btn:active {
    transform: scale(0.95);
}
```

**JavaScript:**
```javascript
function incrementValue(inputId, step = 0.125) {
    const input = document.getElementById(inputId);
    const current = parseFloat(input.value) || 0;
    input.value = (current + step).toFixed(3).replace(/\.?0+$/, '');
    input.dispatchEvent(new Event('input', { bubbles: true }));
}

function decrementValue(inputId, step = 0.125) {
    const input = document.getElementById(inputId);
    const current = parseFloat(input.value) || 0;
    const newValue = Math.max(0.0625, current - step); // Min 1/16"
    input.value = newValue.toFixed(3).replace(/\.?0+$/, '');
    input.dispatchEvent(new Event('input', { bubbles: true }));
}
```

**Where to Apply:**
- Mat width inputs (top/bottom, sides)
- Mat overlap
- Frame width, frame depth
- Rabbet width, rabbet depth
- Material thicknesses

**NOT for:**
- Artwork dimensions (keep current H×W input pair - too complex for steppers)

**Impact:** Easier fine-tuning of dimensions, less typing

---

### 2.2 Enhanced Collapsible Headers (1-2 hours)
**File:** `styles.css`

**Current State:**
- Text header with small arrow icon
- Simple blue hover

**Proposed Enhancement:**
```css
.collapsible-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    background: var(--md-surface-container);
    border-radius: 10px;
    cursor: pointer;
    transition: all var(--md-motion-duration-short) ease;
    margin-bottom: 8px;
}

.collapsible-header:hover {
    background: var(--md-surface-container-high);
}

.collapsible-header.expanded {
    background: var(--md-surface-container-high);
    border-bottom-left-radius: 0;
    border-bottom-right-radius: 0;
    margin-bottom: 0;
}

.collapsible-header-left {
    display: flex;
    align-items: center;
    gap: 12px;
}

.collapsible-icon {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: rgba(39, 124, 160, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--rf-primary);
    font-size: 20px;
}

.collapsible-title {
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
}

.collapsible-header .toggle-icon {
    transition: transform var(--md-motion-duration-medium) var(--md-easing-emphasized);
}

.collapsible-header.expanded .toggle-icon {
    transform: rotate(180deg);
}
```

**HTML Update:**
```html
<div class="collapsible-header" onclick="toggleSection('advanced')">
  <div class="collapsible-header-left">
    <span class="collapsible-icon">⚙</span>
    <span class="collapsible-title">Advanced Options</span>
  </div>
  <span class="toggle-icon">▼</span>
</div>
```

**Impact:** More prominent, easier-to-identify section headers

---

### 2.3 Improved Unit Toggle (1 hour)
**File:** `styles.css`

**Current State:**
- Segmented control, works well
- Could use slight visual polish

**Proposed Enhancement:**
```css
/* Update existing .unit-toggle */
.unit-toggle {
    display: flex;
    background: var(--md-surface-container-high);
    padding: 4px;
    border-radius: 10px;
    gap: 4px;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.1);
}

.unit-toggle button {
    padding: 6px 16px;
    border-radius: 6px;
    background: transparent;
    border: none;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--rf-gray);
    cursor: pointer;
    transition: all var(--md-motion-duration-short) ease;
    position: relative;
}

.unit-toggle button.active {
    background: white;
    color: var(--rf-primary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1),
                0 0 0 1px rgba(39, 124, 160, 0.1);
}

.unit-toggle button:hover:not(.active) {
    color: var(--rf-text);
}
```

**Impact:** Cleaner, more tactile unit switcher

---

## Phase 3: Visualization Polish (Low Risk)

### 3.1 Technical Grid Background (2-3 hours)
**File:** `styles.css`, possibly `core/src/visualization/style.rs`

**Current State:**
- Plain background behind SVG diagrams
- SVG diagrams use current color scheme (orange, teal, gray)

**Proposed Addition:**
```css
#visualization-container {
    position: relative;
    background: var(--md-surface-container);
    border-radius: 12px;
    padding: 16px;
    /* Add subtle technical grid pattern */
    background-image:
        linear-gradient(to right, rgba(39, 124, 160, 0.03) 1px, transparent 1px),
        linear-gradient(to bottom, rgba(39, 124, 160, 0.03) 1px, transparent 1px);
    background-size: 20px 20px;
    background-position: -1px -1px;
}

/* Major grid lines every 100px - very subtle */
#visualization-container::before {
    content: '';
    position: absolute;
    inset: 0;
    background-image:
        linear-gradient(to right, rgba(39, 124, 160, 0.08) 1px, transparent 1px),
        linear-gradient(to bottom, rgba(39, 124, 160, 0.08) 1px, transparent 1px);
    background-size: 100px 100px;
    background-position: -1px -1px;
    pointer-events: none;
    border-radius: 12px;
}
```

**Important Considerations:**
- Grid opacity must be VERY subtle (0.03-0.08) to avoid competing with diagram lines
- May need to adjust SVG diagram colors for better contrast:
  - Consider slightly increasing stroke widths (current: 1.5-2.0 → maybe 2.0-2.5)
  - Or adding subtle glow/outline to important diagram elements
  - Test with both plan view and section view
- Should be toggleable via settings if users find it too busy
- Alternative: Only show grid on hover/focus of visualization area

**Testing Required:**
1. View plan view with grid - check rabbet (orange), mat (gray), frame (dark gray) contrast
2. View section view with grid - check all material layers are still distinct
3. Test on both light and dark backgrounds
4. Verify dimension callouts (blue) remain readable

**Impact:** More technical/blueprint aesthetic matching Stitch example 3, but must not reduce diagram clarity

---

### 3.2 Diagram Label Enhancement (SKIP - Too Complex)
**File:** Would require `core/src/visualization/svg_writer.rs` changes

**Current State:**
- Material labels with leader lines work well
- Dimension callouts are clear

**Decision:** SKIP this enhancement
- Current diagrams are already good quality
- Rust changes add risk and complexity
- Focus CSS changes on container styling only
- Diagram clarity is more important than aesthetic polish

---

## Phase 4: Layout Refinements (Optional Polish)

### 4.0 Tab-Based Layout (Optional Alternative, 3-4 hours)
**File:** `index.html`, `styles.css`

**Current State:**
- Two-column layout: inputs left, visualization/results right
- Works well on desktop, stacks on mobile

**Proposed Alternative (OPTIONAL):**
```html
<!-- Add tab navigation -->
<div class="main-tabs">
  <button class="tab-btn active" onclick="showTab('inputs')">Design Inputs</button>
  <button class="tab-btn" onclick="showTab('visualization')">Visualization</button>
  <button class="tab-btn" onclick="showTab('results')">Results</button>
</div>

<div class="tab-content active" id="inputs-tab">
  <!-- Current left column content -->
</div>

<div class="tab-content" id="visualization-tab">
  <!-- Diagram display -->
</div>

<div class="tab-content" id="results-tab">
  <!-- Calculation results -->
</div>
```

**CSS:**
```css
.main-tabs {
    display: flex;
    gap: 4px;
    background: var(--md-surface-container-high);
    padding: 4px;
    border-radius: 10px;
    margin-bottom: 16px;
}

.tab-btn {
    flex: 1;
    padding: 10px 16px;
    border: none;
    background: transparent;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--rf-gray);
    cursor: pointer;
    transition: all var(--md-motion-duration-short) ease;
}

.tab-btn.active {
    background: white;
    color: var(--rf-primary);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.tab-content {
    display: none;
}

.tab-content.active {
    display: block;
}

/* On desktop, optionally show tabs side-by-side with toggle */
@media (min-width: 1024px) {
    .layout-toggle {
        /* Button to switch between tabbed and two-column */
    }
}
```

**Pros:**
- Cleaner on smaller screens
- Full width for visualization
- Easier mobile navigation

**Cons:**
- Can't see inputs and results simultaneously on desktop
- Requires clicking to switch views
- More complex state management

**Decision Criteria:**
- Implement ONLY if user wants to test both layouts
- Should be easy to toggle via settings or URL param (`?layout=tabs` vs `?layout=columns`)
- Keep two-column as default, tabs as optional
- If non-trivial to make toggleable, **skip entirely and keep current layout**

**Implementation Notes:**
- Add layout preference to localStorage
- Settings modal should have "Layout: Columns / Tabs" radio buttons
- JavaScript to handle tab switching and state
- Ensure auto-calculate still works when switching tabs

### 4.1 Card-Based Results (2-3 hours)
**File:** `index.html`, `styles.css`

**Current State:**
- Results in a flat section
- Works fine but could use more structure

**Proposed Enhancement:**
```html
<!-- Group results into semantic cards -->
<div class="results-cards">
  <div class="result-card">
    <h4 class="card-header">
      <span class="card-icon">📐</span>
      Dimensions
    </h4>
    <div class="card-content">
      <!-- Visible opening, frame outside, etc. -->
    </div>
  </div>

  <div class="result-card">
    <h4 class="card-header">
      <span class="card-icon">📊</span>
      Depth Analysis
    </h4>
    <div class="card-content">
      <!-- Depth comparison with progress bars -->
    </div>
  </div>

  <div class="result-card">
    <h4 class="card-header">
      <span class="card-icon">✂️</span>
      Cut List
    </h4>
    <div class="card-content">
      <!-- Cut list table -->
    </div>
  </div>
</div>
```

**CSS:**
```css
.results-cards {
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.result-card {
    background: var(--md-surface-container);
    border: 1px solid var(--rf-border);
    border-radius: 12px;
    overflow: hidden;
}

.card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--md-surface-container-high);
    border-bottom: 1px solid var(--rf-border);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--rf-primary);
}

.card-icon {
    font-size: 16px;
}

.card-content {
    padding: 16px;
}
```

**Impact:** Better visual grouping, easier to scan results

---

### 4.2 Preview Card for Inputs (3-4 hours, OPTIONAL)
**File:** `index.html`, `styles.css`

**Inspired by:** Stitch example 5 (preview at top)

**Current State:**
- No visual preview in input column
- All abstract numbers

**Proposed Addition:**
```html
<!-- Add at top of left column -->
<div class="design-preview-card">
  <div class="preview-container">
    <!-- SVG mini preview generated via WASM -->
    <div id="mini-preview"></div>
    <div class="preview-dimensions">16" × 20"</div>
  </div>
</div>
```

**CSS:**
```css
.design-preview-card {
    background: var(--md-surface-container);
    border-radius: 16px;
    padding: 12px;
    margin-bottom: 16px;
    border: 1px solid var(--rf-border);
}

.preview-container {
    position: relative;
    aspect-ratio: 16 / 10;
    background: var(--md-surface-container-high);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
}

#mini-preview {
    max-width: 80%;
    max-height: 80%;
}

.preview-dimensions {
    position: absolute;
    bottom: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(0, 0, 0, 0.7);
    color: white;
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-mono);
}
```

**Note:** This is more complex as it requires generating a mini SVG preview. **Skip unless user specifically wants it.**

---

## Implementation Priority

### High Priority (Do First)
1. ✅ **Depth Analysis Progress Bars** - Biggest visual impact, low risk
2. ✅ **Enhanced Warning/Error Cards** - Improves UX, low risk
3. ✅ **Cut List Table** - Professional polish, low risk
4. ✅ **Asymmetric Mat Borders** - User-requested feature, moderate complexity

### Medium Priority (Do After High)
5. **Numeric Stepper Controls** - UX improvement, pairs well with mat borders
6. **Enhanced Collapsible Headers** - Visual polish, low risk
7. **Improved Unit Toggle** - Minor polish
8. **Technical Grid Background** - Aesthetic improvement, test carefully to avoid clutter

### Low Priority (Nice to Have)
9. **Tab-Based Layout** - ONLY if user wants to test alternative, otherwise skip
10. **Card-Based Results** - Mostly cosmetic, moderate effort
11. **Preview Card** - High effort, moderate benefit

### Skip Entirely
- **Diagram Label Enhancement** - Requires Rust changes, current diagrams are good

---

## Testing Checklist (For Each Change)

- [ ] Desktop layout (1920×1080, 1440×900)
- [ ] Tablet layout (768px breakpoint)
- [ ] Mobile layout (500px breakpoint)
- [ ] Auto-calculate still works
- [ ] All inputs remain functional
- [ ] Collapsible sections still toggle
- [ ] Unit conversion updates correctly
- [ ] Export PDF/Text still works
- [ ] Saved configs still load/save

---

## Rollback Strategy

Each phase is independent:
- Changes are isolated to specific CSS classes
- Can revert individual commits without affecting others
- Keep old CSS classes alongside new ones during testing
- Remove old classes only after confirming new ones work

---

## Estimated Timeline

- **Phase 1 (Results Polish):** 4-6 hours
- **Phase 2 (Input Controls + Asymmetric Mats):** 9-11 hours (includes new feature)
- **Phase 3 (Visualization):** 2-3 hours (grid background only, test carefully)
- **Phase 4 (Layout Refinements):** 3-4 hours for tabs (ONLY if requested), 5-7 hours for cards (optional)

**Total:** 15-20 hours for core improvements (Phases 1-3 with asymmetric mats)

**Notes:**
- Asymmetric mat borders is a feature addition, not just polish
- Technical grid background must be very subtle - priority is diagram clarity
- Tab-based layout is optional alternative to two-column - skip if not trivial to toggle

---

## Next Steps

1. Review this plan - decide which phases to implement
2. Create a new git branch: `git checkout -b ui-refresh`
3. Start with Phase 1.1 (depth progress bars)
4. Test thoroughly on localhost
5. Commit each change individually
6. Deploy to production incrementally

**Philosophy:** Small, tested, reversible changes. Each commit should be deployable.
