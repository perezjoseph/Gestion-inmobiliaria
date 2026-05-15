# Implementation Plan: Contract Document Signing

## Overview

Extends the document management system with DOCX export, template CRUD, and a digital signature workflow (authenticated manager signing + presigned tenant links with password protection + automatic document sealing). Implementation follows the established layered pattern: migration → entity → DTOs → service → handler → routes → tests.

## Tasks

- [x] 1. Database migration and entity generation
  - [x] 1.1 Create migration `m20250620_000001_create_firmas_documento.rs`
    - Create `firmas_documento` table with columns: id (UUID PK), documento_id (UUID FK), firmante_tipo (VARCHAR), firmante_nombre (VARCHAR), firma_imagen (BYTEA nullable), ip_address (VARCHAR nullable), user_agent (TEXT nullable), firmado_at (TIMESTAMPTZ nullable), token (VARCHAR UNIQUE nullable), password_hash (VARCHAR nullable), expira_at (TIMESTAMPTZ nullable), estado (VARCHAR NOT NULL DEFAULT 'pendiente'), created_at (TIMESTAMPTZ NOT NULL DEFAULT NOW())
    - Add index on documento_id, unique partial index on token WHERE token IS NOT NULL
    - Add `sellado` (BOOLEAN NOT NULL DEFAULT FALSE) and `sellado_at` (TIMESTAMPTZ nullable) columns to `documentos` table
    - Register migration in `migrations/mod.rs`
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

  - [x] 1.2 Create entity `entities/firma_documento.rs`
    - Define SeaORM `Model` struct with all columns from the migration
    - Define `Relation` to `documento` entity (belongs_to)
    - Add `ActiveModelBehavior` impl
    - Re-export in `entities/mod.rs` and `entities/prelude.rs`
    - _Requirements: 7.1, 7.2_

  - [x] 1.3 Update `entities/documento.rs` to add `sellado` and `sellado_at` fields
    - Add `sellado: bool` and `sellado_at: Option<DateTimeWithTimeZone>` to the Model
    - Add `HasMany` relation to `firma_documento`
    - _Requirements: 6.1, 6.4_

- [x] 2. DTOs and models
  - [x] 2.1 Create `models/firma.rs` with request/response DTOs
    - `FirmarRequest` (firma_imagen: String — base64 PNG)
    - `SolicitarFirmaRequest` (firmante_nombre, email)
    - `SolicitarFirmaResponse` (firma_id, token, expira_at, email_enviado)
    - `FirmaResponse` (id, documento_id, firmante_tipo, firmante_nombre, estado, firmado_at, created_at)
    - `VerificarTokenRequest` (password)
    - `DocumentoFirmaResponse` (documento_id, contenido, firmante_nombre, estado)
    - `FirmarConTokenRequest` (password, firma_imagen)
    - Re-export in `models/mod.rs`
    - _Requirements: 4.1, 5.1, 5.5, 5.9_

  - [x] 2.2 Extend `models/documento.rs` with template CRUD DTOs
    - `CrearPlantillaRequest` (nombre, tipo_documento, entity_type, contenido)
    - `ActualizarPlantillaRequest` (all fields optional)
    - Ensure existing `PlantillaResponse` covers GET needs
    - Re-export in `models/mod.rs`
    - _Requirements: 2.1, 2.2, 2.4_

