# Implementation Plan: Tareas de Fondo Programadas

## Overview

Implementar un scheduler ligero de tareas de fondo dentro del proceso Actix-web existente. Se crea una tabla `ejecuciones_tareas` para registrar el historial, un servicio con el scheduler y las funciones wrapper de cada tarea, handlers de administración, y una nueva función `contratos::marcar_vencidos`. Se sigue la arquitectura existente: handlers → services → entities.

## Tasks

- [x] 1. Database migration
  - [x] 1.1 Create migration `m20250601_000001_create_ejecuciones_tareas`
    - Create file `backend/migrations/m20250601_000001_create_ejecuciones_tareas.rs`
    - Define table `ejecuciones_tareas` with columns: id (UUID PK DEFAULT gen_random_uuid()), nombre_tarea (VARCHAR(100) NOT NULL), iniciado_en (TIMESTAMPTZ NOT NULL DEFAULT now()), duracion_ms (BIGINT NOT NULL), exitosa (BOOLEAN NOT NULL), registros_afectados (BIGINT NOT NULL DEFAULT 0), mensaje_error (TEXT NULLABLE)
    - Add indexes: `idx_ejecuciones_tareas_nombre` on `nombre_tarea`, `idx_ejecuciones_tareas_iniciado_en` on `iniciado_en`, `idx_ejecuciones_tareas_nombre_iniciado` on `(nombre_tarea, iniciado_en)`
    - Follow the exact pattern from `m20250408_000005_create_pagos.rs`
    - _Requirements: 5.1, 5.4_

  - [x] 1.2 Register migration in `backend/migrations/mod.rs`
    - Add `pub mod m20250601_000001_create_ejecuciones_tareas;`
    - Add migration to the `Migrator::migrations()` vec
    - _Requirements: 5.1_

- [x] 2. SeaORM entity
  - [x] 2.1 Create entity `backend/src/entities/ejecucion_tarea.rs`
    - Define `Model` struct with `#[sea_orm(table_name = "ejecuciones_tareas")]`
    - Fields: id (Uuid PK), nombre_tarea (String), iniciado_en (DateTimeWithTimeZone), duracion_ms (i64), exitosa (bool), registros_afectados (i64), mensaje_error (Option<String>)
    - Empty `Relation` enum (no foreign keys)
    - Follow the pattern from `backend/src/entities/pago.rs`
    - _Requirements: 5.1_

  - [x] 2.2 Register entity in `backend/src/entities/mod.rs`
    - Add `pub mod ejecucion_tarea;`
    - _Requirements: 5.1_

- [x] 3. Request/response models
  - [x] 3.1 Create `backend/src/models/background_jobs.rs`
    - Define `EjecucionTareaResponse` (Serialize, camelCase): id (Uuid), nombre_tarea (String), iniciado_en (DateTime<Utc>), duracion_ms (i64), exitosa (bool), registros_afectados (i64), mensaje_error (Option<String>)
    - Define `HistorialQuery` (Deserialize, camelCase): nombre_tarea (Option<String>), exitosa (Option<bool>), page (Option<u64>), per_page (Option<u64>)
    - Define `EjecutarTareaResponse` (Serialize, camelCase): ejecucion (EjecucionTareaResponse)
    - Implement `From<ejecucion_tarea::Model>` for `EjecucionTareaResponse`
    - Follow the pattern from `backend/src/models/pago.rs`
    - _Requirements: 5.1, 7.1, 8.1_

  - [x] 3.2 Register model module in `backend/src/models/mod.rs`
    - Add `pub mod background_jobs;`
    - _Requirements: 5.1_

- [x] 4. Add `contratos::marcar_vencidos` function
  - [x] 4.1 Add `marcar_vencidos` to `backend/src/services/contratos.rs`
    - Add public async function `marcar_vencidos(db: &DatabaseConnection) -> Result<u64, AppError>`
    - Use `update_many` to set estado="vencido" and updated_at=now() where estado="activo" and fecha_fin < today
    - Follow the exact pattern of `pagos::mark_overdue`
    - _Requirements: 2.1, 2.2_

  - [x] 4.2 Remove `#[allow(dead_code)]` from `pagos::mark_overdue`
    - The function is now used by the scheduler, so the allow attribute is no longer needed
    - _Requirements: 1.2_

