---
inclusion: fileMatch
fileMatchPattern: ["frontend/**/*.rs", "backend/**/*.rs", "frontend/Trunk.toml", "frontend/index.html"]
---

# Lessons Learned

Append: `### YYYY-MM-DD — Topic` with problem + fix + versions.

---

### rust_decimal -> JSON strings
Backend `Decimal` serializes as `"1500.00"`. Use `deserialize_f64_from_any` on frontend `f64` fields.

### gloo-net 0.6
`Request::get()` not `Request::new().method()`. Returns `RequestBuilder`; chain `.header()` before `.send().await`.

### use_effect without deps = every render
Always `use_effect_with((), ...)` for mount-only. Return cleanup for timers/listeners.

### UseStateHandle branching
Clone before `if/else` with `spawn_local`, otherwise closure becomes `FnOnce`.

### Trunk wasm-opt + Rust 2024
`data-wasm-opt="0"` in index.html. Manual wasm-opt with `--enable-bulk-memory -Oz` via `scripts/build-frontend.sh`.

### Trunk SPA fallback
`fallback = "/index.html"` in `Trunk.toml [serve]`.

### WASM OOM (debug)
Split large `html!` into sub-components. Stack size: `link-args=-z stack-size=5242880`.

### Dashboard N+1
Batch with `is_in()` + HashMap lookups. `tokio::try_join!()` for independent queries.

### dotenvy in tests
Re-populates env from `.env`. Use `unwrap_or_else(|e| e.into_inner())` on mutex.

### actix-governor 0.6
`seconds_per_request(6)` replaces deprecated `per_second()`.

### Yew Link: no style prop
Use `classes` prop only. Wrap in styled `<span>` if needed.
