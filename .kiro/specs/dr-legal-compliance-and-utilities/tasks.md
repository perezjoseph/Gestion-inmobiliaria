# Implementation Plan: DR Legal Compliance and Utilities

## Overview

Implement Dominican Republic rental law compliance (Ley 4314) and utility management features following the project's domain pattern: migration → entity → DTOs → service → handler → routes → tests. The implementation covers IPC-based rent caps, lease renewal reminders, deposit return enforcement, eviction tracking, utility bill tracking with DR-specific providers, payment responsibility configuration, abnormal consumption detection, and DGII RNC/cédula lookup with caching.

## Tasks

- [x] 1. Database migrations
  - [x] 1.1 Create migration `m20260512_000001_add_utility_fields_to_gastos`
    - Create file `backend/migrations/m20260512_000001_add_utility_fields_to_gastos.rs`
    - Add columns to `gastos` table: nic_contrato (VARCHAR(50) NULL), proveedor_servicio (VARCHAR(20) NULL), consumo (DECIMAL(12,4) NULL), unidad_consumo (VARCHAR(5) NULL), periodo_desde (DATE NULL), periodo_hasta (DATE NULL)
    - Add indexes: `idx_gastos_proveedor_servicio` on (proveedor_servicio), `idx_gastos_unidad_proveedor` on (unidad_id, proveedor_servicio)
    - _Requirements: 6.1_

  - [x] 1.2 Create migration `m20260512_000002_create_desahucios`
    - Create file `backend/migrations/m20260512_000002_create_desahucios.rs`
    - Create table `desahucios` with columns: id (UUID PK), contrato_id (UUID FK → contratos NOT NULL), estado (VARCHAR(20) NOT NULL), fecha_inicio (DATE NOT NULL), fecha_resolucion (DATE NULL), motivo (TEXT NOT NULL), organizacion_id (UUID FK → organizaciones NOT NULL), created_at (TIMESTAMPTZ NOT NULL), updated_at (TIMESTAMPTZ NOT NULL)
    - Add indexes: `idx_desahucios_contrato_id`, `idx_desahucios_organizacion_id`
    - _Requirements: 5.1_

  - [x] 1.3 Create migration `m20260512_000003_create_responsabilidad_servicios`
    - Create file `backend/migrations/m20260512_000003_create_responsabilidad_servicios.rs`
    - Create table `responsabilidad_servicios` with columns: id (UUID PK), unidad_id (UUID FK → unidades NOT NULL), proveedor_servicio (VARCHAR(20) NOT NULL), responsable (VARCHAR(20) NOT NULL), contrato_id (UUID FK → contratos NULL), organizacion_id (UUID FK → organizaciones NOT NULL), created_at (TIMESTAMPTZ NOT NULL), updated_at (TIMESTAMPTZ NOT NULL)
    - Add unique constraint: `uq_responsabilidad_unidad_proveedor_contrato` on (unidad_id, proveedor_servicio, COALESCE(contrato_id, '00000000-0000-0000-0000-000000000000'))
    - Add indexes: `idx_responsabilidad_unidad_id`, `idx_responsabilidad_contrato_id` (partial WHERE contrato_id IS NOT NULL)
    - _Requirements: 7.1, 7.2_

  - [x] 1.4 Create migration `m20260512_000004_create_cache_dgii`
    - Create file `backend/migrations/m20260512_000004_create_cache_dgii.rs`
    - Create table `cache_dgii` with columns: id (UUID PK), cedula_rnc (VARCHAR(20) NOT NULL), nombre_razon_social (VARCHAR(255) NOT NULL), nombre_comercial (VARCHAR(255) NULL), estado (VARCHAR(20) NOT NULL), regimen_de_pagos (VARCHAR(50) NULL), actividad_economica (TEXT NULL), raw_response (JSONB NOT NULL), organizacion_id (UUID FK → organizaciones NOT NULL), cached_at (TIMESTAMPTZ NOT NULL), expires_at (TIMESTAMPTZ NOT NULL), created_at (TIMESTAMPTZ NOT NULL), updated_at (TIMESTAMPTZ NOT NULL)
    - Add unique constraint: `uq_cache_dgii_rnc_org` on (cedula_rnc, organizacion_id)
    - Add indexes: `idx_cache_dgii_cedula_rnc`, `idx_cache_dgii_organizacion_id`, `idx_cache_dgii_expires_at`
    - _Requirements: 8.1, 8.2, 8.4_

  - [x] 1.5 Register all migrations in `backend/migrations/mod.rs`
    - Add `pub mod` declarations for all four new migration modules
    - Add migrations to the `Migrator::migrations()` vec in order
    - _Requirements: 6.1, 5.1, 7.1_

