# Requirements Document

## Introduction

This feature adds expense (gasto) tracking to the property management application. Currently the system tracks income via pagos (rent payments tied to contracts), but has no way to record property-related expenses such as maintenance costs, property taxes, insurance premiums, utilities, and other operational costs. Gastos will be tied to a propiedad and optionally to a specific unidad, categorized by type, and integrated into profitability reports so managers can see net income per property.

## Glossary

- **Sistema_Gastos**: The expense tracking subsystem responsible for creating, reading, updating, deleting, and reporting on gastos.
- **Gasto**: A single expense record representing a cost incurred for a property or unit (e.g., repair, tax payment, insurance premium).
- **Categoria_Gasto**: The classification of an expense. Valid values: `mantenimiento`, `impuestos`, `seguros`, `servicios_publicos`, `administracion`, `legal`, `mejoras`, `otro`.
- **Estado_Gasto**: The payment status of an expense. Valid values: `pendiente`, `pagado`, `cancelado`.
- **Propiedad**: An existing rental property entity in the system.
- **Unidad**: An existing unit entity belonging to a propiedad.
- **Reporte_Rentabilidad**: A profitability report that combines income (pagos) and expenses (gastos) to show net income per property.
- **Usuario_Autorizado**: A user with role `admin` or `gerente`.
- **Usuario_Visualizador**: A user with role `visualizador`.

## Requirements

### Requirement 1: Create Expense

**User Story:** As a property manager, I want to record expenses against my properties, so that I can track all costs associated with each property.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado submits a valid expense payload, THE Sistema_Gastos SHALL create a new Gasto record and return the created Gasto with HTTP status 201.
2. THE Sistema_Gastos SHALL require the following fields for Gasto creation: `propiedad_id`, `categoria`, `descripcion`, `monto`, `moneda`, and `fecha_gasto`.
3. THE Sistema_Gastos SHALL accept the following optional fields for Gasto creation: `unidad_id`, `proveedor`, `numero_factura`, `notas`.
4. WHEN a Gasto is created, THE Sistema_Gastos SHALL validate that `propiedad_id` references an existing Propiedad.
5. WHEN a Gasto is created with a `unidad_id`, THE Sistema_Gastos SHALL validate that the Unidad belongs to the specified Propiedad.
6. WHEN a Gasto is created, THE Sistema_Gastos SHALL validate that `categoria` is a valid Categoria_Gasto value.
7. WHEN a Gasto is created, THE Sistema_Gastos SHALL validate that `moneda` is either `DOP` or `USD`.
8. WHEN a Gasto is created, THE Sistema_Gastos SHALL validate that `monto` is greater than zero.
9. WHEN a Gasto is created, THE Sistema_Gastos SHALL assign a UUID primary key and set `estado` to `pendiente` by default.
10. WHEN a Gasto is created, THE Sistema_Gastos SHALL record an audit trail entry with the action and the creating user's identity.
11. IF a required field is missing or invalid, THEN THE Sistema_Gastos SHALL return HTTP status 422 with a descriptive error message in Spanish.

### Requirement 2: Read Expenses

**User Story:** As a property manager, I want to view and filter my property expenses, so that I can understand where money is being spent.

#### Acceptance Criteria

1. WHEN an authenticated user requests the expense list, THE Sistema_Gastos SHALL return a paginated list of Gasto records.
2. THE Sistema_Gastos SHALL support filtering the expense list by: `propiedad_id`, `unidad_id`, `categoria`, `estado`, `fecha_desde`, and `fecha_hasta`.
3. WHEN a `propiedad_id` filter is provided, THE Sistema_Gastos SHALL return only Gasto records associated with that Propiedad.
4. WHEN `fecha_desde` and `fecha_hasta` filters are provided, THE Sistema_Gastos SHALL return only Gasto records where `fecha_gasto` falls within the specified date range (inclusive).
5. WHEN an authenticated user requests a single Gasto by ID, THE Sistema_Gastos SHALL return the full Gasto record with HTTP status 200.
6. IF a Gasto with the requested ID does not exist, THEN THE Sistema_Gastos SHALL return HTTP status 404 with an error message in Spanish.
7. THE Sistema_Gastos SHALL allow Usuario_Visualizador to read Gasto records in read-only mode.

### Requirement 3: Update Expense

**User Story:** As a property manager, I want to update expense records, so that I can correct errors and mark expenses as paid.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado submits an update for an existing Gasto, THE Sistema_Gastos SHALL update only the provided fields and return the updated Gasto with HTTP status 200.
2. THE Sistema_Gastos SHALL allow updating the following fields: `categoria`, `descripcion`, `monto`, `moneda`, `fecha_gasto`, `unidad_id`, `proveedor`, `numero_factura`, `estado`, `notas`.
3. WHEN a Gasto update changes `unidad_id`, THE Sistema_Gastos SHALL validate that the new Unidad belongs to the Gasto's Propiedad.
4. WHEN a Gasto update changes `estado`, THE Sistema_Gastos SHALL validate that the new value is a valid Estado_Gasto.
5. WHEN a Gasto is updated, THE Sistema_Gastos SHALL record an audit trail entry with the action and the updating user's identity.
6. IF the Gasto with the requested ID does not exist, THEN THE Sistema_Gastos SHALL return HTTP status 404 with an error message in Spanish.
7. IF a Usuario_Visualizador attempts to update a Gasto, THEN THE Sistema_Gastos SHALL return HTTP status 403.

