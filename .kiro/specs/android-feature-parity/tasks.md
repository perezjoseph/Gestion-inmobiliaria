# Implementation Plan: Android Feature Parity

## Overview

Bring the Android app to full parity with the frontend. Work is ordered: (1) wire existing CRUD screens into NavHost (immediate value, no new code), (2) add missing data models and API services, (3) build new feature modules, (4) enhance existing features, (5) integrate CameraX for scanner.

## Tasks

- [x] 1. Wire existing CRUD screens into NavHost
  - [x] 1.1 Wire Propiedades screens (list, detail, form)
    - Replace placeholder comment in `PropManagerNavHost.kt` with `PropiedadesScreen`, `PropiedadDetailScreen`, `PropiedadFormScreen` composables
    - Add proper imports from `com.propmanager.feature.propiedades`
    - Wire navigation callbacks: onNavigateToDetail, onNavigateToForm, onNavigateToDocumentos, onNavigateBack
    - Handle route arguments for `propiedades/{id}` and `propiedades/form?id={id}`
    - _Requirements: 1.1, 1.7, 1.8_

  - [x] 1.2 Wire Inquilinos screens (list, form)
    - Replace placeholder comment with `InquilinosScreen` composable
    - Wire navigation callbacks: onNavigateToForm, onNavigateToScanner (cedula), onNavigateBack
    - Handle savedStateHandle for receiving OCR results from scanner
    - Add route for `inquilinos/form?id={id}` with `InquilinoFormScreen` if separate, or handle inline
    - _Requirements: 1.2, 1.7, 1.8_

  - [x] 1.3 Wire Contratos screens (list, detail, form)
    - Replace placeholder comment with `ContratosScreen`, `ContratoDetailScreen`, `ContratoFormScreen` composables
    - Wire navigation callbacks: onNavigateToDetail, onNavigateToForm, onNavigateBack
    - Handle route arguments for `contratos/{id}` and `contratos/form?id={id}`
    - _Requirements: 1.3, 1.7, 1.8_

  - [x] 1.4 Wire Pagos screens (list, form)
    - Replace placeholder comment with `PagosScreen` composable
    - Wire navigation callbacks: onNavigateToForm, onNavigateBack
    - Handle route for `pagos/form?id={id}` with `PagoFormScreen`
    - _Requirements: 1.4, 1.7, 1.8_

  - [x] 1.5 Wire Gastos screens (list, form)
    - Replace placeholder comment with `GastosScreen` composable
    - Wire navigation callbacks: onNavigateToForm, onNavigateToScanner (receipt), onNavigateBack
    - Handle savedStateHandle for receiving OCR results from scanner
    - Handle route for `gastos/form?id={id}` with `GastoFormScreen`
    - _Requirements: 1.5, 1.7, 1.8_

  - [x] 1.6 Wire Mantenimiento screens (list, detail, form)
    - Replace placeholder comment with `MantenimientoScreen`, `SolicitudDetailScreen`, `SolicitudFormScreen` composables
    - Wire navigation callbacks: onNavigateToDetail, onNavigateToForm, onNavigateBack
    - Handle route arguments for `mantenimiento/{id}` and `mantenimiento/form?id={id}`
    - _Requirements: 1.6, 1.7, 1.8_

- [x] 2. Checkpoint — Verify all CRUD navigation works
  - Build the app and verify all 6 CRUD features navigate correctly between list, detail, and form screens. Ask user to confirm on device if needed.

