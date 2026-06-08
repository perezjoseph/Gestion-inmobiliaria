# Security Audit Remediation Bugfix Design

## Overview

This design addresses 12 security findings from a comprehensive audit of the property management platform. The findings span tenant isolation gaps, payment state machine vulnerabilities, race conditions, SQL injection, missing audit trails, resource exhaustion, access control weaknesses, and configuration hardening. The fix approach is layered by severity — CRITICAL findings receive schema migrations and core logic changes, HIGH findings get transactional safety and parameterized queries, MEDIUM findings add guardrails and audit coverage, and LOW findings harden configuration and edge-case guards.

## Glossary

- **Bug_Condition (C)**: Any request or state where a security finding allows unintended behavior — cross-tenant access, invalid state transitions, race-induced errors, injection vectors, missing audits, unbounded resources, bypassed access control, or misconfigured defenses
- **Property (P)**: The desired secure behavior when the bug condition holds — isolation enforced, transitions validated, races handled gracefully, queries parameterized, audits recorded, resources capped, access checked in code, and configuration hardened
- **Preservation**: Existing correct behavior that must remain unchanged — template fill operations, bulk payment marking, single-user registration, chatbot AI responses, existing audits, sub-cap exports, valid imports, owner-only policy, non-last-admin demotion, non-security dependency ranges, valid-token metrics, and correct production CORS
- **`plantillas.rs`**: The service in `backend/src/services/plantillas.rs` that manages document template CRUD without org filtering
- **`pagos.rs`**: The service in `backend/src/services/pagos.rs` that processes payment state changes without transition validation
- **`auth.rs`**: The service in `backend/src/services/auth.rs` that handles registration with `check_email_unique()` outside the transaction
- **`chatbot.rs`**: The service in `backend/src/services/chatbot.rs` that uses `format!()` for SQL and delegates balance access control to the LLM prompt

## Bug Details

### Bug Condition

The bug manifests across 12 distinct security surfaces. The composite bug condition is: any request that exercises one of the 12 vulnerable code paths in a way that produces insecure behavior.

**Formal Specification:**
```
FUNCTION isBugCondition(input)
  INPUT: input of type SecurityRequest (any API call or system event)
  OUTPUT: boolean
  
  RETURN (input.endpoint IN template_crud_endpoints AND input.targets_other_org_template)
         OR (input.endpoint == "pagos::update" AND input.state_transition NOT IN valid_transitions)
         OR (input.is_concurrent_registration AND input.email == other_concurrent_request.email)
         OR (input.endpoint == "chatbot::list_conversations" AND input.uses_format_interpolation)
         OR (input.is_sensitive_operation AND NOT audit_entry_recorded)
         OR (input.endpoint IN report_export_endpoints AND input.row_count > ROW_CAP)
         OR (input.endpoint == "importacion::importar_propiedades" AND (input.row_count > ROW_LIMIT OR input.has_invalid_enum OR NOT wrapped_in_transaction))
         OR (input.endpoint == "chatbot::query_tenant_balance" AND input.sender_policy == "tenants_and_prospects" AND NOT sender_is_linked_tenant)
         OR (input.endpoint == "usuarios::cambiar_rol" AND input.demotes_last_admin)
         OR (input.dependency IN ["jsonwebtoken", "argon2"] AND NOT version_pinned_exact)
         OR (input.endpoint == "/internal/metrics" AND input.missing_bearer_token AND METRICS_TOKEN_is_set)
         OR (input.environment_var CONTAINS_CI "prod" AND input.environment_var != "production")
END FUNCTION
```

### Examples

- **1.1**: User in Org-A calls `GET /api/v1/plantillas` and sees templates belonging to Org-B because `listar()` has no `organizacion_id` filter
- **1.2**: A `pagado` payment is changed back to `pendiente` via `PUT /api/v1/pagos/{id}` because `update()` only checks the allowlist, not transition validity
- **1.3**: Two simultaneous `POST /api/v1/auth/register` with `email=test@x.com` — both pass `check_email_unique()`, second insert returns 500 instead of 409
- **1.4**: `list_conversations` uses `format!("...WHERE organizacion_id = '{org_id}'...")` — while `org_id` is a Uuid today, this violates defense-in-depth
- **1.5**: Admin calls `cambiar_rol()` to change a user's role — no audit entry is recorded for the role change
- **1.6**: Export request for an org with 200k payments generates a full PDF in memory without row cap
- **1.7**: Import of 50k-row file processes row-by-row without transaction; failure at row 30k leaves 30k orphan records
- **1.8**: Unknown phone number sends "cuánto debo?" to chatbot under `tenants_and_prospects` policy — balance is returned because access check is only in the LLM prompt
- **1.9**: Last admin calls `cambiar_rol(self, "gerente")` — succeeds, leaving zero admins in the org
- **1.10**: `jsonwebtoken = "10"` in Cargo.toml could auto-upgrade to a breaking patch
- **1.11**: `/internal/metrics` is publicly accessible when `METRICS_TOKEN` is configured but the request has no token
- **1.12**: `ENVIRONMENT=Production` (capital P) bypasses the `== "production"` check and falls through to permissive CORS

