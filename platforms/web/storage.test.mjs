/**
 * Node-runnable unit tests for storage.js.
 *
 * storage.js is a browser file: its functions are plain globals declared with
 * `function` and `const`, NOT module exports. To test it under Node without
 * altering browser behavior, we load the source into a node:vm sandbox with
 * fake browser globals (localStorage, console, window, document) and read the
 * function declarations back off the sandbox's context object.
 *
 * Run with:  node --test storage.test.mjs   (from platforms/web)
 *
 * No npm dependencies and no package.json required.
 *
 * Notes on what is / isn't covered:
 * - All the localStorage-backed persistence + versioning + import/export merge
 *   logic is covered (these are the audit-flagged risk areas).
 * - applyTheme() / getEffectiveTheme() touch document.documentElement and
 *   window.matchMedia. We provide minimal stubs and assert the parts that are
 *   reasonable to assert (saveThemePreference side-effect, default fallthrough),
 *   but the DOM mutation itself is not deeply exercised since there is no real
 *   document.
 */

import { test } from 'node:test';
// NB: use the *non-strict* assert. storage.js runs inside a node:vm realm, so
// values returned from it (via JSON.parse) carry that realm's Object/Array
// prototypes. assert/strict's deepStrictEqual rejects those as not
// reference-equal across realms; non-strict deepEqual compares structurally.
// Scalar checks below use assert.equal (loose ==), which is fine for the
// strings/numbers/booleans/null involved.
import assert from 'node:assert';
import vm from 'node:vm';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SOURCE = fs.readFileSync(path.join(__dirname, 'storage.js'), 'utf8');

/**
 * Build a Map-backed fake localStorage matching the Web Storage API surface
 * that storage.js relies on (getItem/setItem/removeItem) plus clear/key/length
 * for completeness.
 */
function makeLocalStorage() {
    const store = new Map();
    return {
        getItem(key) {
            return store.has(key) ? store.get(key) : null;
        },
        setItem(key, value) {
            // The real API coerces values to strings; mirror that so tests
            // can't accidentally pass non-strings through.
            store.set(String(key), String(value));
        },
        removeItem(key) {
            store.delete(key);
        },
        clear() {
            store.clear();
        },
        key(i) {
            return [...store.keys()][i] ?? null;
        },
        get length() {
            return store.size;
        },
        // test-only escape hatch for inspecting raw persisted JSON
        _raw(key) {
            return store.has(key) ? store.get(key) : null;
        },
    };
}

/**
 * Fresh sandbox per test: load storage.js into a vm context with fake browser
 * globals and return { fns, ctx, localStorage, warnings, errors }.
 */
function loadStorage(overrides = {}) {
    const localStorage = overrides.localStorage ?? makeLocalStorage();
    const warnings = [];
    const errors = [];

    const matchMediaMatches = overrides.prefersLight ?? false;

    const sandbox = {
        localStorage,
        console: {
            log() {},
            warn(...args) { warnings.push(args.join(' ')); },
            error(...args) { errors.push(args.join(' ')); },
        },
        // Minimal DOM/window stubs for applyTheme / getEffectiveTheme.
        document: {
            documentElement: {
                _attrs: {},
                setAttribute(name, value) { this._attrs[name] = value; },
                getAttribute(name) { return this._attrs[name]; },
            },
        },
        window: {
            matchMedia(_query) {
                return { matches: matchMediaMatches };
            },
        },
    };

    const ctx = vm.createContext(sandbox);
    vm.runInContext(SOURCE, ctx, { filename: 'storage.js' });

    // Function declarations attach to the context object; const (STORAGE_KEYS)
    // does not, so read it via an in-context expression.
    const KEYS = vm.runInContext('STORAGE_KEYS', ctx);

    return { ctx, localStorage, warnings, errors, KEYS, sandbox };
}

// ---------------------------------------------------------------------------
// Schema versioning: saveVersionedList / loadVersionedList / migrateStoredData
// ---------------------------------------------------------------------------