- [x] 3. Add Unidad model and API support
  - [x] 3.1 Create Unidad domain model in `core/model`
    - Add `Unidad.kt` data class with: id, propiedadId, numeroUnidad, piso, habitaciones, banos, areaM2, precio (BigDecimal), moneda, estado, descripcion
    - _Requirements: 6.6_

  - [x] 3.2 Create Unidad DTOs in `core/model`
    - Add `UnidadDtos.kt` with `UnidadDto`, `CreateUnidadRequest`, `UpdateUnidadRequest` using `@Serializable` and `@SerialName` for camelCase mapping
    - _Requirements: 6.7_

  - [x] 3.3 Extend PropiedadesApiService with unidad endpoints
    - Add `getUnidades(propiedadId)`, `createUnidad(propiedadId, request)`, `updateUnidad(propiedadId, unidadId, request)`, `deleteUnidad(propiedadId, unidadId)`
    - _Requirements: 6.8_

  - [x] 3.4 Add Unidades section to PropiedadDetailScreen
    - Display list of unidades in the propiedad detail screen
    - Add create/edit bottom sheet or inline form for unidad management
    - Add delete with confirmation dialog
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 4. Usuarios feature module
  - [x] 4.1 Create `feature/usuarios` module scaffolding
    - Create `android/feature/usuarios/build.gradle.kts` with standard feature module dependencies
    - Register module in `settings.gradle.kts`
    - Add dependency in `:app` module's `build.gradle.kts`
    - _Requirements: 2.1_

  - [x] 4.2 Create UsuariosApiService in `core/network/api/`
    - Implement `getUsuarios(page, perPage)`, `changeRole(id, rol)`, `toggleActivo(id)` using Retrofit
    - Add to NetworkModule DI
    - _Requirements: 2.7_

  - [x] 4.3 Create Usuario DTOs in `core/model`
    - Add `UsuarioDtos.kt` with `UserListDto`, `ChangeRoleRequest`, pagination response types
    - Reuse existing `UserDto` if compatible, or create admin-specific DTO
    - _Requirements: 2.2_

  - [x] 4.4 Create UsuariosViewModel
    - Implement sealed `UsuariosUiState` (Loading, Success with ImmutableList + pagination, Error)
    - Implement `loadUsuarios(page)`, `changeRole(userId, newRole)`, `toggleActivo(userId)`
    - Handle loading/error states for each action
    - _Requirements: 2.2, 2.3, 2.4_

  - [x] 4.5 Create UsuariosScreen composable
    - Paginated list with nombre, email, rol dropdown, activo toggle
    - Role dropdown: admin, gerente, visualizador
    - Active toggle with confirmation
    - Loading/error states
    - _Requirements: 2.2, 2.3, 2.4_

  - [x] 4.6 Add Usuarios to navigation
    - Add `USUARIOS` route to `Routes.kt`
    - Add composable to NavHost with role guard (redirect non-admin)
    - Add conditional menu item to `MasMenuScreen` (visible only for admin role)
    - _Requirements: 2.5, 2.6_

- [x] 5. Chatbot configuration feature module
  - [x] 5.1 Create `feature/chatbot` module scaffolding
    - Create `android/feature/chatbot/build.gradle.kts`
    - Register module in `settings.gradle.kts`
    - Add dependency in `:app` module
    - _Requirements: 3.1_

  - [x] 5.2 Create ChatbotApiService in `core/network/api/`
    - Implement all chatbot endpoints: getConfig, updateConfig, getStatus, connect, disconnect, testChat, listConversations, getPendingReceipts, confirmReceipt, rejectReceipt
    - Add to NetworkModule DI
    - _Requirements: 3.10_

  - [x] 5.3 Create Chatbot DTOs in `core/model`
    - Add `ChatbotDtos.kt` with: ChatbotConfigResponse, ChatbotConfigUpdateRequest, ConnectionStatusResponse, Capabilities, FaqEntry, TestChatRequest, TestChatResponse, ConversationListItem, ReceiptExtractionResponse, ReceiptConfirmRequest, ReceiptRejectRequest
    - All with `@Serializable` and `@SerialName("camelCase")` annotations
    - _Requirements: 3.2_

  - [x] 5.4 Create ChatbotConfigViewModel
    - Sealed `ChatbotConfigUiState` with Loading, Success (config + status + currentStep), Error
    - Step navigation: next/previous with validation
    - Actions: loadConfig, updateConfig, connect, disconnect, testChat, loadPendingReceipts, confirmReceipt, rejectReceipt
    - _Requirements: 3.2 through 3.9_

  - [x] 5.5 Create ConnectionStep composable
    - Display connection status (color-coded: green/yellow/red)
    - Show connected phone number when connected
    - Connect/Disconnect buttons
    - QR code display when connecting (if provided by API)
    - _Requirements: 3.3_

  - [x] 5.6 Create PersonaStep composable
    - Form fields: display_name, language selector, tone selector, greeting (multiline), system_prompt (multiline)
    - Save button that calls updateConfig with persona fields
    - _Requirements: 3.4_

  - [x] 5.7 Create CapabilitiesStep composable
    - Toggle switches for: receipt_ocr, balance_queries, payment_reminders, maintenance_requests, human_handoff
    - Save on toggle change
    - _Requirements: 3.5_

  - [x] 5.8 Create KnowledgeStep composable
    - FAQ list with add/edit/remove (question + answer pairs)
    - Policies text field (multiline)
    - _Requirements: 3.6_

  - [x] 5.9 Create SenderPolicyStep composable
    - Radio buttons: all, allowlist, inquilinos_only
    - When allowlist selected: show phone number list with add/remove
    - _Requirements: 3.7_

  - [x] 5.10 Create TestChatStep composable
    - Chat-like interface: message input + send button
    - Display AI response and tools_invoked list
    - Scrollable message history for the test session
    - _Requirements: 3.8_

  - [x] 5.11 Create ActivationStep composable
    - Large toggle switch for activo status
    - Summary of current configuration
    - Warning text when activating
    - _Requirements: 3.9_

  - [x] 5.12 Create ChatbotConfigScreen (wizard container)
    - Step indicator (horizontal stepper showing 7 steps)
    - Content area rendering current step composable
    - Next/Previous navigation buttons
    - _Requirements: 3.1_

  - [x] 5.13 Create PendingReceiptsScreen composable
    - List of pending receipt extractions with bank, amount, currency, date, confidence
    - Confirm action: optional contrato selector → POST confirm
    - Reject action: optional reason text field → POST reject
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

  - [x] 5.14 Add Chatbot to navigation
    - Add `CHATBOT_CONFIG` and `CHATBOT_RECEIPTS` routes to `Routes.kt`
    - Add composables to NavHost with admin role guard
    - Add navigation from ConfiguracionScreen to chatbot config (sub-item)
    - Add badge for pending receipts count
    - _Requirements: 3.11, 3.12, 9.5_

