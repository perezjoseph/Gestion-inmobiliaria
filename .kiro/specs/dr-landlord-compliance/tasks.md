# Implementation Plan: DR Landlord Compliance

## Overview

This implementation adds Dominican Republic fiscal compliance capabilities to the property management platform. The work is organized in phases: database migrations and entities first, then core service logic (fiscal classification, ITBIS, payments, NCF, indexation, IPI, condominiums, reporting), followed by API handlers and frontend pages. Each phase builds incrementally on the previous one with no orphaned code.

## Tasks

- [x] 1. Database migrations and entity generation
  - [x] 1.1 Create migration `m20260601_000001_add_tipo_fiscal_to_organizaciones.rs`
    - Add columns: `tipo_fiscal` (VARCHAR NOT NULL DEFAULT 'informal'), `regimen_pagos` (VARCHAR NULL), `fecha_inicio_operaciones` (DATE NULL), `is_ecf_certificado` (BOOLEAN DEFAULT false)
    - _Requirements: 1.1, 1.8_

  - [x] 1.2 Create migration `m20260601_000002_add_fiscal_columns_to_pagos.rs`
    - Add columns: `monto_base`, `monto_itbis`, `monto_itbis_retenido`, `ncf`, `fecha_comprobante`, `tipo_ncf`, `es_parcial`, `saldo_pendiente`, `tipo_linea`
    - _Requirements: 3.1, 3.2, 6.4, 7.7, 7.11_

  - [x] 1.3 Create migration `m20260601_000003_add_catastral_to_propiedades.rs`
    - Add columns: `valor_catastral` (DECIMAL(14,2) NULL), `exento_ipi` (BOOLEAN DEFAULT false), `motivo_exencion` (VARCHAR NULL)
    - _Requirements: 9.3, 9.8_

  - [x] 1.4 Create migration `m20260601_000004_create_cuotas_condominio.rs`
    - Create table `cuotas_condominio` with all columns per design (id, propiedad_id, monto, moneda, frecuencia, fecha_inicio, fecha_fin, es_passthrough, contrato_id, organizacion_id, timestamps)
    - _Requirements: 2.1_

  - [x] 1.5 Create migration `m20260601_000005_create_secuencias_ncf.rs`
    - Create table `secuencias_ncf` with unique constraint on (organizacion_id, tipo_ncf, prefijo)
    - _Requirements: 7.1, 7.8_

  - [x] 1.6 Create migration `m20260601_000006_create_reportes_dgii.rs`
    - Create table `reportes_dgii` with unique constraint on (organizacion_id, tipo_reporte, periodo, estado)
    - _Requirements: 8.7_

  - [x] 1.7 Create migration `m20260601_000007_create_configuraciones_ipi.rs`
    - Create table `configuraciones_ipi` per design schema
    - _Requirements: 9.4, 9.5_

  - [x] 1.8 Create migration `m20260601_000008_create_recibos_informales.rs`
    - Create table `recibos_informales` with unique constraint on referencia_interna
    - _Requirements: 3.5_

  - [x] 1.9 Create migration `m20260601_000009_create_copropietarios.rs`
    - Create table `copropietarios` per design schema
    - _Requirements: 9.10_

  - [x] 1.10 Generate SeaORM entities for all new and modified tables
    - Run `sea-orm-cli generate entity` or manually write entity modules for: cuota_condominio, secuencia_ncf, reporte_dgii, configuracion_ipi, recibo_informal, copropietario
    - Update existing entities for organizacion, pago, propiedad with new columns
    - Register all entities in `entities/mod.rs`
    - _Requirements: 1.1, 2.1, 3.5, 7.1, 8.7, 9.4, 9.10_

- [x] 2. Core DTOs and types
  - [x] 2.1 Create `models/fiscal.rs`
    - Define `TipoFiscal` enum (PersonaJuridica, PersonaFisica, Informal) with serde and display traits
    - Define `ActualizarTipoFiscalRequest`, `EstadoFiscalResponse`, and related DTOs
    - _Requirements: 1.1, 1.7, 1.8_

  - [x] 2.2 Create `models/itbis.rs`
    - Define `ItbisResult`, `RetencionResult` structs
    - _Requirements: 6.3, 6.4, 6.7_

  - [x] 2.3 Create `models/ncf.rs`
    - Define `TipoNCF` enum (B01, B02, B14, B15), `ConfigurarRangoRequest`, `AlertaRango`, `SecuenciaNcfResponse`
    - _Requirements: 7.2, 7.8, 7.9_

  - [x] 2.4 Create `models/reportes_dgii.rs`
    - Define `Registro607`, `Registro606`, `ReporteGenerado`, `RegistroExcluido`, `RegistroPreview`
    - _Requirements: 8.2, 8.3, 8.6_

  - [x] 2.5 Create `models/ipi.rs`
    - Define `IpiLiabilityResponse`, `CopropietarioResponse`, `ConfiguracionIpiRequest`
    - _Requirements: 9.1, 9.6, 9.10_

  - [x] 2.6 Create `models/indexacion.rs`
    - Define `PropuestaRenovacion`, `AprobarRenovacionRequest`, `ContratoProximoVencer`
    - _Requirements: 5.1, 5.3, 5.6_

  - [x] 2.7 Create `models/condominios.rs`
    - Define `CrearCuotaRequest`, `UpdateCuotaRequest`, `CuotaResponse`, `BillingDesglose`
    - _Requirements: 2.1, 2.3, 2.4_

