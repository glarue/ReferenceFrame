# ReferenceFrame Style Matching Plan

**Goal:** Match the visual style of the Stitch UI reference example as closely as possible

**Reference:** `/home/glarue/stitch_ui_examples/stitch_artwork_dimensions_input (4)/code.html`

---

## Analysis of Reference Style

### Color Palette
- **Primary:** #277ca0 (teal blue) - already matches!
- **Backgrounds:** #f6f7f8 (light), #131c1f (dark)
- **Surface:** #ffffff (light), #1c262a (dark cards)
- **Borders:** slate-200 (#e2e8f0), slate-800/50 (rgba with opacity)
- **Text Primary:** slate-900 / white
- **Text Muted:** slate-500 / slate-400
- **Text Dimmed:** slate-400 / slate-600

### Typography Hierarchy
1. **Section Headers:** 14px, bold, uppercase, wide tracking, primary color, with 18px icon
2. **Card Labels:** 10-12px, uppercase, tracking-wider, slate-500, bold
3. **Values:** 16-18px, bold, mono font, primary or white
4. **Small Text:** 10px, uppercase, tracking-widest, slate-400

### Layout Patterns
- **Section spacing:** px-4 mb-5 (16px horizontal, 20px bottom)
- **Card padding:** p-4 (16px) or p-3.5 (14px) for rows
- **Border radius:** 12px (xl) for cards, 8px (lg) for inner elements
- **Shadows:** shadow-sm on all cards
- **Gaps:** gap-2 (8px), gap-3 (12px), gap-4 (16px)

### Component Patterns

#### Results Cards
- Container: `bg-surface, rounded-xl, shadow-sm, border, overflow-hidden`
- Rows: `divide-y divide-slate-100/700, p-3.5, hover:bg-slate-50/white/5`
- Important rows: `bg-primary/5 hover:bg-primary/10`
- Label-value: `flex justify-between items-center`

#### Section Headers
- Icon + text: `flex items-center gap-2`
- Positioned OUTSIDE cards: `px-1 pb-2`
- Material icon size: `text-[18px]`

#### Progress Bars (Depth Gauge)
- Container: `bg-slate-200/700, h-1.5, rounded-full, overflow-hidden`
- Fill: `h-full, rounded-full, bg-slate-500/400 or bg-orange-500`
- Width: 100% of parent

#### Table Structure (Cut List)
- Header: `bg-slate-50/slate-800/30, px-4 py-2, border-b`
- Rows: `px-4 py-3, border-b border-slate-100/800/50`
- Footer: `bg-slate-50/slate-800/30, px-4 py-2`

#### Warning Cards
- Left accent: `border-l-4 border-orange-500`
- Background: `bg-orange-500/10`
- Rounded right only: `rounded-r-lg`
- Icon + content: `flex gap-3 items-start`
- Inline highlights: `bg-orange-200/900/50 px-1 rounded`

#### Data Grids
- Layout: `grid grid-cols-2 gap-y-4 gap-x-6`
- Items with left border: `border-l-2 border-slate-200/700 pl-3`
- Important items: `border-l-2 border-primary, bg-primary/5, -my-2 py-2, rounded-r`

---

## Current State vs. Target

### ✅ Already Implemented
- Material Symbols icons
- Collapsible headers with icons
- Depth gauge with progress bars
- Cut list table format
- Warning cards with left accent
- Numeric stepper controls
- Unit toggle styling
- Technical grid background
- Primary color matches (#277ca0)

### 🔧 Needs Adjustment

#### Typography
- [ ] Font sizes need standardization (10px, 12px, 14px, 16px, 18px scale)
- [ ] Letter spacing needs consistency (tracking-wide, tracking-wider, tracking-widest)
- [ ] Section headers should be 14px (currently 13px)
- [ ] Small labels should be 10px with tracking-widest

#### Colors
- [ ] Results cards need subtle hover states (hover:bg-slate-50/white/5)
- [ ] Important values need bg-primary/5 background
- [ ] Borders need slate-200/slate-800/50 consistency
- [ ] Muted text should use slate-500/400 (not --rf-text-muted)

#### Layout
- [ ] Card shadows should be shadow-sm on all result cards
- [ ] Border radius should be 12px (xl) for cards consistently
- [ ] Section spacing should be px-4 mb-5
- [ ] Row padding should be p-3.5 in result tables

#### Results Display
- [ ] Convert bulleted lists to card-based label-value rows
- [ ] Add divide-y separators between rows
- [ ] Right-align values with mono font
- [ ] Add hover effects to rows
- [ ] Use grid layout for matboard data
- [ ] Add left-border accent to important items

---

## Implementation Plan

### Phase 1: Typography Refinement (1-2 hours)
**Priority: High** - Foundation for visual consistency

1. **Font Size Scale**
   - Update CSS variables to use 10/12/14/16/18px scale
   - Section headers: 14px → `text-sm`
   - Card labels: 12px → `text-xs`
   - Small labels: 10px → `text-[10px]`
   - Values: 16px → `text-base`

2. **Letter Spacing**
   - Section headers: `letter-spacing: 0.08em` (tracking-wider)
   - Labels: `letter-spacing: 0.05em` (tracking-wide)
   - Small text: `letter-spacing: 0.1em` (tracking-widest)

3. **Weight Hierarchy**
   - Headers: font-weight: 700 (bold)
   - Labels: font-weight: 600 (semibold) for larger, 700 (bold) for small
   - Values: font-weight: 700 (bold)

**Files:** `styles.css`

---

### Phase 2: Results Display Conversion (2-3 hours)
**Priority: High** - Most visible impact

1. **Convert Bulleted Lists to Card Rows**
   - Replace `<ul><li>` with card structure
   - Each result becomes a row: `flex justify-between items-center p-3.5`
   - Add dividers: `divide-y divide-slate-100 dark:divide-slate-700/50`
   - Add hover states: `hover:bg-slate-50 dark:hover:bg-white/5`

2. **Dimensions Section**
   ```html
   <section class="px-4 mb-5">
     <h3 class="section-header">
       <span class="material-symbols-outlined text-[18px]">aspect_ratio</span>
       Dimensions
     </h3>
     <div class="result-card">
       <div class="divide-y divide-slate-100 dark:divide-slate-700/50">
         <div class="result-row">
           <span class="result-label">Visible Opening</span>
           <span class="result-value">15 3/4" × 19 3/4"</span>
         </div>
         <!-- More rows... -->
       </div>
     </div>
   </section>
   ```

3. **Important Values Highlight**
   - Glazing cut size: `bg-primary/5 hover:bg-primary/10`
   - Text color: `text-primary`

4. **Matboard Section Grid**
   - Convert to 2-column grid
   - Add left border indicators
   - Highlight window cut with primary color and background

**Files:** `index.html` (JavaScript results generation), `styles.css`

---

### Phase 3: Color Refinement (1-2 hours)
**Priority: Medium** - Subtle but important consistency

1. **Update CSS Variables**
   ```css
   --rf-bg-light: #f6f7f8;
   --rf-bg-dark: #131c1f;
   --rf-surface-light: #ffffff;
   --rf-surface-dark: #1c262a;
   --rf-border-light: #e2e8f0;  /* slate-200 */
   --rf-border-dark: rgba(30, 41, 59, 0.5);  /* slate-800/50 */
   --rf-text-muted: #64748b;  /* slate-500 light */
   --rf-text-muted-dark: #94a3b8;  /* slate-400 dark */
   ```

2. **Apply Hover States**
   - Result rows: `hover:bg-slate-50 dark:hover:bg-white/5`
   - Buttons: existing hover states are good

3. **Border Consistency**
   - All cards: `border: 1px solid var(--rf-border)`
   - Dividers: `border-slate-200 dark:border-slate-800/50`

**Files:** `styles.css`

---

### Phase 4: Layout Consistency (1-2 hours)
**Priority: Medium** - Polish and spacing

1. **Section Spacing**
   - All sections: `px-4 mb-5` (16px horizontal, 20px bottom margin)
   - Section headers: `px-1 pb-2` (small horizontal, 8px bottom)

2. **Card Structure**
   - All result cards: `rounded-xl` (12px)
   - Shadow: `box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05)` (shadow-sm)
   - Border: 1px solid

3. **Row Padding**
   - Result rows: `padding: 14px` (p-3.5)
   - Table headers/footers: `px-4 py-2` (16px horizontal, 8px vertical)
   - Table rows: `px-4 py-3` (16px horizontal, 12px vertical)

**Files:** `styles.css`

---

### Phase 5: Input Controls Polish (2-3 hours)
**Priority: Low** - Inputs already functional, just styling

1. **Stepper Inputs**
   - Match reference rounded corners (8px inner elements)
   - Adjust button sizes if needed
   - Ensure consistent padding

2. **Field Labels**
   - Size: 12px
   - Color: slate-500/400
   - Spacing: consistent with results

3. **Section Cards**
   - Input sections should match result card styling
   - Same shadow, border, radius

**Files:** `styles.css`, possibly `index.html`

---

### Phase 6: Fine Details (1 hour)
**Priority: Low** - Final polish

1. **Scrollbar Styling** (Already have this)
   - Width: 4px
   - Thumb: primary color
   - Track: transparent

2. **Transitions**
   - All hover effects: `transition: background-color 150ms ease, color 150ms ease`
   - Button scales: `active:scale-[0.98]`

3. **Small Touches**
   - Footer backdrop blur (if applicable)
   - Focus states match hover
   - Selection color: `selection:bg-primary/30`

**Files:** `styles.css`

---

## CSS Class System

### Proposed New Classes

```css
/* Section Headers */
.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--rf-primary-blue);
  font-size: 14px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 0 4px 8px;
}

/* Result Cards */
.result-card {
  background: var(--rf-surface-light);
  border: 1px solid var(--rf-border-light);
  border-radius: 12px;
  box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  overflow: hidden;
}

/* Result Rows */
.result-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px;
  transition: background-color 150ms ease;
}

.result-row:hover {
  background: rgba(248, 250, 252, 1); /* slate-50 */
}

.result-row.highlight {
  background: rgba(39, 124, 160, 0.05); /* primary/5 */
}

.result-row.highlight:hover {
  background: rgba(39, 124, 160, 0.1); /* primary/10 */
}

/* Labels and Values */
.result-label {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--rf-text-muted);
}

.result-value {
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 700;
  color: var(--rf-text);
}

.result-value.highlight {
  color: var(--rf-primary-blue);
}

/* Data Grid (for matboard) */
.data-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px 24px;
}

.data-item {
  border-left: 2px solid var(--rf-border-light);
  padding-left: 12px;
}

.data-item.highlight {
  border-color: var(--rf-primary-blue);
  background: rgba(39, 124, 160, 0.05);
  margin: -8px;
  padding: 8px 8px 8px 12px;
  border-radius: 0 4px 4px 0;
}

.data-label {
  display: block;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--rf-text-muted);
  margin-bottom: 4px;
}

.data-value {
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 700;
  color: var(--rf-text);
}
```

---

## Testing Checklist

After each phase:
- [ ] Check light mode appearance
- [ ] Check dark mode appearance (if implemented)
- [ ] Verify responsive behavior (mobile, tablet, desktop)
- [ ] Test all hover states
- [ ] Ensure text remains readable
- [ ] Verify spacing consistency
- [ ] Check alignment of labels and values

---

## Notes

- **Don't break existing functionality** - only change styling
- **Maintain current JavaScript logic** - only update HTML structure in output
- **Keep accessibility** - ensure contrast ratios remain good
- **Test incrementally** - commit after each phase
- **Reference example is mobile-optimized** - our desktop layout can stay two-column

---

## Estimated Timeline

- **Phase 1:** 1-2 hours (Typography)
- **Phase 2:** 2-3 hours (Results Display) - **HIGHEST IMPACT**
- **Phase 3:** 1-2 hours (Colors)
- **Phase 4:** 1-2 hours (Layout)
- **Phase 5:** 2-3 hours (Input Controls)
- **Phase 6:** 1 hour (Details)

**Total:** 8-13 hours

**Recommended Order:** Phase 2 → Phase 1 → Phase 3 → Phase 4 → Phase 6 → Phase 5
(Start with highest visual impact)
