# Design Document: Contract Document Signing

## Architecture Overview

This feature extends the existing document management system with three capabilities:

1. **DOCX Export** — Converts Block_JSON content to Microsoft Word format using `docx-rs`
2. **Template CRUD** — Full create/read/update/delete for document templates
3. **Digital Signature Workflow** — Authenticated manager signing, presigned tenant links with password protection, and automatic document sealing

The architecture follows the established layered pattern: migration → entity → DTOs → service → handler → routes → tests.

---

## Components

### Backend Components

#### 1. DOCX Exporter (`services/documento_editor.rs` — extended)

Adds `exportar_docx` function alongside the existing `exportar_pdf`. Converts Block_JSON to DOCX using the `docx-rs` crate (v0.4.x).

```rust
/// Export a document's `contenido_editable` as a DOCX file.
pub async fn exportar_docx(
    db: &DatabaseConnection,
    documento_id: Uuid,
) -> Result<Vec<u8>, AppError> {
    let doc = documento::Entity::find_by_id(documento_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Documento {documento_id} no encontrado")))?;

    let contenido = doc.contenido_editable.ok_or_else(|| {
        AppError::BadRequest(
            "El documento no tiene contenido editable para exportar".to_string(),
        )
    })?;

    let blocks = contenido
        .get("blocks")
        .and_then(|b| b.as_array())
        .ok_or_else(|| {
            AppError::Validation("Contenido editable sin formato válido".to_string())
        })?;

    let docx = build_docx(blocks)?;
    let mut buf = Vec::new();
    docx.build().pack(&mut std::io::Cursor::new(&mut buf))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error generando DOCX: {e}")))?;

    Ok(buf)
}
```

Block type mapping:
- `heading` → `Paragraph` with `RunProperty` font size (level 1=36 half-points, 2=30, 3=26)
- `paragraph` → `Paragraph` with Arial 11pt (22 half-points)
- `list` → `NumberingId` for ordered, bullet for unordered
- `table` → `Table` with `TableRow`/`TableCell`, bold headers
- `page_break` → `Paragraph` with `PageBreakBefore`

#### 2. Template CRUD (`services/plantillas.rs` — extended)

Adds `crear`, `actualizar`, and `eliminar` functions to the existing `listar` and `rellenar`:

```rust
pub async fn crear(
    db: &DatabaseConnection,
    input: CrearPlantillaRequest,
) -> Result<PlantillaResponse, AppError> { ... }

pub async fn actualizar(
    db: &DatabaseConnection,
    id: Uuid,
    input: ActualizarPlantillaRequest,
) -> Result<PlantillaResponse, AppError> { ... }

pub async fn eliminar(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<(), AppError> { ... }

pub async fn obtener(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<PlantillaResponse, AppError> { ... }
```

#### 3. Signature Service (`services/firmas.rs` — new)

Handles the complete signing workflow:

```rust
/// Manager signs a document (authenticated)
pub async fn firmar_autenticado(
    db: &DatabaseConnection,
    documento_id: Uuid,
    usuario_id: Uuid,
    rol: &str,
    firma_imagen: Vec<u8>,
    ip_address: String,
    user_agent: String,
) -> Result<FirmaResponse, AppError> { ... }

/// Request tenant signature (generates token + password)
pub async fn solicitar_firma(
    db: &DatabaseConnection,
    documento_id: Uuid,
    input: SolicitarFirmaRequest,
) -> Result<SolicitarFirmaResponse, AppError> { ... }

/// Verify token + password, return document for review
pub async fn verificar_token(
    db: &DatabaseConnection,
    token: &str,
    password: &str,
) -> Result<DocumentoFirmaResponse, AppError> { ... }

/// Tenant signs via presigned link
pub async fn firmar_con_token(
    db: &DatabaseConnection,
    token: &str,
    password: &str,
    firma_imagen: Vec<u8>,
    ip_address: String,
    user_agent: String,
) -> Result<FirmaResponse, AppError> { ... }

/// Check if all parties signed and seal if complete
async fn verificar_y_sellar(
    db: &DatabaseConnection,
    documento_id: Uuid,
) -> Result<(), AppError> { ... }
```

#### 4. Signature Handlers (`handlers/firmas.rs` — new)

