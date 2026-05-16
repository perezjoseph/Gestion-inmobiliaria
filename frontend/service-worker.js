// Service Worker for MyHomeVA PWA
// Version-based cache name for cache busting
const CACHE_VERSION = 'v1';
const CACHE_NAME = `myhomeva-${CACHE_VERSION}`;

// App shell files to precache
const PRECACHE_URLS = [
    '/',
    '/index.html',
    '/main.wasm',
    '/main.js',
    '/tailwind.css'
];

// Install: precache the app shell
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            return cache.addAll(PRECACHE_URLS);
        }).then(() => self.skipWaiting())
    );
});

// Activate: clean up old caches
self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) => {
            return Promise.all(
                keys
                    .filter((key) => key.startsWith('myhomeva-') && key !== CACHE_NAME)
                    .map((key) => caches.delete(key))
            );
        }).then(() => self.clients.claim())
    );
});

// Fetch: cache-first for static assets, network-first for API calls
self.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // Skip non-GET requests
    if (event.request.method !== 'GET') {
        return;
    }

    // Network-first for API calls
    if (url.pathname.startsWith('/api/')) {
        return;
    }

    // Cache-first for static assets
    event.respondWith(
        caches.match(event.request).then((cached) => {
            if (cached) {
                return cached;
            }
            return fetch(event.request).then((response) => {
                // Cache successful responses for static assets
                if (response.ok && isStaticAsset(url.pathname)) {
                    const clone = response.clone();
                    caches.open(CACHE_NAME).then((cache) => {
                        cache.put(event.request, clone);
                    });
                }
                return response;
            }).catch(() => {
                // Offline fallback: serve cached index.html for navigation requests
                if (event.request.mode === 'navigate') {
                    return caches.match('/index.html');
                }
                return new Response('', { status: 503, statusText: 'Service Unavailable' });
            });
        })
    );
});

function isStaticAsset(pathname) {
    return pathname.endsWith('.wasm') ||
        pathname.endsWith('.js') ||
        pathname.endsWith('.css') ||
        pathname.endsWith('.png') ||
        pathname.endsWith('.svg') ||
        pathname.endsWith('.ico') ||
        pathname.endsWith('.webmanifest');
}