- [x] 2. SeaORM entities
  - [x] 2.1 Create entity `backend/src/entities/desahucio.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "desahucios")]`
    - Fields: id (Uuid PK), contrato_id (Uuid), estado (String), fecha_inicio (Date), fecha_resolucion (Option<Date>), motivo (String with column_type Text), organizacion_id (Uuid), created_at (DateTimeWithTimeZone), updated_at (DateTimeWithTimeZone)
    - Define `Relation` enum with belongs_to Contrato and belongs_to Organizacion
    - Implement `Related` traits and `ActiveModelBehavior`
    - _Requirements: 5.1_

  - [x] 2.2 Create entity `backend/src/entities/responsabilidad_servicio.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "responsabilidad_servicios")]`
    - Fields: id (Uuid PK), unidad_id (Uuid), proveedor_servicio (String), responsable (String), contrato_id (Option<Uuid>), organizacion_id (Uuid), created_at (DateTimeWithTimeZone), updated_at (DateTimeWithTimeZone)
    - Define `Relation` enum with belongs_to Unidad, optional belongs_to Contrato, belongs_to Organizacion
    - _Requirements: 7.1, 7.2_

  - [x] 2.3 Create entity `backend/src/entities/cache_dgii.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "cache_dgii")]`
    - Fields: id (Uuid PK), cedula_rnc (String), nombre_razon_social (String), nombre_comercial (Option<String>), estado (String), regimen_de_pagos (Option<String>), actividad_economica (Option<String>), raw_response (Json), organizacion_id (Uuid), cached_at (DateTimeWithTimeZone), expires_at (DateTimeWithTimeZone), created_at (DateTimeWithTimeZone), updated_at (DateTimeWithTimeZone)
    - _Requirements: 8.1_

  - [x] 2.4 Extend existing gasto entity with utility fields
    - Add to `backend/src/entities/gasto.rs`: nic_contrato (Option<String>), proveedor_servicio (Option<String>), consumo (Option<Decimal> with column_type Decimal(Some((12,4)))), unidad_consumo (Option<String>), periodo_desde (Option<Date>), periodo_hasta (Option<Date>)
    - _Requirements: 6.1_

  - [x] 2.5 Register new entities in `backend/src/entities/mod.rs`
    - Add `pub mod desahucio;`, `pub mod responsabilidad_servicio;`, `pub mod cache_dgii;`
    - _Requirements: 5.1, 7.1_