test('saveVersionedList writes { version, items } envelope', () => {
    const { ctx, localStorage } = loadStorage();
    ctx.saveVersionedList('k', [{ a: 1 }]);
    const raw = JSON.parse(localStorage._raw('k'));
    assert.equal(raw.version, vm.runInContext('STORAGE_SCHEMA_VERSION', ctx));
    assert.deepEqual(raw.items, [{ a: 1 }]);
});

test('loadVersionedList round-trips a saved list', () => {
    const { ctx } = loadStorage();
    const items = [{ name: 'a', config: { x: 1 } }, { name: 'b', config: { y: 2 } }];
    ctx.saveVersionedList('mykey', items);
    assert.deepEqual(ctx.loadVersionedList('mykey'), items);
});

test('loadVersionedList returns [] for a missing key', () => {
    const { ctx } = loadStorage();
    assert.deepEqual(ctx.loadVersionedList('does-not-exist'), []);
});

test('loadVersionedList upgrades a legacy bare-array payload in place', () => {
    const { ctx, localStorage } = loadStorage();
    const legacy = [{ name: 'old', config: {} }];
    // Simulate PyScript-era unversioned payload.
    localStorage.setItem('legacyKey', JSON.stringify(legacy));

    const loaded = ctx.loadVersionedList('legacyKey');
    assert.deepEqual(loaded, legacy);

    // It must have been rewritten in versioned form.
    const raw = JSON.parse(localStorage._raw('legacyKey'));
    assert.equal(raw.version, vm.runInContext('STORAGE_SCHEMA_VERSION', ctx));
    assert.deepEqual(raw.items, legacy);
    assert.ok(!Array.isArray(raw), 'payload should no longer be a bare array');
});

test('loadVersionedList defaults missing items to []', () => {
    const { ctx } = loadStorage();
    ctx.localStorage.setItem('k', JSON.stringify({ version: 1 })); // no items field
    assert.deepEqual(ctx.loadVersionedList('k'), []);
});

test('loadVersionedList rewrites payload when stored version != current', () => {
    const { ctx, localStorage } = loadStorage();
    const current = vm.runInContext('STORAGE_SCHEMA_VERSION', ctx);
    // Write a payload tagged with an older version number (0).
    localStorage.setItem('k', JSON.stringify({ version: 0, items: [{ z: 1 }] }));
    const loaded = ctx.loadVersionedList('k');
    assert.deepEqual(loaded, [{ z: 1 }]);
    const raw = JSON.parse(localStorage._raw('k'));
    assert.equal(raw.version, current, 'should be rewritten to current version');
});

test('migrateStoredData is a pass-through for current version', () => {
    const { ctx } = loadStorage();
    const data = [{ a: 1 }];
    assert.equal(ctx.migrateStoredData(data, 1), data); // same reference, pass-through
});

test('migrateStoredData warns when stored version is newer than supported', () => {
    const { ctx, warnings } = loadStorage();
    const current = vm.runInContext('STORAGE_SCHEMA_VERSION', ctx);
    const data = [{ a: 1 }];
    const out = ctx.migrateStoredData(data, current + 1);
    assert.equal(out, data, 'still returns the data as-is');
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /newer than supported/);
});

test('loadVersionedList tolerates a future version number via migrate warning', () => {
    const { ctx, warnings } = loadStorage();
    const current = vm.runInContext('STORAGE_SCHEMA_VERSION', ctx);
    ctx.localStorage.setItem('k', JSON.stringify({ version: current + 5, items: [{ q: 1 }] }));
    const loaded = ctx.loadVersionedList('k');
    assert.deepEqual(loaded, [{ q: 1 }]);
    assert.equal(warnings.length, 1);
    assert.match(warnings[0], /newer than supported/);
});

// ---------------------------------------------------------------------------
// Saved configs CRUD
// ---------------------------------------------------------------------------

test('loadSavedConfigs returns [] when nothing stored', () => {
    const { ctx } = loadStorage();
    assert.deepEqual(ctx.loadSavedConfigs(), []);
});

test('loadSavedConfigs returns [] (not throw) on corrupt JSON', () => {
    const { ctx, KEYS, errors } = loadStorage();
    ctx.localStorage.setItem(KEYS.CONFIGS, '{not valid json');
    assert.deepEqual(ctx.loadSavedConfigs(), []);
    assert.equal(errors.length, 1);
});

