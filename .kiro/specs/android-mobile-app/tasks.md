# Implementation Plan: PropManager Android App

## Overview

Build a native Kotlin Android application for the PropManager property management system. The implementation follows a foundation-first approach: Gradle multi-module setup and version catalog, then core modules (model, common, database, network), then data/sync layer, then feature modules (auth first, then CRUD features, then online-only features), and finally cross-cutting concerns (OCR scanner, navigation wiring, error handling). All UI uses Jetpack Compose with Material Design 3, all text in Spanish.

## Tasks

- [x] 1. Project scaffolding and Gradle multi-module setup
  - [x] 1.1 Create `gradle/libs.versions.toml` version catalog
    - Define versions and dependency aliases for: Kotlin, AGP, Hilt, KSP, Room 3, Retrofit, OkHttp, Kotlinx Serialization, WorkManager, ML Kit Text Recognition, Jetpack Compose BOM, Material 3, Navigation, Kotest, Coroutines, EncryptedSharedPreferences
    - Define plugin aliases for android-application, android-library, kotlin-android, kotlin-serialization, hilt, ksp
    - _Requirements: 16.3_

  - [x] 1.2 Update root `android/build.gradle.kts` and `android/settings.gradle.kts`
    - Register all modules in settings.gradle.kts: `:app`, `:core:model`, `:core:common`, `:core:database`, `:core:network`, `:core:data`, `:core:ui`, and all `:feature:*` modules (auth, dashboard, propiedades, inquilinos, contratos, pagos, gastos, mantenimiento, reportes, documentos, notificaciones, auditoria, perfil, configuracion, importacion, scanner)
    - Configure dependency resolution management with google() and mavenCentral()
    - _Requirements: 16.3_

  - [x] 1.3 Create `build.gradle.kts` for each module with correct dependencies
    - `:core:model` — pure Kotlin library, no Android dependencies
    - `:core:common` — Android library, depends on `:core:model`
    - `:core:database` — Android library with Room KSP, depends on `:core:model`
    - `:core:network` — Android library with Retrofit + Kotlinx Serialization, depends on `:core:model`
    - `:core:data` — Android library with WorkManager + Hilt, depends on `:core:database`, `:core:network`, `:core:model`
    - `:core:ui` — Android library with Compose + Material 3, depends on `:core:model`, `:core:common`
    - `:feature:*` — Android libraries with Compose + Hilt, depend on `:core:data`, `:core:model`, `:core:ui`, `:core:common`
    - `:app` — Android application with Hilt, Navigation, depends on all feature modules and `:core:ui`
    - Feature modules never depend on each other
    - _Requirements: 16.3_

  - [x] 1.4 Create base package structure and placeholder files
    - Create `src/main/kotlin/com/propmanager/` directory structure in each module
    - Create `PropManagerApp.kt` with `@HiltAndroidApp` annotation in `:app`
    - Create `MainActivity.kt` as single-activity Compose host in `:app`
    - _Requirements: 16.3_

- [x] 2. Checkpoint — Verify Gradle sync succeeds
  - Ensure the multi-module project syncs without errors, ask the user if questions arise.

- [ ] 3. Core model module (`:core:model`)
  - [x] 3.1 Create domain model data classes
    - Implement `Propiedad`, `Inquilino`, `Contrato`, `Pago`, `Gasto`, `SolicitudMantenimiento`, `NotaMantenimiento` as pure Kotlin data classes
    - Use `BigDecimal` for monetary fields, `LocalDate` for dates, `Instant` for timestamps
    - Include `isPendingSync: Boolean` field on all offline-capable models
    - Implement `UserProfile` data class with id, nombre, email, rol
    - _Requirements: 2.1, 15.2, 15.3_

  - [x] 3.2 Create network DTO data classes
    - Implement `LoginRequest`, `LoginResponse`, `UserDto`, `PropiedadDto`, `InquilinoDto`, `ContratoDto`, `PagoDto`, `GastoDto`, `SolicitudDto`, `NotaDto` with `@Serializable` and `@SerialName` annotations for camelCase mapping
    - Implement `PaginatedResponse<T>`, `ApiError`, and all dashboard/report response DTOs
    - Implement create/update request DTOs for each entity
    - _Requirements: 1.2, 15.4, 15.5_

  - [x] 3.3 Create sealed UiState classes and ValidationResult
    - Implement `sealed class UiState<T>` with Loading, Success, Error variants
    - Implement `sealed class ValidationResult` with Valid and Invalid(message) variants
    - _Requirements: 19.3_

