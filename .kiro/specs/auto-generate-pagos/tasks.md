# Implementation Plan: Auto-Generación de Pagos desde Contratos

## Overview

Implementar la generación automática de pagos mensuales a partir de contratos. Se crea un módulo de generación con funciones puras (`calcular_pagos`, `filtrar_existentes`, `validar_dia_vencimiento`), se modifican los flujos existentes de contratos (`create`, `renovar`, `terminar`) para generar/cancelar pagos dentro de sus transacciones, se agregan dos endpoints nuevos (`preview` y `generar`), y se extiende la página frontend de contratos con un botón "Generar Pagos" y modal de preview. No se crean tablas nuevas; se usa la tabla `pagos` existente con un nuevo estado "cancelado".

## Tasks

- [x] 1. Backend models and pure generation logic
  - [x] 1.1 Create `backend/src/models/pago_generacion.rs` with DTOs
    - Define `GenerarPagosRequest` (Deserialize, camelCase): `dia_vencimiento: Option<u32>`
    - Define `PreviewPagosQuery` (Deserialize, camelCase): `dia_vencimiento: Option<u32>`
    - Define `PreviewPagosResponse` (Serialize, camelCase): `contrato_id: Uuid`, `pagos: Vec<PagoPreview>`, `total_pagos: usize`, `monto_total: Decimal`, `pagos_existentes: usize`, `pagos_nuevos: usize`
    - Define `PagoPreview` (Serialize, camelCase): `monto: Decimal`, `moneda: String`, `fecha_vencimiento: NaiveDate`
    - Define `GenerarPagosResponse` (Serialize, camelCase): `contrato_id: Uuid`, `pagos_generados: usize`, `pagos: Vec<PagoResponse>`
    - Register module in `backend/src/models/mod.rs`
    - _Requirements: 4.1, 4.4, 4.5, 5.5, 6.3_

  - [x] 1.2 Create `backend/src/services/pago_generacion.rs` with pure functions
    - Define `PagoGenerado` struct: `monto: Decimal`, `moneda: String`, `fecha_vencimiento: NaiveDate`
    - Implement `calcular_pagos(fecha_inicio: NaiveDate, fecha_fin: NaiveDate, monto_mensual: Decimal, moneda: &str, dia_vencimiento: u32) -> Vec<PagoGenerado>`: iterate month by month from `fecha_inicio` to `fecha_fin` (inclusive), clamp day to last day of month if needed
    - Implement `filtrar_existentes(pagos_calculados: &[PagoGenerado], fechas_existentes: &[NaiveDate]) -> Vec<PagoGenerado>`: filter out pagos whose (year, month) already exists in `fechas_existentes`
    - Implement `validar_dia_vencimiento(dia: u32) -> Result<(), AppError>`: reject values outside 1..=31
    - Register module in `backend/src/services/mod.rs`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 6.1, 6.2, 6.4, 7.1, 7.2, 7.3_

  - [x] 1.3 Write unit tests for pure functions in `pago_generacion.rs`
    - Test `calcular_pagos` with single month range → returns 1 pago
    - Test `calcular_pagos` with 12-month range → returns 12 pagos
    - Test `calcular_pagos` with `dia_vencimiento=31` in February → uses day 28/29
    - Test `calcular_pagos` with `fecha_inicio == fecha_fin` (same day) → returns 1 pago
    - Test `filtrar_existentes` with all months existing → returns empty
    - Test `filtrar_existentes` with no months existing → returns all
    - Test `filtrar_existentes` with some months existing → returns only missing
    - Test `validar_dia_vencimiento` with 0 → error, 32 → error, 1 and 31 → ok
    - _Requirements: 1.1, 1.3, 1.4, 1.5, 1.6, 6.1, 6.2, 6.4, 7.1, 7.2, 7.3_

  - [x] 1.4 Write unit tests for model serialization/deserialization in `pago_generacion.rs`
    - Test `GenerarPagosRequest` deserialization with camelCase JSON
    - Test `PreviewPagosQuery` deserialization with optional `dia_vencimiento`
    - Test `PreviewPagosResponse` serialization produces camelCase
    - _Requirements: 4.1, 5.5, 6.3_

- [x] 2. Extend pago states and modify ContratoResponse
  - [x] 2.1 Add "cancelado" to `ESTADOS_PAGO` in `backend/src/services/pagos.rs`
    - Change `const ESTADOS_PAGO: &[&str] = &["pendiente", "pagado", "atrasado"];` to include `"cancelado"`
    - _Requirements: 3.1_

  - [x] 2.2 Add `pagos_generados: Option<usize>` field to `ContratoResponse` in `backend/src/models/contrato.rs`
    - Add `#[serde(skip_serializing_if = "Option::is_none")]` attribute
    - Update `From<contrato::Model>` impl in `backend/src/services/contratos.rs` to set `pagos_generados: None`
    - _Requirements: 1.8, 2.4_

