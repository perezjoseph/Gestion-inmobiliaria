# Implementation Plan: Gestión de Unidades

## Overview

Implementar el módulo completo de gestión de unidades (CRUD) siguiendo la arquitectura existente (handlers → services → entities). La entidad `unidad` y su tabla ya existen, por lo que no se requieren migraciones. Se crean modelos request/response, servicio con validación de unicidad, handlers con auth extractors, rutas anidadas bajo propiedades, enriquecimiento de PropiedadResponse con métricas de ocupación, y una pestaña "Unidades" en la vista de detalle de propiedad en el frontend. Se sigue el patrón exacto de `gastos` como referencia.

## Tasks

- [x] 1. Request/response models
  - [x] 1.1 Create `backend/src/models/unidad.rs`
    - Define `CreateUnidadRequest` (Deserialize, camelCase): numero_unidad (String), piso (Option<i32>), habitaciones (Option<i32>), banos (Option<i32>), area_m2 (Option<Decimal>), precio (Decimal), moneda (Option<String>), estado (Option<String>), descripcion (Option<String>)
    - Define `UpdateUnidadRequest` (Deserialize, camelCase): all fields optional
    - Define `UnidadListQuery` (Deserialize, camelCase): estado (Option<String>), page (Option<u64>), per_page (Option<u64>)
    - Define `UnidadResponse` (Serialize, camelCase): id, propiedad_id, numero_unidad, piso (Option), habitaciones (Option), banos (Option), area_m2 (Option<Decimal>), precio (Decimal), moneda, estado, descripcion (Option), gastos_count (Option<u64>), mantenimiento_count (Option<u64>), created_at (DateTime<Utc>), updated_at (DateTime<Utc>)
    - Define `OcupacionResumen` (Serialize, camelCase): total_unidades (u64), unidades_ocupadas (u64), tasa_ocupacion (f64)
    - Follow the pattern from `backend/src/models/gasto.rs`
    - _Requirements: 1.1, 2.1, 3.1, 6.2, 7.2, 8.1_

  - [x] 1.2 Register model module in `backend/src/models/mod.rs`
    - Add `pub mod unidad;`
    - _Requirements: 1.1_

  - [x] 1.3 Write unit tests for model serialization/deserialization
    - Add `#[cfg(test)]` module in `backend/src/models/unidad.rs`
    - Test `CreateUnidadRequest` deserialization with camelCase JSON
    - Test `UpdateUnidadRequest` deserialization with partial fields
    - Test `UnidadListQuery` deserialization with optional fields
    - Test `UnidadResponse` serialization produces camelCase
    - Test `OcupacionResumen` serialization produces camelCase
    - _Requirements: 1.1, 2.1, 8.1_

- [x] 2. Service layer
  - [x] 2.1 Create `backend/src/services/unidades.rs`
    - Define constants: `ESTADOS_UNIDAD` (`disponible`, `ocupada`, `mantenimiento`)
    - Implement `From<unidad::Model>` for `UnidadResponse` (with gastos_count and mantenimiento_count as None)
    - Implement `validate_numero_unidad_unique<C: ConnectionTrait>(db, propiedad_id, numero_unidad, exclude_id: Option<Uuid>)` → Queries unidades with same numero_unidad in same propiedad (excluding given ID), returns `AppError::Conflict` if found
    - Implement `create<C: ConnectionTrait>(db, propiedad_id, org_id, input, usuario_id)`: validate propiedad exists and belongs to org, validate numero_unidad not empty/whitespace, validate uniqueness, validate moneda via `validate_enum`, validate estado via `validate_enum`, validate precio >= 0, create with defaults (estado=disponible, moneda=DOP), register auditoría
    - Implement `get_by_id(db, propiedad_id, org_id, id)`: find unidad, verify belongs to propiedad and org, count gastos and mantenimiento associated
    - Implement `list(db, propiedad_id, org_id, query)`: validate propiedad exists and belongs to org, paginated list with optional estado filter, ordered by numero_unidad ASC
    - Implement `update<C: ConnectionTrait>(db, propiedad_id, org_id, id, input, usuario_id)`: validate unidad exists and belongs to propiedad, validate uniqueness if numero_unidad changed (exclude current id), validate moneda/estado/precio if changed, update provided fields, register auditoría
    - Implement `delete<C: ConnectionTrait>(db, propiedad_id, org_id, id, usuario_id)`: validate unidad exists and belongs to propiedad, delete, register auditoría
    - Implement `get_ocupacion_resumen(db, propiedad_id)`: count total unidades and unidades with estado=ocupada, calculate tasa_ocupacion percentage (0 when no unidades)
    - Use `validate_enum` from `services/validation.rs` for enum validations
    - Use `auditoria::registrar_best_effort` for all audit entries with entity_type "unidad"
    - _Requirements: 1.1–1.8, 2.1–2.5, 3.1–3.7, 4.1–4.2, 5.1–5.2, 6.2, 7.2, 8.1–8.3, 9.1_

  - [x] 2.2 Register service module in `backend/src/services/mod.rs`
    - Add `pub mod unidades;`
    - _Requirements: 1.1_

  - [x] 2.3 Write unit tests for service
    - Add `#[cfg(test)]` module in `backend/src/services/unidades.rs`
    - Test `From<Model>` conversion for `UnidadResponse` (all fields, optional fields as None)
    - Test constants: `ESTADOS_UNIDAD` contains expected values
    - _Requirements: 1.1, 2.3_

