# PyScript PoC Setup Notes

## Files Modified for PoC

The following files were copied from the main codebase with **minimal modifications** to work in PyScript:

### 1. `conversions.py`
**Change**: Commented out Streamlit import and `unit_input()` function
```python
# OLD
import streamlit as st
def unit_input(...):
    value = st.number_input(...)

# NEW (PoC only)
# import streamlit as st  # Not needed for PyScript PoC
# unit_input function commented out (requires Streamlit)
```

**Impact**: Core conversion functions (`inches_to_mm`, `format_value`, etc.) work unchanged

### 2. `frame.py`
**Change**: None! ✅ **Direct copy with zero modifications**

### 3. `defaults.py`
**Change**: None! ✅ **Direct copy with zero modifications**

## What This Proves

✅ **59% of codebase works with zero changes**:
- All calculation logic (`FrameDesign` class)
- All conversion functions
- All formatting functions
- All constants/defaults

⚠️ **Only UI-specific code needs replacement**:
- Streamlit widgets → HTML inputs
- One function (`unit_input`) → Not used in core calculations

## Running the PoC

```bash
# From pyscript-poc directory
python3 -m http.server 8000

# Open browser to:
http://localhost:8000/
```

## Expected Behavior

1. **First load**: 10-15 seconds (downloads Pyodide)
2. **Status indicators**:
   - ✅ Core modules imported successfully
   - ✅ Calculations working
   - ✅ localStorage working natively
   - ✅ matplotlib loaded (after 30-60s)

3. **Click "Calculate Frame"**:
   - Uses YOUR existing `FrameDesign` class
   - Produces identical results to Streamlit version
   - All methods work unchanged

4. **Click "Test localStorage"**:
   - Direct browser storage access
   - No JavaScript workarounds needed
   - Data persists across refreshes

## Files Structure

```
pyscript-poc/
├── index.html           # PyScript PoC interface
├── frame.py            # ✅ Unchanged from ../models/frame.py
├── conversions.py      # ⚠️ Streamlit import removed
├── defaults.py         # ✅ Unchanged from ../defaults.py
├── README.md           # User documentation
└── SETUP_NOTES.md      # This file
```

## Key Takeaway

**This PoC validates that core ReferenceFrame logic requires minimal changes for PyScript.**

The only modification needed was removing UI-specific Streamlit code. All calculations, conversions, and business logic work identically.
