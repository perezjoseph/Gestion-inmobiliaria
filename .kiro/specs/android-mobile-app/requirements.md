# Requirements Document

## Introduction

Native Kotlin Android application for the PropManager property management system in the Dominican Republic. The app targets the gerente (manager) role and provides full CRUD capabilities for properties, tenants, contracts, payments, expenses, and maintenance requests. It consumes the existing Actix-web REST API with JWT authentication, operates offline-first for core data, and leverages on-device AI (ML Kit OCR) for document scanning and data extraction. All UI is in Spanish with Dominican locale conventions (DD/MM/YYYY dates, DOP/USD currency).

## Glossary

- **App**: The PropManager Android application
- **API_Client**: The Retrofit + OkHttp networking layer that communicates with the backend REST API
- **Auth_Manager**: The component responsible for JWT token storage, refresh, and session lifecycle using EncryptedSharedPreferences
- **Local_Database**: The Room database storing offline-first CRUD data on the device
- **Sync_Engine**: The WorkManager-based background synchronization system that reconciles local changes with the backend API
- **OCR_Scanner**: The ML Kit-based on-device component that extracts text from camera images of documents, cédulas, and receipts
- **Navigation_Controller**: The Jetpack Navigation component managing screen transitions and back stack
- **Gerente**: The manager user role with full CRUD permissions on properties, tenants, contracts, payments, expenses, and maintenance
- **Propiedad**: A rental property with location, type, pricing, and status attributes
- **Inquilino**: A tenant identified by cédula (Dominican national ID)
- **Contrato**: A lease agreement linking one propiedad to one inquilino with date range and monthly amount
- **Pago**: An individual rent payment tied to a contrato with due date, payment date, and status
- **Gasto**: An expense record associated with a propiedad, categorized and tracked with amount and currency
- **Solicitud_Mantenimiento**: A maintenance request for a propiedad with priority, status, provider info, and notes
- **Sync_Queue**: The local queue of pending create/update/delete operations awaiting synchronization with the backend
- **Conflict_Resolution**: The strategy applied when a local offline edit conflicts with a server-side change (server-wins with local notification)

## Requirements

### Requirement 1: Authentication and Session Management

**User Story:** As a gerente, I want to log in with my email and password so that I can securely access the property management system from my phone.

#### Acceptance Criteria

1. WHEN the Gerente submits valid email and password credentials, THE Auth_Manager SHALL send a POST request to /api/auth/login and store the returned JWT token in EncryptedSharedPreferences
2. WHEN the Auth_Manager receives a successful login response, THE App SHALL extract the user profile (id, nombre, email, rol) from the response and persist it locally
3. WHEN the Auth_Manager detects that the stored JWT token has expired, THE App SHALL redirect the Gerente to the login screen and clear all stored session data
4. WHEN the API_Client receives a 401 Unauthorized response on any request, THE Auth_Manager SHALL clear the stored token and redirect the Gerente to the login screen
5. WHEN the Gerente taps the logout action, THE Auth_Manager SHALL remove the JWT token and user profile from EncryptedSharedPreferences and navigate to the login screen
6. THE Auth_Manager SHALL store the JWT token exclusively in EncryptedSharedPreferences, never in plain SharedPreferences or local files
7. WHILE the Gerente is authenticated, THE API_Client SHALL attach the JWT token as a Bearer token in the Authorization header of every API request

### Requirement 2: Offline-First Data Architecture

**User Story:** As a gerente, I want to view and edit property data even without internet connectivity so that I can work from remote property locations.

#### Acceptance Criteria