- [x] 3. Handlers with auth extractors
  - [x] 3.1 Create `backend/src/handlers/unidades.rs`
    - Define `UnidadPath` struct (Deserialize): propiedad_id (Uuid), id (Uuid) for routes needing both IDs
    - Implement `list(db, claims: Claims, path: Path<Uuid>, query: Query<UnidadListQuery>)` → Ok(json) — path extracts propiedad_id
    - Implement `get_by_id(db, claims: Claims, path: Path<UnidadPath>)` → Ok(json)
    - Implement `create(db, access: WriteAccess, path: Path<Uuid>, body: Json<CreateUnidadRequest>)` → Created(json), use transaction
    - Implement `update(db, access: WriteAccess, path: Path<UnidadPath>, body: Json<UpdateUnidadRequest>)` → Ok(json), use transaction
    - Implement `delete(db, admin: AdminOnly, path: Path<UnidadPath>)` → NoContent, use transaction
    - Follow the exact pattern from `backend/src/handlers/propiedades.rs`
    - _Requirements: 1.1, 1.8, 2.1, 2.3, 3.1, 3.7, 4.1, 4.3, 4.4_

  - [x] 3.2 Register handler module in `backend/src/handlers/mod.rs`
    - Add `pub mod unidades;`
    - _Requirements: 1.1_

- [x] 4. Route registration
  - [x] 4.1 Add unidades routes to `backend/src/routes.rs`
    - Add a nested `web::scope("/{propiedad_id}/unidades")` block inside the existing `/propiedades` scope
    - Register routes: GET "" → list, POST "" → create, GET "/{id}" → get_by_id, PUT "/{id}" → update, DELETE "/{id}" → delete
    - Place the nested scope AFTER the existing propiedad routes to avoid path conflicts
    - _Requirements: 1.1, 2.1, 3.1, 4.1_

- [x] 5. Enrich PropiedadResponse with occupancy metrics
  - [x] 5.1 Add occupancy fields to `PropiedadResponse` in `backend/src/models/propiedad.rs`
    - Add `total_unidades: Option<u64>`, `unidades_ocupadas: Option<u64>`, `tasa_ocupacion: Option<f64>` fields
    - Update `From<propiedad::Model>` to set these as `None` (populated by service)
    - _Requirements: 8.1, 8.2_

  - [x] 5.2 Modify `services/propiedades.rs` to populate occupancy in `list`
    - After fetching propiedades, batch-query unidades counts using `is_in()` on propiedad_ids to avoid N+1
    - Use a single query with GROUP BY propiedad_id to get total and occupied counts
    - Populate `total_unidades`, `unidades_ocupadas`, `tasa_ocupacion` on each PropiedadResponse
    - _Requirements: 8.1, 8.3_

  - [x] 5.3 Modify `services/propiedades.rs` to populate occupancy in `get_by_id`
    - Call `unidades::get_ocupacion_resumen` to get counts for the single propiedad
    - Populate `total_unidades`, `unidades_ocupadas`, `tasa_ocupacion` on the PropiedadResponse
    - _Requirements: 8.2, 8.3_

