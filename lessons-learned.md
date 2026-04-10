# Lessons Learned

Document non-obvious solutions, gotchas, and important discoveries here.

### 2026-04-09 — rust_decimal::Decimal serializes as JSON strings, not numbers

When the backend uses `rust_decimal::Decimal` (required for monetary precision with PostgreSQL DECIMAL columns), serde serializes these values as JSON strings (e.g., `"0"`, `"1500.00"`), not as JSON numbers. Frontend types using `f64` will fail to deserialize with "invalid type: string, expected f64". Fix: use a custom serde visitor (`deserialize_any`) that accepts both numbers and numeric strings. Applied to all monetary fields in `frontend/src/types/` via shared helpers `deserialize_f64_from_any` and `deserialize_option_f64_from_any` in `frontend/src/types/mod.rs`.

### 2026-04-09 — Dashboard stats endpoint performs writes on every read via mark_overdue

The `dashboard::get_stats` service calls `pagos::mark_overdue(db).await?` as its first operation, which runs an `UPDATE` query to mark overdue payments. This means every GET request to `/api/dashboard/stats` triggers a write transaction, causing unnecessary database load and potential contention under concurrent reads. The overdue marking should be moved to a scheduled background task (e.g., a tokio cron job) or a separate admin endpoint, keeping the stats endpoint as a pure read. Relevant crates: sea-orm 1.x, actix-web 4.x.

### 2026-04-09 — gloo-net 0.6 API Breaking Changes

In gloo-net 0.6, the HTTP request API changed significantly from earlier versions. `Request::new(&url).method(Method::GET)` no longer works. Instead, use method-specific constructors: `Request::get(&url)`, `Request::post(&url)`, `Request::put(&url)`, `Request::delete(&url)`. These return `RequestBuilder` (not `Request`), and `header()` is a method on `RequestBuilder`. The `body()` method converts `RequestBuilder` into a `Request` which can then be `send()`-ed. Chain the builder methods before calling `.send().await`.

### 2026-04-09 — Yew Link Component Does Not Support style Prop

The `Link<Route>` component in yew-router 0.18 does not accept a `style` prop. Only `classes`, `to`, `query`, `state`, and `disabled` are available. To style a Link, use CSS classes via the `classes` prop instead of inline styles. Wrap in a `<span>` or `<div>` with inline style if absolutely necessary.

### 2026-04-09 — Config tests fail with dotenvy when .env file exists

Tests that manipulate environment variables (e.g., removing `DATABASE_URL` to test error paths) are unreliable when `dotenvy::dotenv()` is called inside the function under test. `dotenvy` loads the `.env` file and sets any missing env vars, so removing a var then calling `from_env()` will re-populate it from `.env`. Additionally, if one test panics, the shared `Mutex` becomes poisoned, cascading failures to all subsequent tests. Fix: use `unwrap_or_else(|e| e.into_inner())` on the mutex lock, and avoid testing "missing required var" scenarios that `dotenvy` will override. Relevant: dotenvy 0.15, Rust std::sync::Mutex.

### 2026-04-09 — Yew toast timeout leak without use_effect cleanup

Creating a `gloo_timers::callback::Timeout` inside a render loop (e.g., inside an iterator in `html!{}`) without storing the handle causes the timeout to fire but never get cancelled on re-render or unmount. Each render creates a new timeout, leading to stale closures dispatching actions on unmounted components. Fix: extract a dedicated child component (`ToastItem`) that creates the timeout inside `use_effect_with`, returning a cleanup closure that drops the `Timeout` handle. This ensures the timeout is cancelled when the component unmounts or the dependency changes. Relevant: gloo-timers 0.3, yew 0.21.