- [x] 3. Checkpoint - Verify migrations compile and entities are generated
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Fiscal classification service
  - [x] 4.1 Implement `services/fiscal.rs`
    - Implement `verificar_acceso_fiscal` — reject informal users accessing ITBIS/NCF/606/607 features with 403 error
    - Implement `actualizar_tipo_fiscal` — validate RNC (DGII check-digit) for persona_juridica, cédula (Luhn) for persona_fisica, reject invalid transitions
    - Wire to existing `validacion_fiscal.rs` for RNC/cédula validation
    - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

  - [x] 4.2 Write property tests for fiscal classification (`services/fiscal_pbt.rs`)
    - **Property 1: RNC Check-Digit Validation Round Trip**
    - **Property 2: Cédula Luhn Validation Round Trip**
    - **Property 3: Fiscal Feature Access Gate**
    - **Property 4: Tipo Fiscal Transition Requires Valid Identifier**
    - **Validates: Requirements 1.2, 1.3, 1.5, 1.6, 1.7**

- [x] 5. ITBIS calculation service
  - [x] 5.1 Implement `services/itbis.rs`
    - Implement `calcular_itbis` — apply 18% only when tipo_fiscal is registered AND property is commercial/industrial; zero for residential or informal
    - Implement `calcular_retencion` — 30% retention when tenant is persona_juridica
    - Support configurable rate (16% future-proofing) stored per category
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.7, 6.8, 6.9_

  - [x] 5.2 Write property tests for ITBIS (`services/itbis_pbt.rs`)
    - **Property 8: ITBIS Applicability**
    - **Property 9: Payment Amount Invariant**
    - **Property 10: ITBIS Retention Split**
    - **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.7, 6.8**

- [x] 6. Partial payment and informal receipt service
  - [x] 6.1 Implement partial payment logic in `services/pagos.rs` (extend existing)
    - Implement partial payment recording: validate monto < amount_due, reject if equal
    - Track saldo_pendiente per billing period, mark `pagado` when sum >= amount_due
    - Implement FIFO allocation for payments without fecha_vencimiento reference
    - Implement surplus cascade (pago adelantado) to next unpaid period
    - Support all DGII-aligned metodo_pago values
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.7, 3.8, 3.9_

  - [x] 6.2 Implement `services/recibos_informales.rs`
    - Generate Recibo_Informal with unique sequential referencia_interna (RI-NNNNNN format) for cash payments by informal organizations
    - _Requirements: 3.5, 3.6_

  - [x] 6.3 Write property tests for payments (`services/pagos_parciales_pbt.rs`)
    - **Property 11: Partial Payment Balance Tracking**
    - **Property 12: FIFO Payment Allocation**
    - **Property 13: Informal Receipt Uniqueness**
    - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.8**

- [x] 7. NCF/e-CF sequence service
  - [x] 7.1 Implement `services/ncf.rs`
    - Implement `asignar_ncf` with `SELECT ... FOR UPDATE` row-level locking for gapless sequential generation
    - Implement retry logic for concurrency conflicts only
    - Validate NCF format: `^[A-Z]\d{10}$` — 'E' prefix for e-CF, 'B' for physical
    - Validate generated number falls within DGII-authorized range before persisting
    - Alert at 80% range consumption
    - Implement `configurar_rango` for admin setup of authorized ranges
    - Handle NCF assignment failure gracefully: payment stays `pagado`, flagged for manual resolution
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 7.10, 7.11_

  - [x] 7.2 Write property tests for NCF (`services/ncf_pbt.rs`)
    - **Property 17: NCF Sequential Gapless Generation**
    - **Property 18: NCF Format Compliance**
    - **Property 19: NCF Range Boundary Enforcement**
    - **Validates: Requirements 7.1, 7.3, 7.4, 7.5, 7.9**