## Expected Behavior

### Preservation Requirements

**Unchanged Behaviors:**
- `plantillas::rellenar()` continues to verify entity ownership and fill templates correctly for the caller's org
- `pagos::bulk_marcar_pagado()` continues to validate `pendiente/atrasado → pagado` transitions
- Single-user registration with unique email continues to create org + user and return JWT
- Chatbot AI conversation processing continues to work with same query results (only query mechanism changes)
- Existing audit operations (payment/property/contract/maintenance CRUD) continue recording entries
- Report exports below the row cap continue generating PDF/XLSX with same content and format
- Property imports with valid data below the row limit continue processing and returning `ImportResult`
- Chatbot `owner_only` sender policy continues to restrict access to verified owner phone
- Demotion of a non-last-admin user continues to succeed without restrictions
- Non-security dependencies continue using semver ranges
- `/internal/metrics` with valid token (or unset `METRICS_TOKEN` in dev) continues serving metrics
- `ENVIRONMENT=production` with valid `CORS_ORIGIN` continues enforcing strict CORS

**Scope:**
All requests that do NOT trigger any of the 12 bug conditions should be completely unaffected by these fixes. The changes are additive guards, not behavioral rewrites.

## Hypothesized Root Cause

Based on code analysis, the root causes are:

1. **Missing Tenant Isolation (1.1)**: The `plantilla_documento` entity lacks an `organizacion_id` column entirely. All CRUD functions operate globally — there is no FK to the org table, so filtering was never possible.

2. **Missing State Machine Enforcement (1.2)**: `pagos::update()` at line 300 calls `validate_enum("estado", estado, ESTADOS_PAGO)` which only checks membership in the allowlist, not that the transition from `old_estado` to the new estado is valid. No transition map exists.

3. **Check-Then-Act Race (1.3)**: `register_new_org()` calls `check_email_unique(db, &input.email)` at line 289 BEFORE the transaction begins at line 316. Two concurrent requests both pass the check; the second `insert` hits the DB unique constraint and produces a raw `DbErr` mapped to `AppError::Internal`.

4. **String Interpolation Habit (1.4)**: `list_conversations()` uses `format!()` with `Statement::from_string()` for a custom aggregate query. The values interpolated (`org_id`, `per_page`, `offset`) are typed (Uuid, u64) but the pattern is dangerous and blocks static analysis tooling.

5. **Incomplete Audit Coverage (1.5)**: Audit calls (`registrar_best_effort`) were added to CRUD handlers but not to administrative operations added later (role changes, activation, password changes, exports, imports, manual jobs).

6. **No Row Cap on Exports (1.6)**: `reportes::generar_reporte_ingresos` fetches all matching rows and builds the full response in memory before PDF/XLSX generation — no `LIMIT` or count check.

7. **No Transaction or Limits on Import (1.7)**: `importar_propiedades()` iterates rows and inserts one-by-one against `db` (not a transaction). No row limit, no enum validation for `estado`/`tipo_propiedad`.

8. **Access Control in Prompt, Not Code (1.8)**: `query_tenant_balance()` takes an `inquilino_id` directly. The caller (AI module) is instructed via prompt to verify the sender, but no code-level check ties the sender's phone to the `inquilino_id` before executing the query.

9. **No Admin Count Guard (1.9)**: `cambiar_rol()` validates the new role is in the allowlist but does not check whether the user being changed is currently an admin and whether other active admins exist.

10. **Semver Ranges for Security Crates (1.10)**: `Cargo.toml` uses `"10"` and `"0.5"` which are compatible-version ranges. An accidental patch with a vulnerability could be pulled automatically.

11. **Unauthenticated Internal Endpoint (1.11)**: `internal_metrics()` handler has no extractor — it serves metrics to anyone who can reach the route. Protection relies solely on Kubernetes NetworkPolicy.