- [ ] 6. Plantillas feature module
  - [x] 6.1 Create `feature/plantillas` module scaffolding
    - Create `android/feature/plantillas/build.gradle.kts`
    - Register module in `settings.gradle.kts`
    - Add dependency in `:app` module
    - _Requirements: 4.1_

  - [x] 6.2 Create PlantillasApiService in `core/network/api/`
    - Implement `getPlantillas()`, `createPlantilla(request)`, `updatePlantilla(id, request)`, `deletePlantilla(id)`
    - Add to NetworkModule DI
    - _Requirements: 4.9_

  - [~] 6.3 Create Plantilla DTOs in `core/model`
    - Add `PlantillaDtos.kt` with: PlantillaResponse, CrearPlantillaRequest, ActualizarPlantillaRequest
    - contenido field as `JsonElement` (kotlinx.serialization)
    - _Requirements: 4.3_

  - [~] 6.4 Create PlantillasViewModel
    - Sealed `PlantillasUiState` (Loading, Success with ImmutableList, Error)
    - Sealed `PlantillaFormUiState` for create/edit form state
    - Actions: loadPlantillas, createPlantilla, updatePlantilla, deletePlantilla
    - _Requirements: 4.2 through 4.6_

  - [~] 6.5 Create PlantillasScreen composable
    - List of plantillas with nombre, tipo_documento, entity_type
    - FAB for create new
    - Swipe-to-delete or long-press menu with delete confirmation
    - Tap to edit
    - _Requirements: 4.2, 4.6_

  - [~] 6.6 Create PlantillaFormScreen composable
    - Form fields: nombre (text), tipo_documento (dropdown: contrato_arrendamiento, acuerdo, recibo, carta, notificacion, otro), entity_type (dropdown: propiedad, inquilino, contrato, pago, gasto)
    - Simplified block editor for contenido: list of text/heading blocks, add block button, edit block text inline
    - Variable placeholder insertion (e.g., {{inquilino_nombre}}, {{propiedad_direccion}})
    - Save button
    - _Requirements: 4.3, 4.4, 4.5, 4.7_

  - [~] 6.7 Add Plantillas to navigation
    - Add `PLANTILLAS` and `PLANTILLA_FORM` routes to `Routes.kt`
    - Add composables to NavHost
    - Add menu item to `MasMenuScreen`
    - _Requirements: 4.8_

- [ ] 7. Document Editor and Signing enhancement
  - [~] 7.1 Extend DocumentosApiService
    - Add `updateContenido(documentoId, contenidoEditable)`, `getFirmas(documentoId)`, `firmar(documentoId, firmaImagen)`, `solicitarFirma(documentoId, request)`
    - _Requirements: 5.7_

  - [~] 7.2 Create Firma DTOs in `core/model`
    - Add firma-related DTOs: FirmaResponse, FirmarRequest, SolicitarFirmaRequest, SolicitarFirmaResponse (if not already present)
    - _Requirements: 5.7_

  - [~] 7.3 Create DocumentoEditorScreen composable
    - Render contenido_editable JSON blocks as read-only styled text (headings, paragraphs)
    - Edit mode: tap block to edit in TextField
    - Save button → PUT contenido
    - _Requirements: 5.1, 5.2, 5.3_

  - [~] 7.4 Create SignatureCanvas composable
    - Touch-based drawing canvas using `Modifier.pointerInput` + `Canvas`
    - Clear button to reset
    - Export as base64 PNG
    - _Requirements: 5.6_

  - [~] 7.5 Add firma management to DocumentoEditorScreen
    - Display list of firmas with estado badges
    - "Firmar" button → shows SignatureCanvas → submits firma_imagen
    - "Solicitar Firma" button → form with firmante_nombre + email → submits request
    - _Requirements: 5.4, 5.5, 5.6_

  - [~] 7.6 Add Documento Editor route to NavHost
    - Add `DOCUMENTO_EDITOR` route to `Routes.kt` with documentoId parameter
    - Wire composable in NavHost
    - Add navigation from DocumentosScreen to editor when document has contenido_editable
    - _Requirements: 5.1_