- [ ] 4. Core common module (`:core:common`)
  - [x] 4.1 Implement DateFormatter utility
    - `toDisplay(LocalDate): String` — formats as DD/MM/YYYY
    - `toApi(LocalDate): String` — formats as YYYY-MM-DD
    - `fromApi(String): LocalDate` — parses YYYY-MM-DD
    - `fromDisplay(String): LocalDate` — parses DD/MM/YYYY
    - _Requirements: 15.2, 15.4, 15.5_

  - [x] 4.2 Write property test for DateFormatter round-trip
    - **Property 12: Date formatting round-trip**
    - **Validates: Requirements 15.2, 15.4, 15.5**

  - [x] 4.3 Implement CurrencyFormatter utility
    - `format(BigDecimal, String): String` — formats with RD$ for DOP, US$ for USD, 2 decimal places, thousands separators using Dominican locale
    - _Requirements: 15.3_

  - [x] 4.4 Write property test for CurrencyFormatter
    - **Property 13: Currency formatting correctness**
    - **Validates: Requirements 15.3**

  - [x] 4.5 Implement form validators
    - `PropiedadValidator.validateCreate()` — validates titulo, direccion, ciudad, provincia, tipoPropiedad, precio required
    - `InquilinoValidator.validateCreate()` — validates nombre, apellido, cedula required
    - `ContratoValidator.validateCreate()` — validates propiedad_id, inquilino_id, fecha_inicio, fecha_fin, monto_mensual required; fecha_fin after fecha_inicio
    - `PagoValidator.validateCreate()` — validates contrato_id, monto, fecha_vencimiento required
    - `GastoValidator.validateCreate()` — validates propiedad_id, categoria, descripcion, monto, moneda, fecha_gasto required
    - `SolicitudValidator.validateCreate()` — validates propiedad_id, titulo required
    - _Requirements: 3.7, 4.6, 5.7, 5.8, 6.7, 7.7, 8.8_

  - [x] 4.6 Write property test for form validation — blank required fields
    - **Property 10: Entity form validation rejects blank required fields**
    - **Validates: Requirements 3.7, 4.6, 5.7, 6.7, 7.7, 8.8**

  - [x] 4.7 Write property test for contrato date ordering validation
    - **Property 11: Contrato date ordering validation**
    - **Validates: Requirements 5.8**

- [x] 5. Checkpoint — Verify core:model and core:common compile and tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 6. Core network module (`:core:network`)
  - [x] 6.1 Implement TokenProvider interface and EncryptedTokenProvider
    - Store JWT token in EncryptedSharedPreferences with AES256_GCM encryption
    - Implement `getToken()`, `saveToken()`, `clearToken()`, `saveUserProfile()`, `getUserProfile()`, `clearAll()`
    - _Requirements: 1.1, 1.6_

  - [x] 6.2 Write property test for login response extraction round-trip
    - **Property 1: Login response extraction round-trip**
    - **Validates: Requirements 1.2**

  - [x] 6.3 Implement AuthInterceptor for OkHttp
    - Attach `Authorization: Bearer {token}` header to every request when token is non-null
    - Skip header when token is null
    - _Requirements: 1.7_

  - [x] 6.4 Write property test for AuthInterceptor
    - **Property 2: Auth interceptor attaches Bearer token**
    - **Validates: Requirements 1.7**

  - [x] 6.5 Implement API error parser
    - Parse JSON error body `{"error": "<type>", "message": "<msg>"}` and extract message field
    - Handle malformed JSON gracefully with fallback message
    - _Requirements: 19.1_

  - [x] 6.6 Write property test for API error message extraction
    - **Property 16: API error message extraction**
    - **Validates: Requirements 19.1**

  - [x] 6.7 Implement Retrofit API service interfaces
    - `AuthApiService` — POST login
    - `PropiedadesApiService` — list, getById, create, update, delete
    - `InquilinosApiService` — list, getById, create, update, delete
    - `ContratosApiService` — list, getById, create, update, delete, renovar, terminar, expiring
    - `PagosApiService` — list, create, update, delete, getRecibo
    - `GastosApiService` — list, create, update, delete, resumenCategorias
    - `MantenimientoApiService` — list, getById, create, update, delete, updateEstado, addNota
    - `DashboardApiService` — stats, pagosProximos, contratosCalendario, ocupacionTendencia, ingresosComparacion, gastosComparacion
    - `ReportesApiService` — ingresos, rentabilidad, historialPagos, ocupacionTendencia, PDF/XLSX exports
    - `DocumentosApiService` — list, upload
    - `NotificacionesApiService` — pagosVencidos
    - `AuditoriaApiService` — list with filters
    - `ConfiguracionApiService` — getMoneda, updateMoneda
    - `ImportacionApiService` — importPropiedades, importInquilinos, importGastos
    - `PerfilApiService` — getPerfil, updatePerfil, changePassword
    - _Requirements: 1.1, 3.1–3.6, 4.1–4.5, 5.1–5.6, 6.1–6.6, 7.1–7.6, 8.1–8.7, 9.1–9.7, 10.1–10.5, 11.1–11.4, 12.1, 13.1–13.4, 17.1–17.2, 18.1–18.2, 20.1–20.3_

  - [x] 6.8 Implement NetworkMonitor using ConnectivityManager
    - Expose `isOnline: StateFlow<Boolean>` backed by `ConnectivityManager.NetworkCallback`
    - _Requirements: 16.4_

  - [x] 6.9 Configure Hilt network module
    - Provide OkHttpClient with AuthInterceptor, logging interceptor, timeouts
    - Provide Retrofit instance with Kotlinx Serialization converter and base URL from BuildConfig
    - Provide all API service interfaces via Retrofit.create()
    - Provide NetworkMonitor singleton
    - _Requirements: 1.7_

