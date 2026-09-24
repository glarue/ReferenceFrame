# ReferenceFrame Codebase Audit — September 2026

**Date:** 2026-09-23
**Scope:** Rust core (calc + visualization), WASM/FRB boundary + shareable URL, web frontend, Flutter state/services, Flutter UI, docs/tooling/SSOT.
**Method:** 7 parallel read-only reviewers; each finding verified against source (several web items reproduced in headless Chromium, core items via a probe crate). Top items re-verified by the coordinator. All test suites green at audit time (core 311 lib + 9 edge + golden + 5 doctests; mobile 113 Flutter + 6 bridge; web storage 54).

IDs: **A** = user-visible bug (iOS), **W** = user-visible bug (web), **C** = core (affects both), **T** = tooling/process, **D** = docs, **Q** = quality/dead code/duplication, **G** = test gaps. Risk = fix risk.

---

## Remediation plan (living checklist — tick as done)

**Ground rules**
- Work phase by phase; each phase ends with the verification gate below, then a user review before committing.
- Commits: conventional, split by repo (root = core/web/tooling/docs; mobile = Flutter/bridge/mobile docs), one logical change per commit. Nothing is committed, released (`release.sh --apply`), deployed, or uploaded (fastlane) without the user's OK.
- No change to rendered SVG output unless the item says so; the golden matrix must stay byte-identical otherwise.
- Out-of-scope discoveries get appended to the findings below, not fixed inline.

**Verification gate** (run what the phase touched)
- Core: `cd core && cargo test` (full — `--lib` skips edge/golden/doctests) + `cargo clippy --all-targets` (no new warnings).
- Web: `./build_wasm.sh`; `node --test platforms/web/storage.test.mjs`; headless-browser smoke of the touched flows; bump `?v=` (+ `sw.js` caches on deploy).
- Mobile: `flutter analyze`; `flutter test`; `cd rust && cargo test`; `./rebuild.sh run` in simulator for touched screens (user eyeball for anything visual).

### Phase 1 — Zero-risk fixes + comment/doc accuracy
- [x] T1 mobile repo `core.hooksPath` → shared `hooks/` (git config, not a file)
- [x] T2 `hooks/commit-msg` + `release.sh` accept standard `type(scope)!:`; `breaking_re` covers all types
- [x] W5 close the three `<small>` tags
- [x] W3 load saved display format at init
- [x] A5 validation "reset all" paths push config to DesignState
- [x] A4 `PresetDimensionInput` clears custom string on unit/format change
- [x] A10 "Save as New" snackbar (capture messenger before pop)
- [x] D1 CLAUDE.md: 10-color table + variant formula (roles left to W4/Q7), hooks note, `submit` lane, full `cargo test`
- [x] D2 in-code comment/doc fixes that don't depend on a pending code change (core, viz, bridge, Dart, web); user-facing "7-color palette" string
- [x] D3 APP_STORE_DIRECT_SUBMISSION.md: Pages source = GitHub Actions, correct privacy URL
- Phase 1 leftovers (do with the related item): `settings_screen.dart` empty `.then` "Trigger rebuild" + "used by both" doc (with Q1); `plan_svg.rs:629,911,920` "white" wording for `background_color` fills; `simple.rs` docs (with Q2/Q4 FRB regen); README/PROJECT_STRATEGY "28-byte" (with D4); `history.rs:29` vs web ms (W15); `validation.rs:418` (C2).

### Phase 2 — Dead code removal (no behavior change)
- [x] Q1 Flutter: `export_screen.dart`, unused screen wrappers, `selectedStandardSize` plumbing, `saveToHistoryQuiet`, `*.dart.backup`, unused DesignState/StorageService/AppColors members, dead theme blocks, unused params, stale always-pass integration tests, `analysis_options` stale exclude, unused import
- [x] Q3 core/viz: `FrameSize`, `push_backdropped_text`, `render_zigzag_line`, `use_tick_marks` path, unused `DiagramStyle` fields/`DimensionType` variants, unused `approx` dev-dep (hatch `<pattern>` deferred — changes goldens)
- [x] Q3 web: dead CSS blocks (grep-verified incl. JS `classList`), PDF debug logging + per-keystroke `[DEBUG]` logs, dead `rabbet_width ??` fallback, `platforms/web/build.sh`
- [x] C8 dead section callouts + their wrong-formula tests
- Deferred to Phase 5 (need FRB regen / iOS build check): Q2 unused bridge exports, unused pubspec deps
- Phase 2 notes: also removed 3 scripts that only ran the deleted integration tests, ~34 orphaned CSS custom properties, 7 unused DesignState members, 3 StorageService members; merged DesignState defaults table (fixed `backing_thickness` fallback 0.1875→0.125); fixed `formatDecimalTrimmed(decimals: 0)` + 12 new Dart tests. Kept on purpose: StorageService tape/settings getters (used by `exportAll`/`importAll` — decide with export work), hatch `<pattern>`, C6-related layout fields, MD3 token scales, palette `-light/-dark` vars (drift-checked), W4 semantic tokens. New dead after Phase 2: bridge `getColor` (add to Q2). Docs still referencing removed items (fix in D4): `ARCHITECTURE.md` (build.sh), mobile `UI_TEST_PLAN.md`, `SCREENSHOT_GUIDE.md`, `FASTLANE_KEYS_SCOPE.md`, `ASPECT_RATIO_*.md`.

