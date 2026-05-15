# Implementation Plan: Solicitudes de Mantenimiento

## Overview

Implementar el módulo completo de solicitudes de mantenimiento siguiendo la arquitectura existente (handlers → services → entities). Se crean dos tablas nuevas, entidades SeaORM, modelos request/response, servicio con máquina de estados, handlers con auth extractors, rutas, y una página frontend Yew con listado, formulario y detalle. Se sigue el patrón exacto de `pagos` como referencia.

## Tasks

- [x] 1. Database migrations
  - [x] 1.1 Create migration `m20250411_000001_create_solicitudes_mantenimiento`
    - Create file `backend/migrations/m20250411_000001_create_solicitudes_mantenimiento.rs`
    - Define table `solicitudes_mantenimiento` with all columns per design: id (UUID PK), propiedad_id (FK → propiedades), unidad_id (nullable FK → unidades), inquilino_id (nullable FK → inquilinos), titulo, descripcion, estado, prioridad, nombre_proveedor, telefono_proveedor, email_proveedor, costo_monto (DECIMAL(12,2)), costo_moneda, fecha_inicio, fecha_fin, created_at, updated_at
    - Add indexes: `idx_solicitudes_mant_propiedad_id`, `idx_solicitudes_mant_estado`, `idx_solicitudes_mant_prioridad`, `idx_solicitudes_mant_unidad_id`
    - Follow the exact pattern from `m20250408_000005_create_pagos.rs` for table creation, FK constraints, and index creation
    - _Requirements: 1.1, 9.1, 9.2, 9.3_

  - [x] 1.2 Create migration `m20250411_000002_create_notas_mantenimiento`
    - Create file `backend/migrations/m20250411_000002_create_notas_mantenimiento.rs`
    - Define table `notas_mantenimiento` with columns: id (UUID PK), solicitud_id (FK → solicitudes_mantenimiento ON DELETE CASCADE), autor_id (FK → usuarios), contenido (TEXT NOT NULL), created_at
    - Add index: `idx_notas_mant_solicitud_id`
    - _Requirements: 7.1_

  - [x] 1.3 Register both migrations in `backend/migrations/mod.rs`
    - Add `pub mod` declarations for both new migration modules
    - Add both migrations to the `Migrator::migrations()` vec in order
    - _Requirements: 1.1, 7.1_

- [x] 2. SeaORM entities
  - [x] 2.1 Create entity `backend/src/entities/solicitud_mantenimiento.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "solicitudes_mantenimiento")]`
    - All fields per design: id (Uuid PK), propiedad_id, unidad_id (Option), inquilino_id (Option), titulo, descripcion (Option), estado, prioridad, nombre_proveedor (Option), telefono_proveedor (Option), email_proveedor (Option), costo_monto (Option<Decimal>), costo_moneda (Option), fecha_inicio (Option<DateTimeWithTimeZone>), fecha_fin (Option<DateTimeWithTimeZone>), created_at, updated_at
    - Define `Relation` enum with `belongs_to Propiedad` and `has_many NotaMantenimiento`
    - Follow the pattern from `backend/src/entities/pago.rs`
    - _Requirements: 1.1, 9.1, 9.2, 9.3_

  - [x] 2.2 Create entity `backend/src/entities/nota_mantenimiento.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "notas_mantenimiento")]`
    - Fields: id (Uuid PK), solicitud_id, autor_id, contenido (String), created_at (DateTimeWithTimeZone)
    - Define `Relation` with `belongs_to SolicitudMantenimiento` and `belongs_to Usuario`
    - _Requirements: 7.1_

  - [x] 2.3 Register entities in `backend/src/entities/mod.rs`
    - Add `pub mod solicitud_mantenimiento;` and `pub mod nota_mantenimiento;`
    - _Requirements: 1.1, 7.1_