1. THE Local_Database SHALL store all core entity data (propiedades, inquilinos, contratos, pagos, gastos, solicitudes de mantenimiento) using Room with a schema that mirrors the backend API response models
2. WHEN the App loads any entity list or detail screen, THE App SHALL read data from the Local_Database first and display it immediately
3. WHEN the Gerente creates, updates, or deletes an entity while offline, THE Sync_Queue SHALL record the operation with entity type, entity ID, operation type, payload, and timestamp
4. WHEN network connectivity is restored, THE Sync_Engine SHALL process all pending operations in the Sync_Queue in chronological order using WorkManager
5. WHEN the Sync_Engine successfully synchronizes an operation, THE Sync_Engine SHALL remove the operation from the Sync_Queue and update the Local_Database with the server response
6. IF the Sync_Engine encounters a conflict (HTTP 409) during synchronization, THEN THE Conflict_Resolution SHALL apply a server-wins strategy, update the Local_Database with the server version, and display a notification to the Gerente describing the conflict
7. IF the Sync_Engine encounters a network error during synchronization, THEN THE Sync_Engine SHALL retain the operation in the Sync_Queue and schedule a retry with exponential backoff using WorkManager
8. WHILE the device has network connectivity, THE Sync_Engine SHALL perform a full data refresh from the backend API at a configurable interval (default 15 minutes)
9. THE Local_Database SHALL NOT store documents, PDFs, report files, or analytics data locally

### Requirement 3: Property Management

**User Story:** As a gerente, I want to manage my property portfolio from my phone so that I can add, edit, and review properties on the go.

#### Acceptance Criteria

1. THE App SHALL display a paginated list of propiedades from the Local_Database with titulo, ciudad, tipo_propiedad, precio, moneda, and estado visible for each item
2. WHEN the Gerente applies filters (ciudad, provincia, tipo_propiedad, estado, precio range), THE App SHALL filter the propiedad list from the Local_Database matching the selected criteria
3. WHEN the Gerente taps on a propiedad in the list, THE App SHALL navigate to a detail screen showing all propiedad fields including descripcion, direccion, habitaciones, banos, area_m2, and imagenes
4. WHEN the Gerente submits the create propiedad form with valid data, THE App SHALL insert the propiedad into the Local_Database and enqueue a POST /api/propiedades operation in the Sync_Queue
5. WHEN the Gerente submits the edit propiedad form with valid changes, THE App SHALL update the propiedad in the Local_Database and enqueue a PUT /api/propiedades/{id} operation in the Sync_Queue
6. WHEN the Gerente confirms deletion of a propiedad, THE App SHALL mark the propiedad as deleted in the Local_Database and enqueue a DELETE /api/propiedades/{id} operation in the Sync_Queue
7. THE App SHALL validate that titulo, direccion, ciudad, provincia, tipo_propiedad, and precio are provided before allowing propiedad creation or update

### Requirement 4: Tenant Management

**User Story:** As a gerente, I want to manage tenant records from my phone so that I can register new tenants and update their information in the field.

#### Acceptance Criteria

1. THE App SHALL display a searchable, paginated list of inquilinos from the Local_Database with nombre, apellido, cedula, and telefono visible for each item
2. WHEN the Gerente enters a search term, THE App SHALL filter the inquilino list by nombre, apellido, or cedula matching the search term
3. WHEN the Gerente submits the create inquilino form with valid data, THE App SHALL insert the inquilino into the Local_Database and enqueue a POST /api/inquilinos operation in the Sync_Queue
4. WHEN the Gerente submits the edit inquilino form with valid changes, THE App SHALL update the inquilino in the Local_Database and enqueue a PUT /api/inquilinos/{id} operation in the Sync_Queue
5. WHEN the Gerente confirms deletion of an inquilino, THE App SHALL mark the inquilino as deleted in the Local_Database and enqueue a DELETE /api/inquilinos/{id} operation in the Sync_Queue
6. THE App SHALL validate that nombre, apellido, and cedula are provided before allowing inquilino creation

### Requirement 5: Contract Management

**User Story:** As a gerente, I want to manage lease contracts from my phone so that I can create, renew, and terminate contracts while visiting properties.

#### Acceptance Criteria