- [x] 8. Lease indexation service
  - [x] 8.1 Implement `services/indexacion.rs`
    - Implement `calcular_propuesta_renovacion` — fetch IPC from existing `ipc.rs`, apply formula `monto * (1 + min(ipc, 10%) / 100)`, enforce 10% absolute cap
    - Handle stale IPC data (>90 days): use cached value with warning flag
    - Implement `aprobar_renovacion` — verify calculation integrity, create renewed contrato, record audit trail
    - Implement `contratos_proximos_vencer` — find contracts within 60 days of fecha_fin
    - Support custom escalation clause override (lower-than-IPC increases)
    - Indexation applies per contract anniversary, not calendar year
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10_

  - [x] 8.2 Write property tests for indexation (`services/indexacion_pbt.rs`)
    - **Property 14: Rent Indexation Formula with Legal Cap**
    - **Property 15: Indexation 60-Day Trigger**
    - **Property 16: Indexation Anniversary Alignment**
    - **Validates: Requirements 5.1, 5.2, 5.7, 5.9, 5.10**

- [x] 9. Condominium fee service
  - [x] 9.1 Implement `services/condominios.rs`
    - CRUD for cuota_condominio records (crear, actualizar, listar, eliminar)
    - Implement `calcular_billing_con_cuota` — separate line item for cuota, apply ITBIS to cuota if commercial + registered
    - Enforce temporal boundary: new amounts apply only to billing periods starting after change effective date
    - Cuota increases NOT subject to 10% Ley 85-25 cap
    - Track cuota payment status independently from base rent
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_

  - [x] 9.2 Write property tests for condominiums (`services/condominios_pbt.rs`)
    - **Property 5: Billing Desglose with Condominium Fee**
    - **Property 6: Condominium Fee Change Temporal Boundary**
    - **Property 7: Condominium Fee Increase Uncapped**
    - **Validates: Requirements 2.3, 2.4, 2.5, 2.7**

- [x] 10. Checkpoint - Verify all service modules compile and unit tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. IPI property tax service
  - [x] 11.1 Implement `services/ipi.rs`
    - Implement `calcular_ipi` — sum valor_catastral excluding exento_ipi properties, compute `max(0, total - umbral) * 0.01`
    - IPI applies regardless of tipo_fiscal (informal included)
    - Implement co-owner proportional split per 2026 Supreme Court ruling
    - Validate copropietario percentages sum to 100% per property
    - Implement CONFOTUR exemption handling
    - Display cross-organization ownership warning
    - _Requirements: 9.1, 9.2, 9.3, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10_

  - [x] 11.2 Write property tests for IPI (`services/ipi_pbt.rs`)
    - **Property 27: IPI Calculation**
    - **Property 28: IPI Co-Owner Proportional Split**
    - **Validates: Requirements 9.1, 9.2, 9.7, 9.8, 9.10**

- [x] 12. DGII 606/607 report generation service
  - [x] 12.1 Implement `services/reportes_dgii.rs`
    - Implement `generar_607` — filter payments by fecha_pago within requested month, format per Norma 07-2018 fields
    - Implement `generar_606` — filter expenses by fecha_pago within requested month
    - Implement `formatear_linea_607` and `formatear_linea_606` — pipe-delimited formatting
    - Implement `generar_header` — RNC, period YYYYMM, record count, total amounts
    - Exclude records missing RNC or fecha_comprobante (include in registros_excluidos); include records with complete fiscal data but missing NCF (blank NCF field)
    - Implement `calcular_itbis_neto` (607 ITBIS collected - 606 ITBIS paid)
    - Residential income in 607 must have ITBIS = 0
    - Track report status: borrador/enviado, prevent double-submission
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9_

  - [x] 12.2 Write property tests for reports (`services/reportes_dgii_pbt.rs`)
    - **Property 20: Report 607 Monthly Filtering**
    - **Property 21: Report 607 Field Completeness**
    - **Property 22: Report 606 Field Completeness**
    - **Property 23: Report Format and Header Integrity**
    - **Property 24: Incomplete Record Exclusion**
    - **Property 25: ITBIS Neto Calculation**
    - **Property 26: Residential Income in 607 Has Zero ITBIS**
    - **Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.8, 8.9**