test('saveConfig then getConfigByName round-trips the config', () => {
    const { ctx } = loadStorage();
    const cfg = { width: 10, height: 8 };
    assert.equal(ctx.saveConfig('My Frame', cfg), true);
    assert.deepEqual(ctx.getConfigByName('My Frame'), cfg);
});

test('getConfigByName returns null for unknown name', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.getConfigByName('nope'), null);
});

test('saveConfig overwrites an existing config of the same name', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('dup', { v: 1 });
    ctx.saveConfig('dup', { v: 2 });
    const all = ctx.loadSavedConfigs();
    assert.equal(all.length, 1, 'no duplicate names');
    assert.deepEqual(ctx.getConfigByName('dup'), { v: 2 });
});

test('deleteConfig removes only the named config', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('a', { v: 1 });
    ctx.saveConfig('b', { v: 2 });
    assert.equal(ctx.deleteConfig('a'), true);
    const all = ctx.loadSavedConfigs();
    assert.equal(all.length, 1);
    assert.equal(all[0].name, 'b');
});

// ---------------------------------------------------------------------------
// Custom sizes CRUD
// ---------------------------------------------------------------------------

test('saveCustomSize then loadCustomSizes round-trips', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.saveCustomSize('8x10', 8, 10), true);
    assert.deepEqual(ctx.loadCustomSizes(), [{ name: '8x10', height: 8, width: 10 }]);
});

test('saveCustomSize overwrites same-named size', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomSize('s', 1, 1);
    ctx.saveCustomSize('s', 2, 3);
    const sizes = ctx.loadCustomSizes();
    assert.equal(sizes.length, 1);
    assert.deepEqual(sizes[0], { name: 's', height: 2, width: 3 });
});

test('deleteCustomSize removes only the named size', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomSize('a', 1, 1);
    ctx.saveCustomSize('b', 2, 2);
    assert.equal(ctx.deleteCustomSize('a'), true);
    const sizes = ctx.loadCustomSizes();
    assert.equal(sizes.length, 1);
    assert.equal(sizes[0].name, 'b');
});

// ---------------------------------------------------------------------------
// Current settings
// ---------------------------------------------------------------------------

test('saveCurrentSettings / loadCurrentSettings round-trip', () => {
    const { ctx } = loadStorage();
    const settings = { width: 12, units: 'inches', nested: { a: [1, 2] } };
    ctx.saveCurrentSettings(settings);
    assert.deepEqual(ctx.loadCurrentSettings(), settings);
});

test('loadCurrentSettings returns null when nothing stored', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.loadCurrentSettings(), null);
});

test('loadCurrentSettings returns null (not throw) on corrupt JSON', () => {
    const { ctx, KEYS, errors } = loadStorage();
    ctx.localStorage.setItem(KEYS.SETTINGS, 'garbage{');
    assert.equal(ctx.loadCurrentSettings(), null);
    assert.equal(errors.length, 1);
});

// ---------------------------------------------------------------------------
// Scalar preferences with defaults: unit / theme / display format
// ---------------------------------------------------------------------------

test('loadUnitPreference defaults to "inches"', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.loadUnitPreference(), 'inches');
});

test('saveUnitPreference / loadUnitPreference round-trip', () => {
    const { ctx } = loadStorage();
    ctx.saveUnitPreference('mm');
    assert.equal(ctx.loadUnitPreference(), 'mm');
});

test('loadThemePreference defaults to "system"', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.loadThemePreference(), 'system');
});

test('saveThemePreference / loadThemePreference round-trip', () => {
    const { ctx } = loadStorage();
    ctx.saveThemePreference('dark');
    assert.equal(ctx.loadThemePreference(), 'dark');
});

test('loadDisplayFormat defaults to "fractions"', () => {
    const { ctx } = loadStorage();
    assert.equal(ctx.loadDisplayFormat(), 'fractions');
});

test('saveDisplayFormat / loadDisplayFormat round-trip', () => {
    const { ctx } = loadStorage();
    ctx.saveDisplayFormat('decimal');
    assert.equal(ctx.loadDisplayFormat(), 'decimal');
});

