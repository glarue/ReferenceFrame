#!/usr/bin/env python3
"""Drift check: core/data/presets.json is the single source of truth for the
color palette, but the values are necessarily hand-duplicated in platform code
(CSS variables, Dart consts, inline JS). This script fails if any copy drifts.

Checked against presets.json colors.palette / palette_light / palette_dark:
  - platforms/web/styles.css        --rf-<name>[-light|-dark] variables
  - platforms/mobile/lib/models/color_category.dart  ColorPalette.base/light/dark
  - platforms/web/index.html        COLOR_PALETTE array (base hex)
  - core/src/visualization/style.rs palette reference comment block

Also checks presets.json aspect_ratios against the mobile aspect-ratio presets:
  - platforms/mobile/lib/constants/aspect_ratio_presets.dart (ratio/name/annotation/sizes/order)
(presets.json mirrors mobile here; web's flat STANDARD_SIZES is an intentionally
separate size dropdown and is not covered.)

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
if ar_path.exists():
    print(f"OK: {len(ar_json)} aspect ratios consistent between presets.json and aspect_ratio_presets.dart")
