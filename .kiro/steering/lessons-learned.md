---
inclusion: manual
---

# Lessons Learned

Append: `### YYYY-MM-DD — Topic` with problem + fix + versions.

---

### gloo-net 0.6
`Request::get()` not `Request::new().method()`. Returns `RequestBuilder`; chain `.header()` before `.send().await`.

### Trunk wasm-opt + Rust 2024
`data-wasm-opt="0"` in index.html. Manual wasm-opt with `--enable-bulk-memory -Oz` via `scripts/build-frontend.sh`.

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