- [ ] 7. Core database module (`:core:database`)
  - [x] 7.1 Implement Room entity classes
    - `PropiedadEntity`, `InquilinoEntity`, `ContratoEntity`, `PagoEntity`, `GastoEntity`, `SolicitudMantenimientoEntity`, `NotaMantenimientoEntity` with proper `@Entity`, `@PrimaryKey`, `@ColumnInfo`, `@ForeignKey`, and `@Index` annotations
    - `SyncQueueEntry` entity with autoGenerate PK, entityType, entityId, operation, payload, createdAt, retryCount
    - `DashboardCache` entity with key, data (JSON string), cachedAt
    - Implement `Converters` class with `@TypeConverter` for Instant ↔ Long
    - _Requirements: 2.1, 2.9_

  - [x] 7.2 Implement Room DAO interfaces
    - `PropiedadDao` — observeAll, observeById, observeFiltered, upsert, upsertAll, markDeleted, deleteAll
    - `InquilinoDao` — observeAll, observeById, search by nombre/apellido/cedula, upsert, upsertAll, markDeleted, deleteAll
    - `ContratoDao` — observeAll, observeById, observeExpiring(daysThreshold), upsert, upsertAll, markDeleted, deleteAll
    - `PagoDao` — observeAll, observeFiltered(contratoId, estado, fechaRange), upsert, upsertAll, markDeleted, deleteAll
    - `GastoDao` — observeAll, observeFiltered(propiedadId, categoria, estado, fechaRange), upsert, upsertAll, markDeleted, deleteAll
    - `SolicitudMantenimientoDao` — observeAll, observeFiltered(estado, prioridad, propiedadId), observeById, upsert, upsertAll, markDeleted, deleteAll
    - `NotaMantenimientoDao` — observeBySolicitudId, insert
    - `SyncQueueDao` — getAllPending (ordered by createdAt ASC), enqueue, remove, observePendingCount
    - `DashboardCacheDao` — getByKey, upsert, deleteAll
    - _Requirements: 2.1, 2.2, 3.1, 3.2, 4.1, 4.2, 5.1, 5.6, 6.1, 6.2, 7.1, 7.2, 8.1, 8.2_

  - [x] 7.3 Write property test for sync queue chronological ordering
    - **Property 4: Sync queue chronological processing order**
    - **Validates: Requirements 2.4**

  - [x] 7.4 Write property test for entity filtering
    - **Property 7: Entity filtering returns only matching results**
    - **Validates: Requirements 3.2, 6.2, 7.2, 8.2**

  - [x] 7.5 Write property test for inquilino text search
    - **Property 8: Inquilino text search matches nombre, apellido, or cédula**
    - **Validates: Requirements 4.2**

  - [x] 7.6 Write property test for contracts-expiring date range
    - **Property 9: Contracts-expiring date range filter**
    - **Validates: Requirements 5.6**

  - [x] 7.7 Implement PropManagerDatabase abstract class
    - Annotate with `@Database` listing all 9 entities, version = 1, exportSchema = true
    - Declare abstract DAO accessor methods for all DAOs
    - Register `Converters` via `@TypeConverters`
    - _Requirements: 2.1_

  - [x] 7.8 Implement entity ↔ domain mapping extension functions
    - `PropiedadEntity.toDomain()`, `PropiedadDto.toEntity()`, `Propiedad.toCreateRequest()`, etc.
    - Same pattern for all entities: DTO → Entity, Entity → Domain, Domain → request DTO
    - Handle BigDecimal ↔ String conversion, LocalDate ↔ String, Instant ↔ Long
    - _Requirements: 2.1, 15.2, 15.3_

  - [x] 7.9 Configure Hilt database module
    - Provide `PropManagerDatabase` singleton via `Room.databaseBuilder()`
    - Provide all DAO instances from the database
    - _Requirements: 2.1_