1. THE App SHALL display a paginated list of contratos from the Local_Database with propiedad titulo, inquilino nombre, fecha_inicio, fecha_fin, monto_mensual, moneda, and estado visible for each item
2. WHEN the Gerente taps on a contrato in the list, THE App SHALL navigate to a detail screen showing all contrato fields including deposito and timestamps
3. WHEN the Gerente submits the create contrato form with valid data (propiedad_id, inquilino_id, fecha_inicio, fecha_fin, monto_mensual), THE App SHALL insert the contrato into the Local_Database and enqueue a POST /api/contratos operation in the Sync_Queue
4. WHEN the Gerente submits the renew contrato action with a new fecha_fin and monto_mensual, THE App SHALL update the contrato in the Local_Database and enqueue a POST /api/contratos/{id}/renovar operation in the Sync_Queue
5. WHEN the Gerente submits the terminate contrato action with a fecha_terminacion, THE App SHALL update the contrato estado to terminated in the Local_Database and enqueue a POST /api/contratos/{id}/terminar operation in the Sync_Queue
6. WHEN the Gerente navigates to the contracts-expiring view, THE App SHALL display contratos with fecha_fin within a configurable number of days (default 30) using data from the Local_Database
7. THE App SHALL validate that propiedad_id, inquilino_id, fecha_inicio, fecha_fin, and monto_mensual are provided before allowing contrato creation
8. THE App SHALL validate that fecha_fin is after fecha_inicio when creating or renewing a contrato

### Requirement 6: Payment Management

**User Story:** As a gerente, I want to record and track rent payments from my phone so that I can manage collections while visiting tenants.

#### Acceptance Criteria

1. THE App SHALL display a paginated, filterable list of pagos from the Local_Database with contrato reference, monto, moneda, fecha_vencimiento, fecha_pago, metodo_pago, and estado visible for each item
2. WHEN the Gerente applies filters (contrato_id, estado, fecha range), THE App SHALL filter the pago list from the Local_Database matching the selected criteria
3. WHEN the Gerente submits the create pago form with valid data, THE App SHALL insert the pago into the Local_Database and enqueue a POST /api/pagos operation in the Sync_Queue
4. WHEN the Gerente submits the edit pago form with valid changes, THE App SHALL update the pago in the Local_Database and enqueue a PUT /api/pagos/{id} operation in the Sync_Queue
5. WHEN the Gerente confirms deletion of a pago, THE App SHALL mark the pago as deleted in the Local_Database and enqueue a DELETE /api/pagos/{id} operation in the Sync_Queue
6. WHEN the Gerente requests a receipt for a pago and the device has network connectivity, THE API_Client SHALL request GET /api/pagos/{id}/recibo and open the returned PDF in the device PDF viewer
7. THE App SHALL validate that contrato_id, monto, and fecha_vencimiento are provided before allowing pago creation

### Requirement 7: Expense Management

**User Story:** As a gerente, I want to track property expenses from my phone so that I can record costs immediately when they occur.

#### Acceptance Criteria

1. THE App SHALL display a paginated, filterable list of gastos from the Local_Database with propiedad reference, categoria, descripcion, monto, moneda, fecha_gasto, and estado visible for each item
2. WHEN the Gerente applies filters (propiedad_id, categoria, estado, fecha range), THE App SHALL filter the gasto list from the Local_Database matching the selected criteria
3. WHEN the Gerente submits the create gasto form with valid data, THE App SHALL insert the gasto into the Local_Database and enqueue a POST /api/gastos operation in the Sync_Queue
4. WHEN the Gerente submits the edit gasto form with valid changes, THE App SHALL update the gasto in the Local_Database and enqueue a PUT /api/gastos/{id} operation in the Sync_Queue
5. WHEN the Gerente confirms deletion of a gasto, THE App SHALL mark the gasto as deleted in the Local_Database and enqueue a DELETE /api/gastos/{id} operation in the Sync_Queue
6. WHEN the Gerente requests the category summary for a propiedad, THE API_Client SHALL request GET /api/gastos/resumen-categorias and display the breakdown by categoria with total and cantidad
7. THE App SHALL validate that propiedad_id, categoria, descripcion, monto, moneda, and fecha_gasto are provided before allowing gasto creation

