# UI Sync Strategy: Flutter (iOS) ↔ WASM (Web)

> **Status**: Planned (documented 2026-01-22)
> **Goal**: Maintain visual parity between platforms with minimal manual effort

## Problem Statement

ReferenceFrame has two UI implementations:
- **Flutter** (`platforms/mobile/`) - iOS/Android app
- **Web** (`platforms/web/`) - WASM-based browser app

Both share a Rust core for calculations, but UI code is platform-specific (Dart vs HTML/CSS/JS). Without a systematic sync process, design drift occurs over time.

## Current State Analysis

### What's Already Shared

| Resource | Location | Consumers |
|----------|----------|-----------|
| Color palette (7 colors + variants) | `core/data/presets.json` | Both platforms |
| Dimension presets | `core/data/presets.json` | Both platforms |
| Default values | `core/data/presets.json` | Both platforms |
| Calculation logic | `core/src/*.rs` | Both platforms (via FFI) |

### What's Duplicated (Needs Sync)

| Resource | Flutter Location | Web Location | Sync Method |
|----------|------------------|--------------|-------------|
| Color hex values | `main.dart` AppColors (const) | `styles.css` CSS variables | Manual transcription |
| Semantic color mappings | `color_category.dart` | `styles.css` semantic vars | Manual transcription |
| Spacing values | Scattered in widgets | `styles.css` `--rf-space-*` | None (different approaches) |
| Typography | Material theme defaults | `styles.css` typescale | None (different approaches) |
| Border radii | Widget-level constants | `styles.css` `--md-shape-*` | None |

### Platform-Specific Features

| Feature | Flutter | Web | Notes |
|---------|---------|-----|-------|
| Color customization | Implemented (ColorManager) | Not implemented | Gap to address |
| Dark mode | Implemented (AppTheme) | Partial (CSS only) | Web needs JS toggle |
| Responsive layout | Material adaptive | CSS media queries | Different but equivalent |

---

## Proposed Solution: Token-Based Sync with Change Detection

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                  Source of Truth                         │
│  ┌─────────────────┐  ┌─────────────────────────────┐   │
│  │ presets.json    │  │ design_tokens.json (new)    │   │
│  │ - colors        │  │ - spacing                   │   │
│  │ - presets       │  │ - typography                │   │
│  │ - defaults      │  │ - radii                     │   │
│  └────────┬────────┘  └──────────────┬──────────────┘   │
└───────────┼──────────────────────────┼──────────────────┘
            │                          │
            ▼                          ▼
┌───────────────────────┐    ┌───────────────────────┐
│  Rust FFI Layer       │    │  Rust FFI Layer       │
│  getPresetsJson()     │    │  getDesignTokens()    │
└───────────┬───────────┘    └───────────┬───────────┘
            │                            │
     ┌──────┴──────┐              ┌──────┴──────┐
     ▼             ▼              ▼             ▼
┌─────────┐  ┌─────────┐    ┌─────────┐  ┌─────────┐
│ Flutter │  │   Web   │    │ Flutter │  │   Web   │
│ Dart    │  │ JS/CSS  │    │ Dart    │  │ JS/CSS  │
└─────────┘  └─────────┘    └─────────┘  └─────────┘
```

---

## Implementation Phases

### Phase 1: Extend Design Token System

**Create `core/data/design_tokens.json`:**

```json
{
  "version": 1,
  "spacing": {
    "unit": "px",
    "scale": {
      "xs": 4,
      "sm": 8,
      "md": 12,
      "lg": 16,
      "xl": 24,
      "2xl": 32
    }
  },
  "radius": {
    "unit": "px",
    "scale": {
      "xs": 4,
      "sm": 8,
      "md": 12,
      "lg": 16
    }
  },
  "typography": {
    "label": {
      "size": 10,
      "weight": 600,
      "letterSpacing": 1.2,
      "transform": "uppercase"
    },
    "body": {
      "size": 14,
      "weight": 400,
      "lineHeight": 1.5
    },
    "bodySmall": {
      "size": 12,
      "weight": 400,
      "lineHeight": 1.4
    },
    "heading": {
      "size": 16,
      "weight": 600,
      "lineHeight": 1.3
    },
    "headingLarge": {
      "size": 18,
      "weight": 600,
      "lineHeight": 1.3
    }
  },
  "elevation": {
    "none": 0,
    "sm": 1,
    "md": 2,
    "lg": 4
  },
  "animation": {
    "durationFast": 150,
    "durationNormal": 300,
    "durationSlow": 500,
    "easing": "cubic-bezier(0.2, 0, 0, 1)"
  }
}
```

**Rust API addition (`core/src/lib.rs`):**

```rust
pub fn get_design_tokens() -> String {
    include_str!("../data/design_tokens.json").to_string()
}
```

**Flutter consumption (`lib/constants/design_tokens.dart`):**

```dart
class DesignTokens {
  static late Map<String, dynamic> _tokens;