- [x] 5. Service layer
  - [x] 5.1 Create `backend/src/services/background_jobs.rs`
    - Define constants: `TAREAS_VALIDAS` array with the 4 task names, `INTERVALO_POR_DEFECTO_SECS = 86_400`
    - Implement `iniciar_scheduler(db: DatabaseConnection)`: spawns one `tokio::spawn` per task, each with its own `tokio::time::interval(Duration::from_secs(INTERVALO_POR_DEFECTO_SECS))`. Each loop iteration calls `ejecutar_tarea_con_registro` wrapped in `catch_unwind`. On panic, logs with `tracing::error!`
    - Implement `ejecutar_tarea_por_nombre(db, nombre) -> Result<EjecucionTareaResponse>`: validates nombre is in TAREAS_VALIDAS (else NotFound), calls `ejecutar_tarea_con_registro`
    - Implement `historial(db, query) -> Result<PaginatedResponse<EjecucionTareaResponse>>`: paginated list with optional filters by nombre_tarea and exitosa, ordered by iniciado_en DESC
    - Implement internal `ejecutar_tarea_con_registro(db, nombre) -> Result<EjecucionTareaResponse>`: measures time with `Instant::now()`, dispatches to the correct executor function, registers result in ejecuciones_tareas
    - Implement internal executors: `ejecutar_marcar_pagos_atrasados` (calls `pagos::mark_overdue`), `ejecutar_marcar_contratos_vencidos` (calls `contratos::marcar_vencidos`), `ejecutar_marcar_documentos_vencidos` (calls `documentos::marcar_vencidos`), `ejecutar_generar_notificaciones` (queries all organizations, calls `notificaciones::generar_notificaciones` for each, sums totals)
    - Implement internal `registrar_ejecucion(db, nombre, duracion_ms, exitosa, registros_afectados, mensaje_error) -> Result<EjecucionTareaResponse>`
    - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.3, 3.1, 3.2, 3.3, 4.1, 4.2, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3_

  - [x] 5.2 Register service module in `backend/src/services/mod.rs`
    - Add `pub mod background_jobs;`
    - _Requirements: 1.1_

- [x] 6. Handlers
  - [x] 6.1 Create `backend/src/handlers/background_jobs.rs`
    - Implement `ejecutar_tarea(db: Data<DatabaseConnection>, _admin: AdminOnly, path: Path<String>) -> Result<HttpResponse>`: calls `background_jobs::ejecutar_tarea_por_nombre`, returns Ok(json)
    - Implement `historial(db: Data<DatabaseConnection>, _admin: AdminOnly, query: Query<HistorialQuery>) -> Result<HttpResponse>`: calls `background_jobs::historial`, returns Ok(json)
    - Follow the pattern from `backend/src/handlers/pagos.rs`
    - _Requirements: 7.1, 7.2, 7.4, 7.5, 8.1, 8.4, 8.5_

  - [x] 6.2 Register handler module in `backend/src/handlers/mod.rs`
    - Add `pub mod background_jobs;`
    - _Requirements: 7.1_

- [x] 7. Route registration
  - [x] 7.1 Add tareas routes to `backend/src/routes.rs`
    - Add a new `web::scope("/tareas")` block inside the `/api/v1` scope
    - Register routes: GET "/historial" → historial (static route first), POST "/{nombre}/ejecutar" → ejecutar_tarea
    - _Requirements: 7.1, 8.1_

- [x] 8. Scheduler integration in main.rs
  - [x] 8.1 Add scheduler startup to `backend/src/main.rs`
    - After migrations and before `HttpServer::new`, add: `let scheduler_db = db.clone(); realestate_backend::services::background_jobs::iniciar_scheduler(scheduler_db);`
    - Follow the existing pattern of the `PreviewStore::cleanup_expired` tokio::spawn
    - _Requirements: 6.1, 6.4_

- [x] 9. Checkpoint — Ensure backend compiles and unit tests pass
  - Run `cargo check --workspace` and `cargo test --workspace` to verify all new modules compile and existing tests still pass.

