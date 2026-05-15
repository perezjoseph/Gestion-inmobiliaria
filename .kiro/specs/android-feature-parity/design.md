# Design: Android Feature Parity

## Architecture Overview

All new features follow the established multi-module architecture: `feature/{domain}/` → `ViewModel` → `Repository` → `core/network/api/` + `core/database/`. New modules are Android libraries with Compose + Hilt, depending on `:core:data`, `:core:model`, `:core:ui`, `:core:common`. No feature-to-feature dependencies.

## Module Structure

```
android/
├── feature/
│   ├── usuarios/          (NEW — admin user management)
│   ├── chatbot/           (NEW — WhatsApp AI config wizard)
│   ├── plantillas/        (NEW — document template CRUD)
│   └── (existing modules enhanced: documentos, propiedades, contratos, scanner)
├── core/
│   ├── network/api/
│   │   ├── UsuariosApiService.kt      (NEW)
│   │   ├── ChatbotApiService.kt       (NEW)
│   │   ├── PlantillasApiService.kt    (NEW)
│   │   └── (existing: DocumentosApiService extended, PropiedadesApiService extended)
│   └── model/
│       ├── UsuarioDtos.kt             (NEW)
│       ├── ChatbotDtos.kt             (NEW)
│       ├── PlantillaDtos.kt           (NEW)
│       ├── UnidadDtos.kt              (NEW)
│       └── Unidad.kt                  (NEW domain model)
└── app/src/main/kotlin/com/propmanager/navigation/
    ├── PropManagerNavHost.kt          (MODIFIED — wire all screens)
    ├── Routes.kt                      (MODIFIED — add new routes)
    └── MasMenuScreen.kt              (MODIFIED — add new menu items)
```

## Data Flow

### New API Services

**UsuariosApiService:**
- `GET /api/usuarios?page={page}&per_page={perPage}` → `PaginatedResponse<UserDto>`
- `PUT /api/usuarios/{id}/rol` → body: `{ "rol": "admin|gerente|visualizador" }`
- `PUT /api/usuarios/{id}/toggle-activo` → toggles active status

**ChatbotApiService:**
- `GET /chatbot/config` → `ChatbotConfigResponse`
- `PUT /chatbot/config` → `ChatbotConfigUpdateRequest`
- `GET /chatbot/status` → `ConnectionStatusResponse`
- `POST /chatbot/connect` → `ConnectionStatusResponse`
- `POST /chatbot/disconnect` → `ConnectionStatusResponse`
- `POST /chatbot/test` → `TestChatRequest` → `TestChatResponse`
- `GET /chatbot/conversations` → `List<ConversationListItem>`
- `GET /chatbot/receipts/pending` → `List<ReceiptExtractionResponse>`
- `POST /chatbot/receipts/{id}/confirm` → `ReceiptConfirmRequest`
- `POST /chatbot/receipts/{id}/reject` → `ReceiptRejectRequest`

**PlantillasApiService:**
- `GET /api/plantillas` → `List<PlantillaResponse>`
- `POST /api/plantillas` → `CrearPlantillaRequest` → `PlantillaResponse`
- `PUT /api/plantillas/{id}` → `ActualizarPlantillaRequest` → `PlantillaResponse`
- `DELETE /api/plantillas/{id}` → 204

**PropiedadesApiService (extended):**
- `GET /api/propiedades/{id}/unidades` → `List<UnidadDto>`
- `POST /api/propiedades/{id}/unidades` → `CreateUnidadRequest` → `UnidadDto`
- `PUT /api/propiedades/{id}/unidades/{unidadId}` → `UpdateUnidadRequest` → `UnidadDto`
- `DELETE /api/propiedades/{id}/unidades/{unidadId}` → 204

**DocumentosApiService (extended):**
- `PUT /api/documentos/{id}/contenido` → `{ "contenido_editable": JSON }`
- `GET /api/documentos/{id}/firmas` → `List<FirmaResponse>`
- `POST /api/documentos/{id}/firmas` → `{ "firma_imagen": "base64..." }`
- `POST /api/documentos/{id}/firmas/solicitar` → `SolicitarFirmaRequest` → `SolicitarFirmaResponse`

## State Management

All ViewModels use `sealed interface` for UI state (no boolean flags):

```kotlin
sealed interface UsuariosUiState {
    data object Loading : UsuariosUiState
    data class Success(val users: ImmutableList<UserDto>, val page: Int, val totalPages: Int) : UsuariosUiState
    data class Error(val message: String) : UsuariosUiState
}

sealed interface ChatbotConfigUiState {
    data object Loading : ChatbotConfigUiState
    data class Success(
        val config: ChatbotConfigResponse,
        val connectionStatus: ConnectionStatusResponse,
        val currentStep: Int,
    ) : ChatbotConfigUiState
    data class Error(val message: String) : ChatbotConfigUiState
}

sealed interface PlantillasUiState {
    data object Loading : PlantillasUiState
    data class Success(val plantillas: ImmutableList<PlantillaResponse>) : PlantillasUiState
    data class Error(val message: String) : PlantillasUiState
}
```