- [x] 3. Checkpoint - Ensure compilation passes
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. DOCX export service and endpoint
  - [x] 4.1 Add `docx-rs = "0.4"` dependency to `backend/Cargo.toml`
    - _Requirements: 1.1_

  - [x] 4.2 Implement `exportar_docx` in `services/documento_editor.rs`
    - Add `build_docx(blocks: &[serde_json::Value]) -> Result<Docx, AppError>` helper
    - Map heading blocks → Paragraph with font sizes (level 1=36, 2=30, 3=26 half-points)
    - Map paragraph blocks → Paragraph with Arial 11pt (22 half-points), 15mm margins
    - Map list blocks → numbered (ordered) or bulleted (unordered) lists
    - Map table blocks → Table with bold headers and cell borders
    - Map page_break blocks → Paragraph with PageBreakBefore
    - Return 400 if `contenido_editable` is None, 404 if document not found
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8_

  - [x] 4.3 Add `exportar_docx` handler in `handlers/documentos.rs`
    - GET `/{id}/exportar-docx` with Claims extractor
    - Set Content-Type to `application/vnd.openxmlformats-officedocument.wordprocessingml.document`
    - Set Content-Disposition to `attachment; filename="documento-{id}.docx"`
    - _Requirements: 1.9, 1.10_

  - [x] 4.4 Register DOCX export route in `routes.rs`
    - Add GET `/documentos/{id}/exportar-docx` under authenticated scope
    - _Requirements: 1.1, 1.10_

  - [x] 4.5 Write property test for DOCX export (Property 1: valid output)
    - **Property 1: DOCX export produces valid output for any Block_JSON**
    - Generate arbitrary valid Block_JSON structures, verify output starts with ZIP magic bytes `PK\x03\x04` and is non-empty
    - **Validates: Requirements 1.1**

  - [x] 4.6 Write property test for DOCX export (Property 2: text preservation)
    - **Property 2: DOCX export preserves all text content**
    - Generate Block_JSON with paragraph/heading/list text, extract DOCX XML content, verify all input text strings appear
    - **Validates: Requirements 1.4**

- [x] 5. Template CRUD service and endpoints
  - [x] 5.1 Implement `crear`, `actualizar`, `eliminar`, `obtener` in `services/plantillas.rs`
    - `crear`: validate non-empty nombre and tipo_documento, create record, set updated_at
    - `actualizar`: find by id (404 if missing), validate fields if present, update, set updated_at
    - `eliminar`: find by id (404 if missing), set activo=false (soft-delete)
    - `obtener`: find by id (404 if missing), return template
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.7, 2.9_

  - [x] 5.2 Add template CRUD handlers in `handlers/documentos.rs`
    - POST `/plantillas` → `crear_plantilla` (WriteAccess)
    - GET `/plantillas/{id}` → `obtener_plantilla` (Claims)
    - PUT `/plantillas/{id}` → `actualizar_plantilla` (WriteAccess)
    - DELETE `/plantillas/{id}` → `eliminar_plantilla` (WriteAccess)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6_

  - [x] 5.3 Register template CRUD routes in `routes.rs`
    - Add POST/GET/PUT/DELETE `/documentos/plantillas` and `/documentos/plantillas/{id}` under authenticated scope
    - _Requirements: 2.6_

  - [x] 5.4 Write property test for template CRUD (Property 3: round-trip)
    - **Property 3: Template CRUD round-trip**
    - Create template with arbitrary valid inputs, read by ID, verify all fields match
    - **Validates: Requirements 2.1, 2.4**

  - [x] 5.5 Write property test for template soft-delete (Property 4)
    - **Property 4: Template soft-delete removes from active list**
    - Create template, soft-delete it, verify it does not appear in active list
    - **Validates: Requirements 2.3**

  - [x] 5.6 Write property test for template validation (Property 5)
    - **Property 5: Template validation rejects empty required fields**
    - Generate whitespace-only strings for nombre/tipo_documento, verify rejection
    - **Validates: Requirements 2.7**

  - [x] 5.7 Write property test for placeholder resolution (Property 6)
    - **Property 6: Placeholder resolution replaces all matching keys**
    - Generate templates with `{{key}}` placeholders and matching maps, verify no placeholders remain and values appear
    - **Validates: Requirements 2.8**