- [x] 6. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Fix any compilation errors.

- [x] 7. Frontend types
  - [x] 7.1 Create `frontend/src/types/unidad.rs`
    - Define `Unidad` struct (Deserialize, Serialize, Clone, PartialEq, camelCase): id (String), propiedad_id (String), numero_unidad (String), piso (Option<i32>), habitaciones (Option<i32>), banos (Option<i32>), area_m2 (Option<f64>) with `deserialize_option_f64_from_any`, precio (f64) with `deserialize_f64_from_any`, moneda (String), estado (String), descripcion (Option<String>), gastos_count (Option<u64>), mantenimiento_count (Option<u64>), created_at (String), updated_at (String)
    - Define `CreateUnidad` struct (Serialize, camelCase): numero_unidad, piso (Option), habitaciones (Option), banos (Option), area_m2 (Option<f64>), precio (f64), moneda (Option<String>), estado (Option<String>), descripcion (Option<String>)
    - Define `UpdateUnidad` struct (Serialize, camelCase): all fields optional
    - Define `OcupacionResumen` struct (Deserialize, Clone, PartialEq, camelCase): total_unidades (u64), unidades_ocupadas (u64), tasa_ocupacion (f64) with `deserialize_f64_from_any`
    - Follow the pattern from `frontend/src/types/gasto.rs`
    - _Requirements: 10.1, 10.2_

  - [x] 7.2 Register type module in `frontend/src/types/mod.rs`
    - Add `pub mod unidad;`
    - _Requirements: 10.1_

  - [x] 7.3 Add occupancy fields to frontend `Propiedad` type in `frontend/src/types/propiedad.rs`
    - Add `total_unidades: Option<u64>`, `unidades_ocupadas: Option<u64>`, `tasa_ocupacion: Option<f64>` with `deserialize_option_f64_from_any`
    - _Requirements: 10.7_

- [x] 8. Frontend unidades tab component
  - [x] 8.1 Create `frontend/src/components/propiedades/unidades_tab.rs`
    - Implement `UnidadesTab` functional component that receives `propiedad_id: AttrValue` as prop
    - List view: table with columns (Número, Piso, Hab., Baños, Área, Precio, Estado, Acciones), filter by estado, "Nueva Unidad" button
    - Form view: modal with fields — numero_unidad, piso, habitaciones, baños, área m², descripción, precio, moneda (DOP/USD selector), estado (selector). Client-side validation for required numero_unidad
    - Use `api_get`, `api_post`, `api_put`, `api_delete` from `frontend/src/services/api.rs`
    - Color badges for estado: disponible=green, ocupada=blue, mantenimiento=orange
    - Hide create/edit/delete buttons for `visualizador` role using `can_write()` / `can_delete()` pattern
    - All text in Spanish, prices with currency format (DOP/USD), two decimal precision
    - Split into sub-components if html! blocks exceed 150 lines to avoid WASM OOM
    - Use `use_effect_with((reload_val, page), ...)` pattern for data fetching, not direct filter deps
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

  - [x] 8.2 Register component in `frontend/src/components/propiedades/mod.rs`
    - Add `pub mod unidades_tab;`
    - _Requirements: 10.1_

- [x] 9. Integrate unidades tab into propiedades detail view
  - [x] 9.1 Modify `frontend/src/pages/propiedades.rs` to add "Unidades" tab
    - Add a tab/section in the propiedad detail view that renders `UnidadesTab` with the propiedad_id
    - Add tab navigation between existing detail content and the unidades tab
    - _Requirements: 10.1_

  - [x] 9.2 Show occupancy metrics on property cards in list view
    - Modify the propiedad card/row in the list view to display "X unidades · Y% ocupación" when `total_unidades > 0`
    - _Requirements: 10.7_

- [x] 10. Checkpoint — Ensure full project compiles
  - Run `cargo test --workspace` and ensure all tests pass. Fix any compilation errors.