12. **Exact String Match for ENVIRONMENT (1.12)**: `config.rs` line 194 checks `environment == "production"` — only this exact string triggers the CORS hard-fail. Typos like `"prod"` or `"Production"` silently pass through.

## Correctness Properties

Property 1: Bug Condition - Tenant Isolation Enforced

_For any_ API request to template CRUD endpoints where the caller's `organizacion_id` differs from the template's `organizacion_id`, the fixed system SHALL return `NotFound` (never exposing cross-tenant data) and new templates SHALL be created with the caller's `organizacion_id`.

**Validates: Requirements 2.1**

Property 2: Bug Condition - Payment State Transitions Validated

_For any_ payment update request where the requested state transition is not in the valid transition map (e.g., `pagado → pendiente`, `cancelado → pagado`), the fixed system SHALL reject with `AppError::Validation` containing the invalid transition description.

**Validates: Requirements 2.2**

Property 3: Bug Condition - Concurrent Registration Handled

_For any_ pair of concurrent registration requests with the same email, the fixed system SHALL return success (201) for the first and `AppError::Conflict` (409) with the friendly message for the second, never exposing a raw database error.

**Validates: Requirements 2.3**

Property 4: Bug Condition - SQL Injection Eliminated

_For any_ call to `list_conversations`, the fixed system SHALL use parameterized queries with bind variables (`$1`, `$2`, `$3`) for all dynamic values, producing identical query results to the current `format!()` approach.

**Validates: Requirements 2.4**

Property 5: Bug Condition - Audit Completeness

_For any_ sensitive operation (role change, activation, deactivation, password change, report export, data import, manual job execution), the fixed system SHALL record an audit entry with correct `accion`, `entity_type`, `entity_id`, and `usuario_id`.

**Validates: Requirements 2.5**

Property 6: Bug Condition - Export Row Cap Enforced

_For any_ report export request where the result set exceeds the configured row cap (default 50,000), the fixed system SHALL return `AppError::Validation` before generating the export.

**Validates: Requirements 2.6**

Property 7: Bug Condition - Import Hardened

_For any_ property import where the file exceeds 5,000 data rows, contains invalid `estado`/`tipo_propiedad` values, or encounters a DB error mid-import, the fixed system SHALL enforce the row limit, validate enums per-row, wrap all inserts in a transaction (rolling back on any failure), and record an audit trail.

**Validates: Requirements 2.7**

Property 8: Bug Condition - Chatbot Balance Access Enforced in Code

_For any_ balance query under `tenants_and_prospects` policy where the sender phone is NOT linked to a known `inquilino` in the organization, the fixed system SHALL reject the query with a polite decline message before executing any DB lookups.

**Validates: Requirements 2.8**

Property 9: Bug Condition - Last Admin Guard

_For any_ `cambiar_rol()` call that would demote the last active admin in an organization, the fixed system SHALL reject with `AppError::Validation` containing the Spanish error message.

**Validates: Requirements 2.9**

Property 10: Bug Condition - Security Crates Pinned

_For any_ build where `jsonwebtoken` and `argon2` are specified in `Cargo.toml`, the fixed system SHALL use exact pinned versions (`=X.Y.Z` syntax).

**Validates: Requirements 2.10**

Property 11: Bug Condition - Metrics Authentication

_For any_ request to `/internal/metrics` when `METRICS_TOKEN` is set and the request lacks a valid bearer token, the fixed system SHALL return 401 Unauthorized.

**Validates: Requirements 2.11**

Property 12: Bug Condition - Environment Typo Detection

_For any_ startup where the `ENVIRONMENT` variable contains "prod" (case-insensitive) but does not equal `"production"` exactly, the fixed system SHALL hard-fail with a clear error message.

**Validates: Requirements 2.12**

Property 13: Preservation - Existing Functionality Unchanged

_For any_ input where none of the 12 bug conditions hold (legitimate same-org template access, valid state transitions, single registration, non-chatbot queries, already-audited operations, sub-cap exports, valid imports, owner-only policy, non-last-admin demotion, non-security deps, valid-token metrics, correct ENVIRONMENT), the fixed system SHALL produce exactly the same behavior as the original system.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11, 3.12**

## Fix Implementation

### Changes Required

Assuming our root cause analysis is correct:

**File**: `backend/migrations/m{date}_000001_add_organizacion_id_to_plantillas.rs`