- [x] 6. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Signature service and endpoints
  - [x] 7.1 Create `services/firmas.rs` with core signing logic
    - `firmar_autenticado`: create firma_documento with estado="firmado", capture IP/user_agent/timestamp, store firma_imagen, call `verificar_y_sellar`
    - `solicitar_firma`: generate UUID token, generate random 16-char password, hash with argon2, create firma_documento with estado="pendiente" and expira_at=now+72h, send email
    - `verificar_token`: find by token (404), check expiry (410), verify password (401), return document content
    - `firmar_con_token`: re-verify password, check estado=="pendiente" (409 if not), store signature + metadata, set estado="firmado", call `verificar_y_sellar`
    - `verificar_y_sellar`: query all firmas for document, if propietario+inquilino both "firmado" → set sellado=true, sellado_at=now, generate sealed PDF
    - Validate firma_imagen is valid base64 and < 500KB decoded
    - Re-export in `services/mod.rs`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 6.1, 6.2, 6.3, 6.5, 6.6, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

  - [x] 7.2 Add sealed document guard to `services/documento_editor.rs`
    - In `guardar_contenido` (or equivalent PUT handler), check `documento.sellado == true` → return 403 "El documento está sellado y no puede ser modificado"
    - _Requirements: 6.4, 8.7_

  - [x] 7.3 Create `handlers/firmas.rs` with authenticated and public handlers
    - `firmar`: POST `/documentos/{id}/firmar` (WriteAccess) — extract IP from X-Forwarded-For/peer, user_agent from headers
    - `solicitar_firma`: POST `/documentos/{id}/solicitar-firma` (WriteAccess)
    - `listar_firmas`: GET `/documentos/{id}/firmas` (Claims)
    - `verificar_firma_publica`: POST `/firmas/{token}/verificar` (no auth)
    - `firmar_publica`: POST `/firmas/{token}/firmar` (no auth) — extract IP/user_agent from request
    - Re-export in `handlers/mod.rs`
    - _Requirements: 4.1, 4.6, 5.4, 5.5, 5.9_

  - [x] 7.4 Register signature routes in `routes.rs`
    - Authenticated: POST `/documentos/{id}/firmar`, POST `/documentos/{id}/solicitar-firma`, GET `/documentos/{id}/firmas`
    - Public: POST `/firmas/{token}/verificar`, POST `/firmas/{token}/firmar` with rate limiting (Governor)
    - _Requirements: 4.6, 5.4, 8.4_

  - [x] 7.5 Write property test for signature record completeness (Property 7)
    - **Property 7: Signature record completeness**
    - For any successful signature, verify firma_documento has non-null firma_imagen, non-empty ip_address, non-empty user_agent, firmado_at within 5s of now, estado="firmado"
    - **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 8.1, 8.2, 8.3**

  - [x] 7.6 Write property test for token generation (Property 8)
    - **Property 8: Token generation correctness**
    - For any solicitar-firma, verify token ≥ 32 chars, password_hash is valid argon2, expira_at within 1s of exactly 72h from created_at
    - **Validates: Requirements 5.1, 5.2, 8.4, 8.5, 8.6**

  - [x] 7.7 Write property test for password hashing (Property 9)
    - **Property 9: Password hashing round-trip**
    - Generate arbitrary passwords, hash with argon2, verify original succeeds, verify different string fails
    - **Validates: Requirements 5.2, 8.6**

  - [x] 7.8 Write property test for token access rejection (Property 10)
    - **Property 10: Token access rejects expired or wrong password**
    - Create firma with expired expira_at → verify 410; create firma with valid token but wrong password → verify 401
    - **Validates: Requirements 5.7, 5.8**

  - [x] 7.9 Write property test for signing state guard (Property 11)
    - **Property 11: Tenant signing state guard**
    - For firma with estado != "pendiente", verify signing attempt returns 409
    - **Validates: Requirements 5.10**

  - [x] 7.10 Write property test for document sealing (Property 12)
    - **Property 12: Document sealing triggers on complete signatures**
    - Create document with both propietario and inquilino firmas with estado="firmado", verify sellado=true and sellado_at is set
    - **Validates: Requirements 6.1, 6.5**

  - [x] 7.11 Write property test for sealed immutability (Property 13)
    - **Property 13: Sealed document immutability**
    - For any document with sellado=true, verify PUT to contenido_editable returns 403
    - **Validates: Requirements 6.4, 8.7**

  - [x] 7.12 Write property test for signing order independence (Property 14)
    - **Property 14: Signing order independence**
    - Sign propietario first then inquilino, and vice versa — verify final sealed state is identical
    - **Validates: Requirements 6.6**

- [x] 8. Checkpoint - Ensure all backend tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Frontend: DOCX export button
  - [x] 9.1 Add DOCX export button to `components/common/document_editor.rs`
    - Add "Exportar DOCX" button next to existing "Exportar PDF" button in EditorToolbar
    - Initiate download from `/documentos/{id}/exportar-docx` on click
    - Show "Exportando..." text and disable button while request is in progress
    - Disable button if document has no `documento_id` (unsaved)
    - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [x] 10. Frontend: Template management page
  - [x] 10.1 Create `pages/plantillas.rs` with template list and CRUD UI
    - List view showing active templates (nombre, tipo_documento, entity_type)
    - "Crear Plantilla" button opening form with nombre, tipo_documento (dropdown), entity_type (dropdown), and block editor for contenido
    - Edit button loads template into form
    - Delete button shows confirmation dialog, sends DELETE on confirm
    - Success toast on create/update, error banner on API errors
    - Register page in `pages/mod.rs`
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

  - [x] 10.2 Add template management route and navigation link
    - Add route for `/plantillas` in `app.rs`
    - Add navigation link visible only to admin/gerente roles
    - _Requirements: 3.1_