### Requirement 8: Maintenance Request Management

**User Story:** As a gerente, I want to manage maintenance requests from my phone so that I can create, update, and track repair work while on-site.

#### Acceptance Criteria

1. THE App SHALL display a paginated, filterable list of solicitudes de mantenimiento from the Local_Database with propiedad reference, titulo, estado, prioridad, and costo visible for each item
2. WHEN the Gerente applies filters (estado, prioridad, propiedad_id), THE App SHALL filter the solicitud list from the Local_Database matching the selected criteria
3. WHEN the Gerente submits the create solicitud form with valid data, THE App SHALL insert the solicitud into the Local_Database and enqueue a POST /api/mantenimiento operation in the Sync_Queue
4. WHEN the Gerente submits the edit solicitud form with valid changes, THE App SHALL update the solicitud in the Local_Database and enqueue a PUT /api/mantenimiento/{id} operation in the Sync_Queue
5. WHEN the Gerente changes the estado of a solicitud, THE App SHALL update the estado in the Local_Database and enqueue a PUT /api/mantenimiento/{id}/estado operation in the Sync_Queue
6. WHEN the Gerente adds a nota to a solicitud, THE App SHALL insert the nota in the Local_Database and enqueue a POST /api/mantenimiento/{id}/notas operation in the Sync_Queue
7. WHEN the Gerente confirms deletion of a solicitud, THE App SHALL mark the solicitud as deleted in the Local_Database and enqueue a DELETE /api/mantenimiento/{id} operation in the Sync_Queue
8. THE App SHALL validate that propiedad_id and titulo are provided before allowing solicitud creation


### Requirement 9: Dashboard

**User Story:** As a gerente, I want to see a summary dashboard on my phone so that I can quickly assess the state of my property portfolio.

#### Acceptance Criteria

1. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE API_Client SHALL request GET /api/dashboard/stats and display total propiedades, inquilinos, contratos activos, and pagos pendientes
2. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE App SHALL display upcoming payments from GET /api/dashboard/pagos-proximos with propiedad titulo, inquilino nombre, monto, moneda, and fecha_vencimiento
3. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE App SHALL display expiring contracts from GET /api/dashboard/contratos-calendario with propiedad titulo, inquilino nombre, fecha_fin, dias_restantes, and color indicator
4. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE App SHALL display occupancy trend data from GET /api/dashboard/ocupacion-tendencia as a chart showing monthly occupancy rates
5. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE App SHALL display income comparison data from GET /api/dashboard/ingresos-comparacion showing esperado, cobrado, and diferencia
6. WHEN the Gerente opens the dashboard screen and the device has network connectivity, THE App SHALL display expense comparison data from GET /api/dashboard/gastos-comparacion showing mes_actual, mes_anterior, and porcentaje_cambio
7. WHILE the device lacks network connectivity, THE App SHALL display a cached version of the last successfully loaded dashboard data with a visible indicator showing the data staleness

### Requirement 10: Reports

**User Story:** As a gerente, I want to view financial reports on my phone so that I can analyze property performance without needing a computer.

#### Acceptance Criteria

1. WHEN the Gerente requests an income report with mes and anio parameters, THE API_Client SHALL request GET /api/reportes/ingresos and display the report rows with propiedad titulo, inquilino nombre, monto, moneda, and estado
2. WHEN the Gerente requests a profitability report with mes and anio parameters, THE API_Client SHALL request GET /api/reportes/rentabilidad and display rows with propiedad titulo, total_ingresos, total_gastos, ingreso_neto, and moneda
3. WHEN the Gerente requests a payment history report with fecha_desde and fecha_hasta, THE API_Client SHALL request GET /api/reportes/historial-pagos and display entries with contrato reference, monto, fecha_vencimiento, fecha_pago, and estado
4. WHEN the Gerente requests an occupancy trend report, THE API_Client SHALL request GET /api/reportes/ocupacion/tendencia and display monthly occupancy rates
5. WHEN the Gerente requests a PDF or XLSX export of a report and the device has network connectivity, THE API_Client SHALL request the corresponding /pdf or /xlsx endpoint and open the downloaded file with the appropriate device application
6. WHILE the device lacks network connectivity, THE App SHALL display a message indicating that reports require an active internet connection

