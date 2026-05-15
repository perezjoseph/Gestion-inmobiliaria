---
inclusion: fileMatch
fileMatchPattern: ["backend/**/*.rs", "backend/Cargo.toml", "backend/migrations/**/*"]
---

# Backend + Database

## Architecture
- Actix-web 4, async handlers. Layered: handlers (HTTP) -> services (logic) -> entities (SeaORM).
- Routes in `routes.rs` under `/api/v1/{domain}`. DTOs in `models/`.
- Never manually edit generated entities.

## Database
- PostgreSQL + SeaORM. Never raw SQL. UUIDs for PKs, `DECIMAL` for money, `TIMESTAMPTZ` for dates.
- Migrations: `m{YYYYMMDD}_{NNNNNN}_{name}.rs`. Never modify applied migrations.
- Transactions for multi-step writes. Consistent row ordering to prevent deadlocks.

## Auth + Errors
- JWT via `jsonwebtoken`, passwords via `argon2`. Never plaintext. Extract user from request extensions, never body.
- Errors as JSON: `{ "error": "type", "message": "desc" }` with proper HTTP status codes.
- Env vars only. `tracing` for logging, never `println!`. `#[serde(rename_all = "camelCase")]` for responses.

## Anti-Patterns
- Never block async handlers with sync I/O, `thread::sleep`, or CPU work. Actix runs one worker per core; blocking one starves all requests on that core. Use `spawn_blocking`.
- Never hold `Arc<Mutex<T>>` across `.await`. Deadlocks the async runtime. Prefer `RwLock` for read-heavy state, or per-worker state (Actix clones app per worker, so unshared state needs no locking).
- Never perform writes in GET endpoints (e.g., `mark_overdue` inside `get_stats`). Every read triggers a write transaction, causing contention under concurrent load. Move to background tasks.
- Never validate or compute inside a `DatabaseTransaction`. Holds row locks and connection pool slots. Validate outside, write inside, keep txns short.
- Never loop `find_by_id` for related entities (N+1). 100 parents with 2 relations = 201 queries. Use `find_with_related()` for JOINs, `load_many()` for batch loading, or `is_in()` + `HashMap<Uuid, &Model>` lookups.
- Never skip indexes on FK columns. SeaORM generates entities but not index strategy. Add in migrations for all FKs and frequently filtered columns.
