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

## Tests
- Test fixtures are configuration. If a struct literal (`AppConfig`, `ChatbotEnvConfig`, etc.) appears in 3+ test files, hoist a `Type::for_testing()` constructor next to the type definition. Then use struct-update syntax (`..Type::for_testing()`) when a single field needs to vary.
- Shared test helpers (`db_url`, `setup_db`, `with_db`, `shared_rt_and_db`, `GLOBAL_DB_SERIAL`, `JWT_SECRET`) live in `backend/tests/common/mod.rs` and are imported with `mod common;`. Never copy-paste them between integration test files.
- One canonical value per test concern. Two `JWT_SECRET` constants with different strings in two test files is a bug — JWTs signed by one cannot be verified by the other. Same applies to test URLs, mock tokens, and model names.
- A config field rename should touch one file (the `for_testing()` constructor or the shared helper). If you find yourself sed-sweeping more than 2 test files for a single rename, the abstraction is missing — extract before continuing.
