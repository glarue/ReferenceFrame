// Service Worker for ReferenceFrame WASM
// Caches WASM modules, libraries, and app resources for fast subsequent loads
//
// ============================================================================
// CACHE-INVALIDATION STRATEGY
// ============================================================================
// The project's cache-busting mechanism is one shared version token, passed as
// the `v` query parameter on every app asset URL in index.html (CSS, storage.js,
// the WASM JS glue and the .wasm binary) and in PRECACHE_URLS below; the deploy
// step stamps the token into both files.
// Those params bust the HTTP cache for browsers without service worker
// support, and produce distinct cache keys here.
//
// This service worker complements that convention:
//   - Same-origin app files (page navigations and .html/.css/.js/.wasm URLs)
//     are fetched network-first (into CACHE_NAME), so deploys reach
//     SW-enabled browsers immediately; the cache is only a fallback for
//     offline use. Offline page loads ignore the query string, so shared
//     `?d=` links still open from the cached app shell.
//   - The versioned CDN libraries (jsPDF, svg2pdf, qrcode on cdnjs/unpkg)
//     are served cache-first from RUNTIME_CACHE.
//   - Everything else (e.g. manifest.json, Google Fonts CSS/font files) is
//     served cache-first from CACHE_NAME.
//
// IMPORTANT: bump CACHE_NAME and RUNTIME_CACHE on every deploy. The version
// bump drops stale precached entries (including old token variants) via the
// activate handler below.
// ============================================================================

const CACHE_NAME = 'referenceframe-wasm-v18';
const RUNTIME_CACHE = 'referenceframe-runtime-v18';

// Resources to cache immediately on install
// The version tokens must match index.html exactly (the page requests these
// URLs); the deploy step stamps both files with the same token.
const PRECACHE_URLS = [
    './',
    './index.html',
    './styles.css?v=20260923-phase5',
    './storage.js?v=20260923-phase5',
    './manifest.json',
    './pkg/referenceframe_wasm.js?v=20260923-phase5',
    './pkg/referenceframe_wasm_bg.wasm?v=20260923-phase5',
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
    const isDocument = event.request.mode === 'navigate' ||
                       event.request.destination === 'document';

    // Network-first for same-origin app files: page loads and any
    // .html/.css/.js/.wasm URL. (Cross-origin CDN scripts fall through to the
    // cache-first branch below. The SW is not registered on localhost, so
    // this is not a dev-time aid.)
    if (url.origin === self.location.origin && (
        isDocument ||
        url.pathname.endsWith('.html') ||
        url.pathname.endsWith('.css') ||
        url.pathname.endsWith('.js') ||
        url.pathname.endsWith('.wasm') ||
        url.pathname === '/' ||
        url.pathname.endsWith('/'))) {
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
                    // Offline: fall back to the cache. Page loads ignore the
                    // query string so `?d=` share links resolve to the cached
                    // app shell (the design is decoded client-side).
                    if (isDocument) {
                        return caches.match(event.request, { ignoreSearch: true })
                            .then(cached => cached || caches.match('./index.html'));
                    }
                    return caches.match(event.request);
                })
        );
        return;
    }

    // Cache-first for the CDN libraries (URLs are pinned to exact versions)
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