### Phase 3 — Low-risk real bugs
- [x] A1 single-owner unit preference (+ one-time key migration)
- [x] A2 inverse `defaultsKey` mapping in `resetToDefaults` (+ test) — bridge unknown-field logging dropped (would need a bridge change; the Dart fix + test covers it)
- [x] A3 invert aspect lock on orientation swap; re-lock on `fromJson`
- [x] A6 `DesignState.applyConfig()` (restores `frame_style`; one round-trip/notify)
- [x] A11 "Include mat" restore uses effective default
- [x] A12 side-effect-free format/SVG methods
- [x] C1 stack warning uses `get_rabbet_z_depth_required()` (+ no-mat test)
- [x] C4 fraction formatter carry + sign (+ real tests; goldens re-run)
- [x] C5 reject non-finite input; guard `validate_design`
- [x] C10 shareable URL encoder rounds
- [x] C13 unified mm formatting in validation messages (C17 unit-aware history title → moved to Phase 5: changes a bridge signature, needs FRB regen)
- [x] W1 `setMatWidths()` helper; full reset
- [x] W2 restore-before-first-calculate
- [x] W6 PDF passes all diagram args
- [x] W7 Copy cut list uses fitted sizes / shared results model
- [x] W8 `refreshDerivedUi()` on all load paths
- [x] W9 define/replace undefined CSS vars (user chose to keep the intended monospace numbers); W10 dark depth-gauge contrast
- [x] W11 sw.js origin check + `ignoreSearch`
- [x] W13 complete export/import
- [x] W14 privacy policy: discloses web CDN/GitHub Pages requests; iOS claims unchanged (all 3 identical copies updated; dedupe in D4) — **user to review wording**
- [x] W17 scope recalc listener; W18 PDF snapshot; W19 WASM history error contract
- [x] T3 release.sh skips build-number-only commits; T5 CI (lockfile, pin wasm-pack, deploy `needs:` tests, node test, ~~drop `ios-release.yml`~~ (local-only, left); T6 fastlane lanes/env; T7 rebuild.sh marker + presets.json; T8 script tidy
- [x] G tests accompanying the above (validation non-Rabbet/mm/no-mat, frame sight-size, frozen URL fixtures, golden fails on missing file in CI, Dart storage round-trip)
- Then: `release.sh` dry run → user OK → apply/deploy/TestFlight
- Phase 3 notes / new findings: `ios-release.yml` is a gitignored local-only file (never ran in CI) — left in place, T5 removal not needed. A12 made the calculator error banner (`calculator_screen.dart` `lastError`) unreachable — remove or rewire (Phase 5). release.sh runs a full `cargo update` on mobile `rust/` for every core/bridge bump, silently upgrading registry deps (should update only the core entry). `ios/fastlane/README.md` (auto-gen), `FASTLANE_GUIDE.md`, `FASTLANE_QUICKSTART.md`, `SCREENSHOT_GUIDE.md` still mention the removed `screenshots`/`download_metadata` lanes (D4). `upload_metadata` lane has `skip_screenshots: false` vs manual-screenshot policy. Push to main now runs tests twice (test.yml + deploy's reusable call) — harmless.

### Phase 4 — Needs a decision first (ask, record answer here, then fix)
- [x] A7 Total Wood margin — decision (2026-09-23): **1/16" per piece on both** (web's value), both honor the Blade Width setting; margin moves to presets.json
- [x] C3 `min_visible_opening` — decision: **wire it in, default stays 1/8" per side** (overlap check relaxes from a 1" to a 1/4" minimum opening; web clamp unchanged)
- [x] C9 clamp policy — decision: **one core policy** honoring min lip / min face / min opening settings, used by both platforms (iOS keeps snackbar notices; needs FRB regen)
- [x] W4/Q7 semantic colors — decision: **iOS mapping is the reference**; update presets.json + web, make web CSS use the semantic vars, drift-check semantics
- [x] C2 — decision: **apply rabbet-width checks to sight-size** (Float exempt until Phase 2)
- [x] A13 — decision: **cold launch starts from custom defaults**
- [x] A8 — decision: **reword the descriptions** (theme stays fixed)
- [x] A9 — decision: **yes, decimal mode shows `"` via Rust formatter** (bundled with C9 FRB regen)
- [x] C18 `dominant-baseline` — verified 2026-09-23 on simulator + iOS PDF: flutter_svg and the `pdf` package ignore it, so iOS plan callouts sit ~0.35em off-center (rotated labels sit outside their line); web (browsers) is centered. Cosmetic, readable. `dy` is not portable; only an explicit y shift (as the artwork label uses) renders the same everywhere. **Deferred into C6** (same placement code; needs visual sign-off). Evidence was in the session scratchpad (c18/).
- [x] C17 unit-aware history title (pulled in from Phase 5: rides the same FRB regen)
- Phase 4 notes: mobile bridge gained sibling fns (`apply_input_constraints`, `format_display_value_bridge`, `add_to_history_with_unit`); old `format_dimension_bridge`/`add_to_history`/`validate_frame_design` kept (delegating) — prune in Q2. FRB 2.11.1 regen, content hash 952246595. `ColorPalette::get` resolver fixed (closes C15). New open question: web accent buttons (Export PDF, Save, Save to History, Save Size = raw Seaweed; Share = success green) don't follow any semantic category — should they follow Primary? iOS: Settings "N customized" count uses warningDark (arguably Modified). Other new `ValidationConfig` fields (warnMinMatOpening, min/maxMatOverlap) not exposed in web Settings.
- [x] T9 — not asked; gitignoring CLAUDE.md/.claude looks intentional (private) — left as is

### Phase 5 — Larger refactors (each its own mini-plan when started)

**Mini-plan (2026-09-23).** Three batches, lowest risk first; each ends with the verification gate + a user checkpoint.
- **5a — cleanup, no visual change.** Round 1 (parallel, disjoint files): *mobile* Q2 prune unused bridge fns (incl. Phase-4 superseded siblings, `getColor`) + FRB regen, unused pubspec deps (+ pod install + sim build), dead calculator error banner, `simple.rs` doc leftovers + dedupe the dark-preview override (same values), cache decoded defaults; *web* Q2 prune unused WASM exports, W16 core aspect label, T4 versioned `.wasm` URL in `init()`; *tooling/docs* T4 deploy.yml SHA stamping, release.sh updates only the core entry of the mobile Cargo.lock, fastlane docs + `upload_metadata` skip_screenshots, D4 docs archive/consolidation. Round 2 (after bindings are pruned): *core* delete newly-unreferenced core APIs (C11, deprecated `ParsedDimension` if unused), C12, C14, C16; *viz* Q8 `pub(crate)` modules + `#[cfg(test)]` snapshot I/O, leftover wording.
- 5a notes: `TypicalRanges` moved to presets.json unchanged, but they contradicted presets guidance — **user decided (2026-09-24): widen the hints to match** → frame width 1/2–4", rabbet width 1/4–1/2", rabbet depth 5/16–1/2", matboard 1/32–3/16", artwork 0–1/4", margin 0–1/8" (done). Aspect-ratio group names in presets ("3:2 · Standard Photos") read width:height while the label now reads height:width on both platforms. `pod` isn't on PATH (CocoaPods at /opt/homebrew/lib/ruby/gems/4.0.0/bin) — flutter builds after plugin changes need it. `DesignState.invertAspectRatio()` wrapper now unused.
- **5b — refactors with byte-identical output.** Q4 boundary dedupe (`ShareableParams::from_design`, one camelCase defaults serializer, overlay/spline/hanging/weight helpers into core, `#[serde(default)]` on param structs, `DiagramOptions` options object); Q5 Dart parsers/formatters → Rust-backed utils (+ `saved_sizes_sheet` display format); Q6 viz duplication helpers; iOS SVG caching (Q8).
- **5c — visible changes, need sign-off.** C6 + C18 callout layout/baseline (goldens change; iOS screenshots for review); C7 dark diagram colors into `DiagramStyle` (dark output changes); Q6 hatch `<pattern>` removal (goldens); W12/Q9 accessibility (Dynamic Type check first); W15 history timestamps → seconds (data migration).

- [ ] C6 callout layout ↔ renderer unification (golden churn; iOS visual check) — include C18: replace `dominant-baseline="central"` with an explicit `BASELINE_SHIFT_RATIO` y shift so iOS/PDF match web
- [ ] C7 dark-mode diagram colors into `DiagramStyle` (+ inset-box helper)
- [ ] Q4 boundary: `ShareableParams::from_design`, one camelCase `getDefaults`, overlay/spline/hanging/weight helpers into core, `#[serde(default)]` on param structs, `DiagramOptions` options object
- [ ] Q5 Dart parsers/formatters → Rust-backed shared utils; `saved_sizes_sheet` display format
- [ ] Q6 viz duplication helpers; hatch `<pattern>` removal
- [ ] Q8 `pub(crate)` viz modules; SVG caching on iOS; cached defaults — 5a did `pub(crate)` + `#[cfg(test)]` snapshot + cached defaults; **iOS SVG caching remains (5b)**
- [x] Q2 prune unused bridge exports (FRB regen) + unused pubspec deps (pod install + device build) — 5a: mobile −26 bridge fns (FRB hash −1767989795), web −~25 WASM exports; `url_launcher`/`cupertino_icons`/`ffi` dropped
- [x] T4 deploy.yml stamps SHA into `?v=`/sw caches; versioned wasm URL (4 tokens each in index.html + sw.js PRECACHE_URLS; deploy fails on mismatch)
- [ ] W12/Q9 accessibility passes (web keyboard/labels; iOS semantics/tap targets — Dynamic Type check before layout changes)
- [ ] W15 history timestamps → seconds (HISTORY_VERSION 2) — W16 done in 5a (web label now height:width like iOS, e.g. 8×12 → "2:3")
- [x] C11, C12, C14, C16 small core cleanups (C15 fixed in Phase 4) — plus deleted deprecated `ParsedDimension`, `enforce_constraints`, unused presets/history/validation helpers
- [x] Phase 3/4 leftovers: dead calculator error banner (A12); release.sh full `cargo update` on mobile lock; fastlane docs mention removed lanes; `upload_metadata` skip_screenshots
- [x] D4 archive/consolidate stale docs (root + mobile `RELEASING.md`); refresh README/ARCHITECTURE

---

## A — iOS user-visible bugs

| ID | Sev | Where | Issue | Fix | Risk |
|---|---|---|---|---|---|
| A1 | High | `design_state.dart:240-253`, `storage.dart:73-82`, `app_shell.dart:53-58`, `settings_screen.dart:287` | Unit pref has **two keys** (`pref_use_mm` vs `frame_designer_unit`). Settings toggle writes only the first; AppShell reads the second at launch and overwrites → choosing mm in Settings reverts on cold launch. (AppBar toggle writes both, so it works.) | DesignState owns units; one-time migrate (prefer `frame_designer_unit`); delete AppShell `_loadPreferences` + extra write in `shared_actions.dart:21`. | low |
| A2 | Med | `design_state.dart:306-335` | `resetToDefaults` passes storage key `frame_thickness` as a field name; bridge `update_frame_design` has no such arm (`_ => {}`) → custom Frame Depth default ignored on Reset. | Inverse of `defaultsKey()` (`frame_thickness`→`frame_material_depth`, non-design keys → skip) + unit test; log unknown fields in bridge. | low |
| A3 | Med | `calculator_screen.dart:347-352` | Orientation flip while aspect-locked swaps dims but not the lock → next edit restores old shape. `invertAspectRatio()` exists, 0 callers. Web does invert (`index.html:1300`). `fromJson` also keeps stale lock. | `if (state.isAspectLocked) state.invertAspectRatio();` (or a `swapOrientation()` in DesignState). | low |
| A4 | High | `preset_dimension_input.dart:46-54,65-72` | Preserved custom input string isn't cleared on unit/format change → shows e.g. **"1 3/16 mm"**. `DimensionInput` handles this correctly (`dimension_input.dart:61-68`). | Mirror that logic in `didUpdateWidget`. | none |
| A5 | Med | `settings_screen.dart:152-165, 192-211` | "Reset all validation" / "Reset all settings" save to storage but never call `DesignState.updateValidationConfig()` → calculator uses old limits until restart. | Call it in both. | none |
| A6 | Med | `saved_configs_screen.dart:69-80` | Loading a saved config applies only num/bool → `frame_style` (sight-size) not restored; also N FFI round-trips + N notifies. | `DesignState.applyConfig(map)`: merge, one round-trip, one notify. | low |
| A7 | Med | `design_state.dart:553-560` | Total Wood uses hardcoded saw 0.125 / error 0.5; Settings "Blade Width" only feeds the share URL. Web uses user blade width + 0.0625 → iOS shows ~1.75″ more wood for same design. | **Decide** correct margin; move to presets.json; pass effective blade width. | med (visible number) |
| A8 | Low-Med | `color_category.dart:36,57` vs `main.dart:111-186` | Primary/Warning color descriptions promise app bar/buttons/toggles, but AppTheme uses fixed `AppColors` constants. | Build theme from ColorManager, or fix description text. | low |
| A9 | Low | `design_state.dart:629-631` | Decimal mode formats in Dart without `"` (`12.75`) while fraction mode has it; Rust already supports decimal (`format_dimension(use_decimal)`), bridge hardcodes `false`. | Add `use_decimal` to bridge, delete Dart branch. | low-med (adds `"`) |
| A10 | Low | `save_to_history_dialog.dart:103-119` | "Save as New" snackbar never shows (context unmounted after pop). | Capture `ScaffoldMessenger` before pop. | none |
| A11 | Low | `calculator_screen.dart:142-143,418-427` | Re-enabling "Include mat" restores hardcoded 2.0, not user's custom default; lost on tab switch. | Fall back to `getEffectiveDefault`. | low |
| A12 | Low | `design_state.dart:625-820` | `formatValue`/`generate*Svg` mutate `_lastError` and call `notifyListeners()` in catch during build (latent assertion/rebuild loop if FFI ever throws). | Make them side-effect-free. | low |
| A13 | ? | `design_state.dart:138` | Custom defaults are only applied on explicit Reset, not cold start. **Confirm intended.** | — | — |

## W — Web user-visible bugs

| ID | Sev | Where | Issue | Fix | Risk |
|---|---|---|---|---|---|
| W1 | High | `index.html:2041-2076` (`loadDefaults`), reads at `:1498` | Reset doesn't reset `mat-width-tb/lr` (what `calculate()` reads) → field shows 2, results use old value. Also skips `mat-overlap-separate`, separate-borders checkbox, `frame-style`. | One `setMatWidths()` helper replacing 5 copy-pasted sync blocks. | low |
| W2 | High | `index.html:2715-2724, 2025-2031, 4421-4435` | "Restore last settings" never works: first `calculate()` auto-saves defaults, restore reads them back 500 ms later. | Restore synchronously before first calculate (skip if `?d=`), drop setTimeout. | low |
| W3 | High | `index.html:2312,2317` | Saved display format only loaded when Settings→Appearance opened; reload reverts to fractions. | `currentDisplayFormat = loadDisplayFormat()` in init. | none |
| W4 | High | `styles.css:106-118,1234-1341`; `index.html:1005-1015,2483-2511` | 5 of 9 color customizations are no-ops: `--rf-cut-dimension/incidental/material-property/modified` are set but no rule reads them (rules hardcode `--rf-orange/teal/green/purple`). "Modified" has 3 conflicting definitions (presets purple / JS yellow / rendered blue). | Point rules at semantic vars; reconcile with presets.json (see Q7). | low (visual) |
| W5 | Med | `index.html:188,244,264` | Three unclosed `<small>` tags → parser nests 34 `<small>`s around the rest of the page incl. results/modals (`#advanced-content` renders 11.67px). | Add `</small>`. | none |
| W6 | Med | `index.html:2906` | PDF calls `generateCombinedViewSvgForPdf` with 7 of 15 args → PDF diagram ignores tape/decimal format, corner detail, axis breaks, spline/hanging layers, overlay params. | Pass all args (better: options object; `DiagramOptions` is already `Deserialize`). | low |
| W7 | Med | `index.html:3405-3423` | "Copy" cut list ignores `getFittedComponentDimensions()` → mat "Size" disagrees with Results/PDF "Cut to fit" by default; no glazing/backing section, hardcoded `(x2)`. | Single shared results model for Results/PDF/Copy. | low |
| W8 | Med | `index.html:2655-2702, 3670-3682` | Loading URL/config leaves derived UI stale: range hints stay inches, sight-size hint text stale, `mat-overlap-separate` stale. | `refreshDerivedUi()` called from all load paths. | low |
| W9 | Med | `styles.css` (multiple) | Undefined CSS vars: `--font-mono` ×8, `--font-sans`, `--md-motion-duration-medium`, `--rf-text-primary`, `--md-on-surface-variant`, `--rf-blue-slate` → numbers not monospace, transitions dropped. | Define or replace. | low |
| W10 | Med | `styles.css:941-944,1009` | Dark-mode depth gauge "Available" ≈1.5:1 contrast (near invisible). | Use `var(--rf-text-muted)`. | none |
| W11 | Med | `sw.js:72-78` vs `:101-124` | `.js` network-first branch has no origin check → CDN scripts never hit the cache-first branch; `RUNTIME_CACHE` unused; offline `?d=` links fail (no `ignoreSearch`). | Origin-check; `ignoreSearch:true` for documents. | low |
| W12 | Med | `index.html:195,278,324,406,422`, ~30 labels | Collapsible `h2 role="button"` not focusable/keyboard-operable; modals lack `role="dialog"`/Escape; labels lack `for`. | `<button>` in h2 or tabindex+key handler; `for=`. | low |
| W13 | Med | `storage.js:498-510`; `index.html:4046-4063` | Export omits history, validation config, display/theme/material prefs; import prompt says "delete all existing data" (it doesn't); imported sizes not re-rendered. | Extend export, reword, re-render. | low |
| W14 | Med | `platforms/web/privacy-policy.html:52,68` | Claims "works entirely offline / no external servers"; web loads Google Fonts, cdnjs, unpkg. | Scope to iOS or disclose CDNs. | none |
| W15 | Low | `index.html:3780,3813` | History timestamps stored in **ms**; core doc (`history.rs:29`) and iOS use seconds. Latent until cross-platform import. | Write seconds; normalize >1e11 in `from_json`. | low-med |
| W16 | Low | `index.html:1221-1268` | JS re-implements aspect-ratio label with inconsistent orientation convention; core `getAspectRatioDisplay` unused on web. | Use core. | low |
| W17 | Low | `index.html:2730-2739` | Global `input` listener recalculates + auto-saves on every keystroke in *any* input (PDF title, settings…). | Scope to `dimensionInputIds`. | low |
| W18 | Low | `index.html:2906,3140` | Async PDF export re-reads global `currentDesign` across awaits; `calculate()` frees it. | Snapshot at export start. | low |
| W19 | Low | `wasm_bindings/src/lib.rs:1252-1305`; `index.html:3882,3897` | WASM history fns on parse failure return `{"error","history"}` which web persists as the history. | Throw, or return input unchanged (like mobile). | low |

## C — Core (affects both platforms)

| ID | Sev | Where | Issue | Fix | Risk |
|---|---|---|---|---|---|
| C1 | Med | `validation.rs:574-580` | Stack-overflow warning always adds `matboard_thickness`; `get_rabbet_z_depth_required()` and the section diagram don't when there's no mat → **false "stack exceeds rabbet" warnings for every no-mat and every sight-size design** (e.g. 0.2885″ needed vs 5/16″ → warns 0.344″). | `let total_stack = design.get_rabbet_z_depth_required();` + test. | low |
| C2 | Med | `validation.rs:418-449` | Rabbet-width checks skipped for sight-size per stale comment ("no lip over the art"). Since the retaining-lip fix, sight-size *has* a lip → rabbet_width = frame width passes with 0 errors. | Apply min-lip/min/max to SightSize (skip Float); rewrite comment. | low-med |
| C3 | Med | `validation.rs:70,550`; `frame.rs:145` | `min_visible_opening` config (editable in iOS Settings!) is never read; overlap check hardcodes `art/2 − 0.5`, `enforce_constraints` hardcodes `− 0.125`. | **Decide:** wire it (set default 0.5 to preserve behavior) or remove the setting. | med |
| C4 | Med | `conversions.rs:302-336` | `format_inches_as_fraction`: values just below a whole inch print `12 1/1"` (e.g. 279.39 mm); negatives lose sign and magnitude (−0.75 → `1/4"`), visible in validation messages. Existing test asserts only non-empty. | Carry on num==den; format abs + sign; real test. Rerun goldens. | low |
| C5 | Low-Med | `input_parser.rs:357`; `validation.rs` | `"inf"`, `"NaN"`, `"1e400"` parse as valid; NaN design validates clean; `enforce_constraints` silently maps NaN → 1/16″. | Reject `!is_finite()`; guard in `validate_design`. | low |
| C6 | High (internal) | `layout.rs:105-181`, `plan_svg.rs:1216-1258,288-297` | Callout layout model ≠ renderer: two-line rule differs, vertical offset differs, Mat Cut offset/anchor ignored, `dx` shifts dropped. Collision solver/viewBox act on wrong boxes (sweep: 100 MatCutWidth lines pushed 12–17 px for phantom overlaps; 5 X-shifts lost). Two comments contradict each other. | Layout emits actual placement; renderer consumes it. | **med** (golden + iOS visual check) |
| C7 | Med | `plan_svg.rs:65,73,936-962`, `overlays.rs:27-28,355,360` | Hardcoded colors bypass `DiagramStyle`; `#555` titles on dark bg ≈2.5:1 contrast (web + iOS). | Add fields to `DiagramStyle`, override in `for_dark`; share inset-box helper. | none (light/PDF byte-identical) |
| C8 | Med | `callouts.rs:137-221` | Unused section callouts; `Clearance` uses `frame_material_depth − stack` (geometry/validation use `rabbet_depth`); tests lock in wrong formula. | Delete dead variants/tests. | none |
| C9 | Med | web `frame.rs:133-173` vs `calculator_screen.dart:471,655,685` | Constraint clamping differs: web uses core `enforce_constraints` (overlap ≤ min/2−0.125); iOS Dart clamps (≤ min/2−0.5, rabbet ≤ frame−0.125, ignores configurable min lip/face). Binding names `validate()` contradict core doc ("NOT validation"). | **Decide** thresholds; one core policy returning clamp notices. | med |
| C10 | Low | `shareable_url.rs:117,133` | Encoder truncates (`as u32`) not rounds: 0.57 → 0.5699 → loaded designs never equal originals (defeats history dedupe). | `.round()`. | none |
| C11 | Low | `frame.rs:295-302` | `get_matboard_cut_dimensions` ignores frame style and XY margin; exported on both platforms, unused. `get_matboard_dimensions` is really the rabbet opening. | Delete (or fix); consider alias. | none |
| C12 | Low | `validation.rs:200-223` | `TypicalRanges` hardcoded, contradicts presets.json guidance (rabbet_depth, rabbet_width, margin). | Source from presets.json. | low |
| C13 | Low | `validation.rs:369-378` | mm messages inconsistent (`12.7mm` vs `19 mm`), inline 25.4. | Unify formatter. | none |
| C14 | Low | `validation.rs:604-633` | "Artwork narrower than opening" warning will fire on every Float design in Phase 2. | Gate now or note in FLOAT_FRAME_TODO. | none |
| C15 | Low | `presets.rs:40-52` | `ColorPalette::get("primary_dark")` returns `"#palette_dark.blue"` (dotted refs unresolved). Unused today. | Resolve or delete. | none |
| C16 | Low | `aspect_ratio.rs` | step=0 → NaN; height fn lacks ratio≠0 guard; duplicate unreachable `COMMON_RATIOS` entries. | Guards; dedupe. | none |
| C17 | Low | `history.rs` `generate_title` | Always inches decimals (`8.0" × 12.0" Frame`) — wrong for mm users on iOS untitled saves. | Take unit flag. | low |
| C18 | ? | `plan_svg.rs:1360-1437` | Plan callouts use `dominant-baseline="central"` while project notes say flutter_svg/svg2pdf ignore it. Since iOS looks fine, notes likely overstate — **verify on a screenshot before touching**. | Correct notes or migrate. | high if migrating |

## T — Tooling / process

| ID | Sev | Where | Issue | Fix |
|---|---|---|---|---|
| T1 | High | mobile repo `core.hooksPath` = own `.git/hooks` (no `commit-msg`) | Conventional commits **not enforced** in mobile repo, though release.sh derives app/bridge bumps from them; 3 docs claim otherwise. | `git -C platforms/mobile config core.hooksPath /Users/graham/code/ReferenceFrame/hooks` |
| T2 | Med | `hooks/commit-msg:39`; `release.sh:88` | Regex accepts `feat!(core):`, **rejects** standard `feat(core)!:`; `breaking_re` omits `build`/`revert`. | `^(type)(\([^)]+\))?(!)?: ` in both. |
| T3 | Med | `release.sh:105` | `build(app): bump build number` counts as patch → dry run currently proposes app 1.8.4→1.8.5 from build-number-only commits. | Skip pubspec `+N`-only commits or use a no-bump prefix. |
| T4 | Med | `index.html` `?v=`, `sw.js` | Cache-bust convention missed in practice (still `20260708-materials` / v15 after later core deploys); `?v=` only covers JS glue — `.wasm` binary URL is derived from `import.meta.url` and never busted; `storage.js` has no `?v=`. | Stamp git SHA in `deploy.yml`; pass explicit versioned wasm URL to `init()`. |
| T5 | Med | `.gitignore`, `deploy.yml`, `test.yml` | `wasm_bindings/Cargo.lock` untracked (non-reproducible deploys); wasm-pack via unpinned `curl\|sh`; deploy doesn't wait for tests; `storage.test.mjs` not in CI; `ios-release.yml` can never work (mobile repo absent in CI, tag pattern mismatch). | Track lockfile, pin, `needs:`, add node test, delete ios-release.yml. |
| T6 | Med | mobile `ios/fastlane/Fastfile:242-249`, `:225-239`, `.env.default:12` | `download_metadata` lane calls itself; `screenshots` lane uses scheme with no UI tests and `clear_previous_screenshots`; env var name mismatch (`…_KEY_KEY` vs `…_KEY_FILEPATH`). | Fix or delete lanes. |
| T7 | Low | `rebuild.sh:82` | Marker touched *after* `flutter run` (edits during session missed); doesn't watch `presets.json` (`include_str!`). | Touch before run; add presets.json. |
| T8 | Low | misc | `platforms/web/build.sh` duplicate/stale (port 8000, dead path); `build_wasm.sh:23` hardcoded toolchain path; `release.sh` dead branches, BSD-only `sed -i ''`, no clean-tree check; `getWasmVersion` hardcoded `"2026-01-09-…"`. | Tidy. |
| T9 | Low | `.gitignore:2-4` | `CLAUDE.md` and `.claude/` (incl. `/release-notes`) are gitignored → not versioned. | Intentional? |

## D — Docs

- **D1 CLAUDE.md** — color table says 7 colors (presets/style.rs/web/Dart have 10); roles wrong (Carrot = artwork dims/warning, cut = redOrange, modified = yellow); "light = 70%+30% white" false for 6/10 (same wrong comment in `styles.css:21,52`, `color_category.dart:124`); hooks claim false (T1); no `submit` lane; `cargo test --lib` skips edge/golden/doctests.
- **D2 In-code doc mismatches (fix alongside code):** `validation.rs:418` (C2), `frame.rs:175-178,205` sight-size "no glazing"/"opening smaller"; `assembly_margin` undocumented; `conversions.rs:283-290` formatter index (wrong signatures/outputs); "Ported from Python…identical behavior" headers (`frame.rs`, `conversions.rs`, `aspect_ratio.rs`, `shareable_url.rs`) and `//` not `//!`; `input_parser.rs:150` ParsedDimension consumer; `history.rs:29` seconds; `validation.rs:544` "mat opening negative" false; `overlays.rs:11-15,130-133,263` / `plan_svg.rs:405,970` overlay placement & preview marks; `style.rs:3-4` "cream/sepia"; `svg_util.rs:133,214,669`; `snapshot.rs:43`; `svg.rs:184-186,519-521`; stale `geometry.rs`/`svg.rs` refs in 6 viz files; `frame_preview.dart:73` ("Reduce" should be "Increase"), `:335`; `color_customization_screen.dart:97` **user-facing "7-color palette"** (10 shown); `storage.dart:6,488`; `export_service.dart:134`; `styles.css:3-17,92`; `storage.js:5,8`; `sw.js:16,71,99`; `shareable_url.rs:55,231,493,503,535`.
- **D3 APP_STORE_DIRECT_SUBMISSION.md:17,37-44** (mobile) — tells you to set Pages to **branch mode** (the known full-site-404 trap) and gives an undeployed privacy URL. Fix or archive.
- **D4 Stale/obsolete docs** (archive to `legacy/` or `docs/archive/`):
  - Root: `ARCHITECTURE.md` (wrong module tree, dead dirs, `/home/glarue` paths, wrong bump rules — rewrite short or fold into README), `README.md` (146 tests→~323, "28-byte web-only URL", iOS-only list wrong, no frame styles/spline/weight), `PROJECT_STRATEGY.md`, `DEPLOYMENT_AND_IOS_REVIEW.md`, `REFACTORING.md`, `docs/{DEPLOYMENT,PERFORMANCE_OPTIMIZATION_PLAN,SETUP_NOTES,STATUS_SUMMARY,VALIDATION_STATUS,VECTOR_PDF_PLAN}.md` (PyScript era), `docs/privacy-policy.html` (dead copy).
  - `FLOAT_FRAME_{PLAN,TODO}.md` — say iOS pending; it shipped (app 1.7.0–1.8.4). `ENHANCEMENT_PLAN.md` — add status column (B1, F1–F3, F7, R2, R4, R6, R7 done). `TODO.md:9` stale path.
  - Mobile: consolidate 7 Fastlane/App Store docs into one `RELEASING.md` (keys, `beta` → `submit build:N`, release.sh, metadata). Archive ASPECT_RATIO_*, CHANGELOG_2026-01-21, ORIENTATION_ANIMATION_IMPROVEMENTS, SETTINGS_AND_QR_PLAN, UI_TEST_PLAN, SCREENSHOT_GUIDE. Replace real Issuer ID in `ios/API_KEY_SETUP.md` with placeholder.

## Q — Quality: dead code, duplication, structure

- **Q1 Dead code — Flutter** (~700 lines): `export_screen.dart` (never imported, still being edited), wrappers `CalculatorScreen`/`VisualizationScreen`/`SettingsScreen`, `selectedStandardSize` plumbing, `saveToHistoryQuiet`, tracked `*.dart.backup` ×2, unused DesignState members (`getVisibleDimensions`, `getAspectRatioDisplay`, `formatValueWithDecimal`, `generateSectionViewSvg`, `toJson`, `getDefaults`, `toggleTapeSegments`), StorageService (`importAll`, `clearAll`, tape/settings getters…), `AppColors.getColor*`/`_RustColorBridge`, dead `navigationBarTheme`, several unused params; stale integration tests that always pass (`screenshot_test.dart`, `app_store_screenshots.dart`, `minimal_screenshot_test.dart` w/ hardcoded path); unused deps `url_launcher`, `cupertino_icons`; `analysis_options.yaml` excludes nonexistent `lib/bridge/**`; unused `shared_preferences` import `main.dart:4`.
- **Q2 Dead code — bridges:** ~20 unused iOS exports (`inchesToMmBridge`, `createFrameDesign`, `validateFrameDesign`, `lockAspectRatio`, `getColorHex`, …) and ~20 unused web exports (`WasmAspectLock`, `generateDiagram`, `parseDimension`, `getTypicalRangesJson`, …). Prune or keep deliberately (mobile needs FRB regen).
- **Q3 Dead code — core/web:** `FrameSize`, `push_backdropped_text`, `render_zigzag_line`, `use_tick_marks` path (~35 lines), unused `DiagramStyle` fields, ~10 unproduced `DimensionType` variants, unreferenced `<pattern id="hatch">` (duplicate id in combined view), `approx` dev-dep unused; ~230 lines dead CSS (`alert-card*`, `depth-connector-*`, `confirm-dialog*`…), ~70 lines PDF debug logging + per-keystroke `[DEBUG]` logs, dead `rabbet_width ??` fallback.
- **Q4 Boundary duplication:** design→shareable-params mapping hand-written 3× (web ×2, iOS; iOS doesn't zero mats when `include_mat` false) → `ShareableParams::from_design()` in core; `getDefaults` returns different key shapes per platform (mobile snake `rabbet_width` in camelCase object; Dart fallback `backing 0.1875` vs presets 0.125) → one serde camelCase serializer; `OverlayParams`/spline/hanging/weight bodies duplicated in both bindings; `SplineParams`/`HangingParams` lack `#[serde(default)]` so a partial override silently drops both; `DiagramOptions` built 6× with 10–16 positional args.
- **Q5 Dart duplicates Rust:** two identical Dart fraction parsers (`dimension_input.dart:97`, `preset_dimension_input.dart:162`) vs Rust `parseDimension` used in Settings; 6 copies of the mm/decimal/fraction format switch; `saved_sizes_sheet.dart:14` ignores display format (`8.06"` in fraction mode); `formatDecimalTrimmed(decimals: 0)` turns 100 → "1" (latent).
- **Q6 Viz duplication:** break/no-break stroke literals diverged; `content_area` stroked twice; tight-space arrow threshold in 3 places (3.0 vs 2.5); `": "` two-line split rule in 6 places; section legend width computed twice.
- **Q7 Palette/semantic SSOT drift:** `presets.json` `colors.semantic` disagrees with web and mobile (web/mobile swap secondary↔incidental; modified purple/yellow; error red/flagRed); `styles.css:92` claims it matches iOS (false); drift checker doesn't cover `style.rs` live literals, semantic map, PDF RGB table (`index.html:3173` uses retired `#277da1` labeled `--rf-blue`), `manifest.json`/`theme-color`, or `AppColors` in `main.dart`.
- **Q8 Structure:** visualization modules overly `pub` (make `pub(crate)`, `#[cfg(test)]` snapshot I/O); `WasmValidationResult` lives in pure core; iOS SVG regenerated every build incl. mid-pinch (cache by inputs); `_overlayParamsJson` → 5 FFI+JSON calls per render (cache defaults).
- **Q9 Accessibility (iOS):** icon-only buttons without tooltip/Semantics (more_vert, orientation/lock, collapse chevron, edit/delete, keyboard Done/⌫); tiny tap targets (16px banner close, aspect-ratio Edit/Done links); nav items lack `selected:`; fixed heights will clip at large Dynamic Type — check largest size before changing (visual impact).

## G — Test gaps

- validation: non-Rabbet styles, mm mode, no-mat stack (would have caught C1).
- frame/weight: sight-size fitted dims & z-depth; mat ring; zero slack → `tension: None`.
- shareable URL: frozen literal base64 fixtures for v0/v1/v2 (current tests use the encoder's own pack helpers).
- golden matrix: no dark/PDF/`DetailMode::None`/tape cases; silently *creates* missing goldens (fail when `CI` set).
- tautological tests: `snapshot.rs` `all_within_viewbox`, `layout.rs:310,447`, `callouts.rs:274`, `svg.rs` `thumb_screen_size` replicates outdated algorithm; `frame.rs` `test_validate_*` actually test `enforce_constraints`.
- Dart: `exportAll`/`importAll` round-trip, `formatDecimalTrimmed`, inverse defaults mapping (A2), legacy detail-mode migration; `storage_test` uses camelCase keys not matching real schema; `overlay_layers_test` assumes toggles start off.
- clippy: 38 lib warnings (≈15 self-inflicted `ParsedDimension` deprecation), 95 in tests (mostly `field_reassign_with_default`).

## What's good — leave alone

- presets.json SSOT genuinely enforced for defaults/limits/materials (with tests); `lip_over_art()` cleanly unifies frame styles; `serde(default)` + versioned history/URL formats keep saved data loadable; FRB generated code in sync.
- Shareable URL versioning (version bits cross-checked vs length, clamping) is solid.
- No XSS paths on web; no snake_case access on WASM objects; storage.js well-tested.
- Flutter rebuild scoping follows the project's lessons (Selectors in main/FramePreview; intentional Consumer in CalculatorBody — don't narrow further). FramePreview animation and VisualizationBody gesture handling are careful. **Keep `AppTheme.light/dark` getters uncached** — color customization propagation relies on fresh ThemeData.
- Viz NaN/inf guards held under every degenerate input tried; golden harness (43 byte-exact cases + `.actual.svg` on failure) is valuable.
