# PyScript/Pyodide Performance Optimization Plan

## Current Problem

First-time visitors experience a 30-60 second load time while:
1. Pyodide runtime downloads (~15MB)
2. matplotlib and dependencies download (~30MB)
3. WebAssembly compilation occurs
4. Python interpreter initializes

Subsequent visits are fast due to browser caching, but first impressions matter.

## Optimization Options (Ranked by Feasibility)

### Option 1: HTML Preloading (Easy - Implement First)

**Effort:** Low
**Expected Improvement:** 50-70% reduction in perceived load time
**Server Requirements:** None (works with static hosting)

Add `<link rel="preload">` tags to start downloading resources before they're needed:

```html
<head>
    <!-- Preload Pyodide core files -->
    <link rel="preload"
          href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/pyodide.js"
          as="script"
          crossorigin="anonymous">
    <link rel="preload"
          href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/pyodide.asm.wasm"
          as="fetch"
          crossorigin="anonymous">
    <link rel="preload"
          href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/pyodide.asm.js"
          as="script"
          crossorigin="anonymous">
    <link rel="preload"
          href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/packages.json"
          as="fetch"
          crossorigin="anonymous">

    <!-- Preload matplotlib wheel (largest dependency) -->
    <link rel="preload"
          href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/matplotlib-3.5.2-cp311-cp311-emscripten_3_1_32_wasm32.whl"
          as="fetch"
          crossorigin="anonymous">
</head>
```

**Why it works:** Browser starts downloading in parallel with HTML parsing, rather than waiting for PyScript to request them.

