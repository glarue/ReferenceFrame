// Service Worker for ReferenceFrame PyScript PoC
// Caches PyScript runtime, matplotlib, and app resources for fast subsequent loads

const CACHE_NAME = 'referenceframe-pyscript-v6';
const RUNTIME_CACHE = 'referenceframe-runtime-v3';

// Resources to cache immediately on install
const PRECACHE_URLS = [
    './',
    './index.html',
    './styles.css',
    './app.js',
    './manifest.json',
    './src/main.py',
    './src/frame.py',
    './src/conversions.py',
    './src/defaults.py',
    './src/ui_helpers.py',
    './src/aspect_ratio.py',
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
    if (event.request.destination === 'document' || url.pathname.endsWith('.html') || url.pathname.endsWith('.py') || url.pathname === '/' || url.pathname.endsWith('/')) {
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

    // Cache strategy for CDN resources (PyScript, Pyodide, jsPDF, etc.)
    if (url.hostname === 'cdn.jsdelivr.net' || url.hostname === 'pyscript.net' || url.hostname === 'cdnjs.cloudflare.com' || url.hostname === 'unpkg.com') {
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