## NavHost Wiring Strategy

Replace placeholder comments with actual screen composables. Each CRUD feature follows the pattern:

```kotlin
composable(Routes.PROPIEDADES) {
    val vm: PropiedadesViewModel = hiltViewModel()
    PropiedadesScreen(
        viewModel = vm,
        onNavigateToDetail = { id -> navController.navigate(Routes.propiedadDetail(id)) },
        onNavigateToForm = { id -> navController.navigate(Routes.propiedadForm(id)) },
        onNavigateBack = { navController.popBackStack() },
    )
}
composable(Routes.PROPIEDAD_DETAIL) { backStackEntry ->
    val id = backStackEntry.arguments?.getString("id") ?: return@composable
    val vm: PropiedadesViewModel = hiltViewModel()
    LaunchedEffect(id) { vm.loadDetail(id) }
    PropiedadDetailScreen(
        viewModel = vm,
        onNavigateToForm = { navController.navigate(Routes.propiedadForm(id)) },
        onNavigateToDocumentos = { navController.navigate(Routes.documentos("propiedad", id)) },
        onNavigateBack = { navController.popBackStack() },
    )
}
composable(Routes.PROPIEDAD_FORM) { backStackEntry ->
    val id = backStackEntry.arguments?.getString("id")?.takeIf { it.isNotEmpty() }
    val vm: PropiedadesViewModel = hiltViewModel()
    LaunchedEffect(id) { id?.let { vm.loadDetail(it) } }
    PropiedadFormScreen(
        viewModel = vm,
        editId = id,
        onNavigateBack = { navController.popBackStack() },
    )
}
```

## Role-Based Access Control

The authenticated user's role is available from `AuthViewModel.userProfile.rol`. Admin-only features:
- Usuarios: hidden from nav + route guard redirects non-admin to dashboard
- Chatbot Config: hidden from nav + route guard

Implementation: `MasMenuScreen` receives the user role and conditionally includes admin items. NavHost composables for admin routes check role and redirect if unauthorized.

## Scanner CameraX Integration

```
ScannerScreen
├── CameraPreview (CameraX Preview use case)
├── CaptureButton → ImageCapture.takePicture()
├── Processing overlay (while OCR runs)
└── ResultReview (extracted fields displayed for confirmation)
```

Dependencies: `androidx.camera:camera-camera2`, `androidx.camera:camera-lifecycle`, `androidx.camera:camera-view`. Permission: `android.permission.CAMERA` requested via `rememberLauncherForActivityResult(RequestPermission)`.

## Mobile-Adapted Document Editor

The frontend uses a full block editor with drag-and-drop. For mobile, we simplify:
- Read-only rendered view of block content (headings, paragraphs, variables highlighted)
- Edit mode: tap a block to edit its text content in a TextField
- No drag-and-drop reordering on mobile (complex gesture conflicts with scroll)
- Variable placeholders shown as chips (non-editable, auto-filled from entity data)

## Signature Canvas

Reuse the existing pattern from `ScannerScreen` — a custom Compose `Canvas` with touch path tracking:
- `Modifier.pointerInput` captures touch events
- Draws paths on a `Canvas` composable
- Export as base64 PNG via `Bitmap` → `ByteArrayOutputStream` → `Base64.encodeToString`

## Offline Considerations

- **Usuarios, Chatbot, Plantillas, Document Editor**: Online-only (no Room entities, no sync queue). These are admin/configuration features that require server state.
- **Unidades**: Added to offline-first architecture — new Room entity, DAO, mapper, and sync queue support since they're part of the property management core.
- **CRUD wiring**: No changes needed — existing screens already have offline support via Room + SyncQueue.

## Dependencies (additions to version catalog)

```toml
[libraries]
camerax-camera2 = { module = "androidx.camera:camera-camera2", version.ref = "camerax" }
camerax-lifecycle = { module = "androidx.camera:camera-lifecycle", version.ref = "camerax" }
camerax-view = { module = "androidx.camera:camera-view", version.ref = "camerax" }

[versions]
camerax = "1.4.1"
```

## String Resources

All new UI text in Spanish. New string resources needed:
- `nav_usuarios`, `nav_plantillas`, `nav_chatbot`
- Chatbot step labels: "Conexión", "Personalidad", "Capacidades", "Conocimiento", "Remitentes", "Prueba", "Activar"
- Form labels, error messages, confirmation dialogs for each new feature
- Unidad-related labels: "Unidades", "Número de unidad", "Piso", etc.
