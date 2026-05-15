# Plan de Implementación: Recargos por Mora (Late Fees)

## Overview

Implementar recargos por mora en el sistema de gestión inmobiliaria. Se agregan campos a las tablas `contratos` y `pagos`, un nuevo servicio de cálculo de recargos, configuración a nivel de organización, modificaciones a `mark_overdue` para considerar período de gracia y calcular recargos, y cambios en la interfaz para mostrar y configurar recargos. Se sigue la arquitectura existente (handlers → services → entities).

## Tasks

- [x] 1. Database migration
  - [x] 1.1 Create migration `m20250615_000001_add_recargo_fields`
    - Create file `backend/migrations/m20250615_000001_add_recargo_fields.rs`
    - ALTER TABLE `contratos` ADD COLUMN `recargo_porcentaje` DECIMAL(5,2) NULL
    - ALTER TABLE `contratos` ADD COLUMN `dias_gracia` INTEGER NULL
    - ALTER TABLE `pagos` ADD COLUMN `recargo` DECIMAL(12,2) NULL
    - Follow the pattern from existing migrations for ALTER TABLE
    - _Requirements: 1.1, 1.2, 2.1_

  - [x] 1.2 Register migration in `backend/migrations/mod.rs`
    - Add `pub mod` declaration for the new migration module
    - Add migration to the `Migrator::migrations()` vec in order
    - _Requirements: 1.1, 1.2, 2.1_

- [x] 2. Entity changes
  - [x] 2.1 Add fields to `backend/src/entities/contrato.rs`
    - Add `recargo_porcentaje: Option<Decimal>` with `#[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]`
    - Add `dias_gracia: Option<i32>`
    - _Requirements: 1.1, 1.2_

  - [x] 2.2 Add field to `backend/src/entities/pago.rs`
    - Add `recargo: Option<Decimal>` with `#[sea_orm(column_type = "Decimal(Some((12, 2)))", nullable)]`
    - _Requirements: 2.1_

- [x] 3. Model (DTO) changes
  - [x] 3.1 Update `backend/src/models/contrato.rs`
    - Add `recargo_porcentaje: Option<Decimal>` and `dias_gracia: Option<i32>` to `CreateContratoRequest`
    - Add `recargo_porcentaje: Option<Decimal>` and `dias_gracia: Option<i32>` to `UpdateContratoRequest`
    - Add `recargo_porcentaje: Option<Decimal>` and `dias_gracia: Option<i32>` to `ContratoResponse`
    - Update `From<contrato::Model>` for `ContratoResponse` in `services/contratos.rs` to include new fields
    - Update existing unit tests to include new fields
    - _Requirements: 1.7, 8.1, 8.2_

  - [x] 3.2 Update `backend/src/models/pago.rs`
    - Add `recargo: Option<Decimal>` to `PagoResponse`
    - Update `From<pago::Model>` for `PagoResponse` in `services/pagos.rs` to include new field
    - _Requirements: 2.2, 2.3_

  - [x] 3.3 Add recargo config models to `backend/src/services/configuracion.rs`
    - Add `RecargoDefectoResponse` struct with `porcentaje: Option<Decimal>`
    - Add `UpdateRecargoDefectoRequest` struct with `porcentaje: Decimal`
    - _Requirements: 3.1, 3.3_