**Reference:** [GitHub Issue #1576](https://github.com/pyodide/pyodide/issues/1576) showed 12s → 4s improvement.

---

### Option 2: Parallel Package Loading (Easy - Implement with Option 1)

**Effort:** Low
**Expected Improvement:** Additional 10-20% on top of preloading
**Server Requirements:** None

Use the `packages` parameter in `loadPyodide()` to download packages during initialization rather than after:

```javascript
// Instead of loading packages after Pyodide initializes...
const pyodide = await loadPyodide();
await pyodide.loadPackage("matplotlib");  // Sequential - slow

// Load packages in parallel with initialization
const pyodide = await loadPyodide({
    packages: ["matplotlib"]  // Downloads during bootstrap
});
```

For PyScript, this is configured in `<py-config>`:

```toml
packages = ["matplotlib"]
```

(We already have this, but combined with preloading it's more effective.)

---

### Option 3: Self-Host Pyodide with Edge Caching (Medium)

**Effort:** Medium
**Expected Improvement:** 20-40% (eliminates CDN latency variance)
**Server Requirements:** ~100MB disk, nginx/caddy

Download Pyodide release and serve from your VPS with aggressive caching:

```bash
# Download Pyodide release
wget https://github.com/pyodide/pyodide/releases/download/0.27.0/pyodide-0.27.0.tar.bz2
tar -xjf pyodide-0.27.0.tar.bz2 -C /var/www/pyodide/

# nginx config with long cache
location /pyodide/ {
    root /var/www;
    expires 1y;
    add_header Cache-Control "public, immutable";
    add_header Access-Control-Allow-Origin "*";

    # Compression for .whl and .js files
    gzip on;
    gzip_types application/javascript application/wasm;
}
```

**Benefits:**
- Consistent latency (your server vs. CDN POPs)
- Full control over caching headers
- Can use HTTP/2 server push

---

### Option 4: Use /pyc/ Distribution (Easy)

**Effort:** Very Low
**Expected Improvement:** ~10-15% faster Python startup
**Server Requirements:** None

Pyodide offers a `/pyc/` distribution with pre-compiled Python bytecode:

```html
<!-- Instead of /full/ -->
<script src="https://cdn.jsdelivr.net/pyodide/v0.27.0/pyc/pyodide.js"></script>
```

The `/pyc/` version skips `.py` → `.pyc` compilation at runtime.

---

### Option 5: Memory Snapshots (Hard - Experimental)

**Effort:** High
**Expected Improvement:** 80-90% (potentially sub-second cold starts)
**Server Requirements:** Node.js for snapshot generation, ~50-100MB storage

This is the "holy grail" but currently experimental in Pyodide.

#### How It Works

1. **Build time:** Initialize Pyodide + matplotlib, capture memory state
2. **Deploy:** Serve the snapshot file alongside your app
3. **Runtime:** Restore from snapshot instead of re-initializing

#### Current API (Experimental/Internal)

```javascript
// Generate snapshot (build script, run in Node.js)
const pyodide = await loadPyodide({
    _makeSnapshot: true,
    packages: ["matplotlib"]
});
// After initialization, pyodide has snapshot data

// Load from snapshot (browser)
const pyodide = await loadPyodide({
    _loadSnapshot: await fetch("/pyodide-snapshot.bin").then(r => r.arrayBuffer())
});
```

#### Challenges

1. **API is internal** - marked with `@ignore`, may change
2. **Compatibility** - snapshots may not work across Pyodide versions
3. **File system state** - your `.py` files need special handling
4. **matplotlib state** - font caches, backends may need reinitialization

#### Who Uses This

- [Cloudflare Workers](https://blog.cloudflare.com/python-workers/) - They built custom snapshot support into their runtime, achieving <1s cold starts
- Not yet mainstream for browser deployments

---

### Option 6: Progressive Loading UI (Easy - UX Improvement)

**Effort:** Low
**Expected Improvement:** Perceived performance (not actual)
**Server Requirements:** None

Show meaningful content while Python loads:

```html
<!-- Show static form immediately -->
<div id="app-shell">
    <h1>ReferenceFrame</h1>
    <form id="inputs">...</form>
    <div id="loading-overlay">
        <div class="spinner"></div>
        <p>Loading calculation engine...</p>
        <p class="hint">First visit takes 30-60s (cached after)</p>
        <progress id="load-progress" max="100" value="0"></progress>
    </div>
</div>

<script>
// Update progress as stages complete
const stages = ['Downloading Python...', 'Loading matplotlib...', 'Ready!'];
</script>
```

Users can read the UI and mentally prepare inputs while waiting.

---

## Recommended Implementation Order

### Phase 1: Quick Wins (Static Hosting Compatible)

1. **Add HTML preloading** - 30 minutes work
2. **Verify packages config** - Already done
3. **Try /pyc/ distribution** - 5 minutes to test
4. **Improve loading UI** - 1-2 hours

Expected result: **~50% improvement** (30-60s → 15-30s)

### Phase 2: Self-Hosting (Requires VPS)

1. **Set up nginx with Pyodide files** - 1-2 hours
2. **Configure caching headers** - 30 minutes
3. **Optional: HTTP/2 push** - 1 hour

Expected result: **Additional 20-30% improvement** (15-30s → 10-20s)

### Phase 3: Memory Snapshots (Experimental)

1. **Create Node.js snapshot generator** - 4-8 hours
2. **Integrate snapshot loading** - 2-4 hours
3. **Handle edge cases** - Unknown (experimental API)

Expected result: **Potentially <5s cold starts** (if it works reliably)

---

## Implementation: Phase 1 Details

### Step 1: Add Preload Tags

Edit `index.html`, add to `<head>` before other scripts:

```html
<!-- Pyodide Preloading for faster startup -->
<link rel="preload" href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/pyodide.asm.wasm" as="fetch" crossorigin="anonymous">
<link rel="preload" href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/pyodide.asm.js" as="script" crossorigin="anonymous">
<link rel="preload" href="https://cdn.jsdelivr.net/pyodide/v0.27.0/full/packages.json" as="fetch" crossorigin="anonymous">
```

### Step 2: Update PyScript CDN URL

Check if PyScript uses the `/pyc/` distribution. In current setup:

```html
<script defer src="https://pyscript.net/releases/2024.1.1/core.js"></script>
```

PyScript handles Pyodide internally - may need to check if there's a config option.

### Step 3: Add Progress Indicator

We already have a loading indicator, but could enhance with stages:

```javascript
// In index.html, before py-script
window.pyodideLoadProgress = {
    stages: ['Initializing...', 'Loading Python...', 'Loading matplotlib...', 'Ready!'],
    current: 0,
    update(stage) {
        document.getElementById('load-stage').textContent = this.stages[stage];
        document.getElementById('load-progress').value = (stage / this.stages.length) * 100;
    }
};
```

---

## Measuring Performance

Before implementing, establish baseline:

```javascript
// Add to index.html
const startTime = performance.now();

// After Pyodide ready
const loadTime = performance.now() - startTime;
console.log(`Pyodide ready in ${(loadTime/1000).toFixed(1)}s`);

// Send to analytics (optional)
// navigator.sendBeacon('/analytics', JSON.stringify({loadTime, cached: ...}));
```

Test scenarios:
1. **Hard refresh** (Ctrl+Shift+R) - No cache
2. **Normal refresh** (F5) - With cache
3. **First visit** (incognito) - No cache, no service worker

---

## Sources

- [Pyodide Preloading Issue #1576](https://github.com/pyodide/pyodide/issues/1576)
- [Pyodide 0.26 Release Blog](https://blog.pyodide.org/posts/0.26-release/) - Memory snapshot discussion
- [Cloudflare Python Workers](https://blog.cloudflare.com/python-workers/) - Production snapshot usage
- [Pyodide Loading Packages Docs](https://pyodide.org/en/stable/usage/loading-packages.html)
- [Pyodide JS API](https://pyodide.org/en/stable/usage/api/js-api.html)
