# Implementation Plan

## Overview

This task list implements fixes for 12 security findings from a comprehensive audit. The workflow follows the bug condition methodology: write exploration tests (expect failure on unfixed code), write preservation tests (expect pass on unfixed code), implement fixes, then verify all tests pass.

## Task Dependency Graph

```json
{
  "waves": [
    { "tasks": ["1", "2"] },
    { "tasks": ["3.1", "3.2", "4.1", "5.1", "6.1", "7.1", "8.1", "9.1", "10.1", "11.1", "12.1", "13.1", "14.1"] },
    { "tasks": ["15.1", "15.2"] },
    { "tasks": ["16"] }
  ]
}
```

## Notes

- Tasks 3–14 are independent of each other and can be implemented in any order after tasks 1 and 2 are complete, except 3.2 depends on 3.1 (migration must exist before filtering code).
- The project uses Rust 2024 with Actix-web and SeaORM. All tests go in `backend/src/tests/`.
- Migration naming: `m{YYYYMMDD}_{SEQ}_{name}.rs` per project conventions.
- All user-facing error messages must be in Spanish.
- Property-based tests use the `proptest` crate (already in dev-dependencies).

## Tasks

- [x] 1. Write bug condition exploration tests
  - **Property 1: Bug Condition** - Security Audit Vulnerability Verification
  - **CRITICAL**: These tests MUST FAIL on unfixed code - failure confirms the vulnerabilities exist
  - **DO NOT attempt to fix the tests or the code when they fail**
  - **NOTE**: These tests encode the expected secure behavior - they will validate the fixes when they pass after implementation
  - **GOAL**: Surface counterexamples that demonstrate each vulnerability exists on the current codebase
  - **Scoped PBT Approach**: Each sub-property targets a specific finding's bug condition
  - Test file: `backend/src/tests/security_audit_pbt.rs`
  - **Sub-properties to verify (all should FAIL on unfixed code):**
  - 1a. Tenant Isolation: Call `plantillas::listar(db, org_id_a)` — assert only org_a templates returned. On unfixed code, templates from all orgs are visible (no `organizacion_id` column exists)
  - 1b. State Transitions: Call `pagos::update()` with `pagado → pendiente` — assert `AppError::Validation` returned. On unfixed code, transition succeeds
  - 1c. Registration Race: Spawn concurrent registrations with same email — assert one gets 201, other gets 409. On unfixed code, second gets 500
  - 1d. SQL Parameterization: Inspect generated SQL from `list_conversations()` — assert bind placeholders `$1`, `$2`, `$3` present. On unfixed code, literal values interpolated
  - 1e. Audit Coverage: Call `cambiar_rol()` then query `registros_auditoria` — assert entry exists. On unfixed code, no entry recorded
  - 1f. Export Row Cap: Request export for >50k rows — assert `AppError::Validation` returned. On unfixed code, full export generated
  - 1g. Import Transaction: Import file where row N has invalid data — assert all rows rolled back. On unfixed code, rows 1..N-1 persist as orphans
  - 1h. Balance Access Control: Send balance query from unlinked phone under `tenants_and_prospects` — assert polite decline. On unfixed code, balance returned
  - 1i. Last Admin Guard: Demote the only admin — assert `AppError::Validation`. On unfixed code, demotion succeeds
  - 1j. Crate Pinning: Parse `Cargo.toml` — assert `jsonwebtoken` and `argon2` use `=X.Y.Z` syntax. On unfixed code, semver ranges used
  - 1k. Metrics Auth: Request `/internal/metrics` without token when `METRICS_TOKEN` set — assert 401. On unfixed code, 200 returned
  - 1l. Environment Typo: Set `ENVIRONMENT=prod` and call `AppConfig::from_env()` — assert hard-fail. On unfixed code, startup succeeds silently
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests FAIL (this is correct — it proves the vulnerabilities exist)
  - Document counterexamples found to understand each root cause
  - Mark task complete when tests are written, run, and failures are documented
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 1.12_

