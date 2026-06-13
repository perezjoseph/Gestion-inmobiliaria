// Service Worker for MyHomeVA PWA
// Version-based cache name for cache busting
const CACHE_VERSION = 'v2';
const CACHE_NAME = `myhomeva-${CACHE_VERSION}`;

// App shell files to precache.
// Only stable, always-present URLs belong here. The Rust/WASM and CSS bundles
// are emitted by Trunk with content hashes (filehash = true), so their paths
// are not knowable here — they are cached on demand by the fetch handler.
// Listing non-existent paths (e.g. /main.wasm) would make cache.addAll reject
// atomically, causing the whole install to fail and leaving the old SW active.
const PRECACHE_URLS = [
    '/',
    '/index.html'
];

// Install: precache the app shell
globalThis.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then((cache) => cache.addAll(PRECACHE_URLS))
            .then(() => globalThis.skipWaiting())
    );
});

// Activate: clean up old caches
globalThis.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys()
            .then((keys) => Promise.all(
                keys
                    .filter((key) => key.startsWith('myhomeva-') && key !== CACHE_NAME)
                    .map((key) => caches.delete(key))
            ))
            .then(() => globalThis.clients.claim())
    );
});

// Write a successful static-asset response to the cache, ignoring failures
// (unsupported schemes, quota exceeded, etc.).
function cacheStaticAsset(request, response) {
    const clone = response.clone();
    caches.open(CACHE_NAME)
        .then((cache) => cache.put(request, clone))
        .catch(() => { });
}

// Cache-first strategy: serve from cache, fall back to network, then offline.
async function cacheFirst(request, pathname) {
    const cached = await caches.match(request);
    if (cached) {
        return cached;
    }
    try {
        const response = await fetch(request);
        if (response.ok && isStaticAsset(pathname)) {
            cacheStaticAsset(request, response);
        }
        return response;
    } catch {
        if (request.mode === 'navigate') {
            return caches.match('/index.html');
        }
        return new Response('', { status: 503, statusText: 'Service Unavailable' });
    }
}

// Fetch: cache-first for static assets, network-first for API calls
globalThis.addEventListener('fetch', (event) => {
    const url = new URL(event.request.url);

    // Skip non-http(s) schemes (e.g. chrome-extension://)
    if (!url.protocol.startsWith('http')) {
        return;
    }

    // Skip non-GET requests
    if (event.request.method !== 'GET') {
        return;
    }

    // Network-first for API calls
    if (url.pathname.startsWith('/api/')) {
        return;
    }

    // Cache-first for static assets
    event.respondWith(cacheFirst(event.request, url.pathname));
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