- [x] 8. Checkpoint — Verify core modules compile and DAO tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Core data module — Repositories and sync engine (`:core:data`)
  - [x] 9.1 Implement offline-first repositories
    - `PropiedadesRepository` — observeAll, observeFiltered, getById, create, update, delete, refreshFromServer
    - `InquilinosRepository` — observeAll, search, getById, create, update, delete, refreshFromServer
    - `ContratosRepository` — observeAll, getById, create, renew, terminate, getExpiring, refreshFromServer
    - `PagosRepository` — observeAll, observeFiltered, create, update, delete, refreshFromServer
    - `GastosRepository` — observeAll, observeFiltered, create, update, delete, refreshFromServer
    - `MantenimientoRepository` — observeAll, observeFiltered, getById, create, update, updateEstado, addNota, delete, refreshFromServer
    - All write operations: persist to Room immediately, enqueue SyncQueueEntry
    - All read operations: return Flow from Room DAO, filter out isDeleted entries
    - _Requirements: 2.2, 2.3, 3.4, 3.5, 3.6, 4.3, 4.4, 4.5, 5.3, 5.4, 5.5, 6.3, 6.4, 6.5, 7.3, 7.4, 7.5, 8.3, 8.4, 8.5, 8.6, 8.7_

  - [x] 9.2 Write property test for sync queue entry recording
    - **Property 3: Sync queue records complete entries**
    - **Validates: Requirements 2.3**

  - [x] 9.3 Implement online-only repositories
    - `DashboardRepository` — fetchStats, fetchPagosProximos, fetchContratosCalendario, fetchOcupacionTendencia, fetchIngresosComparacion, fetchGastosComparacion; cache responses in DashboardCache
    - `ReportesRepository` — fetchIngresos, fetchRentabilidad, fetchHistorialPagos, fetchOcupacionTendencia, downloadExport
    - `DocumentosRepository` — fetchDocuments, uploadDocument
    - `NotificacionesRepository` — fetchPagosVencidos
    - `AuditoriaRepository` — fetchAuditLog
    - `PerfilRepository` — fetchPerfil, updatePerfil, changePassword
    - `ConfiguracionRepository` — fetchMoneda, updateMoneda
    - `ImportacionRepository` — importPropiedades, importInquilinos, importGastos
    - _Requirements: 9.1–9.7, 10.1–10.5, 11.1–11.4, 12.1, 13.1–13.4, 17.1–17.2, 18.1–18.2, 20.1–20.5_

  - [x] 9.4 Implement SyncWorker (WorkManager CoroutineWorker)
    - Process all pending SyncQueueEntry items in chronological order
    - For each entry: dispatch to correct API service based on entityType and operation
    - On success: remove entry from queue, upsert server response into Room
    - On HTTP 409: apply server-wins conflict resolution, remove entry, notify user
    - On network error: return Result.retry() for exponential backoff
    - _Requirements: 2.4, 2.5, 2.6, 2.7_

  - [x] 9.5 Write property test for successful sync removes entry
    - **Property 5: Successful sync removes entry and updates local DB**
    - **Validates: Requirements 2.5**

  - [x] 9.6 Write property test for conflict resolution server-wins
    - **Property 6: Conflict resolution applies server-wins**
    - **Validates: Requirements 2.6**

  - [x] 9.7 Implement PeriodicRefreshWorker
    - Refresh all offline-capable repositories from server at configurable interval (default 15 min)
    - Constrain to network-connected state
    - Check for new overdue payments and post Android system notification
    - _Requirements: 2.8, 12.2_

  - [x] 9.8 Implement SyncManager to schedule workers
    - Schedule SyncWorker as OneTimeWorkRequest on connectivity change
    - Schedule PeriodicRefreshWorker with 15-minute interval and network constraint
    - _Requirements: 2.4, 2.7, 2.8_

  - [x] 9.9 Configure Hilt data module
    - Provide all repository singletons
    - Provide SyncManager, WorkManager instance
    - _Requirements: 2.3, 2.4_

