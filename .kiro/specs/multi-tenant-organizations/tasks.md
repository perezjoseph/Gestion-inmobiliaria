# Implementation Plan: Multi-Tenant Organizations

## Overview

This plan introduces `organizaciones` as the root tenant entity, adds invitation-based onboarding, and scopes all existing entities to an organization. The implementation follows the project's domain pattern: migration → entity → DTOs → service → handler → routes → tests. Each task builds incrementally, with checkpoints to validate before moving forward.

## Tasks

- [x] 1. Create database migrations for organizations and data isolation
  - [x] 1.1 Create migration `m20250413_000001_create_organizaciones.rs`
    - Create the `organizaciones` table with all fields from the design: `id` (UUID PK), `tipo`, `nombre`, `estado`, persona_fisica fields (`cedula`, `telefono`, `email_organizacion`), persona_juridica fields (`rnc`, `razon_social`, `nombre_comercial`, `direccion_fiscal`, `representante_legal`, `dgii_data`), and timestamps
    - Add CHECK constraints on `tipo` (`persona_fisica`, `persona_juridica`) and `estado` (`activo`, `inactivo`)
    - Add UNIQUE constraints on `cedula` and `rnc`
    - Register the migration in the migrations `mod.rs` / `Migrator`
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 1.2 Create migration `m20250413_000002_add_organizacion_id.rs`
    - Add nullable `organizacion_id` UUID column to: `usuarios`, `propiedades`, `inquilinos`, `contratos`, `pagos`, `gastos`, `solicitudes_mantenimiento`
    - Create a default organization ("Organización Predeterminada", tipo `persona_fisica`)
    - Update all existing rows in all 7 tables to reference the default org
    - Promote the first user (by `created_at`) to `admin` role
    - Alter all `organizacion_id` columns to NOT NULL with FK to `organizaciones(id)`
    - Create indexes on `organizacion_id` for all 7 tables
    - _Requirements: 2.1, 2.2, 9.1, 9.2, 9.3, 9.4_

  - [x] 1.3 Create migration `m20250413_000003_create_invitaciones.rs`
    - Create the `invitaciones` table with: `id` (UUID PK), `organizacion_id` (FK), `email`, `rol`, `token` (UNIQUE), `usado`, `expires_at`, `created_at`
    - Add CHECK constraint on `rol` (`gerente`, `visualizador`)
    - Create indexes on `token` and `organizacion_id`
    - _Requirements: 8.1, 8.2_

- [x] 2. Create new entities and update existing entity
  - [x] 2.1 Create `organizacion` entity
    - Create `backend/src/entities/organizacion.rs` with the SeaORM `DeriveEntityModel` for the `organizaciones` table matching all columns from migration 1.1
    - Define relations: `HasMany` to `usuario`, `invitacion`, and all org-scoped entities
    - Re-export in `backend/src/entities/mod.rs` and `prelude.rs`
    - _Requirements: 1.1, 1.2, 1.3_

  - [x] 2.2 Create `invitacion` entity
    - Create `backend/src/entities/invitacion.rs` with the SeaORM `DeriveEntityModel` for the `invitaciones` table
    - Define relations: `BelongsTo` organizacion
    - Re-export in `backend/src/entities/mod.rs` and `prelude.rs`
    - _Requirements: 8.1_

  - [x] 2.3 Update `usuario` entity with `organizacion_id`
    - Add `organizacion_id: Uuid` field to `backend/src/entities/usuario.rs`
    - Add `BelongsTo` relation to `organizacion`
    - _Requirements: 2.1_

- [x] 3. Checkpoint — Verify migrations and entities compile
  - Ensure `cargo check` passes with the new migrations and entities. Ask the user if questions arise.

