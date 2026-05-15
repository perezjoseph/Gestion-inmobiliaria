# Tasks: Gastos (Expenses Tracking)

## Task 1: Database Migration and Entity

- [x] 1.1 Create migration file `backend/migrations/m20250412_000001_create_gastos.rs` with `gastos` table schema: UUID primary key, `propiedad_id` UUID NOT NULL FK to `propiedades(id)`, `unidad_id` UUID nullable FK to `unidades(id)`, `categoria` VARCHAR(30) NOT NULL, `descripcion` VARCHAR(500) NOT NULL, `monto` DECIMAL(12,2) NOT NULL, `moneda` VARCHAR(3) NOT NULL DEFAULT 'DOP', `fecha_gasto` DATE NOT NULL, `estado` VARCHAR(20) NOT NULL DEFAULT 'pendiente', `proveedor` VARCHAR(200), `numero_factura` VARCHAR(100), `notas` TEXT, `created_at` and `updated_at` TIMESTAMP WITH TIME ZONE. Add indexes on `propiedad_id`, `unidad_id`, `categoria`, `estado`, `fecha_gasto`. Follow the pattern in `m20250408_000005_create_pagos.rs`.
- [x] 1.2 Register the new migration in `backend/migrations/mod.rs`.
- [x] 1.3 Create SeaORM entity `backend/src/entities/gasto.rs` with `Model`, `Relation` (belongs_to Propiedad, belongs_to Unidad), and `ActiveModelBehavior`. Follow the pattern in `entities/pago.rs`.
- [x] 1.4 Add `pub mod gasto;` to `backend/src/entities/mod.rs`.

## Task 2: DTOs (Request/Response Models)

- [x] 2.1 Create `backend/src/models/gasto.rs` with `CreateGastoRequest`, `UpdateGastoRequest`, `GastoListQuery`, `ResumenCategoriasQuery`, `GastoResponse`, and `ResumenCategoriaRow` structs. Use `#[serde(rename_all = "camelCase")]` on all. Add `#[cfg(test)]` module with serde deserialization/serialization tests. Follow the pattern in `models/pago.rs`.
- [x] 2.2 Add `pub mod gasto;` to `backend/src/models/mod.rs`.
- [x] 2.3 Add `RentabilidadReportQuery`, `RentabilidadReportRow`, and `RentabilidadReportSummary` structs to `backend/src/models/reporte.rs`. Use `#[serde(rename_all = "camelCase")]` and add serde tests.
- [x] 2.4 Add `GastosComparacion` struct to `backend/src/models/dashboard.rs` with `mes_actual: Decimal`, `mes_anterior: Decimal`, `porcentaje_cambio: f64`. Use `#[serde(rename_all = "camelCase")]`.

## Task 3: Gastos Service (CRUD + Validation)

- [x] 3.1 Create `backend/src/services/gastos.rs` with validation constants (`CATEGORIAS_GASTO`, `ESTADOS_GASTO`, `MONEDAS`), `From<gasto::Model> for GastoResponse` impl, and the `create` function: validate `categoria`, `moneda`, `monto > 0`, verify `propiedad_id` exists, verify `unidad_id` belongs to propiedad if provided, insert record with UUID and `estado = "pendiente"`, record audit trail. Follow the pattern in `services/pagos.rs`.
- [x] 3.2 Add `get_by_id` and `list` functions to `services/gastos.rs`. `list` supports pagination and filtering by `propiedad_id`, `unidad_id`, `categoria`, `estado`, `fecha_desde`, `fecha_hasta`. Follow the pagos list pattern.
- [x] 3.3 Add `update` function to `services/gastos.rs`: validate enum fields if provided, validate `monto > 0` if provided, validate `unidad_id` belongs to propiedad if changed, partial update of only provided fields, record audit trail.
- [x] 3.4 Add `delete` function to `services/gastos.rs`: delete by ID, return 404 if not found, record audit trail.
- [x] 3.5 Add `resumen_categorias` function to `services/gastos.rs`: query gastos for a propiedad filtered by `estado = "pagado"` and optional date range, group by `categoria`, sum `monto`, count records, return sorted by total descending.
- [x] 3.6 Add `pub mod gastos;` to `backend/src/services/mod.rs`.
- [x] 3.7 Add unit tests in `services/gastos.rs` `#[cfg(test)]` module: test `From<Model> for GastoResponse` conversion, test validation constants contain expected values.

## Task 4: Gastos Handlers and Routes

- [x] 4.1 Create `backend/src/handlers/gastos.rs` with `create` (WriteAccess, returns 201), `list` (Claims), `get_by_id` (Claims), `update` (WriteAccess), `delete` (WriteAccess), and `resumen_categorias` (Claims) handler functions. Use transactions for create/update/delete. Follow the pattern in `handlers/pagos.rs`.
- [x] 4.2 Add `pub mod gastos;` to `backend/src/handlers/mod.rs`.
- [x] 4.3 Register gastos routes in `backend/src/routes.rs`: add `/gastos` scope with CRUD routes and `/gastos/resumen-categorias` route. Place `resumen-categorias` route before `/{id}` to avoid path conflicts.

## Task 5: Profitability Report

