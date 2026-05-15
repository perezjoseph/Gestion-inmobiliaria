# Requirements: Android Feature Parity with Frontend

## Introduction

Bring the PropManager Android app to full feature parity with the Yew/Leptos frontend. This covers two categories of work: (1) wiring 6 existing CRUD feature screens that are built but not connected in the NavHost, and (2) building 4 new feature modules that exist in the frontend but have no Android equivalent. Additionally addresses minor gaps in existing features (unidades, contratos filtering, scanner CameraX integration).

## Glossary

- **NavHost**: The Jetpack Navigation Compose component (`PropManagerNavHost.kt`) that maps routes to composable screens
- **Feature Module**: A Gradle module under `android/feature/` containing screens, ViewModels, and DI for a single domain
- **Plantilla**: A reusable document template with block-based JSON content that can be applied to generate documents for entities
- **Firma**: A digital signature workflow where a document owner requests signatures from external parties via token-based links
- **Chatbot Config**: The WhatsApp AI assistant configuration wizard (7 steps: connection, persona, capabilities, knowledge, sender policy, test, activation)
- **Unidad**: A sub-unit within a propiedad (e.g., apartment 2A in a building) with its own pricing and status

## Requirements

### Requirement 1: Wire Existing CRUD Screens into Navigation

**User Story:** As a gerente, I want to access all property management screens (propiedades, inquilinos, contratos, pagos, gastos, mantenimiento) with full navigation including list, detail, and form screens so that I can manage all data from the app.

#### Acceptance Criteria

1. WHEN the Gerente navigates to the Propiedades route, THE NavHost SHALL render `PropiedadesScreen` with navigation callbacks for detail view (`PropiedadDetailScreen`) and form (`PropiedadFormScreen`) for create/edit
2. WHEN the Gerente navigates to the Inquilinos route, THE NavHost SHALL render `InquilinosScreen` with navigation callbacks for the inquilino form (`InquilinoFormScreen`) for create/edit, and a callback to launch the cédula scanner for OCR prefill
3. WHEN the Gerente navigates to the Contratos route, THE NavHost SHALL render `ContratosScreen` with navigation callbacks for detail view (`ContratoDetailScreen`) and form (`ContratoFormScreen`) for create/edit
4. WHEN the Gerente navigates to the Pagos route, THE NavHost SHALL render `PagosScreen` with navigation callbacks for the pago form (`PagoFormScreen`) for create/edit
5. WHEN the Gerente navigates to the Gastos route, THE NavHost SHALL render `GastosScreen` with navigation callbacks for the gasto form (`GastoFormScreen`) for create/edit, and a callback to launch the receipt scanner for OCR prefill
6. WHEN the Gerente navigates to the Mantenimiento route, THE NavHost SHALL render `MantenimientoScreen` with navigation callbacks for detail view (`SolicitudDetailScreen`) and form (`SolicitudFormScreen`) for create/edit
7. ALL detail and form screens SHALL include a back navigation callback that pops the back stack
8. THE form screens SHALL accept an optional entity ID parameter — when present, the form loads in edit mode; when absent, it loads in create mode

### Requirement 2: Usuarios (User Management) Module

**User Story:** As an admin, I want to manage user accounts from the Android app so that I can change roles and activate/deactivate users without needing a computer.

#### Acceptance Criteria

1. THE App SHALL provide a new `feature/usuarios` module with `UsuariosScreen` and `UsuariosViewModel`
2. WHEN the admin navigates to the Usuarios screen, THE API_Client SHALL request GET /api/usuarios with pagination parameters and display a paginated list showing nombre, email, rol, and activo status for each user
3. WHEN the admin changes a user's role via the role dropdown, THE API_Client SHALL send PUT /api/usuarios/{id}/rol with the new role and update the list upon success
4. WHEN the admin toggles a user's active status, THE API_Client SHALL send PUT /api/usuarios/{id}/toggle-activo and update the list upon success
5. THE Usuarios screen SHALL only be accessible to users with `admin` role — the navigation item SHALL be hidden for `gerente` and `visualizador` roles
6. THE Usuarios route SHALL be added to the `MasMenuScreen` menu items and to the NavHost
7. THE App SHALL create a `UsuariosApiService` in `core/network/api/` with endpoints for list, change role, and toggle active

### Requirement 3: Chatbot Configuration Module

**User Story:** As an admin, I want to configure the WhatsApp AI assistant from the Android app so that I can manage the chatbot settings, test conversations, and review pending receipt confirmations on the go.

#### Acceptance Criteria