- [x] 3. DTOs and models
  - [x] 3.1 Create `backend/src/models/ipc.rs`
    - Define `IpcResponse` (Serialize, camelCase): valor_ipc (Decimal), fecha_efectiva (NaiveDate), ultimo_fetch_exitoso (DateTime<Utc>)
    - Define `UpdateIpcRequest` (Deserialize, camelCase): valor_ipc (Decimal), fecha_efectiva (NaiveDate)
    - Define `IpcData` (Serialize, Deserialize): valor_ipc (Decimal), fecha_efectiva (NaiveDate), ultimo_fetch_exitoso (DateTime<Utc>) — stored in configuracion JSON
    - _Requirements: 1.3, 1.4, 2.4_

  - [x] 3.2 Create `backend/src/models/desahucio.rs`
    - Define `CreateDesahucioRequest` (Deserialize, camelCase): contrato_id (Uuid), motivo (String)
    - Define `UpdateDesahucioRequest` (Deserialize, camelCase): estado (Option<String>), fecha_resolucion (Option<NaiveDate>), motivo (Option<String>)
    - Define `DesahucioResponse` (Serialize, camelCase): id, contrato_id, estado, fecha_inicio, fecha_resolucion, motivo, created_at, updated_at
    - Define `DesahucioListQuery` (Deserialize, camelCase): contrato_id (Option<Uuid>), estado (Option<String>), page (Option<u64>), per_page (Option<u64>)
    - _Requirements: 5.1, 5.4_

  - [x] 3.3 Create `backend/src/models/responsabilidad_servicio.rs`
    - Define `ResponsabilidadEfectivaResponse` (Serialize, camelCase): proveedor_servicio, responsable, es_override_contrato (bool)
    - Define `UpdateResponsabilidadRequest` (Deserialize, camelCase): responsabilidades (Vec<ResponsabilidadItem>)
    - Define `ResponsabilidadItem` (Deserialize, camelCase): proveedor_servicio (String), responsable (String)
    - _Requirements: 7.1, 7.3, 7.4, 7.5_

  - [x] 3.4 Create `backend/src/models/dgii.rs`
    - Define `DgiiConsultaResponse`, `DgiiNombreResponse`, `DgiiNombreItem`, `DgiiCacheEntry` per design
    - Define internal `MegaplusApiResponse` for API deserialization
    - Define `ConsultaRncQuery` (Deserialize): rnc (String)
    - Define `ConsultaNombreQuery` (Deserialize): buscar (String)
    - _Requirements: 8.1, 8.2_

  - [x] 3.5 Extend existing gasto DTOs
    - Add to `CreateGastoRequest` / `UpdateGastoRequest`: nic_contrato, proveedor_servicio, consumo, unidad_consumo, periodo_desde, periodo_hasta (all Option)
    - Add to `GastoListQuery`: proveedor_servicio (Option<String>), periodo_desde (Option<NaiveDate>), periodo_hasta (Option<NaiveDate>)
    - Add to `GastoResponse`: nic_contrato, proveedor_servicio, consumo, unidad_consumo, periodo_desde, periodo_hasta
    - _Requirements: 6.1, 6.5_

  - [x] 3.6 Register new models in `backend/src/models/mod.rs`
    - Add `pub mod ipc;`, `pub mod desahucio;`, `pub mod responsabilidad_servicio;`, `pub mod dgii;`
    - _Requirements: 1.3, 5.1, 7.1_

- [x] 4. Checkpoint
  - Ensure the project compiles with `cargo build --workspace`. Ask the user if questions arise.

- [x] 5. IPC service
  - [x] 5.1 Create `backend/src/services/ipc.rs`
    - Implement `obtener_ipc_actual(db)` — reads `ipc_banco_central` from configuracion, deserializes IpcData JSON, returns Option<IpcData>
    - Implement `actualizar_ipc_manual(db, input, updated_by)` — upserts configuracion entry with new IPC value and timestamp
    - Implement `fetch_ipc_from_bcrd(db)` — HTTP POST to `api.bancentral.gov.do/api/v2/HistoricoIPC` with BCRD_API_TOKEN, parse response, upsert configuracion, return Ok(1) or Err
    - Implement pure function `calcular_monto_maximo(monto_actual, ipc_porcentaje) -> Decimal` — returns `monto_actual * (1 + ipc_porcentaje / 100)`
    - Use `reqwest` client with 10s timeout for BCRD API calls
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2_

  - [x] 5.2 Write property test for IPC rent cap enforcement
    - **Property 1: IPC rent cap enforcement**
    - **Validates: Requirements 1.1, 1.2**
    - Test that amounts within cap always pass and amounts exceeding cap always fail
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

  - [x] 5.3 Write property test for deposit cap invariant
    - **Property 2: Deposit cap invariant (Ley 4314)**
    - **Validates: Requirements 4.4**
    - Test that deposit <= monto_mensual passes and deposit > monto_mensual fails
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