- [x] 2. Write preservation property tests (BEFORE implementing fixes)
  - **Property 2: Preservation** - Existing Secure Behavior Unchanged
  - **IMPORTANT**: Follow observation-first methodology
  - **GOAL**: Capture baseline behavior for non-buggy inputs so regressions are caught after fixes
  - Test file: `backend/src/tests/security_preservation_pbt.rs`
  - **Observation-first approach**: Run unfixed code with non-buggy inputs, record outputs, write property tests asserting those outputs
  - **Sub-properties to observe and assert (all should PASS on unfixed code):**
  - 2a. Template Fill Preservation: Observe `plantillas::rellenar()` with own-org entity — assert continues filling templates correctly
  - 2b. Bulk Payment Preservation: Observe `bulk_marcar_pagado()` with `pendiente`/`atrasado` payments — assert transitions to `pagado` succeed
  - 2c. Single Registration Preservation: Observe single unique-email registration — assert org + user created, JWT returned
  - 2d. Chatbot Query Preservation: Observe `list_conversations()` pagination results — assert identical result sets after parameterization
  - 2e. Existing Audit Preservation: Observe payment/property CRUD audit entries — assert still recorded
  - 2f. Sub-Cap Export Preservation: Observe export with <50k rows — assert PDF/XLSX generated with same content
  - 2g. Valid Import Preservation: Observe import with valid data and <5k rows — assert same `ImportResult` structure returned
  - 2h. Owner Policy Preservation: Observe `owner_only` sender policy — assert only verified owner phone gets responses
  - 2i. Non-Last Admin Demotion: Observe demotion when 2+ admins exist — assert role change succeeds
  - 2j. Non-Security Deps: Observe other crates in `Cargo.toml` — assert semver ranges preserved
  - 2k. Valid Token Metrics: Observe `/internal/metrics` with valid token — assert metrics served (200)
  - 2l. Correct CORS: Observe `ENVIRONMENT=production` with `CORS_ORIGIN` set — assert strict CORS enforced
  - Run tests on UNFIXED code
  - **EXPECTED OUTCOME**: Tests PASS (this confirms baseline behavior to preserve)
  - Mark task complete when tests are written, run, and passing on unfixed code
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12_

- [x] 3. Fix: Schema migration for tenant isolation

  - [x] 3.1 Create migration `m{date}_000001_add_organizacion_id_to_plantillas.rs`
    - Add `organizacion_id UUID NOT NULL` column to `plantilla_documento` with FK to `organizacion(id)`
    - Backfill existing templates to the single existing org
    - Add index on `organizacion_id` for query performance
    - Register migration in `backend/migrations/mod.rs`
    - _Bug_Condition: input.endpoint IN template_crud_endpoints AND input.targets_other_org_template_
    - _Expected_Behavior: Cross-org access returns AppError::NotFound; own-org CRUD works normally_
    - _Preservation: plantillas::rellenar() continues to work for own-org entities_
    - _Requirements: 2.1, 3.1_

  - [x] 3.2 Update template CRUD functions with org filtering
    - Add `org_id: Uuid` parameter to `listar()`, `obtener()`, `actualizar()`, `eliminar()` in `backend/src/services/plantillas.rs`
    - Filter all queries by `.filter(plantilla_documento::Column::OrganizacionId.eq(org_id))`
    - Set `organizacion_id` on insert in `crear()`
    - Return `AppError::NotFound` for cross-org access attempts
    - Update handlers to pass `org_id` from auth claims
    - Regenerate entity for `plantilla_documento` if needed
    - _Bug_Condition: input.targets_other_org_template_
    - _Expected_Behavior: Only same-org templates visible; cross-org returns NotFound_
    - _Preservation: rellenar() unchanged, own-org CRUD unchanged_
    - _Requirements: 2.1, 3.1_