- [x] 11. Integration tests
  - [x] 11.1 Create `backend/tests/unidades_tests.rs`
    - Test full CRUD cycle: create unidad → get by id → update → list → delete
    - Test numero_unidad uniqueness: create two unidades with same numero_unidad in same propiedad returns 409
    - Test numero_unidad uniqueness on update: update to existing numero_unidad returns 409
    - Test numero_unidad uniqueness across propiedades: same numero_unidad in different propiedades is allowed
    - Test filters: list by estado
    - Test ordering: list returns unidades ordered by numero_unidad ASC
    - Test access control: WriteAccess for create/update, AdminOnly for delete, visualizador gets 403
    - Test validation: empty numero_unidad returns 422, invalid estado returns 422, invalid moneda returns 422, negative precio returns 422
    - Test non-existent propiedad returns 404 on create and list
    - Test non-existent unidad returns 404 on get, update, delete
    - Test gastos_count and mantenimiento_count in get_by_id response
    - Test occupancy metrics in propiedad list and detail responses
    - Test auditoría entries are created for each operation
    - _Requirements: 1.1–1.8, 2.1–2.5, 3.1–3.7, 4.1–4.4, 5.1–5.2, 6.2, 7.2, 8.1–8.3, 9.1_

- [x] 12. Property-based tests
  - [x] 12.1 Write property test: Creation round-trip preserves data (P1)
    - **Property 1: Creation round-trip preserves data**
    - **Validates: Requirements 1.1, 2.3**
    - Generate random valid `CreateUnidadRequest` inputs, create unidad, retrieve by ID, verify all input fields match and defaults applied

  - [x] 12.2 Write property test: Numero_unidad uniqueness within propiedad (P2)
    - **Property 2: Numero_unidad uniqueness within propiedad**
    - **Validates: Requirements 1.3, 5.1, 5.2**
    - Generate random numero_unidad, create first unidad, attempt to create second with same numero_unidad in same propiedad, verify conflict error

  - [x] 12.3 Write property test: List ordering invariant (P3)
    - **Property 3: List ordering invariant**
    - **Validates: Requirements 2.1**
    - Create multiple unidades with random numero_unidad values, list, verify lexicographic ascending order

  - [x] 12.4 Write property test: Filtering returns only matching records (P4)
    - **Property 4: Filtering returns only matching records**
    - **Validates: Requirements 2.2**
    - Create unidades with varied estados, filter by each, verify all returned records match

  - [x] 12.5 Write property test: Update preserves and replaces fields (P5)
    - **Property 5: Update replaces provided fields and preserves others**
    - **Validates: Requirements 3.1**
    - Generate random partial updates, apply to existing unidad, verify updated fields changed and non-updated fields preserved

  - [x] 12.6 Write property test: Update preserves uniqueness (P6)
    - **Property 6: Update preserves numero_unidad uniqueness**
    - **Validates: Requirements 3.3, 5.1**
    - Create two unidades, attempt to update second's numero_unidad to first's, verify conflict error

  - [x] 12.7 Write property test: Invalid enum values are rejected (P7)
    - **Property 7: Invalid enum values are rejected**
    - **Validates: Requirements 1.5, 1.6, 3.4, 3.5**
    - Generate random strings not in valid estado/moneda sets, verify rejection

  - [x] 12.8 Write property test: Negative prices are rejected (P8)
    - **Property 8: Negative prices are rejected**
    - **Validates: Requirements 1.7, 3.6**
    - Generate negative Decimal values, verify rejection when used as precio

  - [x] 12.9 Write property test: Non-existent propiedad references are rejected (P9)
    - **Property 9: Non-existent propiedad references are rejected**
    - **Validates: Requirements 1.2, 2.5**
    - Generate random UUIDs not in database, verify rejection when used as propiedad_id

  - [x] 12.10 Write property test: Occupancy counts are consistent (P10)
    - **Property 10: Occupancy counts are consistent**
    - **Validates: Requirements 8.1, 8.2, 8.3**
    - Create unidades with random estados, get occupancy resumen, verify total = count of all, occupied = count of estado=ocupada, rate = (occupied/total)*100 or 0

- [x] 13. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Fix any compilation errors.

## Notes

- The `unidades` table and `unidad` entity already exist — no migrations needed
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The implementation follows the existing gastos pattern for unit-level tracking under propiedades
- Occupancy enrichment in propiedades uses batch queries with `is_in()` to avoid N+1
- Frontend follows Yew anti-pattern guidelines: sub-components for large html!, `use_effect_with` deps, `AttrValue` for string props
