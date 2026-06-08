// Apply dark theme before paint to prevent FOUC
(function () {
    const t = localStorage.getItem('theme');
    if (t === 'dark' || (!t && globalThis.matchMedia('(prefers-color-scheme: dark)').matches)) {
        document.documentElement.dataset.theme = 'dark';
    }
})();