- [x] 4. Recargos service (new)
  - [x] 4.1 Create `backend/src/services/recargos.rs`
    - Implement `calcular_recargo(monto: Decimal, porcentaje: Decimal) -> Decimal` as a pure function: `(monto * porcentaje / Decimal::from(100)).round_dp(2)`
    - Implement `resolver_porcentaje_recargo(db, contrato) -> Result<Option<Decimal>, AppError>`: check contrato.recargo_porcentaje first, then fall back to configuracion with clave `recargo_porcentaje_defecto`, return None if neither exists
    - Implement `aplicar_recargo(db, pago_id, contrato) -> Result<Option<Decimal>, AppError>`: resolve porcentaje, calculate recargo if Some, update pago.recargo field, return the calculated value
    - _Requirements: 4.1, 4.2, 4.3, 5.1, 5.3, 5.4, 5.5_

  - [x] 4.2 Register service module in `backend/src/services/mod.rs`
    - Add `pub mod recargos;`
    - _Requirements: 4.1_

  - [x] 4.3 Write unit tests for `calcular_recargo`
    - Add `#[cfg(test)]` module in `backend/src/services/recargos.rs`
    - Test: 1000 * 5% = 50.00
    - Test: 1500.50 * 10% = 150.05
    - Test: any monto * 0% = 0.00
    - Test: any monto * 100% = monto
    - Test: rounding to 2 decimal places (e.g., 333.33 * 3.33% = 11.10)
    - _Requirements: 5.1, 5.3, 5.5_

- [x] 5. Configuracion service changes
  - [x] 5.1 Add recargo config functions to `backend/src/services/configuracion.rs`
    - Implement `obtener_recargo_defecto(db) -> Result<Option<Decimal>, AppError>`: read from configuracion with clave `recargo_porcentaje_defecto`, return None if not found
    - Implement `actualizar_recargo_defecto(db, porcentaje, updated_by) -> Result<Decimal, AppError>`: validate 0 <= porcentaje <= 100, upsert in configuracion, register auditoría, return saved value
    - Add unit tests for serialization roundtrip of recargo config
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 10.1_

- [x] 6. Contratos service changes
  - [x] 6.1 Update `backend/src/services/contratos.rs` — create function
    - Add validation: if `recargo_porcentaje` is Some, must be >= 0 and <= 100
    - Add validation: if `dias_gracia` is Some, must be >= 0
    - Set `recargo_porcentaje` and `dias_gracia` on the ActiveModel from input
    - _Requirements: 1.3, 1.4, 1.5, 1.6_

  - [x] 6.2 Update `backend/src/services/contratos.rs` — update function
    - Add validation: if `recargo_porcentaje` is Some, must be >= 0 and <= 100
    - Add validation: if `dias_gracia` is Some, must be >= 0
    - Set `recargo_porcentaje` and `dias_gracia` on the ActiveModel if provided
    - _Requirements: 1.5, 1.6_

  - [x] 6.3 Update `From<contrato::Model> for ContratoResponse` in contratos.rs
    - Add `recargo_porcentaje: m.recargo_porcentaje` and `dias_gracia: m.dias_gracia`
    - _Requirements: 1.7_

- [x] 7. Pagos service changes
  - [x] 7.1 Update `mark_overdue` in `backend/src/services/pagos.rs`
    - Change from bulk `update_many` to a query that JOINs with contratos to get `dias_gracia`
    - For each pago pendiente: consider it overdue only if `today > fecha_vencimiento + dias_gracia` (or `today > fecha_vencimiento` if dias_gracia is NULL)
    - After marking as atrasado, call `recargos::aplicar_recargo` for each affected pago
    - Register auditoría entry with count of pagos affected and recargos calculated
    - _Requirements: 5.1, 6.1, 6.2, 6.3, 10.2_

  - [x] 7.2 Update `update` in `backend/src/services/pagos.rs`
    - When `input.estado == Some("atrasado")` and existing estado is not "atrasado": look up the contrato for this pago, call `recargos::aplicar_recargo`
    - When estado changes from "atrasado" to another estado: set `pago.recargo = None`
    - _Requirements: 5.2_

  - [x] 7.3 Update `From<pago::Model> for PagoResponse` in pagos.rs
    - Add `recargo: m.recargo`
    - _Requirements: 2.2_

