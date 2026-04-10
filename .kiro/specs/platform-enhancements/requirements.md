# Requirements Document

## Introduction

Comprehensive set of platform enhancements to the Dominican Republic real estate property management application (Gestión Inmobiliaria). These enhancements span reporting, notifications, contract lifecycle management, advanced search, user administration, payment receipts, audit logging, dashboard improvements, document uploads, frontend pagination, multi-currency display, offline support, user profiles, and bulk data import. The goal is to close core functional gaps, improve usability for property managers, and add quality-of-life features for day-to-day operations.

## Glossary

- **Sistema**: The Gestión Inmobiliaria application (backend + frontend)
- **Servicio_Reportes**: The backend service responsible for generating reports and aggregating data for income, occupancy, and payment history
- **Servicio_Notificaciones**: The backend service responsible for detecting overdue payments and surfacing alerts
- **Servicio_Contratos**: The backend service managing contract lifecycle including renewal and termination workflows
- **Servicio_Busqueda**: The backend service providing advanced filtering and full-text search across entities
- **Servicio_Usuarios**: The backend service for user administration (listing, role changes, deactivation)
- **Servicio_Recibos**: The backend service responsible for generating printable payment receipts
- **Servicio_Auditoria**: The backend service that records and retrieves audit log entries
- **Servicio_Documentos**: The backend service managing document uploads and retrieval
- **Servicio_Importacion**: The backend service handling bulk CSV/Excel import of properties and tenants
- **Panel_Control**: The frontend dashboard page displaying summary statistics and actionable widgets
- **Interfaz_Busqueda**: The frontend search and filter UI components
- **Interfaz_Usuarios**: The frontend admin user management page
- **Interfaz_Paginacion**: The frontend pagination, sorting, and page-size controls on list views
- **Modulo_PWA**: The frontend Progressive Web App module providing offline caching and installability
- **Modulo_Moneda**: The frontend module responsible for multi-currency display and DOP/USD conversion
- **Pagina_Perfil**: The frontend page where users manage their own profile and password
- **Propiedad**: A rental property record with location, type, pricing, and status
- **Inquilino**: A tenant record identified by cédula (Dominican national ID)
- **Contrato**: A lease agreement linking one Propiedad to one Inquilino with date range and monthly amount
- **Pago**: An individual rent payment tied to a Contrato with due date, payment date, and status
- **Usuario**: A system user with role-based access (admin, gerente, visualizador)
- **Registro_Auditoria**: A single audit log entry recording who changed what entity and when
- **DOP**: Dominican Peso currency
- **USD**: United States Dollar currency

## Requirements

### Requirement 1: Reportes de Ingresos Mensuales

**User Story:** As a property manager, I want to generate monthly income reports per property or tenant, so that I can track revenue and make informed financial decisions.

#### Acceptance Criteria

1. WHEN a user requests an income report for a given month and year, THE Servicio_Reportes SHALL return aggregated income totals grouped by Propiedad, including paid amount, pending amount, and overdue amount for that period.
2. WHEN a user requests an income report filtered by a specific Propiedad, THE Servicio_Reportes SHALL return only payment data associated with Contratos linked to that Propiedad.
3. WHEN a user requests an income report filtered by a specific Inquilino, THE Servicio_Reportes SHALL return only payment data associated with Contratos linked to that Inquilino.
4. THE Servicio_Reportes SHALL calculate occupancy rate as the ratio of Propiedades with estado "ocupada" to total Propiedades, expressed as a percentage with one decimal place.
5. WHEN a user requests a payment history report, THE Servicio_Reportes SHALL return a chronologically ordered list of Pagos for the specified date range, including contrato reference, monto, moneda, fecha_vencimiento, fecha_pago, and estado.

### Requirement 2: Exportación de Reportes a PDF y Excel

**User Story:** As a property manager, I want to export reports to PDF and Excel formats, so that I can share them with property owners and keep offline records.

#### Acceptance Criteria