1. THE App SHALL provide a new `feature/chatbot` module with a multi-step configuration wizard matching the frontend's 7-step flow
2. WHEN the admin navigates to the Chatbot Config screen, THE API_Client SHALL request GET /chatbot/config and GET /chatbot/status to display the current configuration and connection status
3. THE Connection step SHALL display the WhatsApp connection status (connected/disconnected/connecting), connected phone number, and provide connect/disconnect actions via POST /chatbot/connect and POST /chatbot/disconnect
4. THE Persona step SHALL allow editing display_name, language, tone, greeting, and system_prompt fields, saving via PUT /chatbot/config
5. THE Capabilities step SHALL allow toggling receipt_ocr, balance_queries, payment_reminders, maintenance_requests, and human_handoff capabilities
6. THE Knowledge step SHALL allow managing FAQ entries (add/edit/remove question-answer pairs) and editing policies text
7. THE Sender Policy step SHALL allow selecting sender_policy (all/allowlist/inquilinos_only) and managing the allowlist phone numbers
8. THE Test step SHALL provide a chat interface where the admin can send test messages via POST /chatbot/test and see the AI response and tools invoked
9. THE Activation step SHALL allow enabling/disabling the chatbot via the activo toggle
10. THE App SHALL create a `ChatbotApiService` in `core/network/api/` with all chatbot endpoints (config CRUD, connect, disconnect, status, test, conversations, receipts)
11. THE Chatbot Config route SHALL be accessible from the Configuración screen as a sub-navigation item, matching the frontend's `/configuracion/chatbot` pattern
12. THE Chatbot Config screen SHALL only be accessible to users with `admin` role

### Requirement 4: Plantillas (Document Templates) Module

**User Story:** As a gerente, I want to manage document templates from the Android app so that I can create, edit, and apply templates for contracts, receipts, and other documents.

#### Acceptance Criteria

1. THE App SHALL provide a new `feature/plantillas` module with `PlantillasScreen`, `PlantillaFormScreen`, and `PlantillasViewModel`
2. WHEN the Gerente navigates to the Plantillas screen, THE API_Client SHALL request GET /api/plantillas and display a list showing nombre, tipo_documento, and entity_type for each template
3. WHEN the Gerente creates a new plantilla, THE form SHALL collect nombre, tipo_documento (from predefined list: contrato_arrendamiento, acuerdo, recibo, carta, notificacion, otro), entity_type (propiedad, inquilino, contrato, pago, gasto), and contenido (JSON block content)
4. WHEN the Gerente submits the create form, THE API_Client SHALL send POST /api/plantillas with the template data and refresh the list upon success
5. WHEN the Gerente edits an existing plantilla, THE API_Client SHALL send PUT /api/plantillas/{id} with the updated fields
6. WHEN the Gerente deletes a plantilla, THE API_Client SHALL send DELETE /api/plantillas/{id} after confirmation and refresh the list
7. THE contenido editor SHALL provide a simplified block editor appropriate for mobile (text blocks, heading blocks, variable placeholders) — not a full rich-text editor
8. THE Plantillas route SHALL be added to the `MasMenuScreen` menu items and to the NavHost
9. THE App SHALL create a `PlantillasApiService` in `core/network/api/` with CRUD endpoints

### Requirement 5: Document Editor and Signing Enhancement

**User Story:** As a gerente, I want to edit documents generated from templates and request digital signatures from the Android app so that I can complete the full document workflow on mobile.

#### Acceptance Criteria

1. THE existing `feature/documentos` module SHALL be enhanced with a `DocumentoEditorScreen` that displays and allows editing of `contenido_editable` JSON content
2. WHEN the Gerente opens a document with `contenido_editable` set, THE App SHALL render the block content in a mobile-friendly editor view
3. WHEN the Gerente saves edits, THE API_Client SHALL send PUT /api/documentos/{id}/contenido with the updated `contenido_editable` JSON
4. WHEN the Gerente requests a signature on a document, THE App SHALL display a form to enter firmante_nombre and email, then send POST /api/documentos/{id}/firmas/solicitar
5. THE App SHALL display the list of existing firmas (signatures) for a document with their estado (pendiente, firmado, rechazado)
6. WHEN the Gerente wants to sign a document themselves, THE App SHALL display a signature canvas (touch-based drawing) and send POST /api/documentos/{id}/firmas with the firma_imagen as base64
7. THE DocumentosApiService SHALL be extended with endpoints for contenido update, firma solicitation, firma submission, and firma listing

### Requirement 6: Unidades (Property Units) Support

**User Story:** As a gerente, I want to manage individual units within a property (apartments, offices, etc.) from the Android app so that I can track unit-level pricing, status, and assignments.