- [x] 8. Handler changes
  - [x] 8.1 Add recargo config handlers to `backend/src/handlers/configuracion.rs`
    - Implement `obtener_recargo_defecto(db, _claims)` → calls `configuracion::obtener_recargo_defecto`, returns Ok(json)
    - Implement `actualizar_recargo_defecto(db, admin, body)` → calls `configuracion::actualizar_recargo_defecto`, returns Ok(json)
    - _Requirements: 3.1, 3.3_

  - [x] 8.2 Add routes to `backend/src/routes.rs`
    - Add `.route("/recargo", web::get().to(handlers::configuracion::obtener_recargo_defecto))` to `/configuracion` scope
    - Add `.route("/recargo", web::put().to(handlers::configuracion::actualizar_recargo_defecto))` to `/configuracion` scope
    - _Requirements: 3.1, 3.3_

- [x] 9. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 10. Frontend type changes
  - [x] 10.1 Update `frontend/src/types/pago.rs`
    - Add `recargo: Option<f64>` to `Pago` struct with `#[serde(default, deserialize_with = "deserialize_option_f64_from_any")]`
    - _Requirements: 7.1_

  - [x] 10.2 Update `frontend/src/types/contrato.rs`
    - Add `recargo_porcentaje: Option<f64>` to `Contrato` struct with appropriate deserializer
    - Add `dias_gracia: Option<i32>` to `Contrato` struct
    - Add `recargo_porcentaje: Option<f64>` and `dias_gracia: Option<i32>` to `CreateContrato` and `UpdateContrato`
    - _Requirements: 8.1, 8.2_

- [x] 11. Frontend pagos page changes
  - [x] 11.1 Update `frontend/src/pages/pagos.rs`
    - In the payment list table, add a "Recargo" column that shows the recargo amount when present (formatted with currency)
    - In the payment detail view, show "Recargo" and "Monto Total" (monto + recargo) when recargo is present
    - When recargo is None/NULL, omit the recargo display
    - All text in Spanish: "Recargo", "Monto Total"
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 12. Frontend contratos page changes
  - [x] 12.1 Update `frontend/src/pages/contratos.rs`
    - Add "Porcentaje de Recargo (%)" and "Días de Gracia" fields to the create/edit form
    - Show these fields in the contract detail view
    - When recargo_porcentaje is None, show "(usa valor por defecto de la organización)" hint text
    - All text in Spanish
    - _Requirements: 8.1, 8.2, 8.3_

- [x] 13. Frontend configuracion page changes
  - [x] 13.1 Update the configuracion page/component
    - Add a "Recargo por Mora" section showing the current default recargo percentage
    - Add an input field to update the default percentage, with save button
    - Only editable by admin role; disabled for other roles
    - Show confirmation message on successful save
    - All text in Spanish
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [x] 14. Checkpoint — Ensure full project compiles
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

- [x] 15. Integration tests
  - [x] 15.1 Create `backend/tests/late_fees_tests.rs`
    - Test: Create contrato with recargo_porcentaje and dias_gracia → fields stored and returned
    - Test: Create contrato without recargo fields → NULL values returned
    - Test: Update contrato recargo_porcentaje and dias_gracia → fields updated
    - Test: Create contrato with recargo_porcentaje < 0 → 422
    - Test: Create contrato with recargo_porcentaje > 100 → 422
    - Test: Create contrato with dias_gracia < 0 → 422
    - Test: GET /configuracion/recargo when not set → NULL
    - Test: PUT /configuracion/recargo with valid value → stored and returned
    - Test: PUT /configuracion/recargo with invalid value → 422
    - Test: PUT /configuracion/recargo as non-admin → 403
    - Test: mark_overdue with dias_gracia → respects grace period
    - Test: mark_overdue calculates recargo using contrato porcentaje
    - Test: mark_overdue calculates recargo using org default when contrato is NULL
    - Test: mark_overdue with both NULL → recargo stays NULL
    - Test: Manual update estado to "atrasado" → recargo calculated
    - Test: Update estado from "atrasado" to "pagado" → recargo cleared to NULL
    - Test: Pago response includes recargo field
    - _Requirements: 1.1–1.7, 2.1–2.3, 3.1–3.4, 4.1–4.3, 5.1–5.5, 6.1–6.3, 10.1, 10.2_