- [x] 4. Create DTOs for organizations and invitations
  - [x] 4.1 Create organization DTOs
    - Create `backend/src/models/organizacion.rs` with `OrganizacionResponse`, `UpdateOrganizacionRequest`
    - Use `#[serde(rename_all = "camelCase")]` on all structs
    - `UpdateOrganizacionRequest` only includes mutable fields (nombre, telefono, email_organizacion, nombre_comercial, direccion_fiscal, representante_legal, dgii_data) — tipo, cedula, rnc are immutable
    - Re-export in `backend/src/models/mod.rs`
    - _Requirements: 10.1, 10.2, 10.3_

  - [x] 4.2 Create invitation DTOs
    - Create `backend/src/models/invitacion.rs` with `CrearInvitacionRequest`, `InvitacionResponse`
    - `CrearInvitacionRequest` has `email` and `rol` (gerente | visualizador)
    - Re-export in `backend/src/models/mod.rs`
    - _Requirements: 8.1_

  - [x] 4.3 Update registration DTOs
    - Modify `RegisterRequest` in `backend/src/models/usuario.rs` to include: `tipo`, persona_fisica fields (`cedula`, `telefono`, `nombre_organizacion`), persona_juridica fields (`rnc`, `razon_social`, `nombre_comercial`, `direccion_fiscal`, `representante_legal`), and `token_invitacion`
    - All new fields are `Option<String>` since the request is a discriminated union
    - Update `LoginResponse` / `UserResponse` to include `organizacion_id: Uuid`
    - _Requirements: 3.1, 3.3, 3.4, 3.8, 8.3_

- [x] 5. Implement fiscal validation module (RNC and cédula)
  - [x] 5.1 Create `backend/src/services/validacion_fiscal.rs`
    - Implement `validar_rnc(rnc: &str) -> Result<(), AppError>`: verify 9 digits, validate check digit using DGII weighted modulus algorithm (weights `[7, 9, 8, 6, 5, 4, 3, 2]`, sum mod 11, check_digit = `(10 - check) % 9 + 1`)
    - Implement `validar_cedula(cedula: &str) -> Result<(), AppError>`: verify 11 digits, validate check digit using Luhn algorithm (alternating weights `[1, 2]` left-to-right on first 10 digits, sum digits > 9, check_digit = `(10 - sum % 10) % 10`)
    - Implement `formato_rnc`, `formato_cedula`, `parse_rnc`, `parse_cedula` for formatting/parsing round-trips
    - Return 422 errors with Spanish messages per requirements
    - Re-export in `backend/src/services/mod.rs`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4_

  - [x] 5.2 Write unit tests for fiscal validation
    - Create tests in `backend/src/services/validacion_fiscal.rs` (inline `#[cfg(test)]` module)
    - Test known valid RNCs and cédulas pass validation
    - Test invalid check digits are rejected with 422
    - Test wrong-length inputs are rejected
    - Test format → parse → format round-trip for both RNC and cédula
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4_

- [x] 6. Extend JWT Claims and update auth middleware
  - [x] 6.1 Add `organizacion_id` to Claims struct
    - Modify `Claims` in `backend/src/services/auth.rs` to add `organizacion_id: Uuid`
    - Update `encode_jwt` and `decode_jwt` — no logic changes needed, serde handles the new field
    - Update all existing test helpers that construct `Claims` (in `auth.rs` tests, `rbac.rs` tests) to include `organizacion_id`
    - _Requirements: 6.1, 6.2_

  - [x] 6.2 Update login to include `organizacion_id` in JWT
    - Modify `auth::login` in `backend/src/services/auth.rs` to look up the user's `organizacion_id` from the `usuarios` table and include it in JWT claims
    - If user has no `organizacion_id`, return 403 with message "Usuario no pertenece a ninguna organización"
    - _Requirements: 6.3, 6.4_

