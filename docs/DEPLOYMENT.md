# Deployment Guide

This guide covers deploying the ReferenceFrame app to GitHub Pages and other platforms.

## Quick Start: GitHub Pages

### Option A: GitHub Actions (Recommended)

A workflow file has been created at `.github/workflows/deploy.yml` that will automatically deploy to GitHub Pages on every push to main.

**Setup steps:**

1. Push the workflow file to your repository
2. In GitHub: **Settings → Pages → Source: GitHub Actions**
3. Push any change to main branch (or use workflow_dispatch for manual trigger)

The workflow will:
- Deploy `index.html`, `manifest.json`, `sw.js`
- Include the `src/` directory with all Python modules
- Run automatically on push to main

Your app will be at: `https://<username>.github.io/<repo-name>/`

### Option B: Manual Deploy from `gh-pages` Branch

```bash
# From the repository root
git checkout --orphan gh-pages
git reset --hard

# Add only deployment files
git checkout main -- index.html manifest.json sw.js
git checkout main -- src/

# Commit and push
git add -A
git commit -m "Deploy ReferenceFrame"
git push origin gh-pages

# Go back to main branch
git checkout main
```

Then in GitHub: **Settings → Pages → Source: gh-pages branch**

---

## File Structure for Deployment

These files should be at the root of your deployed site:

```
/
├── index.html          # Main app
├── manifest.json       # PWA manifest
├── sw.js               # Service worker
└── src/                # Python modules
    ├── frame.py            # Core calculations
    ├── conversions.py      # Unit formatting
    ├── defaults.py         # Default values
    ├── ui_helpers.py       # UI utilities
    └── aspect_ratio.py     # Aspect ratio logic
```

---

## Platform-Agnostic Considerations

### Base URL Handling

If deploying to a subdirectory (e.g., `github.io/repo-name/`), ensure paths work:

```html
<!-- Use relative paths, not absolute -->
<link rel="stylesheet" href="./styles.css">  <!-- Good -->
<link rel="stylesheet" href="/styles.css">   <!-- Bad on subdirectory -->
```

The current codebase uses relative paths and should work anywhere.

### Service Worker Scope

The service worker (`sw.js`) must be at the root of the deployed directory. It automatically scopes to its location.

### CORS for CDN Resources

All external resources (PyScript, jsPDF, etc.) are loaded from CDNs that support CORS. No server configuration needed.

---

## Deploying to Other Platforms

### Cloudflare Pages

1. Connect GitHub repo to Cloudflare Pages
2. Set build output directory: `.` (root)
3. No build command needed (static files)
4. Optionally add `_headers` file for better caching:

```
/*.wasm
  Cache-Control: public, max-age=31536000, immutable

/*.whl
  Cache-Control: public, max-age=31536000, immutable

/*.py
  Cache-Control: public, max-age=3600
```

### Netlify

1. Connect GitHub repo
2. Set publish directory: `.` (root)
3. Optionally add `netlify.toml`:

```toml
[build]
  publish = "."

[[headers]]
  for = "/*.wasm"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
  for = "/*.py"
  [headers.values]
    Cache-Control = "public, max-age=3600"
```

### Self-Hosted (nginx)

```nginx
server {
    listen 80;
    server_name your-domain.com;
    root /var/www/referenceframe;
    index index.html;

    # MIME type for Python files
    types {
        text/x-python py;
    }

    # Cache static assets aggressively
    location ~* \.(wasm|whl)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

---

## Performance Notes

### What Gets Cached

The service worker (`sw.js`) caches:
- App files (HTML, Python modules) - network-first with cache fallback
- PyScript/Pyodide CDN resources - cache-first after initial load

### First Visit vs. Repeat Visits

| Scenario | Load Time |
|----------|-----------|
| First visit (no cache) | 30-60 seconds |
| Repeat visit (cached) | 1-3 seconds |
| After browser cache cleared | 30-60 seconds |

The service worker ensures cached assets persist even if browser cache is cleared (within same origin).

### Preloading (Implemented)

The `index.html` includes preload hints for critical Pyodide resources to start downloads earlier.

---

## Troubleshooting

### "Service worker not found"

Ensure `sw.js` is at the root of your deployed directory, not in a subdirectory.

### "Python files not loading"

Check browser DevTools → Network tab. Files should load with 200 status. If 404, check:
- Files are in `src/` directory
- Paths in index.html are relative (e.g., `src/frame.py`)

### "CORS error on PyScript"

This shouldn't happen with CDN-hosted PyScript. If self-hosting, ensure proper CORS headers.

### "App works locally but not on GitHub Pages"

Check for absolute paths (`/something`) that should be relative (`./something`).

---

## Testing Deployment Locally

```bash
# From repository root
python3 -m http.server 8000

# Open http://localhost:8000
```

This simulates a static file server like GitHub Pages.

---

## Updating the Deployed App

### With GitHub Actions

Just push to main branch - deployment is automatic.

### With Manual gh-pages Branch

```bash
git checkout gh-pages
git merge main --no-commit
git checkout main -- index.html manifest.json sw.js src/
git commit -m "Update deployment"
git push origin gh-pages
git checkout main
```