- [x] 10. Checkpoint — Verify repositories, sync engine, and property tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Core UI module (`:core:ui`)
  - [x] 11.1 Implement Material 3 theme and color scheme
    - Define `PropManagerTheme` composable with light/dark color schemes
    - Define typography scale and shape scheme
    - _Requirements: 16.3_

  - [x] 11.2 Implement shared Compose components
    - `PropManagerTopAppBar` — with title, optional back navigation, optional actions
    - `PropManagerBottomNavBar` — 5 items: Dashboard, Propiedades, Inquilinos, Contratos, Más
    - `OfflineIndicator` — connectivity status banner shown when offline
    - `SyncStatusBadge` — indicator for entities with pending sync
    - `LoadingScreen`, `ErrorScreen`, `EmptyStateScreen` — reusable state screens
    - `PropManagerTextField` — text field with validation error display below field
    - `DatePickerField` — date input with DD/MM/YYYY display format
    - `CurrencyText` — formatted currency display using CurrencyFormatter
    - `ConfirmDeleteDialog` — confirmation dialog in Spanish
    - `SnackbarHost` — for error/success messages
    - _Requirements: 16.1, 16.4, 16.5, 19.2, 19.3, 15.1_

  - [x] 11.3 Define Spanish string resources
    - Create `strings.xml` with all user-facing text in Spanish
    - Navigation labels, form labels, error messages, button text, empty states, offline messages
    - _Requirements: 15.1_

- [ ] 12. Feature: Authentication (`:feature:auth`)
  - [x] 12.1 Implement AuthViewModel
    - Login form state management with email and password fields
    - Call AuthApiService.login(), store token and profile via TokenProvider
    - Handle 401, network errors, validation errors
    - Expose UiState for login screen
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

  - [x] 12.2 Implement LoginScreen composable
    - Email and password text fields with validation
    - Login button with loading state
    - Error display via Snackbar
    - Spanish labels and error messages
    - _Requirements: 1.1, 15.1, 19.1, 19.3_

  - [x] 12.3 Implement auth state management and session handling
    - Observe token presence to determine auth state (logged in vs logged out)
    - Handle 401 responses globally via OkHttp Authenticator — clear token, navigate to login
    - Implement logout: clear token and profile, navigate to login
    - _Requirements: 1.3, 1.4, 1.5_

  - [x] 12.4 Write unit tests for AuthViewModel
    - Test successful login stores token and profile
    - Test failed login shows error message
    - Test 401 clears session
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [ ] 13. Feature: Dashboard (`:feature:dashboard`)
  - [x] 13.1 Implement DashboardViewModel
    - Fetch stats, pagos proximos, contratos calendario, ocupacion tendencia, ingresos comparacion, gastos comparacion from DashboardRepository
    - Cache responses locally via DashboardCache for offline display
    - Show staleness indicator with last update timestamp when offline
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

  - [x] 13.2 Implement DashboardScreen composable
    - Stats cards: total propiedades, inquilinos, contratos activos, pagos pendientes
    - Upcoming payments list with propiedad, inquilino, monto, moneda, fecha_vencimiento
    - Expiring contracts with color indicator based on dias_restantes
    - Occupancy trend chart (monthly rates)
    - Income comparison: esperado, cobrado, diferencia
    - Expense comparison: mes_actual, mes_anterior, porcentaje_cambio
    - Offline cached data with "Última actualización" indicator
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

  - [x] 13.3 Write unit tests for DashboardViewModel
    - Test online data fetch and display
    - Test offline cached data with staleness indicator
    - _Requirements: 9.1, 9.7_