- [x] 3. Request/response models
  - [x] 3.1 Create `backend/src/models/mantenimiento.rs`
    - Define `CreateSolicitudRequest` (Deserialize, camelCase): propiedad_id, unidad_id (Option), inquilino_id (Option), titulo, descripcion (Option), prioridad (Option), nombre_proveedor (Option), telefono_proveedor (Option), email_proveedor (Option), costo_monto (Option<Decimal>), costo_moneda (Option)
    - Define `UpdateSolicitudRequest` (Deserialize, camelCase): all fields optional
    - Define `CambiarEstadoRequest` (Deserialize, camelCase): estado (String)
    - Define `CreateNotaRequest` (Deserialize, camelCase): contenido (String)
    - Define `SolicitudListQuery` (Deserialize, camelCase): estado (Option), prioridad (Option), propiedad_id (Option), page (Option<u64>), per_page (Option<u64>)
    - Define `SolicitudResponse` (Serialize, camelCase): all solicitud fields + notas (Option<Vec<NotaResponse>>)
    - Define `NotaResponse` (Serialize, camelCase): id, solicitud_id, autor_id, contenido, created_at
    - Follow the pattern from `backend/src/models/pago.rs`
    - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1_

  - [x] 3.2 Register model module in `backend/src/models/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 1.1_

  - [x] 3.3 Write unit tests for model serialization/deserialization
    - Add `#[cfg(test)]` module in `backend/src/models/mantenimiento.rs`
    - Test `CreateSolicitudRequest` deserialization with camelCase JSON
    - Test `SolicitudListQuery` deserialization with optional fields
    - Test `SolicitudResponse` serialization produces camelCase
    - _Requirements: 1.1, 2.1_