- [ ] 8. Contratos filter UI
  - [~] 8.1 Add filter section to ContratosScreen
    - Expandable filter panel matching PagosScreen/GastosScreen pattern
    - Filters: estado dropdown (activo, vencido, cancelado, finalizado, terminado), propiedad selector, fecha range
    - "Filtrar" button to apply, "Limpiar" to reset
    - Update ViewModel to accept filter parameters
    - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 9. Scanner CameraX integration
  - [~] 9.1 Add CameraX dependencies to version catalog and scanner module
    - Add camerax-camera2, camerax-lifecycle, camerax-view to `libs.versions.toml`
    - Add dependencies to `feature/scanner/build.gradle.kts`
    - _Requirements: 8.7_

  - [~] 9.2 Create CameraPreview composable
    - Use `AndroidView` with `PreviewView` for camera preview
    - Bind to lifecycle with `ProcessCameraProvider`
    - Configure Preview + ImageCapture use cases
    - _Requirements: 8.1_

  - [~] 9.3 Implement image capture in ScannerViewModel
    - Replace stub `onCaptureRequested` with actual `ImageCapture.takePicture()` call
    - Convert captured image to `InputImage` for ML Kit
    - Route to `processCedulaImage` or `processReceiptImage` based on mode
    - _Requirements: 8.2, 8.3_

  - [~] 9.4 Update ScannerScreen with camera UI
    - Show CameraPreview when permission granted
    - Capture button overlay
    - Processing indicator while OCR runs
    - Result review screen with extracted fields
    - Error state with retry suggestion
    - _Requirements: 8.1, 8.4, 8.5_

  - [~] 9.5 Add runtime camera permission handling
    - Request `CAMERA` permission using `rememberLauncherForActivityResult(RequestPermission)`
    - Show rationale if denied
    - Show settings redirect if permanently denied
    - _Requirements: 8.6_

- [ ] 10. Navigation and menu updates
  - [~] 10.1 Update Routes.kt with all new route constants
    - Add: USUARIOS, PLANTILLAS, PLANTILLA_FORM, CHATBOT_CONFIG, CHATBOT_RECEIPTS, DOCUMENTO_EDITOR
    - Add helper functions: `plantillaForm(id?)`, `documentoEditor(documentoId)`
    - _Requirements: 10.3_

  - [~] 10.2 Update MasMenuScreen with role-based items
    - Add Plantillas menu item (all roles)
    - Add Usuarios menu item (admin only — conditionally shown)
    - Add Chatbot menu item under Configuración or as separate item (admin only)
    - Receive user role as parameter and filter items
    - _Requirements: 10.1, 10.5_

  - [~] 10.3 Register all new routes in NavHost
    - Add composable entries for: Usuarios, Plantillas, PlantillaForm, ChatbotConfig, ChatbotReceipts, DocumentoEditor
    - Add role guards for admin-only routes
    - _Requirements: 10.2_

- [ ] 11. String resources
  - [~] 11.1 Add all new string resources to `strings.xml`
    - Navigation labels: nav_usuarios, nav_plantillas, nav_chatbot, nav_recibos_pendientes
    - Chatbot steps: chatbot_step_conexion, chatbot_step_personalidad, chatbot_step_capacidades, chatbot_step_conocimiento, chatbot_step_remitentes, chatbot_step_prueba, chatbot_step_activar
    - Form labels for all new features
    - Error messages in Spanish
    - Confirmation dialog texts
    - _Requirements: All (localization)_

- [~] 12. Checkpoint — Full build verification
  - Run `./gradlew assembleDebug` to verify the entire app builds without errors
  - Verify no unused imports or missing dependencies
  - Run `./gradlew lint` to check for issues
