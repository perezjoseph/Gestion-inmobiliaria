// Remove loading screen once WASM app starts
globalThis.addEventListener("TrunkApplicationStarted", function () {
    const el = document.getElementById("loading");
    if (el) el.remove();
});
