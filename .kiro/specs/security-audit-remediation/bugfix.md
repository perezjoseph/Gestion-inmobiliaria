# Bugfix Requirements Document

## Introduction

This document addresses 12 security findings from an intensive security audit of the Dominican Republic property management platform. The findings span critical data isolation gaps, state machine vulnerabilities, race conditions, SQL injection risks, missing audit trails, resource exhaustion vectors, access control weaknesses, and configuration hardening issues. Remediation is grouped by severity (CRITICAL, HIGH, MEDIUM, LOW) with each finding mapped to bug condition methodology for systematic validation.

## Bug Analysis

### Current Behavior (Defect)

**CRITICAL — Tenant Isolation & State Machine**

1.1 WHEN an authenticated user from Organization A calls template CRUD endpoints (list, get, create, update, delete) THEN the system operates on all templates globally without filtering by `organizacion_id`, allowing cross-tenant data access, modification, and deletion

1.2 WHEN the `pagos::update()` function receives a state change request THEN the system validates only that the new `estado` is in the allowlist (`pendiente`, `pagado`, `atrasado`, `cancelado`) but does NOT enforce valid transitions between states, allowing arbitrary state changes such as `pagado` → `pendiente`, `cancelado` → `pagado`, or `pagado` → `atrasado`

**HIGH — Race Conditions & Injection Risks**

1.3 WHEN two concurrent registration requests arrive with the same email address THEN the system calls `check_email_unique()` outside the transaction for both, both pass the check, and the second insert fails with a raw database constraint error instead of the friendly `AppError::Conflict("El email ya está registrado")` message

1.4 WHEN the chatbot service builds SQL queries for conversation listing THEN the system uses `format!()` string interpolation with `Statement::from_string()` for `org_id`, `per_page`, and `offset` parameters, violating defense-in-depth principles against SQL injection

**MEDIUM — Audit & Resource Controls**

1.5 WHEN a user performs sensitive operations (role changes, activation/deactivation, password changes, report exports, data imports, manual background job execution) THEN the system does NOT write entries to `registros_auditoria` for these operations

1.6 WHEN a report export (PDF/XLSX) is requested for an organization with a large dataset THEN the system applies date boundary limits but no row count cap, allowing unbounded memory allocation for a single request

1.7 WHEN a property import file is uploaded THEN the system processes it without a row count limit, without transaction wrapping (partial failures leave orphan records), without validating `estado` and `tipo_propiedad` enum values, and without recording an audit trail

1.8 WHEN the chatbot operates under `tenants_and_prospects` sender policy THEN the system delegates access control for financial balance queries to the LLM prompt rather than enforcing it in code, allowing any phone number to trigger balance lookups

**LOW — Guards & Configuration**

1.9 WHEN an admin calls `cambiar_rol()` to demote the last admin in an organization THEN the system allows the demotion, leaving the organization with no admin user

1.10 WHEN security-critical crates are specified in `Cargo.toml` THEN the system uses semver ranges (`jsonwebtoken = "10"`, `argon2 = "0.5"`) instead of pinned exact versions

1.11 WHEN the `/internal/metrics` endpoint is accessed outside of a Kubernetes cluster with properly configured NetworkPolicy THEN the system serves Prometheus metrics without any authentication, exposing internal metrics publicly

1.12 WHEN the `ENVIRONMENT` variable is set to a value that resembles production intent but doesn't exactly match `"production"` (e.g., `"prod"`, `"Production"`, `"PRODUCTION"`) THEN the system falls back to permissive CORS because only the exact string `"production"` triggers the hard-fail check, creating a misconfiguration risk in non-standard environments

### Expected Behavior (Correct)

**CRITICAL — Tenant Isolation & State Machine**

2.1 WHEN an authenticated user calls template CRUD endpoints THEN the system SHALL filter all template operations by the user's `organizacion_id`, ensuring templates from other organizations are invisible and inaccessible. Implementation requires:
- A new migration adding `organizacion_id UUID NOT NULL` column to `plantilla_documento` with a FK to `organizacion`
- Backfilling existing templates: assign each to the single existing org, or mark as system templates if shared
- All CRUD functions (`listar`, `obtener`, `crear`, `actualizar`, `eliminar`) receive and filter by `org_id`
- The `crear` function sets `organizacion_id` on insert
- Cross-org access attempts return `AppError::NotFound` (not `Forbidden`)

2.2 WHEN the `pagos::update()` function receives a state change request THEN the system SHALL enforce valid transitions only:
- `pendiente` → `pagado`, `atrasado`, `cancelado`
- `atrasado` → `pagado`, `cancelado`
- `pagado` → (terminal, no transitions allowed)
- `cancelado` → (terminal, no transitions allowed)

AND reject all other transitions with `AppError::Validation("Transición de estado no válida: {old} → {new}")` in Spanish

**HIGH — Race Conditions & Injection Risks**

2.3 WHEN two concurrent registration requests arrive with the same email THEN the system SHALL handle uniqueness via two layers:
- Move the `check_email_unique()` call inside the transaction (serializable check-then-insert)
- Add a catch for the DB unique constraint violation (`duplicate key`) that maps it to `AppError::Conflict("El email ya está registrado")` rather than `AppError::Internal`

This ensures that even under race conditions, the user receives a friendly 409 Conflict response instead of a 500 Internal Server Error

2.4 WHEN the chatbot service builds SQL queries THEN the system SHALL use parameterized queries with bind variables (`$1`, `$2`, `$3`) instead of `format!()` string interpolation, regardless of the current type safety of the interpolated values

**MEDIUM — Audit & Resource Controls**