- [x] 7. Implement organization and invitation services
  - [x] 7.1 Create organization service
    - Create `backend/src/services/organizaciones.rs`
    - Implement `get_by_id(db, id) -> Result<OrganizacionResponse, AppError>`
    - Implement `update(db, id, input) -> Result<OrganizacionResponse, AppError>` — reject changes to immutable fields (tipo, cedula, rnc)
    - Re-export in `backend/src/services/mod.rs`
    - _Requirements: 10.1, 10.2, 10.3_

  - [x] 7.2 Create invitation service
    - Create `backend/src/services/invitaciones.rs`
    - Implement `crear(db, org_id, input) -> Result<InvitacionResponse, AppError>`: generate UUID token, set 7-day expiry, validate rol is gerente or visualizador
    - Implement `listar(db, org_id) -> Result<Vec<InvitacionResponse>, AppError>`: list pending (unused, not expired) invitations for the org
    - Implement `revocar(db, org_id, id) -> Result<(), AppError>`: delete invitation by id, scoped to org
    - Implement `validar_token(db, token) -> Result<invitacion::Model, AppError>`: check token exists, not used, not expired; return 410 if expired, 409 if used
    - Re-export in `backend/src/services/mod.rs`
    - _Requirements: 8.1, 8.2, 8.4, 8.5_

  - [x] 7.3 Rewrite registration service for org bootstrap and invitation flows
    - Modify `auth::register` in `backend/src/services/auth.rs` to handle two flows:
      - **New org flow** (no `token_invitacion`): validate tipo, validate RNC/cédula via `validacion_fiscal`, check email/cedula/rnc uniqueness, create org + user (rol=admin) in a single transaction, return JWT with `organizacion_id`
      - **Invitation flow** (`token_invitacion` present): validate token via `invitaciones::validar_token`, create user with invited rol and org_id, mark invitation as used, all in a single transaction, return JWT with `organizacion_id`
    - Return 409 for duplicate email/cedula/rnc with Spanish messages per requirements
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 8.3_

- [x] 8. Checkpoint — Verify services compile and unit tests pass
  - Run `cargo check` and `cargo test` for the backend. Ensure all tests pass. Ask the user if questions arise.

- [x] 9. Create handlers and register routes
  - [x] 9.1 Create organization handler
    - Create `backend/src/handlers/organizaciones.rs`
    - `get`: extract `Claims`, call `organizaciones::get_by_id(db, claims.organizacion_id)` — accessible to all roles
    - `update`: extract `AdminOnly`, call `organizaciones::update(db, claims.organizacion_id, input)` — admin only
    - Re-export in `backend/src/handlers/mod.rs`
    - _Requirements: 10.1, 10.4, 10.5_

  - [x] 9.2 Create invitation handler
    - Create `backend/src/handlers/invitaciones.rs`
    - `crear`: extract `AdminOnly`, call `invitaciones::crear(db, claims.organizacion_id, input)`
    - `listar`: extract `AdminOnly`, call `invitaciones::listar(db, claims.organizacion_id)`
    - `revocar`: extract `AdminOnly`, call `invitaciones::revocar(db, claims.organizacion_id, id)`
    - Re-export in `backend/src/handlers/mod.rs`
    - _Requirements: 8.1, 8.6_

  - [x] 9.3 Update auth handler for new registration flow
    - Modify `backend/src/handlers/auth.rs` to pass `AppConfig` to the register service (needed for JWT generation on registration)
    - Update the register handler to accept the new `RegisterRequest` shape and return `LoginResponse` (with JWT) instead of `UserResponse`
    - _Requirements: 3.1, 3.8_

  - [x] 9.4 Register new routes in `routes.rs`
    - Add `/api/v1/organizacion` scope: GET (all roles), PUT (admin only)
    - Add `/api/v1/invitaciones` scope: POST, GET, DELETE `/{id}` (all admin only)
    - _Requirements: 10.1, 10.5, 8.1, 8.6_