**Change 1 — Schema Migration for Tenant Isolation (1.1)**:
1. Add `organizacion_id UUID NOT NULL` column to `plantilla_documento` with FK to `organizacion(id)`
2. Backfill existing templates: assign to the single existing org (or add a `is_system` flag if shared templates are needed)
3. Add index on `organizacion_id` for query performance

**File**: `backend/src/services/plantillas.rs`

**Change 2 — Template CRUD Org Filtering (1.1)**:
1. Add `org_id: Uuid` parameter to `listar()`, `obtener()`, `actualizar()`, `eliminar()`
2. Filter all queries by `.filter(plantilla_documento::Column::OrganizacionId.eq(org_id))`
3. Set `organizacion_id` on insert in `crear()`
4. Return `AppError::NotFound` for cross-org access attempts

**File**: `backend/src/services/pagos.rs`

**Change 3 — State Transition Validation (1.2)**:
1. Add a `VALID_TRANSITIONS: &[(&str, &[&str])]` constant defining the transition map
2. Add a `validate_transition(old: &str, new: &str) -> Result<(), AppError>` helper
3. Call `validate_transition(&old_estado, &estado)` before applying the state change in `update()`
4. Return `AppError::Validation("Transición de estado no válida: {old} → {new}")`

**File**: `backend/src/services/auth.rs`

**Change 4 — Race Condition Fix (1.3)**:
1. Move `check_email_unique()` call inside the transaction (after `db.begin()`)
2. Add a catch for `DbErr::Query` containing "duplicate key" on the user insert
3. Map the duplicate key error to `AppError::Conflict("El email ya está registrado")`

**File**: `backend/src/services/chatbot.rs`

**Change 5 — Parameterized Queries (1.4)**:
1. Replace `format!()` count query with `Statement::from_sql_and_values(DbBackend::Postgres, sql, [org_id.into()])`
2. Replace `format!()` paginated query with bind parameters `$1`, `$2`, `$3` for `org_id`, `per_page`, `offset`
3. Verify query results are identical

**File**: `backend/src/services/usuarios.rs`, `backend/src/services/perfil.rs`, `backend/src/handlers/reportes.rs`, `backend/src/services/importacion.rs`, `backend/src/handlers/background_jobs.rs`

**Change 6 — Audit Trail Coverage (1.5)**:
1. Add `auditoria::registrar_best_effort()` calls in `cambiar_rol()`, `activar()`, `desactivar()`
2. Add audit in `cambiar_password()` in `perfil.rs`
3. Add audit in report export handlers (`ingresos_pdf`, `ingresos_xlsx`, `rentabilidad_pdf`, `rentabilidad_xlsx`)
4. Add audit in import functions (`importar_propiedades`, `importar_inquilinos`, `importar_gastos`)
5. Add audit in `background_jobs::ejecutar()`

**File**: `backend/src/services/reportes.rs`

**Change 7 — Export Row Cap (1.6)**:
1. Add `REPORT_ROW_CAP: u64 = 50_000` constant (configurable via env var `REPORT_ROW_CAP`)
2. Before building the report, execute a COUNT query for the given filters
3. If count exceeds cap, return `AppError::Validation` with a descriptive message
4. Apply before PDF/XLSX generation, after the count check

**File**: `backend/src/services/importacion.rs`

**Change 8 — Import Hardening (1.7)**:
1. Add `IMPORT_ROW_LIMIT: usize = 5_000` constant
2. After parsing rows, check `data_rows.len() > IMPORT_ROW_LIMIT` — reject with `AppError::Validation`
3. Add `ESTADOS_PROPIEDAD` and `TIPOS_PROPIEDAD` allowlists; validate per-row in `process_propiedad_row()`
4. Wrap the insert loop in `db.begin()` / `txn.commit()` — rollback on any error
5. After completion, call `auditoria::registrar_best_effort()` with import summary

**File**: `backend/src/services/chatbot.rs` (AI module caller)

**Change 9 — Balance Access Control in Code (1.8)**:
1. Before calling `query_tenant_balance()`, verify that the sender phone resolves to a known `inquilino` in the org using the existing `find_tenant_by_phone()` function
2. If no linked tenant found, return a polite decline message without executing the balance query
3. This check applies only when `sender_policy == "tenants_and_prospects"`

**File**: `backend/src/services/usuarios.rs`