- [x] 4. Service layer with state machine logic
  - [x] 4.1 Create `backend/src/services/mantenimiento.rs`
    - Define constants: `ESTADOS_SOLICITUD` (`pendiente`, `en_progreso`, `completado`), `PRIORIDADES` (`baja`, `media`, `alta`, `urgente`), `MONEDAS_COSTO` (`DOP`, `USD`)
    - Implement `From<solicitud_mantenimiento::Model>` for `SolicitudResponse`
    - Implement `From<nota_mantenimiento::Model>` for `NotaResponse`
    - Implement `validar_transicion(estado_actual: &str, nuevo_estado: &str) -> Result<(), AppError>` as a pure function with the state machine rules: pendiente→en_progreso (OK), en_progreso→completado (OK), all others rejected with specific Spanish error messages
    - Implement `create<C: ConnectionTrait>(db, input, usuario_id)`: validate propiedad exists, validate unidad belongs to propiedad (if provided), validate inquilino exists (if provided), validate prioridad, validate moneda/monto, validate titulo not empty, create with estado=pendiente and default prioridad=media, register auditoría
    - Implement `get_by_id(db, id)`: find solicitud, load notas ordered by created_at DESC
    - Implement `list(db, query)`: paginated list with optional filters (estado, prioridad, propiedad_id), ordered by created_at DESC
    - Implement `update<C: ConnectionTrait>(db, id, input, usuario_id)`: validate prioridad if changed, validate moneda/monto if changed, validate monto >= 0, update provided fields, register auditoría
    - Implement `cambiar_estado<C: ConnectionTrait>(db, id, nuevo_estado, usuario_id)`: call `validar_transicion`, set fecha_inicio on pendiente→en_progreso, set fecha_fin on en_progreso→completado, register auditoría
    - Implement `delete<C: ConnectionTrait>(db, id, usuario_id)`: delete solicitud (CASCADE deletes notas), register auditoría
    - Implement `agregar_nota<C: ConnectionTrait>(db, solicitud_id, contenido, usuario_id)`: validate solicitud exists, validate contenido not empty/whitespace, create nota, register auditoría
    - Use `validate_enum` from `services/validation.rs` for enum validations
    - Use `auditoria::registrar` for all audit entries with entity_type "solicitud_mantenimiento" or "nota_mantenimiento"
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 7.4, 8.1, 8.2, 9.4, 9.5, 10.1, 10.2_

  - [x] 4.2 Register service module in `backend/src/services/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 1.1_

  - [x] 4.3 Write unit tests for `validar_transicion`
    - Add `#[cfg(test)]` module in `backend/src/services/mantenimiento.rs`
    - Test all valid transitions: pendiente→en_progreso (OK), en_progreso→completado (OK)
    - Test all invalid transitions: pendiente→completado (Err), completado→pendiente (Err), completado→en_progreso (Err), completado→completado (Err), en_progreso→pendiente (Err)
    - Test `From<Model>` conversions for `SolicitudResponse` and `NotaResponse`
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 5. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 6. Handlers with auth extractors
  - [x] 6.1 Create `backend/src/handlers/mantenimiento.rs`
    - Implement `list(db, _claims: Claims, query: Query<SolicitudListQuery>)` → Ok(json)
    - Implement `get_by_id(db, _claims: Claims, path: Path<Uuid>)` → Ok(json)
    - Implement `create(db, access: WriteAccess, body: Json<CreateSolicitudRequest>)` → Created(json), use transaction
    - Implement `update(db, access: WriteAccess, path: Path<Uuid>, body: Json<UpdateSolicitudRequest>)` → Ok(json), use transaction
    - Implement `cambiar_estado(db, access: WriteAccess, path: Path<Uuid>, body: Json<CambiarEstadoRequest>)` → Ok(json), use transaction
    - Implement `delete(db, admin: AdminOnly, path: Path<Uuid>)` → NoContent, use transaction
    - Implement `agregar_nota(db, access: WriteAccess, path: Path<Uuid>, body: Json<CreateNotaRequest>)` → Created(json), use transaction
    - Follow the exact pattern from `backend/src/handlers/pagos.rs`
    - _Requirements: 1.1, 1.6, 2.1, 2.5, 3.1, 3.4, 4.1, 5.1, 6.1, 7.1, 8.1, 8.3, 8.4_

  - [x] 6.2 Register handler module in `backend/src/handlers/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 1.1_

- [x] 7. Route registration
  - [x] 7.1 Add mantenimiento routes to `backend/src/routes.rs`
    - Add a new `web::scope("/mantenimiento")` block inside the `/api` scope
    - Register routes: GET "" → list, POST "" → create, GET "/{id}" → get_by_id, PUT "/{id}" → update, PUT "/{id}/estado" → cambiar_estado, DELETE "/{id}" → delete, POST "/{id}/notas" → agregar_nota
    - Follow the pattern of existing scope registrations (e.g., `/pagos`)
    - _Requirements: 1.1, 2.1, 3.1, 4.1, 7.1, 8.1_

- [x] 8. Checkpoint — Ensure backend compiles with all new modules wired
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 9. Frontend types
  - [x] 9.1 Create `frontend/src/types/mantenimiento.rs`
    - Define `Solicitud` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): id, propiedad_id, unidad_id (Option), inquilino_id (Option), titulo, descripcion (Option), estado, prioridad, nombre_proveedor (Option), telefono_proveedor (Option), email_proveedor (Option), costo_monto (Option<f64>), costo_moneda (Option), fecha_inicio (Option<String>), fecha_fin (Option<String>), notas (Option<Vec<Nota>>), created_at, updated_at
    - Define `Nota` struct: id, solicitud_id, autor_id, contenido, created_at
    - Define `CreateSolicitud` struct (Serialize, camelCase): propiedad_id, unidad_id (Option), inquilino_id (Option), titulo, descripcion (Option), prioridad (Option), nombre_proveedor (Option), telefono_proveedor (Option), email_proveedor (Option), costo_monto (Option<f64>), costo_moneda (Option)
    - Define `UpdateSolicitud` struct (Serialize, camelCase): all fields optional
    - Define `CambiarEstado` struct (Serialize, camelCase): estado
    - Define `CreateNota` struct (Serialize, camelCase): contenido
    - Follow the pattern from `frontend/src/types/pago.rs`
    - _Requirements: 11.1, 11.2, 11.3_

  - [x] 9.2 Register type module in `frontend/src/types/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 11.1_