// ---------------------------------------------------------------------------
// Theme helpers that touch the DOM/window stubs
// ---------------------------------------------------------------------------

test('applyTheme sets data-theme attribute and persists the choice', () => {
    const { ctx, sandbox } = loadStorage();
    ctx.applyTheme('dark');
    assert.equal(sandbox.document.documentElement.getAttribute('data-theme'), 'dark');
    assert.equal(ctx.loadThemePreference(), 'dark', 'applyTheme also saves');
});

test('getEffectiveTheme returns saved non-system value directly', () => {
    const { ctx } = loadStorage();
    ctx.saveThemePreference('light');
    assert.equal(ctx.getEffectiveTheme(), 'light');
});

test('getEffectiveTheme resolves "system" via matchMedia (light)', () => {
    const { ctx } = loadStorage({ prefersLight: true });
    ctx.saveThemePreference('system');
    assert.equal(ctx.getEffectiveTheme(), 'light');
});

test('getEffectiveTheme resolves "system" via matchMedia (dark)', () => {
    const { ctx } = loadStorage({ prefersLight: false });
    ctx.saveThemePreference('system');
    assert.equal(ctx.getEffectiveTheme(), 'dark');
});

// ---------------------------------------------------------------------------
// Custom colors
// ---------------------------------------------------------------------------

test('loadCustomColors defaults to {}', () => {
    const { ctx } = loadStorage();
    assert.deepEqual(ctx.loadCustomColors(), {});
});

test('saveCustomColor accumulates categories', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomColor('primary', 'blue');
    ctx.saveCustomColor('error', 'flagRed');
    assert.deepEqual(ctx.loadCustomColors(), { primary: 'blue', error: 'flagRed' });
});

test('resetCustomColor removes one category only', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomColor('primary', 'blue');
    ctx.saveCustomColor('error', 'flagRed');
    ctx.resetCustomColor('primary');
    assert.deepEqual(ctx.loadCustomColors(), { error: 'flagRed' });
});

test('resetAllCustomColors clears everything', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomColor('primary', 'blue');
    ctx.resetAllCustomColors();
    assert.deepEqual(ctx.loadCustomColors(), {});
});

test('loadCustomColors returns {} (not throw) on corrupt JSON', () => {
    const { ctx, KEYS, errors } = loadStorage();
    ctx.localStorage.setItem(KEYS.CUSTOM_COLORS, 'nope');
    assert.deepEqual(ctx.loadCustomColors(), {});
    assert.equal(errors.length, 1);
});

// ---------------------------------------------------------------------------
// Custom defaults
// ---------------------------------------------------------------------------

test('loadCustomDefaults defaults to {}', () => {
    const { ctx } = loadStorage();
    assert.deepEqual(ctx.loadCustomDefaults(), {});
});

test('saveCustomDefault accumulates fields', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomDefault('matWidth', 2.5);
    ctx.saveCustomDefault('frameDepth', 0.75);
    assert.deepEqual(ctx.loadCustomDefaults(), { matWidth: 2.5, frameDepth: 0.75 });
});

test('resetCustomDefault removes one field only', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomDefault('matWidth', 2.5);
    ctx.saveCustomDefault('frameDepth', 0.75);
    ctx.resetCustomDefault('matWidth');
    assert.deepEqual(ctx.loadCustomDefaults(), { frameDepth: 0.75 });
});

test('resetAllCustomDefaults clears everything', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomDefault('matWidth', 2.5);
    ctx.resetAllCustomDefaults();
    assert.deepEqual(ctx.loadCustomDefaults(), {});
});

// ---------------------------------------------------------------------------
// Export / Import (the high-risk merge-vs-replace logic)
// ---------------------------------------------------------------------------

test('exportAllData captures all stores plus passed settings/unit', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('c1', { v: 1 });
    ctx.saveCustomSize('s1', 1, 2);
    ctx.saveCustomColor('primary', 'blue');
    ctx.saveCustomDefault('matWidth', 2);

    const json = ctx.exportAllData({ width: 10 }, 'mm');
    const data = JSON.parse(json);

    assert.equal(data.version, '1.0');
    assert.ok(typeof data.exported_at === 'string' && data.exported_at.length > 0);
    assert.deepEqual(data.saved_configs, [{ name: 'c1', config: { v: 1 } }]);
    assert.deepEqual(data.custom_sizes, [{ name: 's1', height: 1, width: 2 }]);
    assert.deepEqual(data.custom_colors, { primary: 'blue' });
    assert.deepEqual(data.custom_defaults, { matWidth: 2 });
    assert.deepEqual(data.current_settings, { width: 10 });
    assert.equal(data.unit, 'mm');
});

