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
//   - App files (.html/.css/.js/.wasm) are fetched network-first, so deploys
//     reach SW-enabled browsers immediately; the cache is only a fallback
//     for offline use.
//   - CDN libraries and other resources are cached cache-first.
//
// IMPORTANT: bump CACHE_NAME and RUNTIME_CACHE on every deploy. The version
// bump drops stale precached entries (including old ?v= variants) via the
// activate handler below.
// ============================================================================

const CACHE_NAME = 'referenceframe-wasm-v10';
const RUNTIME_CACHE = 'referenceframe-runtime-v10';

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

    // Network-first for app files (always get fresh version during development)
    if (event.request.destination === 'document' ||
        url.pathname.endsWith('.html') ||
        url.pathname.endsWith('.css') ||  // Network-first; cache-busted via query params
        url.pathname.endsWith('.js') ||
        url.pathname.endsWith('.wasm') ||  // Network-first; cache-busted via query params
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
                    // Fallback to cache if offline
                    return caches.match(event.request);
                })
        );
        return;
    }

    // WASM files are now handled by network-first above (removed cache-first block)

    // Cache strategy for CDN resources (jsPDF, svg2pdf, qrcode, etc.)
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

    // Cache strategy for local app resources
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