  static Future<void> initialize() async {
    _tokens = jsonDecode(api.getDesignTokens());
  }

  static double spacing(String key) => _tokens['spacing']['scale'][key].toDouble();
  static double radius(String key) => _tokens['radius']['scale'][key].toDouble();
  // ... etc
}
```

**Web consumption (`platforms/web/tokens.js`):**

```javascript
let designTokens = null;

export async function loadDesignTokens() {
  const json = wasm.get_design_tokens();
  designTokens = JSON.parse(json);
  applyTokensToCss(designTokens);
}

function applyTokensToCss(tokens) {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(tokens.spacing.scale)) {
    root.style.setProperty(`--rf-space-${key}`, `${value}px`);
  }
  // ... etc
}
```

---

### Phase 2: Sync Verification Script

**Create `scripts/verify_ui_sync.py`:**

```python
#!/usr/bin/env python3
"""
Verify UI design tokens are synchronized across platforms.

Usage:
  python scripts/verify_ui_sync.py [--fix]

Exit codes:
  0 - All tokens synchronized
  1 - Mismatches found (details printed)
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent

def load_source_colors():
    """Load colors from presets.json (source of truth)."""
    presets = json.loads((ROOT / "core/data/presets.json").read_text())
    return presets["colors"]

def extract_flutter_colors():
    """Extract color hex values from AppColors in main.dart."""
    content = (ROOT / "platforms/mobile/lib/main.dart").read_text()
    pattern = r"static const (\w+) = Color\(0xFF([A-Fa-f0-9]{6})\)"
    return {name: hex_val.upper() for name, hex_val in re.findall(pattern, content)}

def extract_web_colors():
    """Extract color hex values from CSS variables."""
    content = (ROOT / "platforms/web/styles.css").read_text()
    pattern = r"--rf-(\w+(?:-\w+)?): #([A-Fa-f0-9]{6})"
    return {name.replace("-", "_"): hex_val.upper() for name, hex_val in re.findall(pattern, content)}

def verify_colors():
    """Compare colors across all sources."""
    source = load_source_colors()
    flutter = extract_flutter_colors()
    web = extract_web_colors()

    mismatches = []

    # Check palette colors
    for name, hex_val in source["palette"].items():
        hex_upper = hex_val.upper()
        flutter_key = name  # e.g., "red", "teal"
        web_key = name.replace("_", "_")  # CSS uses hyphens, we normalized

        if flutter_key in flutter and flutter[flutter_key] != hex_upper:
            mismatches.append(f"Flutter {flutter_key}: {flutter[flutter_key]} != {hex_upper}")
        if web_key in web and web[web_key] != hex_upper:
            mismatches.append(f"Web {web_key}: {web[web_key]} != {hex_upper}")

    return mismatches

def main():
    print("Verifying UI sync...")

    color_issues = verify_colors()

    if color_issues:
        print(f"\n❌ Found {len(color_issues)} color mismatches:")
        for issue in color_issues:
            print(f"  - {issue}")
        sys.exit(1)
    else:
        print("✓ All colors synchronized")
        sys.exit(0)

if __name__ == "__main__":
    main()
```

**Make executable and add to CI:**

```yaml
# .github/workflows/ci.yml
- name: Verify UI Sync
  run: python scripts/verify_ui_sync.py
```

---

### Phase 3: Change Detection Manifest

**Create `scripts/ui_sync_manifest.json`:**

```json
{
  "description": "Files that affect UI appearance and may need cross-platform sync",
  "flutter_ui_files": [
    "platforms/mobile/lib/main.dart",
    "platforms/mobile/lib/models/color_category.dart",
    "platforms/mobile/lib/services/color_manager.dart",
    "platforms/mobile/lib/widgets/*.dart",
    "platforms/mobile/lib/screens/*.dart"
  ],
  "web_ui_files": [
    "platforms/web/styles.css",
    "platforms/web/index.html",
    "platforms/web/storage.js"
  ],
  "shared_tokens": [
    "core/data/presets.json",
    "core/data/design_tokens.json"
  ],
  "sync_pairs": [
    {
      "description": "Color definitions",
      "flutter": "platforms/mobile/lib/main.dart:AppColors",
      "web": "platforms/web/styles.css::root colors"
    },
    {
      "description": "Semantic color categories",
      "flutter": "platforms/mobile/lib/models/color_category.dart",
      "web": "platforms/web/styles.css:semantic variables"
    },
    {
      "description": "Color customization",
      "flutter": "platforms/mobile/lib/screens/color_customization_screen.dart",
      "web": "NOT IMPLEMENTED"
    }
  ]
}
```

**Git pre-commit hook (`scripts/pre-commit-ui-check.sh`):**

```bash
#!/bin/bash
# Check if UI files changed and remind about sync

MANIFEST="scripts/ui_sync_manifest.json"
CHANGED_FILES=$(git diff --cached --name-only)

UI_CHANGED=false
for file in $CHANGED_FILES; do
  if echo "$file" | grep -qE "(main\.dart|styles\.css|presets\.json|design_tokens\.json)"; then
    UI_CHANGED=true
    break
  fi
done

if [ "$UI_CHANGED" = true ]; then
  echo ""
  echo "⚠️  UI files changed - remember to:"
  echo "   1. Run: python scripts/verify_ui_sync.py"
  echo "   2. Check if parallel changes needed on other platform"
  echo "   3. Update docs/ui-component-mapping.md if adding components"
  echo ""
fi
```

---

### Phase 4: Component Mapping Documentation

**Create `docs/ui-component-mapping.md`:**

```markdown
# UI Component Mapping

Cross-reference of UI components between Flutter and Web platforms.

## Layout Components

| Component | Flutter | Web | Sync Status |
|-----------|---------|-----|-------------|
| App Shell | `AppShell` | N/A (single page) | N/A |
| Card | `Card` + `CardTheme` | `.result-card` | Aligned |
| Section Header | `_buildSectionHeader()` | `.section-header` | Aligned |

## Form Components

| Component | Flutter | Web | Sync Status |
|-----------|---------|-----|-------------|
| Text Input | `TextField` + theme | `input[type=text]` | Aligned |
| Dimension Input | `PresetDimensionInput` | Custom JS | Functionally aligned |
| Toggle Switch | `Switch` + theme | `.toggle-switch` | Aligned |
| Segmented Button | `SegmentedButton` | `.unit-toggle` | Aligned |

## Results Display

| Component | Flutter | Web | Sync Status |
|-----------|---------|-----|-------------|
| Results Card | `ResultsCard` | `.result-card` | Aligned |
| Depth Gauge | `DepthGaugeWidget` | `#depth-gauge` | Aligned |
| Cut List | Custom widget | `.cut-list` | Aligned |

## Interactive Features

| Feature | Flutter | Web | Sync Status |
|---------|---------|-----|-------------|
| Color Customization | `ColorCustomizationScreen` | Not implemented | **Gap** |
| Dark Mode | `AppTheme.dark` | CSS-only partial | **Gap** |
| Export PDF | `ExportService` | jsPDF integration | Aligned |
| Saved Configs | `StorageService` | localStorage | Aligned |

## Color Semantics

| Category | Flutter Getter | Web Variable | Default |
|----------|----------------|--------------|---------|
| Primary | `AppColors.primaryBlue` | `--rf-primary-blue` | Blue |
| Secondary | `AppColors.seagrass` | `--rf-accent` | Teal |
| Success | `AppColors.successGreen` | `--rf-success-green` | Green |
| Warning | `AppColors.warningOrange` | `--rf-warning-orange` | Orange |
| Error | `AppColors.errorRed` | `--rf-error-red` | Red |
| Modified | `AppColors.modified` | `--rf-modified` | Purple |
| Cut Dimension | `AppColors.cutDimension` | `--rf-cut-dimension` | Orange |
| Incidental | `AppColors.incidental` | `--rf-incidental` | Teal |
| Material Property | `AppColors.materialProperty` | `--rf-material-property` | Green |
```

---

## Workflow Summary

### When Changing Design Tokens (colors, spacing, etc.)

1. Edit source of truth (`presets.json` or `design_tokens.json`)
2. Run `python scripts/verify_ui_sync.py`
3. Update Flutter constants if needed
4. Update Web CSS variables if needed
5. Re-run verification to confirm

### When Adding a New UI Component

1. Implement on primary platform first
2. Add entry to `docs/ui-component-mapping.md`
3. Create issue/note for other platform implementation
4. Update mapping when both implemented

### When Modifying Existing Component

1. Check `docs/ui-component-mapping.md` for counterpart
2. Make parallel changes OR document intentional divergence
3. Run verification script
4. Update mapping doc if behavior changed

---

## Implementation Priority

| Phase | Effort | Impact | Priority |
|-------|--------|--------|----------|
| Phase 1: Design Tokens JSON | Medium | High | 1 |
| Phase 2: Sync Verification Script | Low | High | 2 |
| Phase 3: Change Detection | Low | Medium | 3 |
| Phase 4: Component Mapping | Low | Medium | 4 |

**Recommended first step**: Implement Phase 2 (verification script) to establish baseline sync status before adding new tokens.

---

## Open Questions

1. Should we auto-generate Flutter constants from JSON, or keep manual sync with verification?
2. Should web color customization match Flutter UI exactly, or be web-native?
3. How strict should CI enforcement be? (warning vs blocking)

---

*Last updated: 2026-01-22*