#### Acceptance Criteria

1. THE `PropiedadDetailScreen` SHALL display a list of unidades belonging to the propiedad, showing numero_unidad, piso, precio, moneda, and estado for each
2. THE App SHALL provide an inline form or bottom sheet to create/edit unidades within the propiedad detail context
3. WHEN the Gerente creates a unidad, THE API_Client SHALL send POST /api/propiedades/{id}/unidades with the unit data
4. WHEN the Gerente edits a unidad, THE API_Client SHALL send PUT /api/propiedades/{id}/unidades/{unidad_id} with the updated fields
5. WHEN the Gerente deletes a unidad, THE API_Client SHALL send DELETE /api/propiedades/{id}/unidades/{unidad_id} after confirmation
6. THE App SHALL add `Unidad` to `core/model` with fields: id, propiedad_id, numero_unidad, piso, habitaciones, banos, area_m2, precio, moneda, estado, descripcion
7. THE App SHALL add `UnidadDto` and create/update request DTOs to `core/model`
8. THE PropiedadesApiService SHALL be extended with unidad CRUD endpoints

### Requirement 7: Contratos Filter UI

**User Story:** As a gerente, I want to filter the contracts list by status, property, and date range so that I can quickly find specific contracts.

#### Acceptance Criteria

1. THE `ContratosScreen` SHALL display a filter section with estado (activo, vencido, cancelado, finalizado, terminado), propiedad selector, and date range (fecha_inicio, fecha_fin)
2. WHEN the Gerente applies filters, THE App SHALL filter the contratos list from the Local_Database matching the selected criteria
3. WHEN the Gerente clears filters, THE App SHALL reset to showing all contratos
4. THE filter UI SHALL match the pattern used in PagosScreen and GastosScreen (expandable filter section with a "Filtrar" button)

### Requirement 8: Scanner CameraX Integration

**User Story:** As a gerente, I want the document scanner to actually capture photos using the device camera so that OCR extraction works end-to-end.

#### Acceptance Criteria

1. THE `ScannerScreen` SHALL integrate CameraX to display a live camera preview
2. WHEN the Gerente taps the capture button, THE App SHALL capture a high-resolution image from the camera
3. THE captured image SHALL be passed to the appropriate OCR extractor (CedulaOcrExtractor or ReceiptOcrExtractor) based on the scanner mode
4. WHEN OCR extraction completes successfully, THE App SHALL display the extracted fields for review before confirming
5. WHEN OCR extraction fails, THE App SHALL display an error message suggesting better lighting or angle
6. THE App SHALL request CAMERA permission at runtime before showing the camera preview
7. THE CameraX integration SHALL use the ImageCapture use case with CAPTURE_MODE_MAXIMIZE_QUALITY

### Requirement 9: Pending Receipt Confirmations (Chatbot Integration)

**User Story:** As an admin, I want to review and confirm/reject receipt images extracted by the WhatsApp chatbot from the Android app so that I can approve payments on the go.

#### Acceptance Criteria

1. THE Chatbot Config module SHALL include a "Recibos Pendientes" section accessible from the chatbot config screen
2. WHEN the admin navigates to pending receipts, THE API_Client SHALL request GET /chatbot/receipts/pending and display each receipt with bank, amount, currency, date, reference, sender_name, and confidence level
3. WHEN the admin confirms a receipt, THE App SHALL optionally allow selecting a contrato_id to associate the payment, then send POST /chatbot/receipts/{id}/confirm
4. WHEN the admin rejects a receipt, THE App SHALL allow entering an optional rejection_reason, then send POST /chatbot/receipts/{id}/reject
5. THE pending receipts count SHALL be displayed as a badge in the chatbot config navigation item

### Requirement 10: Navigation Updates

**User Story:** As a gerente, I want the "Más" menu and navigation to include all new features so that I can access everything from the app.

#### Acceptance Criteria

1. THE `MasMenuScreen` SHALL add menu items for: Usuarios (admin only), Plantillas, and Chatbot (admin only, under Configuración)
2. THE NavHost SHALL register routes for all new screens: Usuarios list, Plantillas list, Plantilla form, Chatbot config, Documento editor
3. THE Routes object SHALL define constants for all new routes: USUARIOS, PLANTILLAS, PLANTILLA_FORM, CHATBOT_CONFIG, DOCUMENTO_EDITOR
4. THE bottom navigation SHALL remain unchanged (Dashboard, Propiedades, Inquilinos, Contratos, Más)
5. ADMIN-ONLY items in the Más menu SHALL be conditionally shown based on the authenticated user's role from the auth state