- [x] 6. Desahucios service
  - [x] 6.1 Create `backend/src/services/desahucios.rs`
    - Implement `create(db, input, usuario_id, organizacion_id)` — validate contract is activo, set estado="iniciado", fecha_inicio=today, register audit entry
    - Implement `update(db, org_id, id, input, usuario_id)` — validate state transitions (iniciado→en_progreso, en_progreso→completado, iniciado→completado), require fecha_resolucion when completado, register audit entry
    - Implement `list(db, org_id, query)` — paginated, org-scoped, optional filters by contrato_id and estado
    - Implement helper `validate_desahucio_transition(estado, fecha_resolucion)` and `validate_estado_transition(from, to)`
    - Constants: `ESTADOS_DESAHUCIO = ["iniciado", "en_progreso", "completado"]`
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

  - [x] 6.2 Write property test for desahucio state machine
    - **Property 5: Desahucio state machine**
    - **Validates: Requirements 5.1, 5.3**
    - Test valid/invalid transitions and fecha_resolucion requirement for completado
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

- [x] 7. Servicios públicos service
  - [x] 7.1 Create `backend/src/services/servicios_publicos.rs`
    - Implement `obtener_responsabilidades(db, org_id, unidad_id)` — query responsabilidad_servicios for unit, resolve effective responsibility (contract override > unit default)
    - Implement `actualizar_responsabilidad_unidad(db, org_id, unidad_id, input, usuario_id)` — upsert unit-level defaults (contrato_id = NULL)
    - Implement `actualizar_responsabilidad_contrato(db, org_id, contrato_id, input, usuario_id)` — upsert contract-level overrides
    - Implement `verificar_consumo_anormal(db, gasto, organizacion_id)` — query last 10 gastos for same unidad+proveedor, skip if < 3, calculate average, generate consumo_anormal notification if consumo > avg * 1.5, never propagate errors
    - Implement helper `resolve_responsabilidad(unit_default, contract_override) -> &str`
    - Constants: `PROVEEDORES_SERVICIO = ["EDENORTE", "EDESUR", "EDEESTE", "CAASD"]`, `RESPONSABLES = ["propietario", "inquilino"]`
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 8.1, 8.2, 8.3, 8.4_

  - [x] 7.2 Write property test for anomaly detection threshold
    - **Property 3: Anomaly detection threshold**
    - **Validates: Requirements 8.1, 8.2, 8.4**
    - Test that consumption > avg*1.5 triggers alert, <= avg*1.5 does not, and < 3 records skips check
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

  - [x] 7.3 Write property test for responsibility resolution precedence
    - **Property 7: Responsibility resolution precedence**
    - **Validates: Requirements 7.3**
    - Test that contract override always takes precedence over unit default when present
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

- [x] 8. DGII service
  - [x] 8.1 Create `backend/src/services/dgii.rs`
    - Implement `consultar_rnc(db, organizacion_id, rnc)` — normalize RNC (strip dashes), check cache_dgii for non-expired entry, on miss call megaplus API, upsert cache on success, graceful degradation with stale cache
    - Implement `consultar_nombre(buscar)` — proxy to megaplus name search API (no caching)
    - Implement `invalidar_cache(db, organizacion_id, rnc)` — delete cache entry by cedula_rnc + org
    - Implement `validar_cedula_inquilino(db, organizacion_id, cedula)` — best-effort lookup, returns Option, never errors
    - Use `reqwest` client with 10s timeout, read `DGII_API_BASE_URL` from env (default: `https://rnc.megaplus.com.do`)
    - _Requirements: 8.1, 8.2, 8.4_

