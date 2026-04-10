---
inclusion: always
---

# Testing Rules

## Unit Tests
- Always write unit tests in the same file as the code being tested using `#[cfg(test)]` modules.
- Always test both success and error cases.
- Always aim for meaningful coverage of edge cases, not just happy paths.

## Integration Tests
- Always place integration tests in `backend/tests/`.
- Always give each domain area its own test file (e.g., `auth_tests.rs`, `propiedades_tests.rs`).
- Always use a test database or transactions with rollbacks to keep tests isolated.
- Always test full request/response cycles including status codes and response bodies.

## Frontend Tests
- Always place frontend integration tests in `frontend/tests/`.

## Running Tests
- Always run `cargo test --workspace` before considering any task complete.
- Always fix failing tests before proceeding.

## Test Data
- Always use factories or fixtures to create test data.
- Never depend on existing database state for tests.
- Always clean up test data after each test run.

## What to Test
- Always test API endpoint authorization (unauthenticated, wrong role, correct role).
- Always test input validation (missing fields, invalid formats, boundary values).
- Always test business logic (contract overlap detection, late payment calculation).
- Always test error responses match expected format and status codes.