- [x] 10. Integration tests
  - [x] 10.1 Create `backend/tests/background_jobs_tests.rs`
    - Test ejecutar tarea manualmente (marcar_pagos_atrasados) → 200 with execution record
    - Test ejecutar tarea con nombre inválido → 404
    - Test ejecutar tarea como gerente → 403
    - Test ejecutar tarea como visualizador → 403
    - Test consultar historial → paginated response
    - Test filtrar historial por nombre_tarea → only matching records
    - Test filtrar historial por exitosa → only matching records
    - Test consultar historial como gerente → 403
    - Test marcar_pagos_atrasados updates pending overdue payments to atrasado
    - Test marcar_contratos_vencidos updates active expired contracts to vencido
    - Test marcar_documentos_vencidos updates verified expired documents to vencido
    - Test idempotencia: second execution returns 0 registros_afectados
    - _Requirements: 1.1, 1.4, 2.1, 2.4, 3.1, 3.4, 5.1, 5.2, 7.1, 7.2, 7.4, 7.5, 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 11. Property-based tests
  - [x] 11.1 Write property test: Idempotencia de marcar pagos atrasados
    - **Property 1: Idempotencia de marcar pagos atrasados**
    - **Validates: Requirements 1.1, 1.4**
    - Generate payments with random dates and estados, execute mark_overdue twice, verify second returns 0

  - [x] 11.2 Write property test: Idempotencia de marcar contratos vencidos
    - **Property 2: Idempotencia de marcar contratos vencidos**
    - **Validates: Requirements 2.1, 2.4**
    - Generate contracts with random dates and estados, execute marcar_vencidos twice, verify second returns 0

  - [x] 11.3 Write property test: Idempotencia de marcar documentos vencidos
    - **Property 3: Idempotencia de marcar documentos vencidos**
    - **Validates: Requirements 3.1, 3.4**
    - Generate documents with random dates and estados, execute marcar_vencidos twice, verify second returns 0

  - [x] 11.4 Write property test: Registro de ejecución completo
    - **Property 4: Registro de ejecución completo**
    - **Validates: Requirements 5.1**
    - For each valid task name, execute and verify the execution record has all required fields with valid values

  - [x] 11.5 Write property test: Nombre de tarea inválido retorna 404
    - **Property 5: Nombre de tarea inválido retorna 404**
    - **Validates: Requirements 7.2**
    - Generate random strings not in TAREAS_VALIDAS, verify ejecutar_tarea_por_nombre returns NotFound

  - [x] 11.6 Write property test: Historial ordenado por fecha descendente
    - **Property 6: Historial ordenado por fecha descendente**
    - **Validates: Requirements 8.1**
    - Generate multiple executions, list history, verify iniciado_en descending order

  - [x] 11.7 Write property test: Filtrado del historial retorna solo registros coincidentes
    - **Property 7: Filtrado del historial retorna solo registros coincidentes**
    - **Validates: Requirements 8.2, 8.3**
    - Generate executions with varied nombres and exitosa values, filter, verify all returned records match

  - [x] 11.8 Write property test: Post-condición de marcar pagos atrasados
    - **Property 8: Post-condición de marcar pagos atrasados**
    - **Validates: Requirements 1.1**
    - Generate payments with varied dates, execute task, verify no payments remain with estado=pendiente and fecha_vencimiento < today

  - [x] 11.9 Write property test: Post-condición de marcar contratos vencidos
    - **Property 9: Post-condición de marcar contratos vencidos**
    - **Validates: Requirements 2.1**
    - Generate contracts with varied dates, execute task, verify no contracts remain with estado=activo and fecha_fin < today

  - [x] 11.10 Write property test: Post-condición de marcar documentos vencidos
    - **Property 10: Post-condición de marcar documentos vencidos**
    - **Validates: Requirements 3.1**
    - Generate documents with varied dates, execute task, verify no documents remain with estado_verificacion=verificado and fecha_vencimiento < today

- [x] 12. Final checkpoint — Ensure all tests pass
  - Run `cargo test --workspace` and ensure all tests pass.

## Notes

- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- The scheduler uses one `tokio::spawn` per task for fault isolation
- All four business tasks reuse existing service functions (except `contratos::marcar_vencidos` which is new)
- No frontend changes are needed — this is a backend-only feature with admin API endpoints