**Change 10 — Last Admin Guard (1.9)**:
1. In `cambiar_rol()`, after fetching the record, check if the user's current role is `"admin"` AND the new role is not `"admin"`
2. If so, count active admins in the org: `usuario::Entity::find().filter(org_id).filter(rol == "admin").filter(activo == true).count(db)`
3. If count <= 1, return `AppError::Validation("No se puede quitar el último administrador de la organización")`

**File**: `backend/Cargo.toml`

**Change 11 — Pin Security Crates (1.10)**:
1. Change `jsonwebtoken = "10"` to `jsonwebtoken = "=10.0.1"` (or current exact version)
2. Change `argon2 = "0.5"` to `argon2 = "=0.5.3"` (or current exact version)

**File**: `backend/src/app.rs`

**Change 12 — Metrics Authentication (1.11)**:
1. Read `METRICS_TOKEN` env var at startup and store in `AppConfig`
2. In `internal_metrics()`, if `metrics_token` is `Some(token)`, extract the `Authorization: Bearer <token>` header and compare
3. If token is set but request lacks valid bearer, return 401
4. If `METRICS_TOKEN` is unset/empty, serve metrics without auth (backward compatibility)

**File**: `backend/src/config.rs`

**Change 13 — Environment Typo Detection (1.12)**:
1. After reading `ENVIRONMENT` env var, add: if `environment.to_lowercase().contains("prod") && environment != "production"` then hard-fail with clear error message
2. This catches `"prod"`, `"Production"`, `"PRODUCTION"` without affecting `"development"`, `"staging"`, or empty/unset values

## Testing Strategy

### Validation Approach

The testing strategy follows a two-phase approach: first, surface counterexamples that demonstrate each vulnerability on unfixed code, then verify the fix works correctly and preserves existing behavior.

### Exploratory Bug Condition Checking

**Goal**: Surface counterexamples that demonstrate each vulnerability BEFORE implementing fixes. Confirm or refute the root cause analysis.

**Test Plan**: Write tests that exercise each vulnerable code path and assert the insecure behavior exists. Run on UNFIXED code to document the attack surface.

**Test Cases**:
1. **Cross-Tenant Template Access (1.1)**: Create templates for Org-A, call `listar()` without org filter — observe all templates visible (will fail on unfixed code by showing cross-org data)
2. **Invalid State Transition (1.2)**: Call `update()` to change `pagado → pendiente` — observe it succeeds (will fail on unfixed code by allowing invalid transition)
3. **Concurrent Registration Race (1.3)**: Spawn two concurrent registration tasks with same email — observe 500 error (will fail on unfixed code by returning Internal error)
4. **SQL String Interpolation (1.4)**: Verify `list_conversations()` produces SQL containing literal UUID string (will confirm format!() usage)
5. **Missing Audit Entry (1.5)**: Call `cambiar_rol()` then query `registros_auditoria` — observe no entry (will fail on unfixed code)
6. **Unbounded Export (1.6)**: Request export for org with >50k rows — observe full PDF generated (will fail on unfixed code)
7. **Import Without Transaction (1.7)**: Import file where row N fails — observe rows 1..N-1 persisted (will fail on unfixed code)
8. **Balance Without Sender Check (1.8)**: Send balance query from unlinked phone — observe balance returned (will fail on unfixed code)
9. **Last Admin Demotion (1.9)**: Demote the only admin — observe success (will fail on unfixed code)
10. **Semver Range (1.10)**: Parse Cargo.toml and check version strings — observe range syntax (will confirm on unfixed code)
11. **Unauthenticated Metrics (1.11)**: Request `/internal/metrics` without token — observe 200 (will fail on unfixed code)
12. **ENVIRONMENT Typo (1.12)**: Set `ENVIRONMENT=prod` and call `AppConfig::from_env()` — observe success (will fail on unfixed code)

**Expected Counterexamples**:
- Templates visible across orgs without any org_id column
- Terminal payment states accepting transitions backward
- 500 Internal Server Error instead of 409 Conflict on race
- Raw UUID strings interpolated into SQL text

### Fix Checking

**Goal**: Verify that for all inputs where any bug condition holds, the fixed function produces the expected secure behavior.

**Pseudocode:**
```
FOR ALL input WHERE isBugCondition(input) DO
  result := fixedSystem(input)
  ASSERT expectedSecureBehavior(result)
END FOR
```