- [x] 3. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 4. Integrate generation into contratos service
  - [x] 4.1 Add `insertar_pagos_generados` helper in `backend/src/services/contratos.rs`
    - Implement `async fn insertar_pagos_generados<C: ConnectionTrait>(db: &C, contrato_id: Uuid, organizacion_id: Uuid, pagos: &[PagoGenerado]) -> Result<usize, AppError>`: insert each `PagoGenerado` as a `pago::ActiveModel` with `estado = "pendiente"`, `metodo_pago = None`, `fecha_pago = None`
    - _Requirements: 1.1, 1.2_

  - [x] 4.2 Add `cancelar_pagos_futuros` helper in `backend/src/services/contratos.rs`
    - Implement `async fn cancelar_pagos_futuros<C: ConnectionTrait>(db: &C, contrato_id: Uuid, fecha_terminacion: NaiveDate) -> Result<usize, AppError>`: use `update_many` to change `estado` from `"pendiente"` to `"cancelado"` where `contrato_id` matches and `fecha_vencimiento > fecha_terminacion`
    - _Requirements: 3.1, 3.2, 3.3_

  - [x] 4.3 Modify `contratos::create` to generate pagos for active contracts
    - After inserting the contrato and before `txn.commit()`, if `estado == "activo"`, call `calcular_pagos` with default `dia_vencimiento = 1` then `insertar_pagos_generados`
    - Register auditoría entry with action `"generar_pagos_auto"` and count of pagos generated
    - Set `pagos_generados` field on the returned `ContratoResponse`
    - _Requirements: 1.1, 1.2, 1.3, 1.7, 1.8, 8.1_

  - [x] 4.4 Modify `contratos::renovar` to generate pagos for the new contract
    - After inserting the new contrato and before `txn.commit()`, call `calcular_pagos` with the new contract's data then `insertar_pagos_generados`
    - Register auditoría entry with action `"generar_pagos_auto"` and count
    - Set `pagos_generados` field on the returned `ContratoResponse`
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 8.1_

  - [x] 4.5 Modify `contratos::terminar` to cancel future pending pagos
    - After updating the contrato estado and before `txn.commit()`, call `cancelar_pagos_futuros` with `fecha_terminacion`
    - Register auditoría entry with action `"cancelar_pagos_futuros"` and count of pagos cancelled
    - _Requirements: 3.1, 3.2, 3.3, 8.3_

- [x] 5. Checkpoint — Ensure backend compiles and modified contratos logic works
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 6. New handlers and routes for preview and generar
  - [x] 6.1 Add `preview_pagos` and `generar_pagos` handlers in `backend/src/handlers/contratos.rs`
    - `preview_pagos(db, _claims: Claims, path: Path<Uuid>, query: Query<PreviewPagosQuery>)`: load contrato, call `calcular_pagos`, query existing pagos for the contrato, call `filtrar_existentes`, build `PreviewPagosResponse`
    - `generar_pagos(db, access: WriteAccess, path: Path<Uuid>, body: Json<GenerarPagosRequest>)`: validate contrato exists and is active, validate `dia_vencimiento` if provided, call `calcular_pagos`, query existing pagos, call `filtrar_existentes`, insert new pagos in transaction, register auditoría with action `"generar_pagos_manual"`, return `GenerarPagosResponse`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 6.1, 6.3, 6.4, 8.2_

  - [x] 6.2 Register new routes in `backend/src/routes.rs`
    - Add `/{id}/pagos/preview` (GET) → `handlers::contratos::preview_pagos` inside the `/contratos` scope
    - Add `/{id}/pagos/generar` (POST) → `handlers::contratos::generar_pagos` inside the `/contratos` scope
    - _Requirements: 4.1, 5.1_

- [x] 7. Checkpoint — Ensure backend compiles with new endpoints
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 8. Frontend types and UI changes
  - [x] 8.1 Create `frontend/src/types/pago_generacion.rs` with frontend DTOs
    - Define `PreviewPagos` (Deserialize, Serialize, Clone, PartialEq, camelCase): `contrato_id: String`, `pagos: Vec<PagoPreviewItem>`, `total_pagos: u64`, `monto_total: f64` (with `deserialize_f64_from_any`), `pagos_existentes: u64`, `pagos_nuevos: u64`
    - Define `PagoPreviewItem` (Deserialize, Serialize, Clone, PartialEq, camelCase): `monto: f64` (with `deserialize_f64_from_any`), `moneda: String`, `fecha_vencimiento: String`
    - Define `GenerarPagosResponse` (Deserialize, Serialize, Clone, PartialEq, camelCase): `contrato_id: String`, `pagos_generados: u64`
    - Register module in `frontend/src/types/mod.rs`
    - _Requirements: 9.1, 9.3, 9.4, 9.5_

  - [x] 8.2 Modify `frontend/src/pages/contratos.rs` to add "Generar Pagos" button and preview modal
    - Add "Generar Pagos" button in the actions column for active contracts, visible only when `can_write(rol)` is true
    - On click, call `GET /contratos/{id}/pagos/preview` and show a modal with the list of pagos (fecha_vencimiento, monto, moneda), total count, total amount, existing vs new counts
    - Add "Confirmar" button in the modal that calls `POST /contratos/{id}/pagos/generar`
    - On success, show toast with "X pagos generados" message and close modal
    - Update the create contrato success toast to include `pagos_generados` count from the response
    - Hide the button for `visualizador` role and for non-active contracts
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