### Requirement 11: Document Management

**User Story:** As a gerente, I want to upload and view documents associated with properties, tenants, and contracts from my phone so that I can manage paperwork digitally.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the documents section of an entity (propiedad, inquilino, contrato), THE API_Client SHALL request GET /api/documentos/{entity_type}/{entity_id} and display the list of documents with filename, mime_type, file_size, and created_at
2. WHEN the Gerente selects a file to upload for an entity and the device has network connectivity, THE API_Client SHALL send a multipart POST to /api/documentos/{entity_type}/{entity_id} and refresh the document list upon success
3. IF the Gerente attempts to upload a document while the device lacks network connectivity, THEN THE App SHALL display a message indicating that document uploads require an active internet connection
4. WHILE the device lacks network connectivity, THE App SHALL display a message indicating that document viewing requires an active internet connection

### Requirement 12: Notifications

**User Story:** As a gerente, I want to see overdue payment notifications so that I can follow up on late payments promptly.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the notifications screen and the device has network connectivity, THE API_Client SHALL request GET /api/notificaciones/pagos-vencidos and display each overdue payment with propiedad titulo, inquilino nombre and apellido, monto, moneda, and dias_vencido
2. WHEN the Sync_Engine performs a background sync, THE Sync_Engine SHALL check for overdue payments and display an Android system notification if new overdue payments are detected
3. THE App SHALL display a badge count on the notifications navigation item indicating the number of overdue payments

### Requirement 13: Profile Management

**User Story:** As a gerente, I want to view and update my profile and change my password from my phone.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the profile screen, THE API_Client SHALL request GET /api/perfil and display the user profile with nombre, email, and rol
2. WHEN the Gerente submits profile updates (nombre), THE API_Client SHALL send PUT /api/perfil with the updated fields and update the locally stored user profile upon success
3. WHEN the Gerente submits a password change with current password and new password, THE API_Client SHALL send PUT /api/perfil/password and display a success or error message based on the response
4. IF the password change request returns an error, THEN THE App SHALL display the error message from the API response in Spanish

### Requirement 14: On-Device AI — OCR Document Scanning

**User Story:** As a gerente, I want to scan documents, cédulas, and receipts with my phone camera so that I can extract data automatically instead of typing it manually.

#### Acceptance Criteria

1. WHEN the Gerente activates the OCR scanner from the inquilino creation form, THE OCR_Scanner SHALL capture an image of a Dominican cédula and extract nombre, apellido, and cedula number using ML Kit text recognition
2. WHEN the OCR_Scanner extracts text from a cédula image, THE App SHALL pre-fill the inquilino form fields (nombre, apellido, cedula) with the extracted values and allow the Gerente to review and correct before saving
3. WHEN the Gerente activates the OCR scanner from the gasto creation form, THE OCR_Scanner SHALL capture an image of a receipt or invoice and extract monto, fecha, proveedor, and numero_factura using ML Kit text recognition
4. WHEN the OCR_Scanner extracts text from a receipt image, THE App SHALL pre-fill the gasto form fields with the extracted values and allow the Gerente to review and correct before saving
5. THE OCR_Scanner SHALL perform all text recognition on-device using ML Kit without sending images to external servers
6. IF the OCR_Scanner fails to extract readable text from an image, THEN THE App SHALL display a message suggesting the Gerente retake the photo with better lighting or angle

### Requirement 15: Localization and Formatting

**User Story:** As a gerente in the Dominican Republic, I want the app to display all text in Spanish with Dominican date and currency conventions so that the interface feels natural.

#### Acceptance Criteria