- [x] 4. Fix: Payment state transition validation

  - [x] 4.1 Implement state transition map and validation
    - Add `VALID_TRANSITIONS: &[(&str, &[&str])]` constant in `backend/src/services/pagos.rs`
    - Define: `pendiente → [pagado, atrasado, cancelado]`, `atrasado → [pagado, cancelado]`, `pagado → []`, `cancelado → []`
    - Add `validate_transition(old: &str, new: &str) -> Result<(), AppError>` helper
    - Call `validate_transition(&old_estado, &estado)` in `update()` before applying state change
    - Return `AppError::Validation("Transición de estado no válida: {old} → {new}")`
    - _Bug_Condition: input.state_transition NOT IN valid_transitions_
    - _Expected_Behavior: Invalid transitions rejected with Validation error in Spanish_
    - _Preservation: bulk_marcar_pagado() continues validating pendiente/atrasado → pagado_
    - _Requirements: 2.2, 3.2_

- [x] 5. Fix: Registration race condition

  - [x] 5.1 Move uniqueness check inside transaction and handle constraint violation
    - In `backend/src/services/auth.rs`, move `check_email_unique()` call inside the transaction (after `db.begin()`)
    - Add catch for `DbErr::Query` containing "duplicate key" on the user insert
    - Map duplicate key error to `AppError::Conflict("El email ya está registrado")`
    - _Bug_Condition: input.is_concurrent_registration AND input.email == other_concurrent_request.email_
    - _Expected_Behavior: First request succeeds (201), second gets 409 Conflict with friendly message_
    - _Preservation: Single unique-email registration continues returning JWT_
    - _Requirements: 2.3, 3.3_

- [x] 6. Fix: Parameterized queries for chatbot

  - [x] 6.1 Replace format!() with parameterized queries in list_conversations
    - In `backend/src/services/chatbot.rs`, replace `format!()` count query with `Statement::from_sql_and_values(DbBackend::Postgres, sql, [org_id.into()])`
    - Replace `format!()` paginated query with bind parameters `$1`, `$2`, `$3` for `org_id`, `per_page`, `offset`
    - Verify query results are identical to current behavior
    - _Bug_Condition: input.endpoint == "chatbot::list_conversations" AND input.uses_format_interpolation_
    - _Expected_Behavior: SQL uses $1, $2, $3 bind placeholders; identical query results_
    - _Preservation: Chatbot AI conversation processing unchanged; same pagination results_
    - _Requirements: 2.4, 3.4_

- [x] 7. Fix: Audit trail coverage for sensitive operations

  - [x] 7.1 Add audit calls to administrative operations
    - Add `auditoria::registrar_best_effort()` in `backend/src/services/usuarios.rs`: `cambiar_rol()` (accion: `cambiar_rol`), `activar()` (accion: `activar`), `desactivar()` (accion: `desactivar`)
    - Add audit in `backend/src/services/perfil.rs`: `cambiar_password()` (accion: `cambiar_password`)
    - Add audit in `backend/src/handlers/reportes.rs`: `ingresos_pdf`, `ingresos_xlsx`, `rentabilidad_pdf`, `rentabilidad_xlsx` (accion: `exportar`)
    - Add audit in `backend/src/services/importacion.rs`: `importar_propiedades`, `importar_inquilinos`, `importar_gastos` (accion: `importar`)
    - Add audit in `backend/src/handlers/background_jobs.rs`: `ejecutar()` (accion: `ejecutar_tarea_manual`)
    - Each audit entry must include correct `accion`, `entity_type`, `entity_id`, `usuario_id`
    - _Bug_Condition: input.is_sensitive_operation AND NOT audit_entry_recorded_
    - _Expected_Behavior: Audit entry recorded for every sensitive operation_
    - _Preservation: Existing audits (payment/property/contract/maintenance CRUD) unchanged_
    - _Requirements: 2.5, 3.5_

