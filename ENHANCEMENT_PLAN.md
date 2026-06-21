# ReferenceFrame Enhancement Plan

**Date:** 2026-06-21
**Scope:** New features and forward-looking refactors for usability & maintainability
**Method:** Multi-agent review — 6 parallel subsystem surveys → consolidation/dedup → 69 independent adversarial assessments (each candidate verified against source for "already done? actually feasible? actually worth it?").
**Relationship to other docs:** This is the forward-looking companion to `AUDIT_REPORT.md` (2026-06-10), which covered existing maintainability/architectural debt. Items already captured there or in `TODO.md` were deliberately **excluded** (19 candidates dropped — see Appendix B). Nothing here duplicates the audit.

**Effort key:** S = hours · M = ~half-day to two days · L = a week or more. Effort/value reflect the adversarial assessment, which frequently found proposals *understated* effort — those corrections are noted inline.

---

## 0. Verified bug (fix first — not a feature)

Found while assessing the test-coverage candidate, then verified directly in source:

### B1. Web decimal display format infinitely recurses 🔴
**Where:** `platforms/web/index.html:2184` — `formatDisplay()`
```js
case 'decimal':
    return formatDisplay(value, unitMm);   // ← calls itself → stack overflow
```
`formatValueWithDecimal` is imported at `index.html:579` but **never called**. Any user who selects the decimal display format hits infinite recursion. The JS formatter dispatch path is untested, which is why this shipped.
**Fix:** one line — `case 'decimal': return formatValueWithDecimal(value, unitMm);`. Add a thin test (see R5) so it can't recur.
**Effort:** S (trivial).

---

## 1. Tier 1 — Quick wins (recommended now)

High value, small effort, low risk. Each is independently shippable.