- [x] 10. Add org-scoped filtering to all existing services
  - [x] 10.1 Update `propiedades` service and handler
    - Add `org_id: Uuid` parameter to `list`, `get_by_id`, `create`, `update`, `delete` in `backend/src/services/propiedades.rs`
    - Add `.filter(propiedad::Column::OrganizacionId.eq(org_id))` to all queries
    - Set `organizacion_id` on create
    - Update `backend/src/handlers/propiedades.rs` to pass `claims.organizacion_id` to all service calls
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.2 Update `inquilinos` service and handler
    - Same pattern as 10.1: add `org_id` parameter, filter all queries, set on create
    - Update handler to pass `claims.organizacion_id`
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.3 Update `contratos` service and handler
    - Same pattern: add `org_id` parameter, filter all queries, set on create
    - Update handler to pass `claims.organizacion_id`
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.4 Update `pagos` service and handler
    - Same pattern: add `org_id` parameter, filter all queries, set on create
    - Update handler to pass `claims.organizacion_id`
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.5 Update `gastos` service and handler
    - Same pattern: add `org_id` parameter, filter all queries, set on create
    - Update handler to pass `claims.organizacion_id`
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.6 Update `mantenimiento` service and handler
    - Same pattern: add `org_id` parameter, filter all queries, set on create
    - Update handler to pass `claims.organizacion_id`
    - _Requirements: 2.3, 2.4, 2.5, 2.6_

  - [x] 10.7 Update `dashboard`, `auditoria`, `reportes`, `notificaciones`, `documentos`, `configuracion`, `importacion`, and `usuarios` services and handlers
    - Add `org_id` filtering to all query-based services that read org-scoped data
    - Update corresponding handlers to pass `claims.organizacion_id`
    - For `usuarios` service: filter user list by `organizacion_id` so admins only see their own org's users
    - _Requirements: 2.3, 2.4, 2.6, 7.1, 7.2, 7.3_

- [x] 11. Checkpoint — Full backend compilation and existing tests
  - Run `cargo check`, `cargo clippy`, and `cargo test`. Fix any compilation errors from the org_id changes across all services and handlers. Ask the user if questions arise.

- [ ] 12. Write integration tests for organization flows
  - [x] 12.1 Write tests for registration and org bootstrap
    - Create `backend/src/services/organizaciones_tests.rs` (or inline tests)
    - Test: persona_fisica registration creates org + admin user in one transaction
    - Test: persona_juridica registration creates org + admin user
    - Test: duplicate email returns 409
    - Test: duplicate cedula returns 409
    - Test: duplicate RNC returns 409
    - Test: invalid RNC returns 422
    - Test: invalid cédula returns 422
    - Test: JWT contains organizacion_id after registration
    - _Requirements: 3.1, 3.2, 3.5, 3.6, 3.7, 3.8, 4.3, 5.3_

  - [x] 12.2 Write tests for invitation flow
    - Test: admin can create invitation with gerente/visualizador role
    - Test: non-admin cannot create invitation (403)
    - Test: registration with valid invitation token joins existing org with invited role
    - Test: expired invitation returns 410
    - Test: used invitation returns 409
    - Test: admin can list and revoke invitations
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

  - [x] 12.3 Write tests for org-scoped data isolation
    - Test: user in org A cannot see propiedades from org B (returns empty list or 404)
    - Test: create propiedad sets organizacion_id from claims
    - Test: login returns JWT with correct organizacion_id
    - Test: user without org gets 403 on login
    - _Requirements: 2.4, 2.5, 2.6, 6.3, 6.4_

- [x] 13. Final checkpoint — All backend tests pass
  - Run `cargo fmt`, `cargo clippy`, and `cargo test`. Ensure zero warnings and all tests pass. Ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- The design has no Correctness Properties section, so no property-based test tasks are included
- Frontend (Yew/WASM) and Android (Kotlin/Compose) changes are not included — they should be handled as separate implementation tasks after the backend is stable
- All user-facing error messages are in Spanish per project conventions
- Migrations use the project naming convention: `m{YYYYMMDD}_{SEQ}_{name}.rs`