- [x] 8. Fix: Export row cap

  - [x] 8.1 Add configurable row cap to report exports
    - Add `REPORT_ROW_CAP: u64 = 50_000` constant in `backend/src/services/reportes.rs` (configurable via `REPORT_ROW_CAP` env var)
    - Before building the report, execute a COUNT query for the given filters
    - If count exceeds cap, return `AppError::Validation` with descriptive message before PDF/XLSX generation
    - Apply to all export handlers: `ingresos_pdf`, `ingresos_xlsx`, `rentabilidad_pdf`, `rentabilidad_xlsx`
    - _Bug_Condition: input.endpoint IN report_export_endpoints AND input.row_count > ROW_CAP_
    - _Expected_Behavior: Validation error returned before memory allocation_
    - _Preservation: Exports below 50k rows generate identical output_
    - _Requirements: 2.6, 3.6_

- [x] 9. Fix: Import hardening

  - [x] 9.1 Add row limit, transaction wrapping, enum validation, and audit to imports
    - Add `IMPORT_ROW_LIMIT: usize = 5_000` constant in `backend/src/services/importacion.rs`
    - After parsing rows, check `data_rows.len() > IMPORT_ROW_LIMIT` — reject with `AppError::Validation`
    - Add `ESTADOS_PROPIEDAD` (`disponible`, `ocupada`, `mantenimiento`) and `TIPOS_PROPIEDAD` allowlists
    - Validate `estado` and `tipo_propiedad` per-row in `process_propiedad_row()` — reject invalid values
    - Wrap the insert loop in `db.begin()` / `txn.commit()` — rollback on any error
    - After completion, call `auditoria::registrar_best_effort()` with import summary (entity_type: `importacion`, accion: `importar_propiedades`, details with total/success/failure counts)
    - _Bug_Condition: input.row_count > ROW_LIMIT OR input.has_invalid_enum OR NOT wrapped_in_transaction_
    - _Expected_Behavior: Row limit enforced; enums validated; all-or-nothing transaction; audit trail recorded_
    - _Preservation: Valid imports with <5k rows and valid enums produce same ImportResult_
    - _Requirements: 2.7, 3.7_

- [x] 10. Fix: Chatbot balance access control in code

  - [x] 10.1 Add code-level sender verification before balance queries
    - In `backend/src/services/chatbot.rs` (AI module caller), before calling `query_tenant_balance()`:
    - Verify sender phone resolves to a known `inquilino` in the org using `find_tenant_by_phone()`
    - If no linked tenant found, return polite decline message without executing the balance query
    - This check applies only when `sender_policy == "tenants_and_prospects"`
    - _Bug_Condition: input.sender_policy == "tenants_and_prospects" AND NOT sender_is_linked_tenant_
    - _Expected_Behavior: Polite decline message; no DB balance query executed_
    - _Preservation: owner_only policy unchanged; linked tenants still get balance info_
    - _Requirements: 2.8, 3.8_

- [x] 11. Fix: Last admin guard

  - [x] 11.1 Add admin count check to cambiar_rol
    - In `backend/src/services/usuarios.rs::cambiar_rol()`, after fetching the record:
    - Check if user's current role is `"admin"` AND new role is not `"admin"`
    - If so, count active admins in the org: `usuario::Entity::find().filter(org_id).filter(rol == "admin").filter(activo == true).count(db)`
    - If count <= 1, return `AppError::Validation("No se puede quitar el último administrador de la organización")`
    - _Bug_Condition: input.demotes_last_admin_
    - _Expected_Behavior: Rejection with Spanish validation error_
    - _Preservation: Non-last-admin demotion continues succeeding_
    - _Requirements: 2.9, 3.9_