- [ ] 14. Feature: Propiedades (`:feature:propiedades`)
  - [x] 14.1 Implement PropiedadesViewModel
    - Observe paginated propiedad list from repository with filters (ciudad, provincia, tipo_propiedad, estado, precio range)
    - CRUD operations: create, update, delete with form validation via PropiedadValidator
    - Detail view state for single propiedad
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

  - [x] 14.2 Implement PropiedadesListScreen, PropiedadDetailScreen, PropiedadFormScreen composables
    - List: titulo, ciudad, tipo_propiedad, precio, moneda, estado per item; filter controls
    - Detail: all fields including descripcion, direccion, habitaciones, banos, area_m2, imagenes
    - Form: validated input fields for create/edit with Spanish labels and error messages
    - Sync status badge on items with pending changes
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 16.5_

  - [x] 14.3 Write unit tests for PropiedadesViewModel
    - Test filter application, CRUD state transitions, validation errors
    - _Requirements: 3.1, 3.2, 3.7_

- [ ] 15. Feature: Inquilinos (`:feature:inquilinos`)
  - [x] 15.1 Implement InquilinosViewModel
    - Observe searchable, paginated inquilino list from repository
    - Search by nombre, apellido, or cedula
    - CRUD operations with form validation via InquilinoValidator
    - Accept pre-filled data from OCR scanner
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

  - [x] 15.2 Implement InquilinosListScreen, InquilinoFormScreen composables
    - List: nombre, apellido, cedula, telefono per item; search bar
    - Form: validated input fields with "Escanear Cédula" button to launch OCR scanner
    - _Requirements: 4.1, 4.2, 4.3, 4.6, 14.1, 14.2_

  - [x] 15.3 Write unit tests for InquilinosViewModel
    - Test search filtering, CRUD, OCR data pre-fill
    - _Requirements: 4.1, 4.2, 4.6_

- [ ] 16. Feature: Contratos (`:feature:contratos`)
  - [x] 16.1 Implement ContratosViewModel
    - Observe paginated contrato list from repository with propiedad titulo and inquilino nombre resolved
    - CRUD operations with form validation via ContratoValidator (including fecha_fin > fecha_inicio)
    - Renew action: new fecha_fin and monto_mensual
    - Terminate action: fecha_terminacion
    - Expiring contracts view with configurable day threshold (default 30)
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_

  - [x] 16.2 Implement ContratosListScreen, ContratoDetailScreen, ContratoFormScreen composables
    - List: propiedad titulo, inquilino nombre, fecha_inicio, fecha_fin, monto_mensual, moneda, estado
    - Detail: all fields including deposito and timestamps
    - Form: propiedad and inquilino selectors, date pickers, amount input
    - Renew and terminate action dialogs
    - Expiring contracts tab with dias_restantes color indicator
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_

  - [x] 16.3 Write unit tests for ContratosViewModel
    - Test date validation, renew/terminate flows, expiring filter
    - _Requirements: 5.7, 5.8, 5.6_

- [ ] 17. Feature: Pagos (`:feature:pagos`)
  - [x] 17.1 Implement PagosViewModel
    - Observe paginated, filterable pago list from repository (contrato_id, estado, fecha range)
    - CRUD operations with form validation via PagoValidator
    - Receipt download: fetch PDF via API and open with device PDF viewer
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_

  - [x] 17.2 Implement PagosListScreen, PagoFormScreen composables
    - List: contrato reference, monto, moneda, fecha_vencimiento, fecha_pago, metodo_pago, estado; filter controls
    - Form: contrato selector, amount, date pickers, payment method dropdown
    - Receipt download button (online only)
    - _Requirements: 6.1, 6.2, 6.3, 6.6, 6.7_

  - [x] 17.3 Write unit tests for PagosViewModel
    - Test filter application, CRUD, receipt download flow
    - _Requirements: 6.1, 6.2, 6.7_