- [x] 9. Modifications to existing services
  - [x] 9.1 Modify `backend/src/services/contratos.rs` — IPC cap + deposit cap
    - In `renovar()`: call `ipc::obtener_ipc_actual(db)`, if Some calculate max allowed amount, reject if input.monto_mensual exceeds cap, if None log warning and proceed
    - In `create()` and `update()`: validate deposito <= monto_mensual when deposito is provided
    - _Requirements: 1.1, 1.2, 1.5, 4.4_

  - [x] 9.2 Modify `backend/src/services/gastos.rs` — utility validation + anomaly trigger
    - In `create()`: when categoria == "servicio_publico", require proveedor_servicio, validate consumo > 0 if provided, validate periodo_desde < periodo_hasta if both provided, after insert call `servicios_publicos::verificar_consumo_anormal()` (best-effort, log errors)
    - In `list()`: add filtering by proveedor_servicio, periodo_desde, periodo_hasta query params
    - _Requirements: 6.2, 6.3, 6.4, 6.5, 8.1, 8.2_

  - [x] 9.3 Modify `backend/src/services/notificaciones.rs` — renewal reminders + deposit alerts
    - Implement `generar_renovacion_reminders(db, organizacion_id)` — find active contracts with fecha_fin within 90 days, generate notifications at 90/60/30 day thresholds with dedup via mensaje marker
    - Implement `generar_deposito_devolucion(db, organizacion_id)` — find terminated contracts with estado_deposito=cobrado, generate pending (10-14 days) and overdue (>15 days) notifications
    - Add calls to both in `generar_notificaciones()` pipeline
    - Add fields to `GenerarNotificacionesResponse`: contrato_renovacion (u64), deposito_devolucion (u64)
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 4.3_

  - [x] 9.4 Write property test for renewal reminder thresholds
    - **Property 4: Renewal reminder threshold correctness**
    - **Validates: Requirements 3.1, 3.2, 3.3**
    - Test that correct set of thresholds (90, 60, 30) fire based on days remaining
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

  - [x] 9.5 Modify `backend/src/services/background_jobs.rs` — actualizar_ipc task
    - Add `"actualizar_ipc"` to `TAREAS_VALIDAS`
    - Add match arm in `ejecutar_tarea_con_registro()` calling `ipc::fetch_ipc_from_bcrd(db)`
    - _Requirements: 2.1, 2.2, 2.3_

  - [x] 9.6 Modify `backend/src/services/inquilinos.rs` — DGII validation
    - In `create()` and `update()`: after successful DB operation, call `dgii::validar_cedula_inquilino(db, organizacion_id, cedula)` best-effort (log result, never block creation)
    - _Requirements: 8.1_

  - [x] 9.7 Write property test for utility field validation
    - **Property 6: Utility field validation**
    - **Validates: Requirements 6.2, 6.3, 6.4**
    - Test proveedor_servicio enum validation, consumo > 0, periodo ordering
    - Add to `backend/tests/dr_legal_compliance_pbt.rs`

- [x] 10. Checkpoint
  - Ensure all tests pass with `cargo test --workspace`. Ask the user if questions arise.

- [x] 11. Handlers
  - [x] 11.1 Create `backend/src/handlers/ipc.rs`
    - `GET /configuracion/ipc` handler (WriteAccess) — calls `ipc::obtener_ipc_actual`, returns IpcResponse or 404
    - `PUT /configuracion/ipc` handler (AdminOnly) — calls `ipc::actualizar_ipc_manual`, returns updated IpcResponse
    - _Requirements: 1.3, 1.4, 2.4_

  - [x] 11.2 Create `backend/src/handlers/desahucios.rs`
    - `POST /desahucios` handler (WriteAccess) — calls `desahucios::create`
    - `PUT /desahucios/{id}` handler (WriteAccess) — calls `desahucios::update`
    - `GET /desahucios` handler (WriteAccess) — calls `desahucios::list` with query params
    - _Requirements: 5.4_

  - [x] 11.3 Create `backend/src/handlers/servicios_publicos.rs`
    - `GET /propiedades/{propiedad_id}/unidades/{id}/servicios` handler (WriteAccess) — calls `servicios_publicos::obtener_responsabilidades`
    - `PUT /propiedades/{propiedad_id}/unidades/{id}/servicios` handler (WriteAccess) — calls `servicios_publicos::actualizar_responsabilidad_unidad`
    - `PUT /contratos/{id}/servicios` handler (WriteAccess) — calls `servicios_publicos::actualizar_responsabilidad_contrato`
    - _Requirements: 7.3, 7.4, 7.5_

  - [x] 11.4 Create `backend/src/handlers/dgii.rs`
    - `GET /dgii/consulta` handler (WriteAccess) — calls `dgii::consultar_rnc` with query.rnc
    - `GET /dgii/consulta/nombre` handler (WriteAccess) — calls `dgii::consultar_nombre` with query.buscar
    - `DELETE /dgii/cache/{rnc}` handler (AdminOnly) — calls `dgii::invalidar_cache`
    - _Requirements: 8.1, 8.2_

  - [x] 11.5 Register handlers in `backend/src/handlers/mod.rs`
    - Add `pub mod ipc;`, `pub mod desahucios;`, `pub mod servicios_publicos;`, `pub mod dgii;`
    - _Requirements: 5.4, 7.4_