- [ ] 12. Fix: Pin security crates in Cargo.toml

  - [~] 12.1 Pin jsonwebtoken and argon2 to exact versions
    - In `backend/Cargo.toml`, change `jsonwebtoken = "10"` to `jsonwebtoken = "=10.0.1"` (verify current exact version with `cargo metadata` or lock file)
    - Change `argon2 = "0.5"` to `argon2 = "=0.5.3"` (verify current exact version)
    - Run `cargo check` to confirm compilation succeeds
    - Do NOT change version ranges for non-security dependencies
    - _Bug_Condition: input.dependency IN ["jsonwebtoken", "argon2"] AND NOT version_pinned_exact_
    - _Expected_Behavior: Exact =X.Y.Z syntax used for security crates_
    - _Preservation: All other crates keep semver ranges_
    - _Requirements: 2.10, 3.10_

- [ ] 13. Fix: Metrics endpoint authentication

  - [~] 13.1 Add bearer token authentication to /internal/metrics
    - Read `METRICS_TOKEN` env var at startup and store in `AppConfig` in `backend/src/config.rs`
    - In `backend/src/app.rs` or the metrics handler, if `metrics_token` is `Some(token)`:
      - Extract `Authorization: Bearer <token>` header from request
      - Compare with stored token
      - If missing or invalid, return 401 Unauthorized
    - If `METRICS_TOKEN` is unset/empty, serve metrics without auth (backward compat for dev)
    - _Bug_Condition: input.endpoint == "/internal/metrics" AND input.missing_bearer_token AND METRICS_TOKEN_is_set_
    - _Expected_Behavior: 401 Unauthorized returned_
    - _Preservation: Valid token requests and unset-token dev environments continue serving metrics_
    - _Requirements: 2.11, 3.11_

- [ ] 14. Fix: Environment variable typo detection

  - [~] 14.1 Add startup validation for ENVIRONMENT variable
    - In `backend/src/config.rs`, after reading `ENVIRONMENT` env var:
    - If `environment.to_lowercase().contains("prod") && environment != "production"`, hard-fail with: `"ENVIRONMENT contiene 'prod' pero no es 'production'. Use ENVIRONMENT=production"`
    - This catches `"prod"`, `"Production"`, `"PRODUCTION"` without affecting `"development"`, `"staging"`, or empty/unset
    - _Bug_Condition: input.environment_var CONTAINS_CI "prod" AND input.environment_var != "production"_
    - _Expected_Behavior: Startup hard-fail with clear error message_
    - _Preservation: ENVIRONMENT=production with valid CORS_ORIGIN continues working_
    - _Requirements: 2.12, 3.12_

- [ ] 15. Verify bug condition exploration tests now pass

  - [~] 15.1 Re-run bug condition exploration tests after all fixes
    - **Property 1: Expected Behavior** - Security Vulnerabilities Resolved
    - **IMPORTANT**: Re-run the SAME tests from task 1 — do NOT write new tests
    - The tests from task 1 encode the expected secure behavior for each finding
    - When these tests pass, it confirms all 12 vulnerabilities are resolved
    - Run `cargo test security_audit_pbt` on FIXED code
    - **EXPECTED OUTCOME**: All tests PASS (confirms bugs are fixed)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 2.12_

  - [~] 15.2 Re-run preservation tests after all fixes
    - **Property 2: Preservation** - Existing Behavior Unchanged
    - **IMPORTANT**: Re-run the SAME tests from task 2 — do NOT write new tests
    - Run `cargo test security_preservation_pbt` on FIXED code
    - **EXPECTED OUTCOME**: All tests PASS (confirms no regressions)
    - Confirm all preservation properties still hold after fixes
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12_

- [~] 16. Checkpoint - Ensure all tests pass
  - Run full test suite: `cargo test` in backend workspace
  - Verify no compilation errors: `cargo check`
  - Verify no clippy warnings on changed files: `cargo clippy`
  - Ensure all 12 bug condition tests pass (expected behavior satisfied)
  - Ensure all 12 preservation tests pass (no regressions)
  - Ensure existing test suite passes (no unintended breakage)
  - Ask the user if questions arise


