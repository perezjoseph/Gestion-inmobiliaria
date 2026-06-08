---
inclusion: fileMatch
fileMatchPattern: ["frontend/**/*.rs", "frontend/Cargo.toml", "frontend/index.html", "frontend/Trunk.toml", "frontend/styles/**/*"]
---

# Frontend

## Stack
- Yew 0.23 + Trunk. Functional components with hooks. Tailwind CSS. Routes in `app.rs`.
- Yew contexts/reducers for shared state. JWT in localStorage. `gloo-net` 0.6 in `services/api.rs`. Handle 401 -> login redirect.

## Hooks
- Never `use_effect(move || ...)` without deps. It runs on every render, allocating new closures in WASM memory (which never shrinks). Use `use_effect_with((), ...)` for mount-only.
- Always return cleanup closures for timers/listeners. Without cleanup, old timers fire stale closures causing "closure invoked after being dropped" WASM crashes.
- Clone `UseStateHandle` into a shared variable before `if/else` branches with `spawn_local`. Otherwise the handle moves into the first branch, making the closure `FnOnce` (incompatible with Yew `Callback`).

## Serialization + Localization
- Backend `Decimal` -> JSON string. Use `deserialize_f64_from_any` on frontend `f64` fields.
- All UI text in Spanish. DD/MM/YYYY dates. Proper DOP/USD currency formatting.
- `trunk build --release` for prod. `fallback = "/index.html"` in `Trunk.toml` for SPA routing.

## Anti-Patterns
- Never create `html!` blocks over 150 lines. The macro generates proportional intermediate allocations, causing WASM OOM in debug and slow compilation. Split into sub-components.
- Never pass `String`, `Vec`, `HashMap` directly as props. Each render clones the entire structure. Wrap in `Rc<T>` or use `AttrValue` (which is `Rc<str>`) for strings.
- Never create `Timeout`/`Interval` without storing the handle and returning a cleanup closure. Each render creates a new timer while old ones keep firing on dropped closures.
- Never load Google Fonts via external `<link>` — violates CSP and blocks rendering. Fonts are self-hosted in `frontend/fonts/` with `@font-face` in `styles/fonts.css`.
- Never put filter/search input state directly in `use_effect_with` deps. Every keystroke triggers a re-render, re-runs the effect, and fires an API call. Instead, depend only on `(reload_val, page)` and let the "Filtrar" button bump `reload`. For search inputs, use a separate `applied_search` state that is set only on submit (see `inquilinos.rs`).
