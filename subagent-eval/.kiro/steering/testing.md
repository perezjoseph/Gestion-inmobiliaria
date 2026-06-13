---
inclusion: fileMatch
fileMatchPattern: ["**/*_pbt.rs", "**/*_tests.rs", "**/tests/**/*.rs"]
---

# Testing Rules

## Organization
- Integration: `backend/tests/{domain}_tests.rs`. PBT: `{domain}_pbt.rs`. Unit: `#[cfg(test)]` in-file.
- One file per domain. `#[actix_web::test]` for async. Shared `setup_db()` helpers.

## Coverage
- Always test: unauth (401), wrong role (403), correct role (200/201), validation (400/422), business edge cases.

## Property-Based (proptest)
- Backend: `TestRunner` directly. Frontend: `proptest!` macro.
- Custom strategies at top of file. Annotate: `// Feature: {feature}, Property N: description`.
- Never hardcode case counts. Use `crate::pbt_cases()` from `tests/main.rs` for integration tests, or `crate::test_support::pbt_cases()` for in-crate unit tests. Both read `PROPTEST_CASES` env var (default: 100). CI sets this to 10 for speed.

## Data
- `Uuid::new_v4()` for IDs, `Decimal::new(value, scale)` for money. Never depend on pre-existing DB state.

## Running
- `cargo test --workspace` before done. `-- --test-threads=1` for shared DB tests.
- `unwrap_or_else(|e| e.into_inner())` for mutex locks. Smallest possible lock scope.
