# Implementation Plan: Seguimiento de Depósitos de Garantía

## Overview

Agregar seguimiento del ciclo de vida de depósitos de garantía a los contratos existentes. Se agregan cinco columnas a la tabla `contratos`, se actualiza la entidad SeaORM, se extienden los modelos request/response, se agrega lógica de máquina de estados al servicio de contratos, se crea un endpoint dedicado `PUT /contratos/{id}/deposito`, y se modifica la UI del detalle del contrato para mostrar una sección de depósito. Se sigue el patrón exacto de la arquitectura existente (handlers → services → entities).

## Tasks

- [x] 1. Database migration
  - [x] 1.1 Create migration `m20250615_000001_add_deposit_tracking_to_contratos`
    - Create file `backend/migrations/m20250615_000001_add_deposit_tracking_to_contratos.rs`
    - Add five columns to `contratos` table: `estado_deposito` (VARCHAR(20) NULLABLE), `fecha_cobro_deposito` (TIMESTAMP WITH TIME ZONE NULLABLE), `fecha_devolucion_deposito` (TIMESTAMP WITH TIME ZONE NULLABLE), `monto_retenido` (DECIMAL(12,2) NULLABLE), `motivo_retencion` (TEXT NULLABLE)
    - Execute UPDATE to set `estado_deposito = 'pendiente'` for all existing contratos where `deposito IS NOT NULL AND deposito > 0`
    - Follow the exact pattern from existing migrations for ALTER TABLE and column additions
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [x] 1.2 Register migration in `backend/migrations/mod.rs`
    - Add `pub mod` declaration for the new migration module
    - Add migration to the `Migrator::migrations()` vec in order
    - _Requirements: 1.1_

- [x] 2. SeaORM entity update
  - [x] 2.1 Add deposit tracking fields to `backend/src/entities/contrato.rs`
    - Add five fields to `Model` struct: `estado_deposito: Option<String>`, `fecha_cobro_deposito: Option<DateTimeWithTimeZone>`, `fecha_devolucion_deposito: Option<DateTimeWithTimeZone>`, `monto_retenido: Option<Decimal>` (with `#[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]`), `motivo_retencion: Option<String>`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 3. Request/response model updates
  - [x] 3.1 Add `CambiarEstadoDepositoRequest` to `backend/src/models/contrato.rs`
    - Define struct with `#[derive(Debug, Deserialize)]` and `#[serde(rename_all = "camelCase")]`
    - Fields: `estado: String`, `monto_retenido: Option<Decimal>`, `motivo_retencion: Option<String>`
    - _Requirements: 4.1_

  - [x] 3.2 Add deposit fields to `ContratoResponse` in `backend/src/models/contrato.rs`
    - Add five fields: `estado_deposito: Option<String>`, `fecha_cobro_deposito: Option<DateTime<Utc>>`, `fecha_devolucion_deposito: Option<DateTime<Utc>>`, `monto_retenido: Option<Decimal>`, `motivo_retencion: Option<String>`
    - _Requirements: 4.4_

  - [x] 3.3 Update `From<contrato::Model> for ContratoResponse` in `backend/src/services/contratos.rs`
    - Map the five new fields from the entity Model to the ContratoResponse
    - _Requirements: 4.4_

  - [x] 3.4 Add unit tests for new model serialization/deserialization
    - Test `CambiarEstadoDepositoRequest` deserialization with camelCase JSON
    - Test `CambiarEstadoDepositoRequest` deserialization with retention fields
    - Test `ContratoResponse` serialization includes new deposit fields
    - _Requirements: 4.1, 4.4_

