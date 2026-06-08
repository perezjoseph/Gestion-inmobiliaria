// Global error handlers for WASM loading failures
globalThis.addEventListener('error', function (e) {
    console.error('[WASM DEBUG] Global error:', e.message, e.filename, e.lineno);
    const el = document.getElementById('loading');
    if (el) {
        el.innerHTML = '<div style="text-align:center;padding:2rem;"><h2 style="color:red;">Error loading app</h2><pre style="text-align:left;max-width:600px;margin:1rem auto;overflow:auto;font-size:12px;">' + e.message + '\n' + (e.filename || '') + ':' + (e.lineno || '') + '</pre></div>';
    }
});
globalThis.addEventListener('unhandledrejection', function (e) {
    console.error('[WASM DEBUG] Unhandled rejection:', e.reason);
    const el = document.getElementById('loading');
    if (el) {
        el.innerHTML = '<div style="text-align:center;padding:2rem;"><h2 style="color:red;">Error loading app</h2><pre style="text-align:left;max-width:600px;margin:1rem auto;overflow:auto;font-size:12px;">' + (e.reason?.message ?? e.reason) + '\n' + (e.reason?.stack ?? '') + '</pre></div>';
    }
});