- [x] 10. Frontend mantenimiento page
  - [x] 10.1 Create `frontend/src/pages/mantenimiento.rs`
    - Implement `Mantenimiento` functional component with three views: list, create/edit form, detail
    - List view: paginated table with columns (Propiedad, Título, Prioridad, Estado, Proveedor, Costo, Acciones), filters by estado and prioridad, "Nueva Solicitud" button
    - Form view: modal/card with fields — propiedad (selector), unidad (selector filtered by selected propiedad), inquilino (selector), título, descripción, prioridad (selector), nombre proveedor, teléfono proveedor, email proveedor, costo monto, costo moneda (DOP/USD). Client-side validation for required título
    - Detail view: full solicitud info, state transition buttons (only valid transitions), notes section with add-note form
    - Use `api_get`, `api_post`, `api_put`, `api_delete` from `frontend/src/services/api.rs`
    - Color badges for prioridad: urgente=red, alta=orange, media=yellow, baja=green
    - Color badges for estado: pendiente=warning, en_progreso=info, completado=success
    - Hide create/edit/delete/change-state buttons for `visualizador` role using `can_write()` / `can_delete()` pattern
    - All text in Spanish, dates in DD/MM/YYYY format, costs with currency format
    - Follow the pattern from `frontend/src/pages/pagos.rs`
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 11.8_

  - [x] 10.2 Register page module in `frontend/src/pages/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 11.1_

  - [x] 10.3 Add route and navigation
    - Add `Mantenimiento` variant to `Route` enum in `frontend/src/app.rs` with `#[at("/mantenimiento")]`
    - Add match arm in `switch()` function: `Route::Mantenimiento => html! { <ProtectedRoute><Mantenimiento /></ProtectedRoute> }`
    - Add import for `crate::pages::mantenimiento::Mantenimiento`
    - Add "Mantenimiento" link to sidebar in `frontend/src/components/layout/sidebar.rs` with a wrench/tool icon
    - _Requirements: 11.1_

- [x] 11. Frontend mantenimiento components
  - [x] 11.1 Create `frontend/src/components/mantenimiento/mod.rs`
    - Implement reusable badge components for prioridad and estado
    - `prioridad_badge(prioridad: &str)` → returns styled HTML span with color coding (urgente=red, alta=orange, media=yellow, baja=green)
    - `estado_badge(estado: &str)` → returns styled HTML span with color coding (pendiente=warning, en_progreso=info, completado=success)
    - _Requirements: 11.7, 11.8_

  - [x] 11.2 Register component module in `frontend/src/components/mod.rs`
    - Add `pub mod mantenimiento;`
    - _Requirements: 11.1_

- [x] 12. Checkpoint — Ensure full project compiles
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 13. Integration tests
  - [x] 13.1 Create `backend/tests/mantenimiento_tests.rs`
    - Test full CRUD cycle: create solicitud → get by id → update → list → delete
    - Test state machine flow: create → cambiar_estado to en_progreso → cambiar_estado to completado
    - Test invalid state transitions return 422
    - Test add and list notes on a solicitud
    - Test filters: list by estado, prioridad, propiedad_id
    - Test access control: WriteAccess for create/update/cambiar_estado/agregar_nota, AdminOnly for delete, visualizador gets 403
    - Test FK validations: non-existent propiedad_id returns 404, non-existent inquilino_id returns 404, unidad not belonging to propiedad returns 422
    - Test validation: empty titulo returns 422, invalid prioridad returns 422, invalid moneda returns 422, negative costo_monto returns 422, empty note contenido returns 422
    - Test auditoría entries are created for each operation
    - _Requirements: 1.1–1.6, 2.1–2.6, 3.1–3.4, 4.1–4.4, 5.1–5.3, 6.1–6.3, 7.1–7.4, 8.1–8.4, 9.4, 9.5, 10.1, 10.2_