- [x] 13. Multi-property dashboard service
  - [x] 13.1 Implement dashboard comparison logic in `services/dashboard.rs` (extend existing)
    - Compute per-property analytics: ingresos totales, gastos totales, rentabilidad neta, tasa de ocupación, morosidad %, cuotas condominio totales
    - Support filtering by tipo_propiedad (residencial, comercial, mixto)
    - Normalize monetary values to single display currency using Banco Central exchange rate
    - Compute metrics only when date range filter is applied; show static info only when no date range
    - Rentabilidad neta formula: `(ingresos - gastos - cuotas) / valor_catastral * 100`, capped at 200%, flag properties with valor_catastral < RD$100,000
    - Include ITBIS column for registered orgs with commercial properties
    - Show valor_catastral per property for IPI contribution visibility
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8_

  - [x] 13.2 Write property tests for dashboard (`services/dashboard_comparativo_pbt.rs`)
    - **Property 29: Dashboard Date Range Filtering**
    - **Property 30: Rentabilidad Neta Formula**
    - **Property 31: Currency Normalization**
    - **Validates: Requirements 4.3, 4.5, 4.6**

- [x] 14. Checkpoint - All services implemented and tested
  - Ensure all tests pass, ask the user if questions arise.

- [x] 15. API handlers
  - [x] 15.1 Implement `handlers/fiscal.rs`
    - `PUT /api/v1/organizacion/fiscal/tipo-fiscal` — update tipo_fiscal with validation
    - `GET /api/v1/organizacion/fiscal/estado` — return current fiscal state
    - Enforce `AdminOnly` extractor
    - _Requirements: 1.1, 1.5, 1.6, 1.7_

  - [x] 15.2 Implement `handlers/condominios.rs`
    - `POST /api/v1/propiedades/{id}/condominios` — create cuota
    - `PUT /api/v1/propiedades/{id}/condominios/{cuota_id}` — update cuota
    - `GET /api/v1/propiedades/{id}/condominios` — list cuotas
    - `DELETE /api/v1/propiedades/{id}/condominios/{cuota_id}` — remove cuota
    - Enforce `WriteAccess` extractor
    - _Requirements: 2.1, 2.2, 2.5_

  - [x] 15.3 Implement `handlers/ncf.rs`
    - `GET /api/v1/ncf/secuencias` — list NCF sequences for org
    - `POST /api/v1/ncf/configurar-rango` — configure authorized range
    - `GET /api/v1/ncf/alertas` — check range consumption alerts
    - Enforce `AdminOnly` extractor, gate with `verificar_acceso_fiscal`
    - _Requirements: 7.1, 7.8, 7.9_

  - [x] 15.4 Implement `handlers/reportes_dgii.rs`
    - `POST /api/v1/reportes-dgii/607` — generate 607 report
    - `POST /api/v1/reportes-dgii/606` — generate 606 report
    - `GET /api/v1/reportes-dgii/preview/{tipo}/{periodo}` — preview report as JSON
    - `PUT /api/v1/reportes-dgii/{id}/estado` — mark as enviado
    - Enforce `WriteAccess` extractor, gate with `verificar_acceso_fiscal`
    - _Requirements: 8.1, 8.4, 8.7_

  - [x] 15.5 Implement `handlers/ipi.rs`
    - `GET /api/v1/ipi/calculo` — compute IPI liability
    - `PUT /api/v1/ipi/umbral` — update IPI threshold
    - `GET /api/v1/ipi/copropietarios/{propiedad_id}` — list co-owners
    - `POST /api/v1/ipi/copropietarios` — add co-owner
    - Enforce `WriteAccess` extractor
    - _Requirements: 9.1, 9.5, 9.6, 9.10_

  - [x] 15.6 Implement `handlers/indexacion.rs`
    - `GET /api/v1/indexacion/propuesta/{contrato_id}` — get renewal proposal
    - `POST /api/v1/indexacion/aprobar/{contrato_id}` — approve renewal
    - `GET /api/v1/indexacion/proximos-vencer` — list contracts expiring within 60 days
    - Enforce `WriteAccess` extractor
    - _Requirements: 5.1, 5.3, 5.4, 5.6, 5.7_

  - [x] 15.7 Register all new handlers in `routes.rs`
    - Add route scopes for fiscal, condominios, ncf, reportes-dgii, ipi, indexacion
    - _Requirements: all_