- [x] 11. Frontend: Signature panel and canvas
  - [x] 11.1 Create `components/common/signature_canvas.rs`
    - HTML5 Canvas component with touch and mouse event handling for drawing
    - Clear button to reset canvas
    - Submit button that exports canvas as PNG base64
    - Props: `on_submit: Callback<String>` (base64 data), `on_cancel: Callback<()>`
    - _Requirements: 10.2, 10.3, 11.3, 11.4_

  - [x] 11.2 Add signature panel to document editor page
    - Display signature status panel below editor showing each party's status
    - "Firmar Documento" button → opens signature canvas modal
    - "Solicitar Firma del Inquilino" button → opens form for name + email
    - Submit drawn signature → POST `/documentos/{id}/firmar`
    - Submit signature request → POST `/documentos/{id}/solicitar-firma` + success toast
    - "Sellado" badge when document is sealed
    - Set editor to readonly when sealed
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7_

- [x] 12. Frontend: Public signing page
  - [x] 12.1 Create `pages/firma_publica.rs` with three-state flow
    - State 1: Password input form
    - State 2: Document content (readonly) + signature canvas (after password verified via POST `/firmas/{token}/verificar`)
    - State 3: Confirmation message "Documento firmado exitosamente" (after POST `/firmas/{token}/firmar`)
    - Handle expired link (410) → show "Este enlace de firma ha expirado. Contacte al administrador de la propiedad."
    - Handle wrong password (401) → show "Contraseña incorrecta" with retry
    - Register page in `pages/mod.rs`
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_

  - [x] 12.2 Add public route `/firmas/{token}` in `app.rs`
    - Route must not require authentication
    - _Requirements: 11.1_

- [x] 13. Checkpoint - Ensure frontend compiles
  - Ensure all tests pass, ask the user if questions arise.

- [x] 14. Integration tests
  - [x] 14.1 Write integration tests in `tests/firmas_tests.rs`
    - Test authenticated signing flow end-to-end
    - Test solicitar-firma creates pending record with valid token
    - Test public token verification (valid, expired, wrong password)
    - Test public signing (success, already signed conflict)
    - Test document sealing after both parties sign
    - Test sealed document rejects content edits (403)
    - Test DOCX export returns valid response with correct headers
    - Test template CRUD operations (create, read, update, soft-delete)
    - Test RBAC enforcement (WriteAccess for write endpoints)
    - _Requirements: 1.1, 2.1, 2.3, 4.1, 5.1, 5.7, 5.8, 5.10, 6.1, 6.4_

- [x] 15. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from the design document
- Unit tests validate specific examples and edge cases
- The backend uses Rust with Actix-web, SeaORM, and the existing layered architecture
- The frontend uses Leptos (Rust WASM) with the existing component patterns
- Migration naming follows existing convention: `m{YYYYMMDD}_{SEQ}_{name}.rs`
- PBT files follow naming convention: `firmas_pbt.rs` in `services/`

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3"] },
    { "id": 2, "tasks": ["2.1", "2.2", "4.1"] },
    { "id": 3, "tasks": ["4.2", "5.1", "7.1"] },
    { "id": 4, "tasks": ["4.3", "4.4", "5.2", "5.3", "7.2", "7.3", "7.4"] },
    { "id": 5, "tasks": ["4.5", "4.6", "5.4", "5.5", "5.6", "5.7", "7.5", "7.6", "7.7", "7.8", "7.9", "7.10", "7.11", "7.12"] },
    { "id": 6, "tasks": ["9.1", "10.1", "11.1"] },
    { "id": 7, "tasks": ["10.2", "11.2", "12.1"] },
    { "id": 8, "tasks": ["12.2"] },
    { "id": 9, "tasks": ["14.1"] }
  ]
}
```