1. WHEN a user requests a PDF export of an income report, THE Servicio_Reportes SHALL generate a PDF document containing the report title, date range, tabular data with columns for propiedad, inquilino, monto, moneda, estado, and summary totals.
2. WHEN a user requests an Excel export of an income report, THE Servicio_Reportes SHALL generate an XLSX file with the same tabular data and summary totals as the PDF export.
3. THE Servicio_Reportes SHALL include the generation timestamp and the requesting user's name in the header of exported documents.
4. IF the report contains no data for the requested period, THEN THE Servicio_Reportes SHALL return the exported document with an empty data section and a message indicating no records were found.

### Requirement 3: Notificaciones de Pagos Vencidos

**User Story:** As a property manager, I want to see visual alerts for overdue payments on the dashboard, so that I can take immediate action on delinquent accounts.

#### Acceptance Criteria

1. THE Servicio_Notificaciones SHALL identify a Pago as overdue when the Pago estado is "pendiente" and the current date exceeds the fecha_vencimiento.
2. WHEN the Panel_Control loads, THE Panel_Control SHALL display a prominently styled alert section listing all overdue Pagos, showing the Propiedad titulo, Inquilino nombre and apellido, monto, moneda, and the number of days overdue.
3. THE Panel_Control SHALL display a badge on the "Pagos Atrasados" stat card showing the total count of overdue Pagos.
4. WHEN a Pago transitions from "pendiente" to "pagado", THE Servicio_Notificaciones SHALL remove that Pago from the overdue list.
5. THE Panel_Control SHALL sort overdue Pagos by days overdue in descending order, showing the most delinquent payments first.

### Requirement 4: Renovación de Contratos

**User Story:** As a property manager, I want to renew expiring contracts with updated terms, so that I can retain tenants without creating contracts from scratch.

#### Acceptance Criteria

1. WHEN a user initiates a contract renewal for an existing Contrato, THE Servicio_Contratos SHALL create a new Contrato with the same propiedad_id and inquilino_id, a fecha_inicio equal to the day after the original Contrato fecha_fin, and user-specified fecha_fin and monto_mensual.
2. WHEN a contract renewal is created, THE Servicio_Contratos SHALL set the original Contrato estado to "finalizado".
3. THE Servicio_Contratos SHALL validate that the new Contrato date range does not overlap with any other active Contrato for the same Propiedad.
4. IF the original Contrato estado is not "activo", THEN THE Servicio_Contratos SHALL reject the renewal request with an error indicating only active contracts can be renewed.

### Requirement 5: Terminación Anticipada de Contratos

**User Story:** As a property manager, I want to terminate a contract early, so that I can handle tenant departures or lease violations.

#### Acceptance Criteria

1. WHEN a user requests early termination of an active Contrato, THE Servicio_Contratos SHALL set the Contrato estado to "terminado" and update the fecha_fin to the user-specified termination date.
2. THE Servicio_Contratos SHALL set the associated Propiedad estado to "disponible" when a Contrato is terminated and no other active Contrato exists for that Propiedad.
3. IF the specified termination date is before the Contrato fecha_inicio, THEN THE Servicio_Contratos SHALL reject the request with an error indicating the termination date is invalid.
4. IF the Contrato estado is not "activo", THEN THE Servicio_Contratos SHALL reject the termination request with an error indicating only active contracts can be terminated.

### Requirement 6: Alertas de Vencimiento de Contratos

**User Story:** As a property manager, I want to be alerted when contracts are about to expire at 30, 60, and 90 days, so that I can proactively plan renewals or find new tenants.

#### Acceptance Criteria

1. THE Servicio_Contratos SHALL provide an endpoint that returns active Contratos expiring within a user-specified number of days, defaulting to 90 days.
2. THE Panel_Control SHALL display a "Contratos por Vencer" section showing active Contratos grouped into 30-day, 60-day, and 90-day expiration buckets.
3. WHEN a Contrato has 30 or fewer days until fecha_fin, THE Panel_Control SHALL display that Contrato with a warning visual indicator.
4. WHEN a Contrato has 15 or fewer days until fecha_fin, THE Panel_Control SHALL display that Contrato with a critical visual indicator.

### Requirement 7: Filtros Avanzados de Propiedades

**User Story:** As a property manager, I want to filter properties by city, province, type, status, and price range, so that I can quickly find specific properties in my portfolio.

#### Acceptance Criteria