- [x] 16. Checkpoint - API layer compiles, handler tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 17. Frontend pages (Leptos)
  - [x] 17.1 Create Configuración Fiscal page (`pages/configuracion_fiscal.rs`)
    - Form to set tipo_fiscal, RNC/cédula input with validation feedback, NCF range configuration
    - Route: `/configuracion/fiscal`
    - _Requirements: 1.1, 1.2, 1.3, 7.8_

  - [x] 17.2 Create Cuotas Condominio component (`pages/condominios.rs`)
    - CRUD interface for condominium fees per property, passthrough toggle, billing preview
    - Route: `/propiedades/{id}/condominios`
    - _Requirements: 2.1, 2.2, 2.3_

  - [x] 17.3 Create Recibos Informales page (`pages/recibos_informales.rs`)
    - View/create informal receipts, partial payment recording with notes
    - Route: `/recibos-informales`
    - _Requirements: 3.5, 3.7_

  - [x] 17.4 Create Dashboard Comparativo page (`pages/dashboard_comparativo.rs`)
    - Multi-property comparison table, sortable columns, date range and property type filters, currency toggle
    - Route: `/dashboard/comparativo`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [x] 17.5 Create Indexación page (`pages/indexacion.rs`)
    - List upcoming renewals, show IPC proposal vs current amount, approve/override flow
    - Route: `/indexacion`
    - _Requirements: 5.1, 5.3, 5.4, 5.8_

  - [x] 17.6 Create Reportes DGII page (`pages/reportes_dgii.rs`)
    - Generate/preview 606/607 reports, show excluded records, download pipe-delimited TXT, mark as submitted
    - Route: `/reportes-dgii`
    - _Requirements: 8.1, 8.4, 8.6, 8.7_

  - [x] 17.7 Create IPI page (`pages/ipi.rs`)
    - Display IPI liability breakdown, manage copropietarios, configure threshold, show payment deadlines
    - Route: `/ipi`
    - _Requirements: 9.1, 9.4, 9.6, 9.10_

  - [x] 17.8 Register all new pages in frontend router (`app.rs`)
    - Add routes for all new pages, update navigation menu
    - _Requirements: all_

- [ ] 18. Integration wiring and notification system
  - [x] 18.1 Wire NCF assignment into payment flow
    - After payment is marked `pagado` for registered org, call `asignar_ncf` to assign appropriate NCF type based on tenant fiscal status
    - Handle NCF failure gracefully (payment stays pagado, flag for manual resolution)
    - _Requirements: 7.4, 7.6, 7.7_

  - [x] 18.2 Wire ITBIS calculation into payment creation
    - During payment creation for commercial/industrial contracts under registered orgs, call `calcular_itbis` and store monto_base/monto_itbis
    - Apply retention logic when tenant is persona_juridica
    - _Requirements: 6.3, 6.4, 6.5, 6.7_

  - [x] 18.3 Wire condominium fee into billing cycle
    - When generating billing for contracts with passthrough cuotas, include cuota line item with independent payment tracking
    - _Requirements: 2.3, 2.4, 2.6_

  - [-] 18.4 Implement notification triggers
    - 60-day contract expiration notification for indexation review
    - 30-day IPI payment deadline notification
    - NCF range 80% consumption alert notification
    - _Requirements: 5.7, 9.4, 7.9_

  - [~] 18.5 Write integration tests for end-to-end flows
    - Payment flow: create pago → ITBIS → NCF assignment → 607 inclusion
    - Lease renewal: proposal → approval → new contrato with audit trail
    - IPI calculation across multiple properties with co-owners
    - _Requirements: 6.3, 7.4, 5.6, 8.1, 9.1, 9.10_

- [~] 19. Final checkpoint - Full build passes, all tests green
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document using `proptest`
- Unit tests validate specific examples and edge cases
- The design leverages existing infrastructure (IPC service, DGII service, validación fiscal) — implementations should wire into these
- All user-facing text must be in Spanish per project localization rules
- Migrations follow naming convention `m{YYYYMMDD}_{SEQ}_{name}.rs`
- New services follow the pattern: entity → DTOs → service → handler → routes → tests

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "1.2", "1.3", "1.4", "1.5", "1.6", "1.7", "1.8", "1.9"] },
    { "id": 1, "tasks": ["1.10", "2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7"] },
    { "id": 2, "tasks": ["4.1", "5.1", "6.1", "6.2"] },
    { "id": 3, "tasks": ["4.2", "5.2", "6.3", "7.1", "8.1", "9.1"] },
    { "id": 4, "tasks": ["7.2", "8.2", "9.2", "11.1", "12.1", "13.1"] },
    { "id": 5, "tasks": ["11.2", "12.2", "13.2"] },
    { "id": 6, "tasks": ["15.1", "15.2", "15.3", "15.4", "15.5", "15.6"] },
    { "id": 7, "tasks": ["15.7", "17.1", "17.2", "17.3", "17.4", "17.5", "17.6", "17.7"] },
    { "id": 8, "tasks": ["17.8", "18.1", "18.2", "18.3", "18.4"] },
    { "id": 9, "tasks": ["18.5"] }
  ]
}
```