- [x] 5.1 Add `generar_reporte_rentabilidad` function to `backend/src/services/reportes.rs`: query propiedades (optionally filtered by `propiedad_id`), for each property sum paid pagos as income and paid gastos as expenses for the given `mes`/`anio`, compute `ingreso_neto = total_ingresos - total_gastos`, return `RentabilidadReportSummary`. Use `tokio::try_join!` for concurrent income/expense queries.
- [x] 5.2 Add `exportar_rentabilidad_pdf` function to `backend/src/services/reportes.rs`: generate PDF using `genpdf` with property rows showing income, expenses, net income. Follow the pattern in `exportar_pdf`.
- [x] 5.3 Add `exportar_rentabilidad_xlsx` function to `backend/src/services/reportes.rs`: generate XLSX using `rust_xlsxwriter` with property rows. Follow the pattern in `exportar_xlsx`.
- [x] 5.4 Add profitability report handlers to `backend/src/handlers/reportes.rs`: `rentabilidad` (JSON), `rentabilidad_pdf`, `rentabilidad_xlsx`.
- [x] 5.5 Register profitability routes in `backend/src/routes.rs` under `/reportes` scope: `/rentabilidad`, `/rentabilidad/pdf`, `/rentabilidad/xlsx`.
- [x] 5.6 Add unit tests in `services/reportes.rs`: test `exportar_rentabilidad_pdf` and `exportar_rentabilidad_xlsx` produce valid bytes, test empty report handling.

## Task 6: Dashboard Integration

- [x] 6.1 Add `total_gastos_mes: Decimal` field to `DashboardStats` struct in `backend/src/services/dashboard.rs`. Update `get_stats` to query sum of `monto` from gastos where `estado = "pagado"` and `fecha_gasto` is in the current month.
- [x] 6.2 Add `gastos_comparacion` function to `backend/src/services/dashboard.rs`: query paid gastos totals for current and previous month, compute `porcentaje_cambio`. Handle zero previous month case.
- [x] 6.3 Add `gastos_comparacion` handler to `backend/src/handlers/dashboard.rs`.
- [x] 6.4 Register `/dashboard/gastos-comparacion` route in `backend/src/routes.rs`.
- [x] 6.5 Add unit tests in `services/dashboard.rs`: test percentage change calculation edge cases (zero previous, equal months, increase, decrease).

## Task 7: CSV Import for Gastos

- [x] 7.1 Add `importar_gastos` function to `backend/src/services/importacion.rs`: parse CSV rows, validate required columns (`propiedad_id`, `categoria`, `descripcion`, `monto`, `moneda`, `fecha_gasto`), support optional columns (`unidad_id`, `proveedor`, `numero_factura`, `notas`), call `gastos::create` for each valid row, collect errors with row numbers, return `ImportResult`. Return 422 if no valid rows.
- [x] 7.2 Add `importar_gastos` handler to `backend/src/handlers/importacion.rs` using `WriteAccess`. Follow the existing `importar_propiedades` handler pattern.
- [x] 7.3 Register `/importar/gastos` route in `backend/src/routes.rs`.
- [x] 7.4 Add unit tests in `services/importacion.rs`: test CSV parsing for gastos columns, test required field validation.

## Task 8: Property-Based Tests

- [x] 8.1 Create `backend/tests/gastos_pbt.rs` with property-based tests using `proptest`. Implement Property 6 (enum validation rejects invalid values): generate arbitrary strings, verify strings not in allowed sets are rejected by `validate_enum`. Tag: `// Feature: gastos-expenses-tracking, Property 6: Enum validation rejects invalid values`. Minimum 100 iterations.
- [x] 8.2 Add Property 12 (profitability net income invariant) to `gastos_pbt.rs`: generate random `Decimal` pairs for income/expenses, verify `ingreso_neto == total_ingresos - total_gastos`. Tag: `// Feature: gastos-expenses-tracking, Property 12: Profitability net income invariant`.
- [x] 8.3 Add Property 15 (percentage change calculation) to `gastos_pbt.rs`: generate random non-negative `Decimal` pairs, verify percentage change formula correctness including zero-denominator edge cases. Tag: `// Feature: gastos-expenses-tracking, Property 15: Percentage change calculation`.
- [x] 8.4 Add Property 11 (category summary sorted descending) to `gastos_pbt.rs`: generate random `Vec<ResumenCategoriaRow>`, sort by total descending, verify each row's total >= next row's total. Tag: `// Feature: gastos-expenses-tracking, Property 11: Category summary sorted by total descending`.
- [x] 8.5 Add Property 14 (CSV import row accounting) to `gastos_pbt.rs`: generate random `ImportResult` values, verify `exitosos + fallidos.len() == total_filas`. Tag: `// Feature: gastos-expenses-tracking, Property 14: CSV import valid/invalid row accounting`.

## Task 9: Integration Tests

- [x] 9.1 Create `backend/tests/gastos_tests.rs` with integration tests: full CRUD cycle (create 201, get 200, update 200, delete 204, get-after-delete 404), RBAC tests (visualizador gets 200 on list/get, 403 on create/update/delete), pagination test, filter tests (propiedad_id, categoria, date range).
- [x] 9.2 Add CSV import integration tests to `gastos_tests.rs`: valid CSV import, mixed valid/invalid rows, empty CSV returns 422.
- [x] 9.3 Add profitability report integration tests to `gastos_tests.rs`: JSON endpoint returns correct structure, PDF endpoint returns bytes, XLSX endpoint returns bytes.
- [x] 9.4 Add dashboard integration tests to `gastos_tests.rs`: `total_gastos_mes` present in stats, `gastos-comparacion` endpoint returns correct structure.

## Task 10: Final Verification

- [x] 10.1 Run `cargo test --workspace` and fix any compilation errors or test failures.
- [x] 10.2 Run `cargo clippy --workspace` and fix any warnings.