1. WHEN a user applies a ciudad filter, THE Servicio_Busqueda SHALL return only Propiedades matching the specified ciudad value.
2. WHEN a user applies a provincia filter, THE Servicio_Busqueda SHALL return only Propiedades matching the specified provincia value.
3. WHEN a user applies a tipo_propiedad filter, THE Servicio_Busqueda SHALL return only Propiedades matching the specified tipo_propiedad value.
4. WHEN a user applies an estado filter, THE Servicio_Busqueda SHALL return only Propiedades matching the specified estado value.
5. WHEN a user applies a price range filter with precio_min and precio_max, THE Servicio_Busqueda SHALL return only Propiedades with precio between precio_min and precio_max inclusive.
6. WHEN a user applies multiple filters simultaneously, THE Servicio_Busqueda SHALL return only Propiedades matching all specified filter criteria using AND logic.
7. THE Interfaz_Busqueda SHALL display filter controls for ciudad, provincia, tipo_propiedad, estado, and price range on the Propiedades list page.

### Requirement 8: Filtros Avanzados de Pagos

**User Story:** As a property manager, I want to filter payments by status and date range, so that I can quickly review outstanding or historical payments.

#### Acceptance Criteria

1. WHEN a user applies an estado filter on Pagos, THE Servicio_Busqueda SHALL return only Pagos matching the specified estado value.
2. WHEN a user applies a date range filter with fecha_desde and fecha_hasta, THE Servicio_Busqueda SHALL return only Pagos with fecha_vencimiento between fecha_desde and fecha_hasta inclusive.
3. WHEN a user applies a contrato_id filter, THE Servicio_Busqueda SHALL return only Pagos associated with the specified Contrato.
4. THE Interfaz_Busqueda SHALL display filter controls for estado, date range, and contrato on the Pagos list page.

### Requirement 9: Búsqueda de Inquilinos por Nombre o Cédula

**User Story:** As a property manager, I want to search tenants by name or cédula, so that I can quickly locate tenant records.

#### Acceptance Criteria

1. WHEN a user enters a search term, THE Servicio_Busqueda SHALL return Inquilinos where the nombre, apellido, or cedula contains the search term as a case-insensitive substring match.
2. THE Interfaz_Busqueda SHALL display a search input field on the Inquilinos list page.
3. WHEN the search term is fewer than 2 characters, THE Interfaz_Busqueda SHALL display all Inquilinos without filtering.

### Requirement 10: Gestión de Usuarios (Admin)

**User Story:** As an admin, I want to list users, change their roles, and deactivate accounts, so that I can control access to the system.

#### Acceptance Criteria

1. THE Servicio_Usuarios SHALL provide an endpoint that returns a paginated list of all Usuarios, including id, nombre, email, rol, activo, and created_at.
2. WHEN an admin changes a Usuario rol, THE Servicio_Usuarios SHALL update the rol field to the specified value, restricted to "admin", "gerente", or "visualizador".
3. WHEN an admin deactivates a Usuario, THE Servicio_Usuarios SHALL set the activo field to false for that Usuario.
4. WHEN an admin reactivates a Usuario, THE Servicio_Usuarios SHALL set the activo field to true for that Usuario.
5. THE Servicio_Usuarios SHALL restrict all user management endpoints to Usuarios with rol "admin".
6. IF a deactivated Usuario attempts to authenticate, THEN THE Sistema SHALL reject the login attempt with an error indicating the account is inactive.
7. THE Interfaz_Usuarios SHALL display a table of all Usuarios with columns for nombre, email, rol, activo status, and action buttons for role change and deactivation.
8. THE Interfaz_Usuarios SHALL only be accessible to Usuarios with rol "admin".

### Requirement 11: Recibos de Pago

**User Story:** As a property manager, I want to generate printable payment receipts, so that I can provide tenants with proof of payment for tax and legal purposes.

#### Acceptance Criteria