### F1. Theme-aware (dark-mode) SVG diagrams — **strong recommend**
**Effort:** M (~half-day) · **Value:** high (web) · **Verdict:** yes
Web dark-mode users currently get a glaring white diagram panel; mobile has a half-built ad-hoc dark override (2 fields, transparent bg). The architecture is favorable: core already threads `style: &DiagramStyle` through every render path.
- **Core:** add `DiagramStyle::for_dark()` in `core/src/visualization/style.rs` (sibling to `for_pdf` at line 248), overriding background/line/dimension colors from the **presets.json dark variants** (honors SSOT; don't invent hex).
- **Web:** add a theme param to the WASM diagram bindings (`platforms/web/wasm_bindings/src/lib.rs`), pass resolved theme from `index.html` (already tracks `currentTheme` + re-renders on change), and add a dark override for `.svg-container` background.
- **Mobile:** replace the ad-hoc override with the shared `for_dark()` so both platforms match.
- **Keep PDF/combined view light** (printing on dark wastes ink).
- **Risks:** must rebuild WASM + bump `?v=` cache-bust; dark dimension colors (4 hues) must stay distinguishable & contrast-legible; may need new golden snapshot cases.

### R1. Edge-case regression test module — **strong recommend**
**Effort:** S–M (half-day) · **Value:** medium · **Verdict:** yes · **Risk:** ~zero (test-only)
Port the **three** regression gaps the audit flagged (from the archived Python suite) into core Rust tests in `core/src/*` or a new `core/tests/edge_cases.rs`:
1. zero/negative artwork dimensions are rejected,
2. round-half-up behavior at the 0.5 boundary,
3. mm/inches toggle misinterpretation.
Pure core, benefits all platforms, no shipping code touched. **Scope to these three only** — the other scenarios the survey suggested already have tests. Watch for: round-half-up may reveal Rust round-half-away-from-zero *diverges* from the Python spec — decide bug-vs-intended rather than snapshotting current behavior.

### F2. Print stylesheet for web
**Effort:** S (a few hours) · **Value:** low-medium · **Verdict:** maybe
Add an `@media print` block in `platforms/web/styles.css` + a Print button near the results actions calling `window.print()`. SVG already renders inline so it prints natively; no CDN libs (works offline, unlike the jsPDF path). Hide `.header-row`/`.inputs-column`/`.modal-overlay`/footer; **force light colors** in the print block (theme is CSS-var driven or printouts waste ink). Overlaps PDF export, so it's a low-risk convenience, not a headline.

### F3. Haptic feedback on iOS
**Effort:** S–M (~half-day) · **Value:** medium (polish) · **Verdict:** maybe · iOS-only
Settings-gated toggle (reuse the `_AutoSaveToggle`/`storage.dart` bool-pref pattern) + light taps on primary actions (the ~7 `action_menu_sheet` items + export). **Correct the proposal:** there is no "submit" event (calculator is fully reactive), so drop "success-on-submit"; do "warning" haptics only with **edge-detection** (buzz on transition *into* an error state, not every keystroke). Respect the iOS system haptics setting.

---

## 2. Tier 2 — High-value features worth planning (larger)

Real product value, but each is cross-cutting. Plan deliberately; don't let "one field" framing hide the blast radius.

### F4. Weighted / bottom-weighted mat — **highest product value**
**Effort:** M–L · **Value:** high (professional-standard framing) · **Verdict:** maybe (worth it, but scope honestly)
The single most valuable *framing* feature missing. The data model assumes vertically **symmetric** borders (`mat_width_top_bottom` shared top & bottom), so this is **not** "a single field":
- **Core:** new `#[serde(default)]` offset field + Default/interpolate/clamp (easy), but the dimension getters and especially **visualization centering** (`geometry.rs:664-666, 692-693` hardcode centered openings) must shift the opening up — plan view, section view, and callout placement all silently assume a centered opening.
- **Shareable URL:** needs a packed field → **format version bump** — but this is the *cheap* part now, because audit H5 reserved the version byte **specifically** for asymmetric mats (cited from `SAVED_CONFIGS_PLAN.md`).
- **Mobile:** `frame_preview.dart` re-implements the SVG geometry and must independently reproduce the off-center opening or web/iOS diverge.
- **Tests:** golden SVG matrix regen + review (don't rubber-stamp a rendering regression).
- **Key risk:** breaks the centered-opening invariant; a calc-only partial ships a visibly wrong preview. Do it whole or not at all.

### F5. Material & cost estimation engine
**Effort:** M (day or two) · **Value:** high · **Verdict:** maybe (keep minimal)
Pure-core `core/src/cost.rs` consuming existing geometry — `get_total_wood_length` (linear ft) + area helpers (matboard/glazing/backing) × rates. **Rates live in a `material_costs` section of presets.json** (established `presets.rs` pattern), exposed via one WASM + one FFI binding mirroring `getCutListJson`.
- **Critical caveat:** ship **user-editable rates**, *not* a bundled supplier price DB — that's wrong on day one and a perpetual maintenance trap.
- **Risks:** if rates aren't centralized they triplicate (Rust/web/Dart) — must be in presets.json with drift coverage; persisted user rates touch the unversioned storage schemas (audit H6 — use `#[serde(default)]`). The HTML monolith makes the web "Cost" tab the messiest surface.

### F6. Design notes / metadata field
**Effort:** M (~a day) · **Value:** medium · **Verdict:** maybe
Add `pub notes: String` to `FrameDesign` (`core/src/frame.rs:33`); `#[serde(default)]` is already present so old saved data loads with empty notes (no migration). Persists **free** through saved-configs and history (HistoryEntry embeds the full design).
- **New bridge fn needed:** existing setters are typed `f64`/`bool`; add `update_frame_design_string` + regen FFI.
- **Real cost:** two independent PDF-footer implementations (web jsPDF vs mobile `pdf` package).
- **Accept/document the limitation:** the fixed-size shareable URL **cannot carry free text** — a URL-shared design silently drops notes. Length-cap + escape (untrusted text flows into PDF/SVG/HTML).

### F7. Design history UI on web (Part A only)
**Effort:** S–M (under a day) · **Value:** medium (parity) · **Verdict:** maybe
Core (`core/src/history.rs`) and WASM bindings are **already built and tested**; only the web JS UI is missing. Add a collapsible history panel + "Save to History" button in `index.html`, reusing `storage.js`'s versioned-list helpers — mirrors mobile's `history_screen.dart`. No core changes.
- **Split off Part B** (true undo/redo + Cmd+Z + auto-save-on-edit): that's net-new (mobile has none, core has no redo concept), wrong data structure (snapshot store dedups/caps at 50; per-keystroke would spam it), and a separate deliberate proposal.

### F8. Double / multiple stacked mats
**Effort:** L (week+) · **Value:** medium-high (high-end niche) · **Verdict:** maybe (big)
Replacing flat mat scalars with `Vec<MatLayer>` rewrites nearly every mat method, **breaks `FrameDesign::interpolate`** (variable-length Vec doesn't lerp for the Flutter spring animation), churns ~40 `frame.rs` tests, requires looped section/plan rendering with per-layer colors, a **URL format version bump**, and new array-shaped FFI/WASM setters (the scalar string-keyed setters can't express a layer list). Upside: lands once in core, benefits web + iOS. Single-mat is the dominant case, so this is a deliberate investment in the pro segment.

---

## 3. Tier 3 — Maintainability refactors (opportunistic)

Do these **when you're already in the area** or when a feature forces the issue. None is user-visible.

### R2. Split `geometry.rs` (1944 lines) into submodules
**Effort:** M (half-day) · **Verdict:** maybe — only if navigation is actually hurting (audit deferred monolith-splitting as "only if it starts hurting").
**Correct the proposed shape:** text metrics are ~50 lines (not 400-500), and `compute_corner_detail` is a method *inside* `impl PlanViewGeometry`. Honest split: `mod.rs` (shared structs/consts/text + re-exports), `plan.rs` (`impl PlanViewGeometry`), `section.rs` (`impl SectionViewGeometry`), `breaks.rs` (axis-break free fns). Preserve the `pub` re-exports → public API unchanged, no consumer/WASM/FFI edits. Pure mechanical move — **don't refactor logic in the same pass** (golden-image risk). Costs git-blame continuity on the most-churned viz file.

### R3. Split `settings_screen.dart` (1800 lines)
**Effort:** M · **Verdict:** maybe — best deferred until a new settings category needs adding.
The real difficulty is the **centralized state** (`_SettingsContent` threads ~15 props), not the widgets. A shallow move-the-builders split delivers cosmetic LOC reduction only. To get the real win, decentralize state (a small settings `ChangeNotifier` — *not* into `DesignState`, which would worsen the god-object). **Highest-value extraction:** the two near-identical edit dialogs → a shared `_DimensionEditDialog`. Note: appearance + data import/export are already separate screens; real targets are units/display, defaults editor, validation limits. No regression net exists (zero settings widget tests).

### R4. Unit tests for `DesignState`
**Effort:** S–M for the right scope · **Verdict:** maybe
`DesignState` has zero tests. **The cheap, real win is the FFI-free logic slice** (testable with `SharedPreferences.setMockInitialValues`, matching the existing `storage_test` pattern): theme-index↔`ThemeMode` mapping, `_defaultsKey()` mapping, default-fallback constants, tape-segment preservation in `setDecimalDisplay`, the legacy detail-mode migration in `_initialize`. **Avoid** the proposal's "mock the Rust API" framing — the generated FFI is static free functions with no seam, so that requires a prerequisite injection refactor (scope creep into audit M5). Test the FFI-free slice; leave the rest.

### R5. Targeted JS/widget test coverage (+ the B1 fix)
**Effort:** M (trimmed) · **Verdict:** maybe — trim hard.
Of the surveyed bundle, only three pieces are worth it: (a) **fix B1** + a thin `storage.js` test (the genuine 17.5KB gap: `migrateStoredData`, merge-vs-replace import/export); (b) widget tests for `saved_configs_screen.dart` and `color_customization_screen.dart` (Flutter harness already exists — incremental, no new infra). **Skip** the WASM-FFI integration test and JS formatter tests (redundant with core's 278 tests). Note `storage.js` is loaded as globals, not modules — testing it needs a shim or a small module refactor.

### R6. Standardize mobile dialog/sheet helpers
**Effort:** M · **Verdict:** maybe
Extract `showConfirmDialog` / `showTextInputDialog` next to `FocusUtils` and route the ~9 duplicated sites through `FocusUtils` for consistent focus handling. **Reject** the "convert AlertDialogs to bottom sheets" idea — most are destructive confirmations/text prompts where AlertDialog is the correct iOS-idiomatic choice; converting them is a UX regression. The two `StatefulBuilder` edit dialogs won't fit a generic helper.

### R7. Reconcile aspect-ratio presets into core SSOT
**Effort:** M · **Verdict:** maybe
The `aspect_ratios` block in presets.json is **dead data** — core never deserializes it; web (`STANDARD_SIZES` in `index.html:~3466`) and mobile (`aspect_ratio_presets.dart`) hardcode their own, and they **already disagree** (Dart has an `annotation` field the JSON lacks; web has extra sizes like Letter). This is a *reconciliation* job, not a mechanical port: widen the JSON schema, add typed core accessors, migrate both platforms to read via the existing `get_presets_json()` FFI/WASM, **and extend `check_presets_drift.py`** to cover it (closes an H3-adjacent gap). Stopping at "core parse" adds a 4th copy with zero realized benefit.

---

## 4. Tier 4 — Considered, lower priority (with rationale)

Vetted and judged not worth it now — recorded so they aren't re-proposed:

| Idea | Why deprioritized |
|------|-------------------|
| **Onboarding tour + contextual help** | Contextual "?" help is cheap-ish; the full guided tour is per-platform UI with no shared-core leverage, lands in the HTML monolith, adds unversioned "seen" keys. If anything, do per-section help backed by a presets.json glossary; skip the tour. |
| **Cut-list CSV/JSON export** | Low marginal value (cut list already in PDF + HTML table). Mobile already has an **unwired** `shareText()` — un-orphan that first. If built, put a `CutList::to_csv()` in core (emit decimals + unit column, not fractions) and drop the speculative supplier-links scope. |
| **PWA install prompt + offline badge** | Install-prompt half is fine (~hours, Chromium only — dead on iOS Safari). **Drop the offline/sync badge** — the app is pure localStorage with no sync; the badge would be reassurance theater. |
| **ISO A-series / metric-native presets** | A-series are *sizes* sharing one ratio (proposal conflated with aspect ratios). Adding sizes is cheap but the lists are hardcoded per-platform (no drift check); metric values render as ugly fractions in inch mode; "metric-native defaults" is large new cross-platform infrastructure. |
| **Stock-length waste/yield estimator** | Only the "design exceeds one stock length" warning (in `validation.rs`) is a clean low-effort win. The compelling batch/offcut value needs a multi-design/job model the single-frame app lacks. |
| **Float-mount / shadow-box spacer** | A new persisted thickness field = ~13 hand-duplicated sites across 4 layers + a URL format version bump, for thin standalone value (no quoting engine to make depth meaningful). |
| **Side-by-side comparison** | The SVG compositor is genuinely cheap (reuse `generate_combined_view`'s nesting), but diff-overlay + dual-platform selection UI + multi-up PDF inflate it to week+ for a feature most single-design users won't reach. |
| **Siri Shortcuts / URL-scheme** | URL-scheme deep-linking is ~a day (building blocks exist); real Siri needs native App Intents (Swift) — niche for a calculator. Couples to the shareable URL format (do H5-style versioning first). |
| **Cut-pieces (exploded) view** | Parked by design; the numbers already exist via `get_cut_list`; the spec's combined-view integration is stale against the now content-aware engine; week+ across core/web/iOS for a cosmetic schematic. |
| **iPad adaptive / split-view layout** | Reusable body widgets help, but "sidebars instead of dialogs" forces dual-presentation across 12 dialog sites + a parallel NavigationRail path + doubled test/screenshot surface. Week+; audience uncertain. |
| **Preset field metadata help** | Overlaps the existing `TypicalRanges`/`getTypicalRangeHint` system; a static glossary delivers most value far cheaper than a core round-trip + new tooltip UI on both platforms. |
| **Calculation-model doc** | The DAG, z-stack, gates, and a worked example already live in `frame.rs`/`validation.rs` docstrings + test comments; a prose doc adds drift risk with no drift check. A `//!` module doc beats a separate file. |
| **Full-URL shareable API in core** | Pitch is partly false — the web app's share path is deployment-relative (`window.location`), so core can't own the domain. ~2-3 literals to dedupe; fold into other sharing work, not standalone. |

---

## Suggested sequencing

1. **Now (one sitting):** B1 decimal-recursion fix → R1 edge-case tests → R5's storage.js test (locks in B1).
2. **Quick polish:** F1 dark-mode diagrams (best value/effort) · F2 print CSS · F3 haptics.
3. **Pick one product bet:** F4 weighted mat (highest framing value) **or** F5 cost estimator (kept minimal) — both are core-first, benefit all platforms.
4. **Cheap parity:** F7 web history panel (Part A) · F6 notes field.
5. **Opportunistic refactors:** R2/R3/R4/R6/R7 only when touching the area or when a feature forces it.

The recurring theme across assessments: **put new capability in the shared core and route platforms through one binding** — every feature implemented per-platform re-creates the SSOT drift the audit spent its effort fixing.

---

## Appendix A — Method & honesty notes

- 76 agents total: 6 subsystem surveys (111 raw ideas) → 1 consolidator (→ 45 features + 24 refactors, 19 dropped) → 69 adversarial assessors.
- The assessors were deliberately skeptical: of 69, **21 were already implemented**, 40 rated "skip", leaving 28 live candidates (the ones above). They repeatedly corrected effort/scope claims — those corrections are folded in.
- Every "already done" verdict was checked against source; B1 was verified by reading `index.html` directly.

## Appendix B — Dropped as already-covered (not re-proposed)

Already in `AUDIT_REPORT.md`/`TODO.md`: presets.json SSOT drift CI (H2/H3/M7) · division-by-zero guards (M1) · mobile formatting consolidation (M2) · serde versioning + migration stubs (H5/H6) · dead-code/stale-doc deletion (H1/H4/M8) · build-script hardening (L7/L8) · web ARIA (L3) · extract pdf-export.js (L4) · text_width_scale calibration (TODO) · strip HTML hardcoded defaults (M3) · FFI/WASM error surfacing (M6) · Consumer→Selector + await setUnits (M5) · web dark-mode *toggle* (exists) · SW cache strategy (M4) · WASM version surfacing (M9) · golden-test conversion of `wide_frame_svg_dump.rs` (L1).