- [x] 4. Service layer — deposit state machine logic
  - [x] 4.1 Add deposit state constants and validation function to `backend/src/services/contratos.rs`
    - Define constant: `ESTADOS_DEPOSITO: &[&str] = &["pendiente", "cobrado", "devuelto", "retenido"]`
    - Implement `validar_transicion_deposito(estado_actual: &str, nuevo_estado: &str) -> Result<(), AppError>` as a pure function
    - Valid transitions: pendiente→cobrado, cobrado→devuelto, cobrado→retenido
    - Invalid transitions return specific Spanish error messages
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 4.2 Implement `cambiar_estado_deposito` service function in `backend/src/services/contratos.rs`
    - Find contrato by ID (404 if not found)
    - Validate contrato has deposito > 0 (422 if not)
    - Validate estado enum with `validate_enum`
    - Call `validar_transicion_deposito`
    - For `retenido`: validate monto_retenido present, > 0, <= deposito; validate motivo_retencion present and non-empty
    - Open transaction, update fields based on new estado:
      - `cobrado`: set `fecha_cobro_deposito = now()`
      - `devuelto`: set `fecha_devolucion_deposito = now()`
      - `retenido`: set `fecha_devolucion_deposito = now()`, `monto_retenido`, `motivo_retencion`
    - Register auditoría with action "cambiar_estado_deposito"
    - Commit and return ContratoResponse
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 5.1_

  - [x] 4.3 Modify `contratos::create` to set default deposit status
    - After creating the contrato, if `deposito` is `Some(d)` where `d > 0`, set `estado_deposito = "pendiente"`
    - If `deposito` is `None` or `Some(0)`, leave `estado_deposito` as `None`
    - _Requirements: 1.6, 1.7_

  - [x] 4.4 Write unit tests for `validar_transicion_deposito`
    - Add tests in `#[cfg(test)]` module in `backend/src/services/contratos.rs`
    - Test all valid transitions: pendiente→cobrado (OK), cobrado→devuelto (OK), cobrado→retenido (OK)
    - Test all invalid transitions: pendiente→devuelto (Err), pendiente→retenido (Err), cobrado→pendiente (Err), devuelto→pendiente (Err), devuelto→cobrado (Err), devuelto→retenido (Err), retenido→pendiente (Err), retenido→cobrado (Err), retenido→devuelto (Err)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 5. Handler and route
  - [x] 5.1 Add `cambiar_estado_deposito` handler to `backend/src/handlers/contratos.rs`
    - Implement handler with `WriteAccess` extractor, `Path<Uuid>`, and `Json<CambiarEstadoDepositoRequest>`
    - Delegate to `contratos::cambiar_estado_deposito` service function
    - Return `HttpResponse::Ok().json(result)`
    - Follow the exact pattern from existing handlers (e.g., `terminar`)
    - _Requirements: 4.1, 2.7_

  - [x] 5.2 Add route to `backend/src/routes.rs`
    - Add `"/{id}/deposito"` route with `web::put().to(handlers::contratos::cambiar_estado_deposito)` inside the `/contratos` scope
    - Place before the `/{id}` catch-all routes to avoid conflicts
    - _Requirements: 4.1_

- [x] 6. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 7. Frontend type updates
  - [x] 7.1 Add deposit fields to `Contrato` type in `frontend/src/types/contrato.rs`
    - Add fields: `estado_deposito: Option<String>`, `fecha_cobro_deposito: Option<String>`, `fecha_devolucion_deposito: Option<String>`, `monto_retenido: Option<f64>` (with `#[serde(default, deserialize_with = "deserialize_f64_from_any")]` if needed), `motivo_retencion: Option<String>`
    - _Requirements: 6.1_

  - [x] 7.2 Add `CambiarEstadoDeposito` struct to `frontend/src/types/contrato.rs`
    - Define struct with `#[derive(Serialize)]` and `#[serde(rename_all = "camelCase")]`
    - Fields: `estado: String`, `monto_retenido: Option<f64>`, `motivo_retencion: Option<String>`
    - _Requirements: 6.1_

- [x] 8. Frontend deposit section
  - [x] 8.1 Add `DepositoSection` component to `frontend/src/pages/contratos.rs`
    - Create a `DepositoSection` functional component that receives the contrato data and user role
    - Show section "Depósito de Garantía" only when contrato has deposito > 0
    - Display: deposit amount with currency, estado badge (pendiente=amarillo/warning, cobrado=azul/info, devuelto=verde/success, retenido=rojo/error), fecha_cobro_deposito, fecha_devolucion_deposito in DD/MM/YYYY format
    - When estado_deposito == "retenido": show monto_retenido, calculated monto devuelto (deposito - monto_retenido), and motivo_retencion
    - Action buttons based on current state: pendiente → "Marcar como Cobrado", cobrado → "Devolver Depósito" + "Retener Depósito", devuelto/retenido → no buttons
    - Hide action buttons for visualizador role using `can_write()`
    - All text in Spanish
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [x] 8.2 Add retention modal to `frontend/src/pages/contratos.rs`
    - Create a modal component for retention that shows when "Retener Depósito" is clicked
    - Fields: monto_retenido (number input), motivo_retencion (textarea)
    - Client-side validation: monto_retenido required and > 0, motivo_retencion required
    - On confirm: call `api_put` to `/contratos/{id}/deposito` with estado="retenido", monto_retenido, motivo_retencion
    - On success: show toast, reload contrato data
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 6.3_

  - [x] 8.3 Integrate `DepositoSection` into contrato detail/edit view
    - Render `DepositoSection` in the contrato editing view or detail section
    - Wire up API calls for "Marcar como Cobrado" and "Devolver Depósito" buttons (direct PUT to `/contratos/{id}/deposito`)
    - Show toast on success, reload data on state change
    - _Requirements: 6.1, 6.4_