1. WHEN a user requests a receipt for a Pago with estado "pagado", THE Servicio_Recibos SHALL generate a PDF document containing the Propiedad direccion, Inquilino nombre and cedula, Contrato reference, monto, moneda, fecha_pago, metodo_pago, and a unique receipt number.
2. THE Servicio_Recibos SHALL format the receipt with the company header, date in DD/MM/YYYY format, and currency formatted according to Dominican Republic conventions.
3. IF the Pago estado is not "pagado", THEN THE Servicio_Recibos SHALL reject the receipt request with an error indicating receipts can only be generated for completed payments.
4. THE Panel_Control SHALL display a "Generar Recibo" button on each paid Pago row in the Pagos list page.

### Requirement 12: Historial de Cambios (Audit Log)

**User Story:** As an admin, I want to see who changed what and when, so that I can maintain accountability in a multi-user environment.

#### Acceptance Criteria

1. WHEN a user creates, updates, or deletes a Propiedad, Inquilino, Contrato, or Pago, THE Servicio_Auditoria SHALL record a Registro_Auditoria containing the usuario_id, entity type, entity id, action type (crear, actualizar, eliminar), a JSON snapshot of the changed fields, and a timestamp.
2. THE Servicio_Auditoria SHALL provide an endpoint that returns a paginated list of Registro_Auditoria entries, filterable by entity type, entity id, usuario_id, and date range.
3. THE Servicio_Auditoria SHALL restrict audit log viewing to Usuarios with rol "admin".
4. THE Servicio_Auditoria SHALL store audit entries in a dedicated database table with indexes on entity_type, entity_id, and usuario_id.
5. THE Sistema SHALL record audit entries within the same database transaction as the originating operation to guarantee consistency.

### Requirement 13: Dashboard Mejorado

**User Story:** As a property manager, I want an enhanced dashboard with occupancy trends, income comparisons, upcoming due dates, and an expiring contracts calendar, so that I can get a comprehensive overview at a glance.

#### Acceptance Criteria

1. THE Panel_Control SHALL display an occupancy rate trend chart showing monthly occupancy percentages for the last 12 months.
2. THE Panel_Control SHALL display an income comparison widget showing actual collected income versus expected income (sum of monto_mensual for active Contratos) for the current month.
3. THE Panel_Control SHALL display a list of upcoming Pago due dates for the next 30 days, sorted by fecha_vencimiento ascending.
4. THE Panel_Control SHALL display a calendar view showing Contratos with fecha_fin within the next 90 days, color-coded by urgency (green for 60-90 days, yellow for 30-60 days, red for under 30 days).
5. WHEN a user clicks on a calendar entry, THE Panel_Control SHALL navigate to the detail view of that Contrato.

### Requirement 14: Carga de Documentos

**User Story:** As a property manager, I want to attach scanned contracts, cédula copies, and property photos to records, so that I can keep all documentation in one place.

#### Acceptance Criteria

1. WHEN a user uploads a document for a Propiedad, THE Servicio_Documentos SHALL store the file and add the file URL to the Propiedad imagenes JSONB field.
2. WHEN a user uploads a document for an Inquilino, THE Servicio_Documentos SHALL store the file and add the file URL to a documentos JSONB field on the Inquilino record.
3. WHEN a user uploads a document for a Contrato, THE Servicio_Documentos SHALL store the file and add the file URL to a documentos JSONB field on the Contrato record.
4. THE Servicio_Documentos SHALL accept files with MIME types image/jpeg, image/png, application/pdf, with a maximum file size of 10 MB per file.
5. IF a user uploads a file exceeding 10 MB, THEN THE Servicio_Documentos SHALL reject the upload with an error indicating the file size limit.
6. IF a user uploads a file with a disallowed MIME type, THEN THE Servicio_Documentos SHALL reject the upload with an error indicating the allowed file types.
7. THE Sistema SHALL display uploaded documents as a gallery or list on the entity detail view, with the ability to preview images and download PDFs.

### Requirement 15: Paginación y Ordenamiento en el Frontend

**User Story:** As a property manager, I want pagination controls and column sorting on all list views, so that I can efficiently navigate large datasets.

#### Acceptance Criteria