test('export then import (replace) into a fresh store reproduces the data', () => {
    const src = loadStorage();
    src.ctx.saveConfig('c1', { v: 1 });
    src.ctx.saveCustomSize('s1', 3, 4);
    src.ctx.saveCustomColor('primary', 'teal');
    src.ctx.saveCustomDefault('matWidth', 1.5);
    const json = src.ctx.exportAllData({ width: 7 }, 'mm');

    const dst = loadStorage();
    const result = dst.ctx.importData(json, 'replace');
    assert.equal(result.success, true);
    assert.deepEqual(dst.ctx.loadSavedConfigs(), [{ name: 'c1', config: { v: 1 } }]);
    assert.deepEqual(dst.ctx.loadCustomSizes(), [{ name: 's1', height: 3, width: 4 }]);
    assert.deepEqual(dst.ctx.loadCustomColors(), { primary: 'teal' });
    assert.deepEqual(dst.ctx.loadCustomDefaults(), { matWidth: 1.5 });
    assert.equal(dst.ctx.loadUnitPreference(), 'mm');
});

test('importData rejects payloads without version 1.0', () => {
    const { ctx } = loadStorage();
    const r1 = ctx.importData(JSON.stringify({ saved_configs: [] }), 'merge');
    assert.equal(r1.success, false);
    assert.match(r1.message, /Unsupported backup version/);

    const r2 = ctx.importData(JSON.stringify({ version: '2.0' }), 'merge');
    assert.equal(r2.success, false);
});

test('importData returns failure (not throw) on malformed JSON', () => {
    const { ctx, errors } = loadStorage();
    const r = ctx.importData('this is not json', 'merge');
    assert.equal(r.success, false);
    assert.match(r.message, /Import failed:/);
    assert.equal(errors.length, 1);
});

test('importData defaults to merge mode when mode omitted', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('existing', { keep: true });
    const json = JSON.stringify({
        version: '1.0',
        saved_configs: [{ name: 'incoming', config: { v: 1 } }],
    });
    const r = ctx.importData(json); // no mode arg -> merge
    assert.equal(r.success, true);
    const names = ctx.loadSavedConfigs().map(c => c.name).sort();
    assert.deepEqual(names, ['existing', 'incoming']);
    assert.match(r.message, /^Merged /);
});

test('import merge keeps existing config on name collision (no overwrite)', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('shared', { source: 'existing' });
    const json = JSON.stringify({
        version: '1.0',
        saved_configs: [
            { name: 'shared', config: { source: 'imported' } },
            { name: 'fresh', config: { source: 'imported' } },
        ],
    });
    const r = ctx.importData(json, 'merge');
    assert.equal(r.success, true);
    // Existing wins; only non-colliding 'fresh' is added.
    assert.deepEqual(ctx.getConfigByName('shared'), { source: 'existing' });
    assert.deepEqual(ctx.getConfigByName('fresh'), { source: 'imported' });
    assert.equal(ctx.loadSavedConfigs().length, 2);
});

test('import replace overwrites existing configs entirely', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('old', { source: 'existing' });
    const json = JSON.stringify({
        version: '1.0',
        saved_configs: [{ name: 'new', config: { source: 'imported' } }],
    });
    const r = ctx.importData(json, 'replace');
    assert.equal(r.success, true);
    const names = ctx.loadSavedConfigs().map(c => c.name);
    assert.deepEqual(names, ['new'], 'pre-existing config replaced away');
    assert.match(r.message, /^Imported /);
});