```rust
// Authenticated endpoints (require JWT + WriteAccess)
pub async fn firmar(db, access, path, body) -> Result<HttpResponse, AppError>
pub async fn solicitar_firma(db, access, path, body) -> Result<HttpResponse, AppError>
pub async fn listar_firmas(db, claims, path) -> Result<HttpResponse, AppError>

// Public endpoints (no JWT required)
pub async fn verificar_firma_publica(db, path, body) -> Result<HttpResponse, AppError>
pub async fn firmar_publica(db, path, body, req) -> Result<HttpResponse, AppError>
```

#### 5. Entity (`entities/firma_documento.rs` — new)

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "firmas_documento")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub documento_id: Uuid,
    pub firmante_tipo: String,       // "propietario" | "inquilino"
    pub firmante_nombre: String,
    #[sea_orm(column_type = "VarBinary(StringLen::None)", nullable)]
    pub firma_imagen: Option<Vec<u8>>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub firmado_at: Option<DateTimeWithTimeZone>,
    #[sea_orm(unique)]
    pub token: Option<String>,
    pub password_hash: Option<String>,
    pub expira_at: Option<DateTimeWithTimeZone>,
    pub estado: String,              // "pendiente" | "firmado" | "expirado" | "cancelado"
    pub created_at: DateTimeWithTimeZone,
}
```

#### 6. Migration (`m20250620_000001_create_firmas_documento.rs` — new)

```sql
CREATE TABLE firmas_documento (
    id UUID PRIMARY KEY,
    documento_id UUID NOT NULL REFERENCES documentos(id),
    firmante_tipo VARCHAR NOT NULL,
    firmante_nombre VARCHAR NOT NULL,
    firma_imagen BYTEA,
    ip_address VARCHAR,
    user_agent TEXT,
    firmado_at TIMESTAMPTZ,
    token VARCHAR UNIQUE,
    password_hash VARCHAR,
    expira_at TIMESTAMPTZ,
    estado VARCHAR NOT NULL DEFAULT 'pendiente',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_firmas_documento_documento_id ON firmas_documento(documento_id);
CREATE UNIQUE INDEX idx_firmas_documento_token ON firmas_documento(token) WHERE token IS NOT NULL;
```

Additionally, add a `sellado` boolean column to the `documentos` table:

```sql
ALTER TABLE documentos ADD COLUMN sellado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE documentos ADD COLUMN sellado_at TIMESTAMPTZ;
```

---

### Frontend Components

#### 1. Template Management Page (`pages/plantillas.rs` — new)

Full CRUD interface for templates:
- List view with active templates (nombre, tipo_documento, entity_type)
- Create/Edit form with block editor integration
- Soft-delete with confirmation dialog

#### 2. Document Editor Extensions (`components/common/document_editor.rs` — extended)

- Add "Exportar DOCX" button to `EditorToolbar`
- Add signature panel below editor content area
- Signature panel shows: current signature status per party, "Firmar Documento" button, "Solicitar Firma del Inquilino" button
- "Sellado" badge when document is sealed
- Readonly mode enforcement when sealed

#### 3. Signature Canvas (`components/common/signature_canvas.rs` — new)

HTML5 Canvas component for drawing signatures:

```rust
#[derive(Properties, PartialEq)]
pub struct SignatureCanvasProps {
    pub on_submit: Callback<Vec<u8>>,  // PNG image bytes
    pub on_cancel: Callback<()>,
}
```

- Touch and mouse event handling for drawing
- Clear button to reset canvas
- Submit button that exports canvas as PNG data URL → bytes

#### 4. Public Signing Page (`pages/firma_publica.rs` — new)

Three-state page:
1. **Password entry** — form with password input
2. **Document review + sign** — readonly document view with signature canvas
3. **Confirmation** — success message after signing

No authentication required. Route: `/firmas/{token}`

---

## Interfaces (DTOs)

### New Request/Response Models (`models/firma.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmarRequest {
    pub firma_imagen: String,  // base64-encoded PNG
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitarFirmaRequest {
    pub firmante_nombre: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitarFirmaResponse {
    pub firma_id: Uuid,
    pub token: String,
    pub expira_at: DateTime<Utc>,
    pub email_enviado: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmaResponse {
    pub id: Uuid,
    pub documento_id: Uuid,
    pub firmante_tipo: String,
    pub firmante_nombre: String,
    pub estado: String,
    pub firmado_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificarTokenRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoFirmaResponse {
    pub documento_id: Uuid,
    pub contenido: serde_json::Value,
    pub firmante_nombre: String,
    pub estado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmarConTokenRequest {
    pub password: String,
    pub firma_imagen: String,  // base64-encoded PNG
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrearPlantillaRequest {
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualizarPlantillaRequest {
    pub nombre: Option<String>,
    pub tipo_documento: Option<String>,
    pub entity_type: Option<String>,
    pub contenido: Option<serde_json::Value>,
}
```

---

## API Routes

### Authenticated Routes (under `/api/v1/documentos`)

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/{id}/exportar-docx` | `documentos::exportar_docx` | Claims |
| POST | `/plantillas` | `documentos::crear_plantilla` | WriteAccess |
| GET | `/plantillas/{id}` | `documentos::obtener_plantilla` | Claims |
| PUT | `/plantillas/{id}` | `documentos::actualizar_plantilla` | WriteAccess |
| DELETE | `/plantillas/{id}` | `documentos::eliminar_plantilla` | WriteAccess |
| POST | `/{id}/firmar` | `firmas::firmar` | WriteAccess |
| POST | `/{id}/solicitar-firma` | `firmas::solicitar_firma` | WriteAccess |
| GET | `/{id}/firmas` | `firmas::listar_firmas` | Claims |

### Public Routes (under `/api/v1/firmas`)

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| POST | `/{token}/verificar` | `firmas::verificar_firma_publica` | None |
| POST | `/{token}/firmar` | `firmas::firmar_publica` | None |

Public routes use rate limiting (Governor) to prevent brute-force password attempts.

---

## Data Flow

### Manager Signing Flow

```
Frontend (click "Firmar") → Signature Canvas → base64 PNG
  → POST /documentos/{id}/firmar { firma_imagen }
  → firmas::firmar_autenticado()
    → Create firma_documento (estado="firmado", firmante_tipo from role)
    → verificar_y_sellar() → check if both parties signed → seal if complete
  → 200 FirmaResponse
```

### Tenant Signing Flow

```
Manager: POST /documentos/{id}/solicitar-firma { firmante_nombre, email }
  → Generate token (uuid v4), generate random password (16 chars)
  → Hash password with argon2
  → Create firma_documento (estado="pendiente", token, password_hash, expira_at=now+72h)
  → Send email with link + plaintext password
  → 201 SolicitarFirmaResponse

Tenant: Navigate to /firmas/{token}
  → Frontend shows password form
  → POST /firmas/{token}/verificar { password }
    → Verify token exists, not expired, password matches argon2 hash
    → 200 DocumentoFirmaResponse (document content)
  → Frontend shows document + signature canvas
  → POST /firmas/{token}/firmar { password, firma_imagen }
    → Re-verify password, store signature, set estado="firmado"
    → verificar_y_sellar() → seal if complete
    → 200 FirmaResponse
```

### Document Sealing Flow

```
verificar_y_sellar(documento_id):
  1. Query all firmas_documento for this document
  2. Check if at least one "firmado" with firmante_tipo="propietario"
     AND at least one "firmado" with firmante_tipo="inquilino"
  3. If both present:
     a. Set documento.sellado = true, documento.sellado_at = now
     b. Generate sealed PDF with signature images embedded
     c. Store sealed PDF as new documento record (tipo_documento="documento_sellado")
```

---

## Error Handling

| Scenario | Error Type | HTTP Status | Message |
|----------|-----------|-------------|---------|
| Document not found | `NotFound` | 404 | "Documento {id} no encontrado" |
| Template not found | `NotFound` | 404 | "Plantilla {id} no encontrada" |
| No editable content | `BadRequest` | 400 | "El documento no tiene contenido editable para exportar" |
| Token not found | `NotFound` | 404 | "Token de firma no encontrado" |
| Token expired | `Gone` | 410 | "El enlace de firma ha expirado" |
| Wrong password | `Unauthorized` | 401 | "Contraseña incorrecta" |
| Already signed | `Conflict` | 409 | "Esta firma ya fue procesada" |
| Document sealed | `Forbidden` | 403 | "El documento está sellado y no puede ser modificado" |
| Empty nombre | `Validation` | 422 | "El nombre de la plantilla es requerido" |
| Empty tipo_documento | `Validation` | 422 | "El tipo de documento es requerido" |
| Invalid firma_imagen | `Validation` | 422 | "La imagen de firma es inválida" |

---

## Security Considerations

1. **Token entropy**: Use `Uuid::new_v4()` for tokens (122 bits of randomness, exceeds 32-byte minimum)
2. **Password hashing**: argon2 with default parameters (already in deps)
3. **Rate limiting**: Public `/firmas` routes use Governor with strict limits (1 req/6s, burst 5) to prevent brute-force
4. **IP capture**: Extract from `X-Forwarded-For` header first, fall back to peer address
5. **Base64 validation**: Validate firma_imagen is valid base64 and reasonable size (< 500KB decoded) before storing
6. **Sealed immutability**: Check `documento.sellado` in `guardar_contenido` before allowing edits

---

## Dependencies

### New Crate

```toml
docx-rs = "0.4"
```

### Existing (already in Cargo.toml)

- `argon2` — password hashing
- `uuid` — token generation
- `reqwest` — email sending (HTTP call to email service)
- `numaelis-rckive-genpdf` — sealed PDF generation

---

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system — essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: DOCX export produces valid output for any Block_JSON

*For any* valid Block_JSON structure (containing any combination of heading, paragraph, list, table, and page_break blocks), the DOCX exporter SHALL produce a non-empty byte array that begins with the ZIP/DOCX magic bytes (`PK\x03\x04`).

**Validates: Requirements 1.1**

### Property 2: DOCX export preserves all text content

*For any* Block_JSON containing paragraph, heading, or list blocks with text content, the DOCX exporter output SHALL contain every text string from the input blocks when the DOCX XML is extracted and searched.

**Validates: Requirements 1.4**

### Property 3: Template CRUD round-trip

*For any* valid template input (non-empty nombre, non-empty tipo_documento, valid entity_type, valid JSON contenido), creating the template and then reading it by ID SHALL return a template with identical nombre, tipo_documento, entity_type, and contenido values.

**Validates: Requirements 2.1, 2.4**

### Property 4: Template soft-delete removes from active list

*For any* existing active template, after soft-deleting it, the template SHALL NOT appear in the list of active templates returned by the listar endpoint.

**Validates: Requirements 2.3**

### Property 5: Template validation rejects empty required fields

*For any* string composed entirely of whitespace (including empty string) used as nombre or tipo_documento, the template creation or update SHALL be rejected with a validation error.

**Validates: Requirements 2.7**

### Property 6: Placeholder resolution replaces all matching keys

*For any* template containing `{{key}}` placeholders and a replacement map containing those keys, resolving the template SHALL produce output where none of the matched placeholder patterns remain and all corresponding values appear in their place.

**Validates: Requirements 2.8**

### Property 7: Signature record completeness

*For any* successful signature submission (authenticated or via token), the resulting firma_documento record SHALL contain: non-null firma_imagen, non-empty ip_address, non-empty user_agent, firmado_at within 5 seconds of current time, and estado equal to "firmado".

**Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 8.1, 8.2, 8.3**

### Property 8: Token generation correctness

*For any* solicitar-firma request, the generated token SHALL have at least 32 characters (sufficient entropy), the stored password_hash SHALL be a valid argon2 hash, and expira_at SHALL be within 1 second of exactly 72 hours from created_at.

**Validates: Requirements 5.1, 5.2, 8.4, 8.5, 8.6**

### Property 9: Password hashing round-trip

*For any* generated plaintext password, hashing it with argon2 and then verifying the original password against the hash SHALL succeed, while verifying any different string SHALL fail.

**Validates: Requirements 5.2, 8.6**

### Property 10: Token access rejects expired or wrong password

*For any* firma_documento with expira_at in the past, attempting to verify the token SHALL return a Gone error (410). *For any* firma_documento with a valid token but incorrect password, verification SHALL return Unauthorized (401).

**Validates: Requirements 5.7, 5.8**

### Property 11: Tenant signing state guard

*For any* firma_documento whose estado is not "pendiente" (i.e., "firmado", "expirado", or "cancelado"), attempting to sign via the token endpoint SHALL return a Conflict error (409).

**Validates: Requirements 5.10**

### Property 12: Document sealing triggers on complete signatures

*For any* document that has at least one firma_documento with firmante_tipo="propietario" and estado="firmado" AND at least one with firmante_tipo="inquilino" and estado="firmado", the document SHALL have sellado=true and sellado_at set.

**Validates: Requirements 6.1, 6.5**

### Property 13: Sealed document immutability

*For any* document where sellado=true, attempting to update contenido_editable SHALL return a Forbidden error (403) with the message "El documento está sellado y no puede ser modificado".

**Validates: Requirements 6.4, 8.7**

### Property 14: Signing order independence

*For any* document requiring signatures from both parties, the final sealed state SHALL be identical regardless of whether the propietario signs first or the inquilino signs first.

**Validates: Requirements 6.6**