1. THE Interfaz_Paginacion SHALL display page navigation controls (previous, next, page numbers) on the Propiedades, Inquilinos, Contratos, and Pagos list pages.
2. THE Interfaz_Paginacion SHALL display a page size selector allowing the user to choose between 10, 20, and 50 items per page.
3. WHEN a user clicks a column header, THE Interfaz_Paginacion SHALL sort the list by that column in ascending order; clicking the same column header again SHALL toggle to descending order.
4. THE Interfaz_Paginacion SHALL display the total record count and current page range (e.g., "Mostrando 1-20 de 150").
5. THE Interfaz_Paginacion SHALL preserve the current filter and search parameters when changing pages or sort order.

### Requirement 16: Multi-Moneda Mejorada (DOP/USD)

**User Story:** As a property manager, I want to see DOP and USD amounts with conversion display, so that I can understand property values in both currencies commonly used in Dominican Republic real estate.

#### Acceptance Criteria

1. THE Modulo_Moneda SHALL display monetary amounts with the currency code prefix (DOP or USD) and proper thousands/decimal formatting.
2. WHERE a user enables conversion display, THE Modulo_Moneda SHALL show an approximate equivalent amount in the alternate currency next to the original amount.
3. THE Modulo_Moneda SHALL use a configurable exchange rate stored in the application settings, defaulting to a manually set DOP/USD rate.
4. THE Modulo_Moneda SHALL display the exchange rate and its last-updated date in the application settings page.

### Requirement 17: Modo Offline / PWA

**User Story:** As a property manager visiting sites without reliable internet, I want to access key data offline, so that I can review property and tenant information in the field.

#### Acceptance Criteria

1. THE Modulo_PWA SHALL register a service worker that caches the application shell (HTML, CSS, WASM, JS) for offline access.
2. THE Modulo_PWA SHALL cache the most recently viewed Propiedades, Inquilinos, and Contratos data for offline read access.
3. WHEN the device is offline, THE Modulo_PWA SHALL display a visual indicator informing the user that the application is in offline mode.
4. WHEN the device is offline, THE Modulo_PWA SHALL disable create, update, and delete operations and display a message indicating that modifications require an internet connection.
5. THE Modulo_PWA SHALL provide a web app manifest enabling the application to be installed on mobile devices with an appropriate icon and name.

### Requirement 18: Perfil de Usuario

**User Story:** As a user, I want to change my password and update my personal information, so that I can keep my account secure and up to date.

#### Acceptance Criteria

1. WHEN a user submits a password change request with the current password and a new password, THE Sistema SHALL verify the current password, hash the new password, and update the Usuario password_hash.
2. IF the current password provided does not match the stored password_hash, THEN THE Sistema SHALL reject the request with an error indicating the current password is incorrect.
3. WHEN a user updates their nombre or email, THE Sistema SHALL update the corresponding fields on the Usuario record.
4. IF the new email is already in use by another Usuario, THEN THE Sistema SHALL reject the update with an error indicating the email is already registered.
5. THE Pagina_Perfil SHALL display the current user nombre, email, and rol as read-only, with editable fields for nombre, email, and password change.

### Requirement 19: Importación Masiva desde CSV/Excel

**User Story:** As a property manager onboarding an existing portfolio, I want to bulk import properties or tenants from CSV or Excel files, so that I can avoid manual data entry for large datasets.

#### Acceptance Criteria

1. WHEN a user uploads a CSV file with Propiedad data, THE Servicio_Importacion SHALL parse each row and create Propiedad records, validating that required fields (titulo, direccion, ciudad, provincia, tipo_propiedad, precio) are present.
2. WHEN a user uploads a CSV file with Inquilino data, THE Servicio_Importacion SHALL parse each row and create Inquilino records, validating that required fields (nombre, apellido, cedula) are present.
3. WHEN a user uploads an XLSX file, THE Servicio_Importacion SHALL read the first sheet and process rows using the same validation rules as CSV import.
4. IF a row fails validation, THEN THE Servicio_Importacion SHALL skip that row and include it in an error summary returned to the user, specifying the row number and the validation error.
5. THE Servicio_Importacion SHALL return a summary containing the count of successfully imported records and a list of failed rows with error details.
6. IF a CSV contains an Inquilino with a cedula that already exists, THEN THE Servicio_Importacion SHALL skip that row and report it as a duplicate in the error summary.
7. THE Servicio_Importacion SHALL restrict bulk import operations to Usuarios with rol "admin" or "gerente".