test('import merge skips duplicate custom sizes by name', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomSize('dup', 1, 1);
    const json = JSON.stringify({
        version: '1.0',
        custom_sizes: [
            { name: 'dup', height: 9, width: 9 },
            { name: 'unique', height: 2, width: 2 },
        ],
    });
    ctx.importData(json, 'merge');
    const sizes = ctx.loadCustomSizes();
    assert.equal(sizes.length, 2);
    // Existing 'dup' kept (1x1), 'unique' added.
    assert.deepEqual(sizes.find(s => s.name === 'dup'), { name: 'dup', height: 1, width: 1 });
    assert.deepEqual(sizes.find(s => s.name === 'unique'), { name: 'unique', height: 2, width: 2 });
});

test('import merge: existing custom colors take precedence over imported', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomColor('primary', 'existingBlue');
    const json = JSON.stringify({
        version: '1.0',
        custom_colors: { primary: 'importedRed', accent: 'importedTeal' },
    });
    ctx.importData(json, 'merge');
    // { ...imported, ...existing } => existing wins for primary; accent filled in.
    assert.deepEqual(ctx.loadCustomColors(), {
        primary: 'existingBlue',
        accent: 'importedTeal',
    });
});

test('import merge: existing custom defaults take precedence over imported', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomDefault('matWidth', 3);
    const json = JSON.stringify({
        version: '1.0',
        custom_defaults: { matWidth: 99, frameDepth: 0.5 },
    });
    ctx.importData(json, 'merge');
    assert.deepEqual(ctx.loadCustomDefaults(), { matWidth: 3, frameDepth: 0.5 });
});

test('import replace: custom colors/defaults overwrite wholesale', () => {
    const { ctx } = loadStorage();
    ctx.saveCustomColor('primary', 'existingBlue');
    ctx.saveCustomDefault('matWidth', 3);
    const json = JSON.stringify({
        version: '1.0',
        custom_colors: { primary: 'importedRed' },
        custom_defaults: { frameDepth: 0.5 },
    });
    ctx.importData(json, 'replace');
    assert.deepEqual(ctx.loadCustomColors(), { primary: 'importedRed' });
    assert.deepEqual(ctx.loadCustomDefaults(), { frameDepth: 0.5 });
});

test('import message counts reflect the incoming payload size, not the merge result', () => {
    const { ctx } = loadStorage();
    ctx.saveConfig('dup', { v: 0 }); // will collide and be skipped on merge
    const json = JSON.stringify({
        version: '1.0',
        saved_configs: [
            { name: 'dup', config: { v: 1 } },
            { name: 'b', config: {} },
        ],
        custom_sizes: [{ name: 's', height: 1, width: 1 }],
    });
    const r = ctx.importData(json, 'merge');
    // Counts come from the payload arrays: 2 configs, 1 size — even though only
    // 1 config was actually merged in.
    assert.match(r.message, /Merged 2 configurations and 1 custom sizes/);
});

test('importData stores incoming lists in the current versioned envelope', () => {
    const { ctx, KEYS } = loadStorage();
    const json = JSON.stringify({
        version: '1.0',
        saved_configs: [{ name: 'c', config: {} }],
    });
    ctx.importData(json, 'replace');
    const raw = JSON.parse(ctx.localStorage._raw(KEYS.CONFIGS));
    assert.equal(raw.version, vm.runInContext('STORAGE_SCHEMA_VERSION', ctx));
    assert.ok(Array.isArray(raw.items));
});

// ---------------------------------------------------------------------------
// clearAllData
// ---------------------------------------------------------------------------

test('clearAllData removes every known storage key', () => {
    const { ctx, KEYS, localStorage } = loadStorage();
    // Populate every key.
    ctx.saveConfig('c', {});
    ctx.saveCustomSize('s', 1, 1);
    ctx.saveUnitPreference('mm');
    ctx.saveCurrentSettings({ a: 1 });
    ctx.saveThemePreference('dark');
    ctx.saveCustomColor('primary', 'blue');
    ctx.saveCustomDefault('matWidth', 2);
    ctx.saveDisplayFormat('decimal');
    assert.ok(localStorage.length > 0);

    assert.equal(ctx.clearAllData(), true);

    for (const key of Object.values(KEYS)) {
        assert.equal(localStorage.getItem(key), null, `${key} should be cleared`);
    }
    assert.equal(localStorage.length, 0);
});