1. THE App SHALL display all user-facing text, labels, error messages, and navigation items in Spanish
2. THE App SHALL format all dates in DD/MM/YYYY format for display and convert to YYYY-MM-DD format when sending to the API
3. THE App SHALL format currency values with the appropriate symbol (RD$ for DOP, US$ for USD) and thousands separators
4. THE App SHALL send all API requests with dates in YYYY-MM-DD format and datetimes in ISO8601 format as expected by the backend
5. THE App SHALL parse all API response dates from YYYY-MM-DD format and datetimes from ISO8601 format

### Requirement 16: Navigation and UI Architecture

**User Story:** As a gerente, I want intuitive navigation between all app sections so that I can quickly access any feature.

#### Acceptance Criteria

1. THE App SHALL provide a bottom navigation bar with items for Dashboard, Propiedades, Inquilinos, Contratos, and a "Más" (More) menu
2. WHEN the Gerente taps the "Más" menu item, THE App SHALL display a menu with access to Pagos, Gastos, Mantenimiento, Reportes, Documentos, Notificaciones, Auditoría, and Perfil
3. THE App SHALL use Jetpack Compose for all UI screens with Material Design 3 components
4. THE App SHALL display a connectivity status indicator when the device is offline
5. THE App SHALL display sync status indicators on entities that have pending changes in the Sync_Queue

### Requirement 17: Audit Log Viewing

**User Story:** As a gerente, I want to view the audit log from my phone so that I can track changes made to the system.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the audit log screen and the device has network connectivity, THE API_Client SHALL request GET /api/auditoria and display a paginated list of audit entries with usuario_id, entity_type, entity_id, accion, and created_at
2. WHEN the Gerente applies filters (entity_type, fecha range), THE API_Client SHALL include the filter parameters in the GET /api/auditoria request
3. WHILE the device lacks network connectivity, THE App SHALL display a message indicating that the audit log requires an active internet connection

### Requirement 18: Currency Configuration

**User Story:** As a gerente, I want to view and set the default currency so that new records use the correct currency.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the currency configuration screen and the device has network connectivity, THE API_Client SHALL request GET /api/configuracion/moneda and display the current default currency
2. WHEN the Gerente updates the default currency, THE API_Client SHALL send PUT /api/configuracion/moneda with the selected currency and update the locally cached configuration upon success

### Requirement 19: Error Handling and User Feedback

**User Story:** As a gerente, I want clear error messages in Spanish so that I understand what went wrong and how to fix it.

#### Acceptance Criteria

1. WHEN the API_Client receives an error response from the backend, THE App SHALL extract the "message" field from the JSON error body and display it to the Gerente in a Snackbar or dialog
2. WHEN the App detects a network timeout or connection failure, THE App SHALL display "Sin conexión a internet. Los cambios se guardarán localmente." as a Snackbar message
3. WHEN the App detects a form validation error, THE App SHALL highlight the invalid field and display a descriptive error message in Spanish below the field
4. IF the API_Client receives an HTTP 422 validation error, THEN THE App SHALL parse the error message and display it associated with the relevant form field
5. IF the API_Client receives an HTTP 500 error, THEN THE App SHALL display "Error interno del servidor. Intente nuevamente más tarde."

### Requirement 20: Data Import

**User Story:** As a gerente, I want to import property, tenant, and expense data from CSV or XLSX files on my phone so that I can bulk-load data without a computer.

#### Acceptance Criteria

1. WHEN the Gerente selects a CSV or XLSX file for property import and the device has network connectivity, THE API_Client SHALL send the file as a multipart POST to /api/importar/propiedades and display the import result with total_filas, exitosos, and any errors
2. WHEN the Gerente selects a CSV or XLSX file for tenant import and the device has network connectivity, THE API_Client SHALL send the file as a multipart POST to /api/importar/inquilinos and display the import result
3. WHEN the Gerente selects a CSV or XLSX file for expense import and the device has network connectivity, THE API_Client SHALL send the file as a multipart POST to /api/importar/gastos and display the import result
4. IF any import rows fail, THEN THE App SHALL display the fila number and error message for each failed row
5. WHILE the device lacks network connectivity, THE App SHALL display a message indicating that data import requires an active internet connection
