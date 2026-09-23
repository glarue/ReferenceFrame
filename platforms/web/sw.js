// Service Worker for ReferenceFrame WASM
// Caches WASM modules, libraries, and app resources for fast subsequent loads
//
// ============================================================================
// CACHE-INVALIDATION STRATEGY
// ============================================================================
// The project's documented cache-busting mechanism is `?v=YYYYMMDD-description`
// query params on CSS and WASM/JS imports in index.html (see CLAUDE.md).
// Those params bust the HTTP cache for browsers without service worker
// support, and produce distinct cache keys here.
//
// This service worker complements that convention:
//   - Documents and any .html/.css/.js/.wasm URL are fetched network-first
//     (into CACHE_NAME), so deploys reach SW-enabled browsers immediately;
//     the cache is only a fallback for offline use. There is no origin
//     check, so the CDN scripts (jsPDF, svg2pdf, qrcode — all .js) also take
//     this path, and the CDN cache-first branch / RUNTIME_CACHE below are
//     effectively unused. Offline fallback matches the exact URL (no
//     ignoreSearch), so offline `?d=` share links miss the cache.
//     NOTE: see audit W11.
//   - Everything else (e.g. manifest.json, Google Fonts CSS/font files) is
//     served cache-first from CACHE_NAME.
//
// IMPORTANT: bump CACHE_NAME and RUNTIME_CACHE on every deploy. The version
// bump drops stale precached entries (including old ?v= variants) via the
// activate handler below.
// ============================================================================

const CACHE_NAME = 'referenceframe-wasm-v15';
const RUNTIME_CACHE = 'referenceframe-runtime-v15';

// Resources to cache immediately on install
const PRECACHE_URLS = [
    './',
    './index.html',
    './styles.css',
    './storage.js',
    './manifest.json',
    './pkg/referenceframe_wasm.js',
    './pkg/referenceframe_wasm_bg.wasm',
];

// Install event - precache essential resources
self.addEventListener('install', event => {
    console.log('[SW] Installing service worker...');
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('[SW] Precaching app resources');
                return cache.addAll(PRECACHE_URLS);
            })
            .then(() => self.skipWaiting())
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
    console.log('[SW] Activating service worker...');
    event.waitUntil(
        caches.keys().then(cacheNames => {
            return Promise.all(
                cacheNames.map(cacheName => {
                    if (cacheName !== CACHE_NAME && cacheName !== RUNTIME_CACHE) {
                        console.log('[SW] Deleting old cache:', cacheName);
                        return caches.delete(cacheName);
                    }
                })
            );
        }).then(() => self.clients.claim())
    );
});

// Fetch event - serve from cache when possible, with network fallback
self.addEventListener('fetch', event => {
    const url = new URL(event.request.url);

    // Network-first for documents and any .html/.css/.js/.wasm URL. No origin
    // check: cross-origin CDN scripts (.js) match here too. (The SW is not
    // registered on localhost, so this is not a dev-time aid.) NOTE: see audit W11.
    if (event.request.destination === 'document' ||
        url.pathname.endsWith('.html') ||
        url.pathname.endsWith('.css') ||  // Network-first; styles.css is cache-busted via ?v= in index.html
        url.pathname.endsWith('.js') ||
        url.pathname.endsWith('.wasm') ||  // Network-first; the glue resolves the .wasm URL relative to import.meta.url, so it is fetched WITHOUT the ?v= param
        url.pathname === '/' ||
        url.pathname.endsWith('/')) {
        event.respondWith(
            fetch(event.request)
                .then(response => {
                    // Cache the fresh response
                    if (response && response.status === 200) {
                        const responseToCache = response.clone();
                        caches.open(CACHE_NAME).then(cache => {
                            cache.put(event.request, responseToCache);
                        });
                    }
                    return response;
                })
                .catch(() => {
                    // Fallback to cache if offline (exact-URL match; no
                    // ignoreSearch, so e.g. `?d=` share links miss — audit W11)
                    return caches.match(event.request);
                })
        );
        return;
    }

    // (.wasm requests are handled by the network-first branch above.)

    // Cache-first for CDN hosts. NOTE: effectively unreachable today — the CDN
    // scripts (jsPDF, svg2pdf, qrcode) all end in .js and are taken by the
    // network-first branch above, so RUNTIME_CACHE stays empty. See audit W11.
    if (url.hostname === 'cdnjs.cloudflare.com' ||
        url.hostname === 'unpkg.com') {
        event.respondWith(
            caches.open(RUNTIME_CACHE).then(cache => {
                return cache.match(event.request).then(cachedResponse => {
                    if (cachedResponse) {
                        console.log('[SW] Serving from cache:', event.request.url);
                        return cachedResponse;
                    }

                    console.log('[SW] Fetching and caching:', event.request.url);
                    return fetch(event.request).then(response => {
                        // Only cache successful responses
                        if (response && response.status === 200) {
                            cache.put(event.request, response.clone());
                        }
                        return response;
                    });
                });
            })
        );
        return;
    }

    // Cache-first (into CACHE_NAME) for everything else, same- or cross-origin
    // (e.g. manifest.json, Google Fonts CSS/font files)
    event.respondWith(
        caches.match(event.request).then(cachedResponse => {
            if (cachedResponse) {
                console.log('[SW] Serving from cache:', event.request.url);
                return cachedResponse;
            }

            console.log('[SW] Fetching:', event.request.url);
            return fetch(event.request).then(response => {
                // Don't cache non-GET requests or non-successful responses
                if (event.request.method !== 'GET' || !response || response.status !== 200) {
                    return response;
                }

                // Cache the response for future use
                const responseToCache = response.clone();
                caches.open(CACHE_NAME).then(cache => {
                    cache.put(event.request, responseToCache);
                });

                return response;
            });
        }).catch(error => {
            console.error('[SW] Fetch failed:', error);
            // Could return a custom offline page here
            throw error;
        })
    );
});