- [x] 12. Routes registration
  - [x] 12.1 Register all new routes in `backend/src/routes.rs`
    - Add IPC routes under `/api/v1/configuracion/ipc`
    - Add desahucios routes under `/api/v1/desahucios`
    - Add servicios routes nested under existing `/api/v1/propiedades/{propiedad_id}/unidades/{id}/servicios` and `/api/v1/contratos/{id}/servicios`
    - Add DGII routes under `/api/v1/dgii`
    - _Requirements: 1.3, 1.4, 5.4, 7.4, 7.5, 8.1_

  - [x] 12.2 Register new services in `backend/src/services/mod.rs`
    - Add `pub mod ipc;`, `pub mod desahucios;`, `pub mod servicios_publicos;`, `pub mod dgii;`
    - _Requirements: 1.1, 5.1, 7.1_

- [x] 13. Checkpoint
  - Ensure the project compiles and all routes are registered correctly. Ask the user if questions arise.

- [x] 14. Integration tests
  - [x] 14.1 Create `backend/tests/ipc_tests.rs`
    - Test GET /configuracion/ipc returns 404 when not configured, 200 when configured
    - Test PUT /configuracion/ipc with AdminOnly (200), WriteAccess non-admin (403), unauthenticated (401)
    - Test IPC cap validation in contract renewal (accepted/rejected/no-ipc-configured)
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [x] 14.2 Create `backend/tests/desahucios_tests.rs`
    - Test full CRUD lifecycle: create (contract must be activo), update state transitions, list with pagination
    - Test validation: completado without fecha_resolucion (422), non-active contract (422)
    - Test auth: unauthenticated (401), wrong role (403), correct role (200/201)
    - Test audit trail is created on create/update
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

  - [x] 14.3 Create `backend/tests/servicios_publicos_tests.rs`
    - Test responsibility resolution: unit default vs contract override precedence
    - Test update unit default and contract override endpoints
    - Test utility gasto creation triggers anomaly detection (>= 3 prior records, > 50% threshold)
    - Test anomaly detection skips when < 3 prior records
    - Test auth on all endpoints
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 8.1, 8.2, 8.3, 8.4_

  - [x] 14.4 Create `backend/tests/dgii_tests.rs`
    - Test RNC lookup with cache miss (mocked API), cache hit, stale cache fallback
    - Test name lookup endpoint
    - Test cache invalidation (AdminOnly)
    - Test invalid RNC format returns 422
    - Test auth on all endpoints
    - _Requirements: 8.1, 8.2, 8.4_

  - [x] 14.5 Extend `backend/tests/contratos_tests.rs`
    - Test deposit cap validation: deposito > monto_mensual rejected (422), deposito == monto_mensual accepted
    - Test renewal with IPC cap: amount within cap accepted, amount exceeding cap rejected with max_allowed in response
    - _Requirements: 1.1, 1.2, 4.4_

  - [x] 14.6 Extend `backend/tests/gastos_tests.rs`
    - Test utility gasto creation: proveedor_servicio required when categoria=servicio_publico, consumo > 0 validation, periodo ordering validation
    - Test filtering by proveedor_servicio and periodo date range
    - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [x] 15. Final checkpoint
  - Ensure all tests pass with `cargo test --workspace -- --test-threads=1`. Ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Integration tests validate end-to-end flows with auth and database
- The DGII service uses external API calls — integration tests should mock the HTTP client
- Background job `actualizar_ipc` runs daily via the existing scheduler infrastructure