- [x] 9. Checkpoint — Ensure full project compiles
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 10. Integration tests
  - [x] 10.1 Create `backend/tests/pago_generacion_tests.rs`
    - Test: create active contrato → verify pagos generated with estado "pendiente" and correct montos/fechas
    - Test: create non-active contrato → verify no pagos generated
    - Test: renovar contrato → verify pagos generated for new period with new monto
    - Test: terminar contrato → verify pending future pagos cancelled, pagado/atrasado pagos unchanged
    - Test: GET preview → verify response with pago list, totals, existing/new counts
    - Test: GET preview with non-existent contrato → 404
    - Test: GET preview does not create records in DB
    - Test: POST generar with active contrato → verify pagos created
    - Test: POST generar with non-active contrato → 422
    - Test: POST generar with non-existent contrato → 404
    - Test: POST generar as visualizador → 403
    - Test: POST generar with invalid dia_vencimiento → 422
    - Test: POST generar with existing pagos → verify deduplication (only missing months generated)
    - Test: verify auditoría entries for auto generation, manual generation, and cancellation
    - _Requirements: 1.1–1.8, 2.1–2.4, 3.1–3.3, 4.1–4.5, 5.1–5.6, 6.1–6.4, 7.1–7.3, 8.1–8.3_

- [x] 11. Property-based tests
  - [x] 11.1 Write property test: Month count correctness
    - **Property 1: Month count correctness**
    - **Validates: Requirements 1.1, 1.4, 1.5, 1.6**
    - Generate random valid date ranges `(fecha_inicio, fecha_fin)` where `fecha_inicio <= fecha_fin`, call `calcular_pagos`, verify the count of `PagoGenerado` items equals the number of distinct (year, month) pairs in the range. When both dates fall in the same month, verify exactly one item.

  - [x] 11.2 Write property test: Generated pago fields match contract
    - **Property 2: Generated pago fields match contract**
    - **Validates: Requirements 1.2**
    - Generate random `monto_mensual` (Decimal > 0) and `moneda` in `{"DOP", "USD"}`, call `calcular_pagos`, verify every `PagoGenerado.monto == monto_mensual` and every `PagoGenerado.moneda == moneda`.

  - [x] 11.3 Write property test: Date calculation with day clamping
    - **Property 3: Date calculation with day clamping**
    - **Validates: Requirements 1.3, 6.1, 6.2**
    - Generate random date ranges and `dia_vencimiento` in `1..=31`, call `calcular_pagos`, verify every `PagoGenerado.fecha_vencimiento.day() == min(dia_vencimiento, last_day_of_that_month)`. When `dia_vencimiento` defaults to 1, verify every day is 1.

  - [x] 11.4 Write property test: Cancellation only affects correct pagos
    - **Property 4: Cancellation only affects correct pagos**
    - **Validates: Requirements 3.1, 3.2, 3.3**
    - Generate sets of pagos with varied estados and fechas, apply `cancelar_pagos_futuros`, verify: (a) pendiente + fecha_vencimiento > fecha_terminacion → cancelado, (b) pagado/atrasado → unchanged, (c) pendiente + fecha_vencimiento <= fecha_terminacion → unchanged.

  - [x] 11.5 Write property test: Preview totals are consistent
    - **Property 5: Preview totals are consistent**
    - **Validates: Requirements 4.4, 4.5**
    - Generate random contract data, build `PreviewPagosResponse`, verify `total_pagos == len(pagos)`, `monto_total == sum(montos)`, and `pagos_existentes + pagos_nuevos == total_pagos`.

  - [x] 11.6 Write property test: Deduplication filters by year-month
    - **Property 6: Deduplication filters by year-month**
    - **Validates: Requirements 5.2, 7.1, 7.2, 7.3**
    - Generate random `PagoGenerado` lists and existing `fecha_vencimiento` dates, call `filtrar_existentes`, verify returned pagos have no (year, month) overlap with existing dates, and `filtered_count + matched_count == original_count`.

  - [x] 11.7 Write property test: Invalid dia_vencimiento is rejected
    - **Property 7: Invalid dia_vencimiento is rejected**
    - **Validates: Requirements 6.4**
    - Generate `u32` values where `value < 1` or `value > 31`, call `validar_dia_vencimiento`, verify each returns a validation error.

- [x] 12. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate the 7 universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- The implementation language is Rust, matching the existing codebase and design document
- No new database migrations are needed — all pagos use the existing `pagos` table
- Backend `Decimal` serializes as JSON strings; frontend `f64` fields must use `deserialize_f64_from_any`