### Requirement 4: Delete Expense

**User Story:** As a property manager, I want to delete expense records that were entered by mistake, so that my financial data stays accurate.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado requests deletion of a Gasto, THE Sistema_Gastos SHALL delete the Gasto record and return HTTP status 204.
2. WHEN a Gasto is deleted, THE Sistema_Gastos SHALL record an audit trail entry with the action and the deleting user's identity.
3. IF the Gasto with the requested ID does not exist, THEN THE Sistema_Gastos SHALL return HTTP status 404 with an error message in Spanish.
4. IF a Usuario_Visualizador attempts to delete a Gasto, THEN THE Sistema_Gastos SHALL return HTTP status 403.

### Requirement 5: Expense Categories Summary

**User Story:** As a property manager, I want to see a breakdown of expenses by category for a property, so that I can identify the largest cost areas.

#### Acceptance Criteria

1. WHEN an authenticated user requests a category summary for a Propiedad, THE Sistema_Gastos SHALL return the total expense amount grouped by Categoria_Gasto.
2. THE Sistema_Gastos SHALL support filtering the category summary by `fecha_desde` and `fecha_hasta`.
3. THE Sistema_Gastos SHALL include only Gasto records with `estado` equal to `pagado` in the category summary totals.
4. THE Sistema_Gastos SHALL return the summary sorted by total amount in descending order.

### Requirement 6: Profitability Report

**User Story:** As a property manager, I want to see a profitability report that combines income and expenses per property, so that I can evaluate the financial performance of each property.

#### Acceptance Criteria

1. WHEN an authenticated user requests a profitability report, THE Reporte_Rentabilidad SHALL return a list of properties with total income (from pagos with `estado` equal to `pagado`), total expenses (from gastos with `estado` equal to `pagado`), and net income (income minus expenses) for the specified period.
2. THE Reporte_Rentabilidad SHALL require `mes` and `anio` query parameters to define the reporting period.
3. THE Reporte_Rentabilidad SHALL support an optional `propiedad_id` filter to limit the report to a single Propiedad.
4. THE Reporte_Rentabilidad SHALL return monetary values in the Propiedad's configured `moneda`.
5. THE Reporte_Rentabilidad SHALL support PDF export of the profitability report.
6. THE Reporte_Rentabilidad SHALL support XLSX export of the profitability report.

### Requirement 7: Expense Data Persistence

**User Story:** As a system administrator, I want expense data to be stored reliably with proper schema constraints, so that data integrity is maintained.

#### Acceptance Criteria

1. THE Sistema_Gastos SHALL store Gasto records in a `gastos` database table with UUID primary key, foreign keys to `propiedades` and optionally `unidades`, DECIMAL(12,2) for `monto`, VARCHAR for `moneda`, `categoria`, `estado`, `descripcion`, `proveedor`, `numero_factura`, TEXT for `notas`, DATE for `fecha_gasto`, and TIMESTAMP WITH TIME ZONE for `created_at` and `updated_at`.
2. THE Sistema_Gastos SHALL enforce a foreign key constraint from `gastos.propiedad_id` to `propiedades.id`.
3. THE Sistema_Gastos SHALL enforce a foreign key constraint from `gastos.unidad_id` to `unidades.id` when `unidad_id` is present.
4. THE Sistema_Gastos SHALL create database indexes on `propiedad_id`, `unidad_id`, `categoria`, `estado`, and `fecha_gasto` columns.
5. THE Sistema_Gastos SHALL implement the database schema as a SeaORM migration file.

### Requirement 8: CSV Import of Expenses

**User Story:** As a property manager, I want to import expenses from a CSV file, so that I can bulk-load historical expense data.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado uploads a valid CSV file, THE Sistema_Gastos SHALL parse each row and create corresponding Gasto records.
2. THE Sistema_Gastos SHALL require the following CSV columns: `propiedad_id`, `categoria`, `descripcion`, `monto`, `moneda`, `fecha_gasto`.
3. THE Sistema_Gastos SHALL accept the following optional CSV columns: `unidad_id`, `proveedor`, `numero_factura`, `notas`.
4. WHEN a CSV row fails validation, THE Sistema_Gastos SHALL skip the invalid row, continue processing remaining rows, and include the row number and error description in the response.
5. THE Sistema_Gastos SHALL return a summary with the count of successfully imported records and a list of errors for failed rows.
6. IF the CSV file is empty or contains no valid rows, THEN THE Sistema_Gastos SHALL return HTTP status 422 with a descriptive error message in Spanish.

### Requirement 9: Dashboard Integration

**User Story:** As a property manager, I want to see expense totals on the dashboard, so that I have a quick overview of my financial position.

#### Acceptance Criteria

1. THE Sistema_Gastos SHALL expose a monthly expense total endpoint that returns the sum of `monto` for Gasto records with `estado` equal to `pagado` in the current month.
2. THE Sistema_Gastos SHALL expose a monthly expense comparison endpoint that returns the total expenses for the current month and the previous month, along with the percentage change.
3. WHEN the dashboard stats are requested, THE Sistema_Gastos SHALL include `total_gastos_mes` (current month paid expenses total) in the dashboard statistics response.
