// Register service worker for PWA support
if ('serviceWorker' in navigator) {
    try {
        await navigator.serviceWorker.register('/service-worker.js');
    } catch (err) {
        console.warn('[SW] Registration failed:', err);
    }
}