- [x] 14. Property-based tests
  - [x] 14.1 Add `proptest` to `[dev-dependencies]` in `backend/Cargo.toml`
    - Add `proptest = "1"` to dev-dependencies section
    - _Requirements: N/A (testing infrastructure)_

  - [x] 14.2 Write property test: Creation round-trip preserves data
    - **Property 1: Creation round-trip preserves data**
    - **Validates: Requirements 1.1, 2.5, 6.4**
    - Generate random valid `CreateSolicitudRequest` inputs, create solicitud, retrieve by ID, verify all input fields match and estado is "pendiente"

  - [x] 14.3 Write property test: List ordering invariant
    - **Property 2: List ordering invariant**
    - **Validates: Requirements 2.1**
    - Create multiple solicitudes, list without filters, verify `created_at` descending order for every consecutive pair

  - [x] 14.4 Write property test: Filtering returns only matching records
    - **Property 3: Filtering returns only matching records**
    - **Validates: Requirements 2.2, 2.3, 2.4**
    - Create solicitudes with varied estados/prioridades, filter by each, verify all returned records match the filter

  - [x] 14.5 Write property test: Update preserves and replaces fields
    - **Property 4: Update replaces provided fields and preserves others**
    - **Validates: Requirements 3.1, 5.1, 5.2**
    - Generate random partial updates, apply to existing solicitud, verify updated fields changed and non-updated fields preserved

  - [x] 14.6 Write property test: Valid state transitions set timestamps
    - **Property 5: Valid state transitions set timestamps**
    - **Validates: Requirements 4.1, 4.2**
    - Create solicitud, transition pendiente→en_progreso, verify fecha_inicio set; transition en_progreso→completado, verify fecha_fin set

  - [x] 14.7 Write property test: Invalid state transitions are rejected
    - **Property 6: Invalid state transitions are rejected**
    - **Validates: Requirements 4.3, 4.4**
    - Generate all invalid (estado_actual, nuevo_estado) combinations, verify each returns validation error

  - [x] 14.8 Write property test: Invalid enum values are rejected
    - **Property 7: Invalid enum values are rejected**
    - **Validates: Requirements 1.4, 3.3, 6.2**
    - Generate random strings not in valid prioridad/moneda sets, verify rejection

  - [x] 14.9 Write property test: Negative cost amounts are rejected
    - **Property 8: Negative cost amounts are rejected**
    - **Validates: Requirements 6.3**
    - Generate negative Decimal values, verify rejection when used as costo_monto

  - [x] 14.10 Write property test: Empty or whitespace-only notes are rejected
    - **Property 9: Empty or whitespace-only notes are rejected**
    - **Validates: Requirements 7.2**
    - Generate whitespace-only strings, verify rejection and notes count unchanged

  - [x] 14.11 Write property test: Notes ordering invariant
    - **Property 10: Notes ordering invariant**
    - **Validates: Requirements 7.3**
    - Add multiple notes to a solicitud, retrieve detail, verify notes ordered by created_at DESC

  - [x] 14.12 Write property test: Cascade delete removes solicitud and all notes
    - **Property 11: Cascade delete removes solicitud and all notes**
    - **Validates: Requirements 8.1**
    - Create solicitud with notes, delete solicitud, verify both solicitud and notes absent

  - [x] 14.13 Write property test: Unit-property ownership validation
    - **Property 12: Unit-property ownership validation**
    - **Validates: Requirements 9.4**
    - Generate unidad_id/propiedad_id pairs where unidad does not belong to propiedad, verify rejection

  - [x] 14.14 Write property test: Non-existent FK references are rejected
    - **Property 13: Non-existent FK references are rejected**
    - **Validates: Requirements 1.2, 9.5**
    - Generate random UUIDs not in database, verify rejection when used as propiedad_id or inquilino_id

- [x] 15. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- The implementation language is Rust, matching the existing codebase and design document