- [x] 16. Property-based tests
  - [x] 16.1 Write property test: Cálculo de recargo es correcto (P1)
    - **Property 1: Cálculo de recargo es correcto**
    - **Validates: Requirements 5.1, 5.3**
    - Generate random monto (0.01..999999.99) and porcentaje (0.00..100.00), verify `calcular_recargo(monto, porcentaje) == (monto * porcentaje / 100).round_dp(2)`
    - Pure function test, no database needed

  - [x] 16.2 Write property test: Round-trip de campos de contrato (P2)
    - **Property 2: Round-trip de campos de contrato**
    - **Validates: Requirements 1.1, 1.2, 1.7**
    - Generate random valid recargo_porcentaje (0..100) and dias_gracia (0..365), create contrato, retrieve by ID, verify fields match

  - [x] 16.3 Write property test: Resolución contrato tiene prioridad (P3)
    - **Property 3: Resolución de porcentaje — contrato tiene prioridad**
    - **Validates: Requirements 4.1**
    - Generate contrato with recargo_porcentaje set + org default set, verify resolver returns contrato value

  - [x] 16.4 Write property test: Resolución fallback a organización (P4)
    - **Property 4: Resolución de porcentaje — fallback a organización**
    - **Validates: Requirements 4.2**
    - Generate contrato with recargo_porcentaje NULL + org default set, verify resolver returns org value

  - [x] 16.5 Write property test: Resolución ambos NULL produce None (P5)
    - **Property 5: Resolución de porcentaje — ambos NULL produce None**
    - **Validates: Requirements 4.3, 5.4**
    - Generate contrato with recargo_porcentaje NULL + no org default, verify resolver returns None

  - [x] 16.6 Write property test: Validación de rango de porcentaje (P6)
    - **Property 6: Validación de rango de recargo_porcentaje**
    - **Validates: Requirements 1.5, 3.2**
    - Generate values outside [0, 100], verify rejection on contrato create/update and config update

  - [x] 16.7 Write property test: Validación de dias_gracia no negativo (P7)
    - **Property 7: Validación de dias_gracia no negativo**
    - **Validates: Requirements 1.6**
    - Generate negative integers, verify rejection on contrato create/update

  - [x] 16.8 Write property test: Período de gracia retrasa atraso (P8)
    - **Property 8: Período de gracia retrasa el marcado de atraso**
    - **Validates: Requirements 6.1, 6.2, 6.3**
    - Generate pagos with various dias_gracia values, verify mark_overdue respects grace period

  - [x] 16.9 Write property test: Recargo se calcula al marcar atrasado (P9)
    - **Property 9: Recargo se calcula al marcar como atrasado**
    - **Validates: Requirements 5.1, 5.2**
    - Generate pagos marked as atrasado with known porcentaje, verify recargo matches formula

  - [x] 16.10 Write property test: Recargo con porcentaje 0 produce 0.00 (P10)
    - **Property 10: Recargo con porcentaje 0 produce 0.00**
    - **Validates: Requirements 5.5**
    - Generate pagos with porcentaje 0, verify recargo is exactly 0.00 (not NULL)

- [x] 17. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass. Ask the user if questions arise.

## Notes

- The migration adds columns to existing tables — no new tables are created.
- The `mark_overdue` function changes from a bulk `update_many` to a per-pago approach to support grace period and recargo calculation. This is acceptable because `mark_overdue` runs as a background/scheduled task, not in a hot path.
- The `calcular_recargo` function is pure and can be thoroughly tested with property-based tests without database setup.
- `proptest` is already in dev-dependencies from the mantenimiento feature.
- Frontend `Decimal` fields use `deserialize_f64_from_any` / `deserialize_option_f64_from_any` per the lessons-learned rule about `rust_decimal` JSON strings.
- All UI text is in Spanish per project conventions.
