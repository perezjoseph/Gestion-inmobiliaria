# Lessons Learned

Document non-obvious solutions, gotchas, and important discoveries here.

### 2026-04-09 — rust_decimal::Decimal serializes as JSON strings, not numbers

When the backend uses `rust_decimal::Decimal` (required for monetary precision with PostgreSQL DECIMAL columns), serde serializes these values as JSON strings (e.g., `"0"`, `"1500.00"`), not as JSON numbers. Frontend types using `f64` will fail to deserialize with "invalid type: string, expected f64". Fix: use a custom serde visitor (`deserialize_any`) that accepts both numbers and numeric strings. Applied to all monetary fields in `frontend/src/types/` via shared helpers `deserialize_f64_from_any` and `deserialize_option_f64_from_any` in `frontend/src/types/mod.rs`.

### 2026-04-09 — Dashboard stats endpoint performs writes on every read via mark_overdue

The `dashboard::get_stats` service calls `pagos::mark_overdue(db).await?` as its first operation, which runs an `UPDATE` query to mark overdue payments. This means every GET request to `/api/dashboard/stats` triggers a write transaction, causing unnecessary database load and potential contention under concurrent reads. The overdue marking should be moved to a scheduled background task (e.g., a tokio cron job) or a separate admin endpoint, keeping the stats endpoint as a pure read. Relevant crates: sea-orm 1.x, actix-web 4.x.