Concretely for each finding:
- 1.1: `listar(db, org_id_a)` returns ONLY org_a templates; `obtener(db, org_b_template_id, org_id_a)` returns NotFound
- 1.2: `update(db, org_id, pago_id, {estado: "pendiente"})` where current is `pagado` → returns Validation error
- 1.3: Concurrent registrations → one succeeds, other gets 409 Conflict
- 1.4: Generated SQL contains `$1`, `$2`, `$3` bind placeholders, not literal values
- 1.5: After each sensitive operation, audit entry exists with correct fields
- 1.6: Export with >50k rows → returns Validation error before memory allocation
- 1.7: Import with invalid row → entire transaction rolled back; import with >5k rows → rejected
- 1.8: Balance query from unlinked phone → polite decline, no DB query executed
- 1.9: Last admin demotion → returns Validation error
- 1.10: Cargo.toml contains `= "=X.Y.Z"` syntax for security crates
- 1.11: `/internal/metrics` without token when `METRICS_TOKEN` is set → 401
- 1.12: `ENVIRONMENT=prod` → startup failure with clear message

### Preservation Checking

**Goal**: Verify that for all inputs where the bug condition does NOT hold, the fixed function produces the same result as the original function.

**Pseudocode:**
```
FOR ALL input WHERE NOT isBugCondition(input) DO
  ASSERT originalSystem(input) = fixedSystem(input)
END FOR
```

**Testing Approach**: Property-based testing is recommended for preservation checking because:
- It generates many test cases automatically across the input domain
- It catches edge cases that manual unit tests might miss
- It provides strong guarantees that behavior is unchanged for all non-buggy inputs

**Test Plan**: Observe behavior on UNFIXED code first for legitimate operations, then write property-based tests capturing that behavior.

**Test Cases**:
1. **Template Fill Preservation (3.1)**: Verify `rellenar()` continues to work for own-org entities after adding org filtering
2. **Bulk Payment Preservation (3.2)**: Verify `bulk_marcar_pagado()` continues to accept `pendiente/atrasado → pagado` transitions
3. **Single Registration Preservation (3.3)**: Verify single unique-email registration returns JWT as before
4. **Chatbot Query Preservation (3.4)**: Verify `list_conversations()` with parameterized queries returns identical results to format!() version
5. **Existing Audit Preservation (3.5)**: Verify payment/property CRUD still records audit entries
6. **Sub-Cap Export Preservation (3.6)**: Verify exports below 50k rows generate identical output
7. **Valid Import Preservation (3.7)**: Verify imports with valid data and <5k rows produce same `ImportResult`
8. **Owner Policy Preservation (3.8)**: Verify `owner_only` policy still restricts to verified owner phone
9. **Non-Last Admin Demotion (3.9)**: Verify demotion of non-last admin continues succeeding
10. **Non-Security Deps (3.10)**: Verify other crates keep their semver ranges
11. **Valid Token Metrics (3.11)**: Verify `/internal/metrics` with correct token still serves metrics
12. **Correct Production CORS (3.12)**: Verify `ENVIRONMENT=production` with `CORS_ORIGIN` set still enforces strict CORS

### Unit Tests

- State transition validation: test all valid transitions succeed, all invalid transitions fail
- Last admin guard: test with 1 admin (reject), 2 admins (allow)
- ENVIRONMENT validation: test exact "production", "prod", "Production", "development", empty
- Metrics auth: test with token set + valid bearer, token set + invalid bearer, token set + no bearer, token unset
- Import row limit: test at boundary (5000, 5001)
- Import enum validation: test valid/invalid `estado` and `tipo_propiedad` values
- Export row cap: test at boundary (50000, 50001)
- Payment transition map: exhaustive test of all state pairs

### Property-Based Tests

- Generate random `(old_estado, new_estado)` pairs and verify the transition map is correctly enforced — valid transitions succeed, invalid transitions fail
- Generate random org_id pairs and template sets — verify cross-org access always returns NotFound
- Generate random row counts for imports and verify the limit is enforced correctly at boundaries
- Generate random phone numbers for chatbot balance queries — verify only linked phones get results
- Generate random concurrent registration pairs — verify at most one succeeds per email

### Integration Tests

- Full registration flow with duplicate email under concurrent load (tokio::spawn multiple tasks)
- End-to-end template CRUD with multi-tenant setup — verify isolation
- Payment lifecycle with all valid transitions followed by invalid attempts
- Import flow with transaction rollback verification (check DB is clean after failure)
- Report export with large dataset — verify rejection before memory spike
- Chatbot conversation listing — verify parameterized query returns same pagination results
- `/internal/metrics` endpoint with and without METRICS_TOKEN configuration
- Application startup with various ENVIRONMENT values