2.5 WHEN a user performs sensitive operations THEN the system SHALL record an audit entry via `registrar_best_effort()` with the appropriate `accion`, `entity_type`, `entity_id`, and `usuario_id` for:
- Role changes (`services/usuarios.rs::cambiar_rol`) — accion: `cambiar_rol`
- User activation (`services/usuarios.rs::activar`) — accion: `activar`
- User deactivation (`services/usuarios.rs::desactivar`) — accion: `desactivar`
- Password changes (`services/perfil.rs::cambiar_password`) — accion: `cambiar_password`
- Report exports (`handlers/reportes.rs::ingresos_pdf`, `ingresos_xlsx`, `rentabilidad_pdf`, `rentabilidad_xlsx`) — accion: `exportar`
- Data imports (`services/importacion.rs::importar_*`) — accion: `importar`
- Manual background job execution (`handlers/background_jobs.rs::ejecutar`) — accion: `ejecutar_tarea_manual`

2.6 WHEN a report export is requested THEN the system SHALL enforce a configurable row count cap (default: 50,000 rows) and return `AppError::Validation` with a descriptive message when the cap is exceeded, before generating the export

2.7 WHEN a property import file is uploaded THEN the system SHALL:
- Enforce a row count limit (default: 5,000 data rows) and reject files exceeding it with `AppError::Validation` before processing
- Wrap all inserts in a single database transaction, rolling back ALL on any individual row failure
- Validate `estado` against `ESTADOS_PROPIEDAD` (`disponible`, `ocupada`, `mantenimiento`) — reject invalid values per-row
- Validate `tipo_propiedad` against a defined allowlist — reject invalid values per-row
- Record an audit trail entry on completion with `entity_type: "importacion"`, `accion: "importar_propiedades"`, and a summary of total/success/failure counts

2.8 WHEN the chatbot operates under `tenants_and_prospects` sender policy and receives a financial query THEN the system SHALL verify that the sender phone is linked to a known `inquilino` in the organization before executing balance lookups, rejecting unlinked phones with a polite decline message

**LOW — Guards & Configuration**

2.9 WHEN an admin calls `cambiar_rol()` to change a user's role away from `admin` THEN the system SHALL check that at least one other active admin remains in the organization, and reject the change with `AppError::Validation("No se puede quitar el último administrador de la organización")` if this would leave zero admins

2.10 WHEN security-critical crates are specified in `Cargo.toml` THEN the system SHALL use exact pinned versions (e.g., `jsonwebtoken = "=10.0.1"`, `argon2 = "=0.5.3"`) for `jsonwebtoken` and `argon2`

2.11 WHEN the `/internal/metrics` endpoint is accessed THEN the system SHALL require a bearer token (configurable via `METRICS_TOKEN` env var) and return 401 Unauthorized when the token is missing or invalid; WHEN `METRICS_TOKEN` is unset or empty THEN the endpoint SHALL remain unauthenticated (backward compatibility for dev/local environments where NetworkPolicy isn't available)

2.12 WHEN the `ENVIRONMENT` variable is set to a value that contains "prod" (case-insensitive) but doesn't equal `"production"` exactly THEN the system SHALL hard-fail at startup with a clear error: `"ENVIRONMENT contiene 'prod' pero no es 'production'. Use ENVIRONMENT=production"`. This catches common typos like `"prod"`, `"Production"`, `"PRODUCTION"` without affecting legitimate non-production environments (e.g., `"development"`, `"staging"`, empty/unset)

### Unchanged Behavior (Regression Prevention)

3.1 WHEN the `plantillas::rellenar()` function is called with a valid entity belonging to the caller's organization THEN the system SHALL CONTINUE TO verify entity ownership and fill templates correctly

3.2 WHEN `pagos::bulk_marcar_pagado()` is called with payments in `pendiente` or `atrasado` state THEN the system SHALL CONTINUE TO validate that only those states can transition to `pagado` and process the bulk operation correctly

3.3 WHEN a single registration request arrives with a unique email THEN the system SHALL CONTINUE TO create the user and organization within a transaction and return a valid JWT

3.4 WHEN the chatbot service processes messages and generates AI responses THEN the system SHALL CONTINUE TO function correctly with the same query results (only the query mechanism changes from string interpolation to parameterized)

3.5 WHEN existing audit operations (payment CRUD, property CRUD, contract CRUD, maintenance CRUD) are performed THEN the system SHALL CONTINUE TO record audit entries as they do today

3.6 WHEN a report export is requested with a dataset below the row cap THEN the system SHALL CONTINUE TO generate and return the PDF/XLSX export with the same content and format

3.7 WHEN a property import file with valid data and fewer rows than the limit is uploaded THEN the system SHALL CONTINUE TO process all rows and return the same `ImportResult` structure with success/failure counts

3.8 WHEN the chatbot operates under `owner_only` sender policy THEN the system SHALL CONTINUE TO restrict access to the verified owner phone only, without changes

3.9 WHEN an admin demotes a non-last-admin user THEN the system SHALL CONTINUE TO change the role successfully without additional restrictions

3.10 WHEN non-security-critical dependencies use semver ranges THEN the system SHALL CONTINUE TO use those ranges (only `jsonwebtoken` and `argon2` are pinned)

3.11 WHEN the `/internal/metrics` endpoint is accessed with a valid token (or when `METRICS_TOKEN` is unset, maintaining backward compatibility in dev) THEN the system SHALL CONTINUE TO serve Prometheus metrics

3.12 WHEN `ENVIRONMENT=production` is correctly set with a valid `CORS_ORIGIN` THEN the system SHALL CONTINUE TO enforce strict CORS as it does today
