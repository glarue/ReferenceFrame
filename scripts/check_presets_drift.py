#!/usr/bin/env python3
"""Drift check: core/data/presets.json is the single source of truth for the
color palette, but the values are necessarily hand-duplicated in platform code
(CSS variables, Dart consts, inline JS). This script fails if any copy drifts.

Checked against presets.json colors.palette / palette_light / palette_dark:
  - platforms/web/styles.css        --rf-<name>[-light|-dark] variables
  - platforms/mobile/lib/models/color_category.dart  ColorPalette.base/light/dark
  - platforms/web/index.html        COLOR_PALETTE array (base hex)
  - core/src/visualization/style.rs palette reference comment block

Also checks the semantic color mapping (category -> factory palette color, e.g.
secondary -> dark_cyan) in presets.json colors.semantic against:
  - platforms/web/index.html        SEMANTIC_CATEGORIES defaults (camelCase)
  - platforms/web/styles.css        --rf-<category>: var(--rf-<color>) in :root
                                    (plus the --rf-<category>-dark variants)
  - platforms/mobile/lib/models/color_category.dart  factoryColorName (camelCase)
(iOS is the reference for this mapping; presets.json mirrors it.)

Also checks presets.json aspect_ratios against the mobile aspect-ratio presets:
  - platforms/mobile/lib/constants/aspect_ratio_presets.dart (ratio/name/annotation/sizes/order)
(presets.json mirrors mobile here. Web needs no check: its size dropdown is built
from presets.json aspect_ratios at runtime via getPresetsJson. Not covered: the
hand-maintained mobile `standardSizes` list in lib/state/storage.dart and the
AppColors consts in lib/main.dart.)

Run from anywhere: python3 scripts/check_presets_drift.py
Exits nonzero on any mismatch. Wired into CI (.github/workflows/test.yml).
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

errors = []


def snake_to_kebab(name):
    return name.replace("_", "-")


def snake_to_camel(name):
    parts = name.split("_")
    return parts[0] + "".join(p.title() for p in parts[1:])


def camel_to_snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def kebab_to_snake(name):
    return name.replace("-", "_")


def check(label, name, expected, actual):
    if actual is None:
        errors.append(f"{label}: '{name}' missing (expected #{expected.lower()})")
    elif actual.lower() != expected.lower():
        errors.append(
            f"{label}: '{name}' is #{actual.lower()}, presets.json says #{expected.lower()}"
        )


presets = json.loads((ROOT / "core/data/presets.json").read_text())
colors = presets["colors"]
variants = {
    "base": colors["palette"],
    "light": colors["palette_light"],
    "dark": colors["palette_dark"],
}
palette_names = list(colors["palette"].keys())

# --- styles.css: --rf-flag-red / --rf-flag-red-light / --rf-flag-red-dark ---
css = (ROOT / "platforms/web/styles.css").read_text()
for variant, table in variants.items():
    suffix = "" if variant == "base" else f"-{variant}"
    for name, expected in table.items():
        var = f"--rf-{snake_to_kebab(name)}{suffix}"
        m = re.search(rf"{re.escape(var)}:\s*#([0-9a-fA-F]{{6}})\b", css)
        check(f"styles.css ({variant})", var, expected, m.group(1) if m else None)

# --- color_category.dart: ColorPalette.base / .light / .dark maps ---
# The mobile repo is nested and gitignored by the root repo, so it is absent
# in CI checkouts — check it when present (i.e., on dev machines), skip otherwise.
dart_path = ROOT / "platforms/mobile/lib/models/color_category.dart"
dart = dart_path.read_text() if dart_path.exists() else None
for variant, table in variants.items() if dart else []:
    section = re.search(
        rf"static const Map<String, Color> {variant} = \{{(.*?)\}};", dart, re.S
    )
    if not section:
        errors.append(f"color_category.dart: ColorPalette.{variant} map not found")
        continue
    body = section.group(1)
    for name, expected in table.items():
        camel = snake_to_camel(name)
        m = re.search(rf"'{camel}':\s*Color\(0xFF([0-9A-Fa-f]{{6}})\)", body)
        check(f"color_category.dart ({variant})", camel, expected, m.group(1) if m else None)

# Dart name list should cover the palette exactly
expected_camel = [snake_to_camel(n) for n in palette_names]
if dart is None:
    print("note: platforms/mobile not present (CI checkout) — skipping Dart checks")
else:
    dart_names = re.findall(r"^\s*'(\w+)',\s*$", dart, re.M)
    if dart_names and dart_names != expected_camel:
        errors.append(
            f"color_category.dart: ColorPalette.names {dart_names} != presets.json order {expected_camel}"
        )

# --- index.html: COLOR_PALETTE array (base hex only) ---
html = (ROOT / "platforms/web/index.html").read_text()
block = re.search(r"const COLOR_PALETTE = \[(.*?)\];", html, re.S)
if not block:
    errors.append("index.html: COLOR_PALETTE array not found")
else:
    entries = dict(
        re.findall(r"name:\s*'(\w+)'.*?hex:\s*'#([0-9a-fA-F]{6})'", block.group(1))
    )
    for name, expected in variants["base"].items():
        camel = snake_to_camel(name)
        check("index.html COLOR_PALETTE", camel, expected, entries.get(camel))
    extra = set(entries) - set(expected_camel)
    if extra:
        errors.append(f"index.html COLOR_PALETTE: colors not in presets.json: {sorted(extra)}")

# --- style.rs: palette reference comment block (base hex) ---
style_rs = (ROOT / "core/src/visualization/style.rs").read_text()
commented = re.findall(r"//\s+#([0-9a-fA-F]{6})\s", style_rs)
palette_set = {v.lower() for v in variants["base"].values()}
known_other = {"8b6914", "b8d4e3", "f5f0e1", "e8e8e8", "cccccc", "a0a0a0"}  # material fills
stale = [h for h in commented if h.lower() not in palette_set | known_other]
if stale:
    errors.append(
        f"style.rs: commented palette hex values not in presets.json palette: {stale}"
    )

# --- semantic color mapping: presets.json colors.semantic vs web JS, CSS, Dart ---
# presets.json keys/values are snake_case ("cut_dimension": "red_orange"); web
# JS and Dart use camelCase, CSS uses kebab-case. Everything is compared in
# snake_case. Only the category keys are compared (not "_comment",
# "custom_value", or the "<category>_dark" variant entries, handled below).
semantic_json = colors.get("semantic", {})
semantic = {
    k: v for k, v in semantic_json.items()
    if not k.startswith("_") and not k.endswith(("_dark", "_light")) and k != "custom_value"
}
semantic_variants = {k: v for k, v in semantic_json.items() if k.endswith(("_dark", "_light"))}
semantic_sources = {}  # label -> {category: color} (snake_case)

for category, color in semantic.items():
    if color not in colors["palette"]:
        errors.append(f"presets.json colors.semantic: '{category}' -> '{color}' is not a palette color")

js_block = re.search(r"const SEMANTIC_CATEGORIES = \[(.*?)\];", html, re.S)
if not js_block:
    errors.append("index.html: SEMANTIC_CATEGORIES array not found")
else:
    semantic_sources["index.html SEMANTIC_CATEGORIES"] = {
        camel_to_snake(k): camel_to_snake(v)
        for k, v in re.findall(r"key:\s*'(\w+)'.*?default:\s*'(\w+)'", js_block.group(1))
    }

root_block = re.search(r":root\s*\{(.*?)\n\}", css, re.S)
css_root = root_block.group(1) if root_block else ""
css_map = {}
for category in semantic:
    m = re.search(rf"--rf-{snake_to_kebab(category)}:\s*var\(--rf-([a-z-]+)\)", css_root)
    if m:
        css_map[category] = kebab_to_snake(m.group(1))
semantic_sources["styles.css :root"] = css_map
# "<category>_dark": "palette_dark.<color>" -> --rf-<category>-dark: var(--rf-<color>-dark)
for key, value in semantic_variants.items():
    category, variant = key.rsplit("_", 1)
    color = value.split(".", 1)[1] if "." in value else value
    m = re.search(rf"--rf-{snake_to_kebab(category)}-{variant}:\s*var\(--rf-([a-z-]+)\)", css_root)
    expected_var = f"{snake_to_kebab(color)}-{variant}"
    if not m:
        errors.append(f"styles.css: --rf-{snake_to_kebab(category)}-{variant} missing (presets.json {key} = {value})")
    elif m.group(1) != expected_var:
        errors.append(f"styles.css: --rf-{snake_to_kebab(category)}-{variant} is var(--rf-{m.group(1)}), presets.json {key} = {value}")

if dart is not None:
    semantic_sources["color_category.dart factoryColorName"] = {
        camel_to_snake(cat): camel_to_snake(color)
        for cat, color in re.findall(
            r"SemanticColorCategory\.(\w+):\s*ColorCategoryInfo\(.*?factoryColorName:\s*'(\w+)'", dart, re.S
        )
    }

for label, mapping in semantic_sources.items():
    for category, expected in semantic.items():
        actual = mapping.get(category)
        if actual is None:
            errors.append(f"{label}: semantic category '{category}' missing (presets.json: {expected})")
        elif actual != expected:
            errors.append(f"{label}: '{category}' defaults to '{actual}', presets.json says '{expected}'")
    for category in set(mapping) - set(semantic):
        errors.append(f"{label}: semantic category '{category}' not in presets.json colors.semantic")

# --- aspect_ratio_presets.dart vs presets.json aspect_ratios ---
# presets.json mirrors the mobile AspectRatioPresets (ratio, name, annotation,
# common_sizes, and display order). Mobile is nested/gitignored, so absent in CI
# checkouts — checked on dev machines, like the Dart color checks above.
ar_json = {k: v for k, v in presets.get("aspect_ratios", {}).items() if not k.startswith("_")}
ar_path = ROOT / "platforms/mobile/lib/constants/aspect_ratio_presets.dart"
if not ar_path.exists():
    print("note: aspect_ratio_presets.dart not present (CI checkout) — skipping aspect-ratio check")
else:
    ar_dart = ar_path.read_text()
    block_re = re.compile(
        r"static const (\w+) = AspectRatioPreset\(\s*"
        r"ratio:\s*([\d.]+),\s*"
        r"name:\s*'([^']*)',\s*"
        r"annotation:\s*'([^']*)',\s*"
        r"key:\s*'([^']*)',\s*"
        r"commonSizes:\s*\[(.*?)\],\s*\);",
        re.S,
    )
    size_re = re.compile(r"StandardSize\(short:\s*([\d.]+),\s*long:\s*([\d.]+),\s*name:\s*'([^']*)'\)")
    by_key, var_to_key = {}, {}
    for var, ratio, name, annot, key, sizes_body in block_re.findall(ar_dart):
        var_to_key[var] = key
        by_key[key] = {
            "ratio": float(ratio),
            "name": name,
            "annotation": annot,
            "sizes": [(float(s), float(l), nm) for s, l, nm in size_re.findall(sizes_body)],
        }
    all_m = re.search(r"static const all = \[(.*?)\];", ar_dart, re.S)
    dart_order = (
        [var_to_key[v] for v in re.findall(r"^\s*(\w+),", all_m.group(1), re.M) if v in var_to_key]
        if all_m else []
    )
    json_keys = list(ar_json.keys())
    if dart_order != json_keys:
        errors.append(
            f"aspect_ratios: display order {dart_order} (dart) != {json_keys} (presets.json)"
        )
    for key, jv in ar_json.items():
        dv = by_key.get(key)
        if dv is None:
            errors.append(f"aspect_ratios: '{key}' in presets.json but not in aspect_ratio_presets.dart")
            continue
        if abs(dv["ratio"] - float(jv["ratio"])) > 1e-9:
            errors.append(f"aspect_ratios '{key}': ratio {dv['ratio']} (dart) != {jv['ratio']} (presets.json)")
        if dv["name"] != jv["name"]:
            errors.append(f"aspect_ratios '{key}': name {dv['name']!r} (dart) != {jv['name']!r} (presets.json)")
        if dv["annotation"] != jv.get("annotation"):
            errors.append(f"aspect_ratios '{key}': annotation {dv['annotation']!r} (dart) != {jv.get('annotation')!r} (presets.json)")
        jsizes = [(float(s["short"]), float(s["long"]), s["name"]) for s in jv["common_sizes"]]
        if dv["sizes"] != jsizes:
            errors.append(f"aspect_ratios '{key}': sizes {dv['sizes']} (dart) != {jsizes} (presets.json)")
    for key in by_key:
        if key not in ar_json:
            errors.append(f"aspect_ratios: '{key}' in aspect_ratio_presets.dart but not in presets.json")

if errors:
    print(f"PRESETS DRIFT DETECTED ({len(errors)} mismatch(es)):", file=sys.stderr)
    for e in errors:
        print(f"  - {e}", file=sys.stderr)
    sys.exit(1)

total = sum(len(t) for t in variants.values())
print(f"OK: {total} palette values consistent across CSS, Dart, HTML, and style.rs")
print(f"OK: {len(semantic)} semantic color defaults consistent across presets.json, "
      + ", ".join(semantic_sources))
if ar_path.exists():
    print(f"OK: {len(ar_json)} aspect ratios consistent between presets.json and aspect_ratio_presets.dart")