- [ ] 18. Feature: Gastos (`:feature:gastos`)
  - [x] 18.1 Implement GastosViewModel
    - Observe paginated, filterable gasto list from repository (propiedad_id, categoria, estado, fecha range)
    - CRUD operations with form validation via GastoValidator
    - Category summary fetch from API
    - Accept pre-filled data from OCR receipt scanner
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7_

  - [x] 18.2 Implement GastosListScreen, GastoFormScreen composables
    - List: propiedad reference, categoria, descripcion, monto, moneda, fecha_gasto, estado; filter controls
    - Form: propiedad selector, category dropdown, amount, date picker, "Escanear Recibo" button for OCR
    - Category summary view with breakdown by categoria
    - _Requirements: 7.1, 7.2, 7.3, 7.6, 7.7, 14.3, 14.4_

  - [x] 18.3 Write unit tests for GastosViewModel
    - Test filter application, CRUD, OCR data pre-fill, category summary
    - _Requirements: 7.1, 7.2, 7.7_

- [ ] 19. Feature: Mantenimiento (`:feature:mantenimiento`)
  - [x] 19.1 Implement MantenimientoViewModel
    - Observe paginated, filterable solicitud list from repository (estado, prioridad, propiedad_id)
    - CRUD operations with form validation via SolicitudValidator
    - Status change action
    - Add nota to solicitud
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8_

  - [x] 19.2 Implement MantenimientoListScreen, SolicitudDetailScreen, SolicitudFormScreen composables
    - List: propiedad reference, titulo, estado, prioridad, costo; filter controls
    - Detail: all fields, notas list, add nota input, status change dropdown
    - Form: propiedad selector, titulo, descripcion, prioridad, provider info, cost fields
    - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6, 8.8_

  - [x] 19.3 Write unit tests for MantenimientoViewModel
    - Test filter application, CRUD, status change, add nota
    - _Requirements: 8.1, 8.2, 8.8_

- [x] 20. Checkpoint — Verify all CRUD feature modules compile and tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 21. Feature: Reportes (`:feature:reportes`)
  - [x] 21.1 Implement ReportesViewModel and ReportesScreen
    - Income report with mes/anio parameters: propiedad titulo, inquilino nombre, monto, moneda, estado
    - Profitability report: propiedad titulo, total_ingresos, total_gastos, ingreso_neto, moneda
    - Payment history report with fecha range: contrato reference, monto, fecha_vencimiento, fecha_pago, estado
    - Occupancy trend report: monthly occupancy rates
    - PDF/XLSX export: download and open with device app
    - Offline message: "Los reportes requieren conexión a internet."
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ] 22. Feature: Documentos (`:feature:documentos`)
  - [x] 22.1 Implement DocumentosViewModel and DocumentosScreen
    - List documents for entity (propiedad, inquilino, contrato) with filename, mime_type, file_size, created_at
    - Upload file via multipart POST (online only)
    - Offline messages for upload and viewing
    - _Requirements: 11.1, 11.2, 11.3, 11.4_

- [ ] 23. Feature: Notificaciones (`:feature:notificaciones`)
  - [x] 23.1 Implement NotificacionesViewModel and NotificacionesScreen
    - Fetch overdue payments list: propiedad titulo, inquilino nombre/apellido, monto, moneda, dias_vencido
    - Badge count on navigation item
    - _Requirements: 12.1, 12.3_

- [ ] 24. Feature: Auditoría (`:feature:auditoria`)
  - [x] 24.1 Implement AuditoriaViewModel and AuditoriaScreen
    - Paginated audit log: usuario_id, entity_type, entity_id, accion, created_at
    - Filters: entity_type, fecha range
    - Offline message: "El registro de auditoría requiere conexión a internet."
    - _Requirements: 17.1, 17.2, 17.3_

- [ ] 25. Feature: Perfil (`:feature:perfil`)
  - [x] 25.1 Implement PerfilViewModel and PerfilScreen
    - Display profile: nombre, email, rol
    - Update nombre via PUT /api/perfil
    - Change password with current and new password fields
    - Error messages in Spanish from API response
    - _Requirements: 13.1, 13.2, 13.3, 13.4_

- [ ] 26. Feature: Configuración (`:feature:configuracion`)
  - [x] 26.1 Implement ConfiguracionViewModel and ConfiguracionScreen
    - Display current default currency
    - Update default currency via PUT /api/configuracion/moneda
    - _Requirements: 18.1, 18.2_