- [x] 9. Checkpoint — Ensure full project compiles
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 10. Integration tests
  - [x] 10.1 Create `backend/tests/deposit_tracking_tests.rs`
    - Test create contrato with deposito > 0 → estado_deposito = "pendiente"
    - Test create contrato without deposito → estado_deposito = null
    - Test full flow: pendiente → cobrado → devuelto, verify timestamps set
    - Test full flow: pendiente → cobrado → retenido with monto and motivo, verify all fields
    - Test invalid transitions return 422 with specific messages
    - Test retention without monto_retenido → 422
    - Test retention with monto_retenido <= 0 → 422
    - Test retention with monto_retenido > deposito → 422
    - Test retention without motivo_retencion → 422
    - Test change estado on contrato without deposito → 422
    - Test visualizador attempts change → 403
    - Test non-existent contrato → 404
    - Test invalid estado enum value → 422
    - Test deposit fields present in GET /contratos/{id} response
    - Test auditoría entries created for each estado change
    - _Requirements: 1.6, 1.7, 2.1–2.7, 3.1–3.4, 4.1–4.4, 5.1_

- [x] 11. Property-based tests
  - [x] 11.1 Write property test: Deposit status defaults correctly on creation
    - **Property 1: Deposit status defaults correctly on creation**
    - **Validates: Requirements 1.6, 1.7**
    - Generate random deposit amounts (Some(positive), Some(0), None), create contrato, verify estado_deposito is "pendiente" when deposito > 0 and None otherwise

  - [x] 11.2 Write property test: Valid deposit state transitions set timestamps
    - **Property 2: Valid deposit state transitions set timestamps**
    - **Validates: Requirements 2.1, 2.2, 2.3**
    - Create contrato with deposit, transition pendiente→cobrado verify fecha_cobro_deposito set; transition cobrado→devuelto verify fecha_devolucion_deposito set; transition cobrado→retenido verify fecha_devolucion_deposito set

  - [x] 11.3 Write property test: Invalid deposit state transitions are rejected
    - **Property 3: Invalid deposit state transitions are rejected**
    - **Validates: Requirements 2.4, 2.5**
    - Generate all invalid (estado_actual, nuevo_estado) combinations for deposit, verify each returns validation error

  - [x] 11.4 Write property test: Retention requires valid monto and motivo
    - **Property 4: Retention requires valid monto and motivo**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
    - Generate random deposit amounts and retention amounts, verify: valid when 0 < monto_retenido <= deposito with non-empty motivo; invalid otherwise

  - [x] 11.5 Write property test: Deposit status change round-trip preserves data
    - **Property 5: Deposit status change round-trip preserves data**
    - **Validates: Requirements 4.1, 4.4**
    - Apply valid transitions, retrieve contrato, verify all deposit fields match expected values

  - [x] 11.6 Write property test: Invalid deposit estado enum values are rejected
    - **Property 6: Invalid deposit estado enum values are rejected**
    - **Validates: Requirements 4.3**
    - Generate random strings not in valid estado_deposito set, verify rejection

  - [x] 11.7 Write property test: Deposit operations on contracts without deposit are rejected
    - **Property 7: Deposit operations on contracts without deposit are rejected**
    - **Validates: Requirements 2.6**
    - Create contratos without deposit (None or 0), attempt estado change, verify rejection

- [x] 12. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

## Notes

- This is a lightweight feature — no new tables, just five new columns on `contratos`
- The deposit state machine is simpler than the maintenance request state machine (4 states, 3 valid transitions)
- The `validar_transicion_deposito` function is pure and easily testable
- Frontend changes are contained to the contratos detail view — no new pages or routes
- Existing contrato data is backfilled in the migration (deposito > 0 → estado_deposito = "pendiente")
- All monetary fields use DECIMAL(12,2) matching the existing pattern
- Property tests validate the state machine logic and validation rules exhaustively