- [ ] 27. Feature: Importación (`:feature:importacion`)
  - [x] 27.1 Implement ImportacionViewModel and ImportacionScreen
    - File picker for CSV/XLSX files
    - Import propiedades, inquilinos, gastos via multipart POST
    - Display results: total_filas, exitosos, error details per failed row
    - Offline message: "La importación requiere conexión a internet."
    - _Requirements: 20.1, 20.2, 20.3, 20.4, 20.5_

- [ ] 28. Feature: Scanner — ML Kit OCR (`:feature:scanner`)
  - [x] 28.1 Implement CedulaOcrExtractor
    - Capture camera image, process with ML Kit TextRecognition (on-device only)
    - Parse Dominican cédula layout: extract nombre, apellido, cédula number via regex patterns
    - Return `CedulaOcrResult` with extracted fields and confidence score
    - Handle extraction failure with user-friendly message
    - _Requirements: 14.1, 14.2, 14.5, 14.6_

  - [x] 28.2 Write property test for cédula OCR text parsing
    - **Property 14: Cédula OCR text parsing**
    - **Validates: Requirements 14.1**

  - [x] 28.3 Implement ReceiptOcrExtractor
    - Capture camera image, process with ML Kit TextRecognition (on-device only)
    - Parse receipt: extract monto (currency patterns), fecha, proveedor, numero_factura
    - Return `ReceiptOcrResult` with extracted fields and confidence score
    - Handle extraction failure with user-friendly message
    - _Requirements: 14.3, 14.4, 14.5, 14.6_

  - [x] 28.4 Write property test for receipt OCR text parsing
    - **Property 15: Receipt OCR text parsing**
    - **Validates: Requirements 14.3**

  - [x] 28.5 Implement ScannerScreen composable
    - Camera preview with capture button
    - Processing indicator while ML Kit runs
    - Result preview with extracted fields, allow user to confirm or retake
    - Return extracted data to calling feature screen
    - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.6_

- [x] 29. Checkpoint — Verify all feature modules compile and tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 30. Navigation and app wiring
  - [x] 30.1 Implement Navigation3 graph in `:app`
    - Define `PropManagerNavHost` with nested navigation graphs for each feature
    - Wire auth graph: login → dashboard (on success)
    - Wire bottom nav: Dashboard, Propiedades, Inquilinos, Contratos, Más
    - Wire "Más" overflow menu: Pagos, Gastos, Mantenimiento, Reportes, Documentos, Notificaciones, Auditoría, Perfil, Configuración, Importación
    - Wire scanner routes: invoked from inquilino form (cédula) and gasto form (receipt), return extracted data
    - Handle deep links and back stack correctly
    - _Requirements: 16.1, 16.2_

  - [x] 30.2 Implement MainActivity with auth-gated navigation
    - Check token presence on launch: if authenticated → dashboard, if not → login
    - Host PropManagerNavHost inside PropManagerTheme with Scaffold (top bar, bottom nav, snackbar host)
    - Display OfflineIndicator when NetworkMonitor.isOnline is false
    - _Requirements: 1.3, 16.1, 16.4_

  - [x] 30.3 Wire global error handling
    - Implement centralized error handler for API responses
    - HTTP 401 → clear session, navigate to login
    - HTTP 422 → parse and display field-level errors
    - HTTP 500 → display "Error interno del servidor. Intente nuevamente más tarde."
    - Network errors → display "Sin conexión a internet. Los cambios se guardarán localmente."
    - Other errors → extract message from JSON body, display in Snackbar
    - _Requirements: 19.1, 19.2, 19.3, 19.4, 19.5_

  - [x] 30.4 Wire notification badge on bottom nav
    - Observe overdue payment count from NotificacionesRepository
    - Display badge count on Notificaciones item in "Más" menu
    - _Requirements: 12.3_

  - [x] 30.5 Initialize WorkManager sync scheduling in Application.onCreate
    - Schedule PeriodicRefreshWorker with 15-minute interval
    - Schedule SyncWorker on connectivity change via NetworkMonitor observation
    - _Requirements: 2.4, 2.7, 2.8, 12.2_

- [x] 31. Final checkpoint — Verify full app compiles, all tests pass, navigation works
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation after each major layer
- Property tests validate universal correctness properties using Kotest property-based testing
- Unit tests validate specific examples, edge cases, and ViewModel state transitions
- All UI text, labels, and error messages must be in Spanish
- The implementation language is Kotlin throughout, using Jetpack Compose for UI
