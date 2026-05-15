# Design Document: Document Management

## Overview

This design extends the existing `documentos` system with legal document classification, verification workflows, expiration tracking, compliance profiles, a template-based document editor, and OCR-to-editable-document conversion. It follows the established layered architecture (handlers → services → entities) and integrates with the existing OCR service, audit trail, and RBAC middleware.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (Yew/WASM)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │DocumentGallery│  │ComplianceView│  │   DocumentEditor       │ │
│  │  (extended)   │  │  (new)       │  │ (rich-text, new)       │ │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬─────────────┘ │
│         │                  │                     │               │
│  ┌──────┴──────────────────┴─────────────────────┴─────────────┐ │
│  │                   services/api.rs (extended)                 │ │
│  └──────────────────────────┬──────────────────────────────────┘ │
└─────────────────────────────┼───────────────────────────────────┘
                              │ HTTP/JSON
┌─────────────────────────────┼───────────────────────────────────┐
│                     Backend (Actix-web)                          │
│  ┌──────────────────────────┴──────────────────────────────────┐ │
│  │              handlers/documentos.rs (extended)               │ │
│  │  upload, listar, verificar, eliminar, cumplimiento,          │ │
│  │  por_vencer, plantillas, generar, digitalizar, guardar_edit  │ │
│  └──────────────────────────┬──────────────────────────────────┘ │
│  ┌──────────────────────────┴──────────────────────────────────┐ │
│  │              services/documentos.rs (extended)               │ │
│  │  + services/plantillas.rs (new)                              │ │
│  │  + services/documento_editor.rs (new)                        │ │
│  └──────┬───────────────┬──────────────────┬───────────────────┘ │
│         │               │                  │                     │
│  ┌──────┴──────┐ ┌──────┴──────┐ ┌────────┴────────┐            │
│  │entities/    │ │services/    │ │services/         │            │
│  │documento.rs │ │ocr_client.rs│ │auditoria.rs      │            │
│  │(extended)   │ │(existing)   │ │(existing)        │            │
│  └──────┬──────┘ └─────────────┘ └──────────────────┘            │
│         │                                                        │
│  ┌──────┴──────────────────────────────────────────────────────┐ │
│  │                    PostgreSQL                                │ │
│  │  documentos (extended), plantillas_documento (new)           │ │
│  └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   OCR Service     │
                    │  (PaddleOCR,      │
                    │   existing)       │
                    └───────────────────┘
```

## Database Schema Changes

### Migration 1: `m20250430_000001_extend_documentos_legal.rs`

Extends the existing `documentos` table with legal metadata columns.

```sql
ALTER TABLE documentos
  ADD COLUMN tipo_documento VARCHAR(50) NOT NULL DEFAULT 'otro',
  ADD COLUMN estado_verificacion VARCHAR(20) NOT NULL DEFAULT 'pendiente',
  ADD COLUMN fecha_vencimiento DATE,
  ADD COLUMN verificado_por UUID REFERENCES usuarios(id),
  ADD COLUMN fecha_verificacion TIMESTAMPTZ,
  ADD COLUMN notas_verificacion TEXT,
  ADD COLUMN numero_documento VARCHAR(100);

CREATE INDEX idx_documentos_tipo_documento ON documentos(tipo_documento);
CREATE INDEX idx_documentos_estado_verificacion ON documentos(estado_verificacion);
CREATE INDEX idx_documentos_fecha_vencimiento ON documentos(fecha_vencimiento);
```

After migration, all existing rows get `tipo_documento = 'otro'` and `estado_verificacion = 'pendiente'` via the DEFAULT values.

### Migration 2: `m20250430_000002_add_documentos_editor.rs`

Adds editor content and update tracking columns.

```sql
ALTER TABLE documentos
  ADD COLUMN contenido_editable JSONB,
  ADD COLUMN updated_at TIMESTAMPTZ;
```

### Migration 3: `m20250430_000003_create_plantillas_documento.rs`

Creates the document templates table.

```sql
CREATE TABLE plantillas_documento (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  nombre VARCHAR(100) NOT NULL,
  tipo_documento VARCHAR(50) NOT NULL,
  entity_type VARCHAR(50) NOT NULL,
  contenido JSONB NOT NULL,
  activo BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_plantillas_entity_type ON plantillas_documento(entity_type);
CREATE INDEX idx_plantillas_tipo_documento ON plantillas_documento(tipo_documento);
```

Built-in templates are seeded via the migration's `up()` method.


## Data Models (DTOs)

### Backend: `backend/src/models/documento.rs` (extended)

```rust
// Existing — extended with new fields
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoResponse {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
    // New fields
    pub tipo_documento: String,
    pub estado_verificacion: String,
    pub fecha_vencimiento: Option<NaiveDate>,
    pub verificado_por: Option<Uuid>,
    pub fecha_verificacion: Option<DateTime<Utc>>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    pub contenido_editable: Option<serde_json::Value>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoListQuery {
    pub tipo_documento: Option<String>,
    pub estado_verificacion: Option<String>,
    pub fecha_vencimiento_desde: Option<NaiveDate>,
    pub fecha_vencimiento_hasta: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificarDocumentoRequest {
    pub estado_verificacion: String,  // "verificado", "rechazado", "pendiente"
    pub notas_verificacion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PorVencerQuery {
    pub dias: Option<i64>,  // default 30, min 1, max 365
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoResponse {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub documentos: Vec<CumplimientoItem>,
    pub porcentaje: u8,  // 0-100
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoItem {
    pub tipo_documento: String,
    pub nombre: String,       // Spanish display name
    pub requerido: bool,
    pub estado: String,       // "presente", "pendiente", "vencido", "rechazado", "faltante"
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardarEditorRequest {
    pub contenido_editable: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaResponse {
    pub id: Uuid,
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaRellenadaResponse {
    pub plantilla_id: Uuid,
    pub nombre: String,
    pub tipo_documento: String,
    pub contenido: serde_json::Value,  // with {{placeholders}} resolved
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitalizarResponse {
    pub document_type: String,
    pub contenido_editable: serde_json::Value,
    pub campos_baja_confianza: Vec<String>,  // field names with confidence < 0.80
    pub documento_original_id: Uuid,         // ID of the stored scan
}
```

### Frontend: `frontend/src/types/documento.rs` (extended)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoResponse {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: String,
    pub created_at: String,
    // New fields
    pub tipo_documento: Option<String>,
    pub estado_verificacion: Option<String>,
    pub fecha_vencimiento: Option<String>,
    pub verificado_por: Option<String>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    pub contenido_editable: Option<serde_json::Value>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoResponse {
    pub entity_type: String,
    pub entity_id: String,
    pub documentos: Vec<CumplimientoItem>,
    pub porcentaje: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoItem {
    pub tipo_documento: String,
    pub nombre: String,
    pub requerido: bool,
    pub estado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaResponse {
    pub id: String,
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DigitalizarResponse {
    pub document_type: String,
    pub contenido_editable: serde_json::Value,
    pub campos_baja_confianza: Vec<String>,
    pub documento_original_id: String,
}
```

## Entity Changes

### `backend/src/entities/documento.rs` (extended)

Add new columns to the SeaORM entity (regenerated after migration):

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "documentos")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTimeWithTimeZone,
    // New columns
    pub tipo_documento: String,
    pub estado_verificacion: String,
    pub fecha_vencimiento: Option<Date>,
    pub verificado_por: Option<Uuid>,
    pub fecha_verificacion: Option<DateTimeWithTimeZone>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    pub contenido_editable: Option<Json>,
    pub updated_at: Option<DateTimeWithTimeZone>,
}
```

### `backend/src/entities/plantilla_documento.rs` (new)

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "plantillas_documento")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub contenido: Json,
    pub activo: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}
```


## API Endpoints

All endpoints under `/api/v1/documentos`. Existing endpoints are extended; new ones are added.

### Extended Existing Endpoints

| Method | Path | Auth | Description | Req |
|--------|------|------|-------------|-----|
| `POST` | `/{entity_type}/{entity_id}` | WriteAccess | Upload with `tipo_documento` (multipart field) | R1,R2,R3 |
| `GET` | `/{entity_type}/{entity_id}` | Claims | List with filter query params | R7 |

### New Endpoints

| Method | Path | Auth | Description | Req |
|--------|------|------|-------------|-----|
| `PUT` | `/{id}/verificar` | WriteAccess | Update verification status | R4 |
| `DELETE` | `/{id}` | WriteAccess | Delete document + file | R9 |
| `GET` | `/por-vencer` | Claims | List documents expiring soon | R5 |
| `GET` | `/cumplimiento/{entity_type}/{entity_id}` | Claims | Entity compliance profile | R6 |
| `GET` | `/plantillas` | Claims | List available templates | R11 |
| `GET` | `/plantillas/{id}/rellenar/{entity_type}/{entity_id}` | Claims | Get template pre-filled with entity data | R11 |
| `POST` | `/digitalizar/{entity_type}/{entity_id}` | WriteAccess | OCR scan → editable document | R13 |
| `PUT` | `/{id}/contenido` | WriteAccess | Save editor content | R14 |
| `GET` | `/{id}/exportar-pdf` | Claims | Export document as PDF | R12 |

### Route Registration in `routes.rs`

```rust
.service(
    web::scope("/documentos")
        .wrap(Governor::new(&write_governor_conf))
        // Existing (extended)
        .route("/{entity_type}/{entity_id}", web::post().to(handlers::documentos::upload))
        .route("/{entity_type}/{entity_id}", web::get().to(handlers::documentos::listar))
        // New — static paths first to avoid conflicts with {entity_type}
        .route("/por-vencer", web::get().to(handlers::documentos::por_vencer))
        .route("/plantillas", web::get().to(handlers::documentos::listar_plantillas))
        .route(
            "/plantillas/{id}/rellenar/{entity_type}/{entity_id}",
            web::get().to(handlers::documentos::rellenar_plantilla),
        )
        .route(
            "/cumplimiento/{entity_type}/{entity_id}",
            web::get().to(handlers::documentos::cumplimiento),
        )
        .route(
            "/digitalizar/{entity_type}/{entity_id}",
            web::post().to(handlers::documentos::digitalizar),
        )
        .route("/{id}/verificar", web::put().to(handlers::documentos::verificar))
        .route("/{id}/contenido", web::put().to(handlers::documentos::guardar_contenido))
        .route("/{id}/exportar-pdf", web::get().to(handlers::documentos::exportar_pdf))
        .route("/{id}", web::delete().to(handlers::documentos::eliminar))
)
```

## Service Layer Design

### `backend/src/services/documentos.rs` (extended)

Existing functions `upload()` and `listar_documentos()` are extended. New functions added:

```
upload()              — extended: accept tipo_documento, validate per entity_type catalog,
                        validate NCF format/uniqueness for comprobante_fiscal_ncf,
                        validate cedula match for inquilino cedula docs, audit trail
listar_documentos()   — extended: accept filter params (tipo_documento, estado_verificacion,
                        fecha_vencimiento range), return extended response
verificar()           — new: update estado_verificacion, record verificado_por/fecha,
                        require notas for rejection, clear fields on reset to pendiente, audit
eliminar()            — new: delete DB record + file from disk, audit trail
por_vencer()          — new: query docs where fecha_vencimiento within N days, estado=verificado
cumplimiento()        — new: compare required docs per entity_type vs actual docs,
                        calculate percentage
marcar_vencidos()     — new: batch update verificado→vencido where fecha_vencimiento < today
                        (called from upload/listar or a scheduled task)
```

#### Document Type Catalog (constants in `documentos.rs`)

```rust
pub const TIPOS_INQUILINO: &[&str] = &[
    "cedula", "comprobante_ingresos", "carta_referencia",
    "contrato_trabajo", "carta_no_antecedentes",
];
pub const TIPOS_PROPIEDAD: &[&str] = &[
    "titulo_propiedad", "certificacion_no_gravamen", "plano_catastral",
    "certificacion_uso_suelo", "poliza_seguro",
];
pub const TIPOS_CONTRATO: &[&str] = &[
    "contrato_arrendamiento", "acta_notarial", "registro_dgii", "addendum",
];
pub const TIPOS_PAGO: &[&str] = &[
    "recibo_pago", "comprobante_fiscal_ncf", "comprobante_transferencia",
];
pub const TIPOS_GASTO: &[&str] = &[
    "factura_proveedor", "comprobante_fiscal_ncf", "recibo_pago",
];

pub const REQUERIDOS_INQUILINO: &[&str] = &["cedula", "comprobante_ingresos"];
pub const REQUERIDOS_PROPIEDAD: &[&str] = &["titulo_propiedad"];
pub const REQUERIDOS_CONTRATO: &[&str] = &["contrato_arrendamiento"];
```

#### NCF Validation (in `validacion_fiscal.rs`)

```rust
/// Validate NCF format: single uppercase letter + exactly 10 digits (e.g., B0100000001)
pub fn validar_ncf(ncf: &str) -> Result<(), AppError> {
    let re = regex::Regex::new(r"^[A-Z]\d{10}$").unwrap();
    if !re.is_match(ncf) {
        return Err(AppError::Validation(
            "NCF inválido: debe ser una letra mayúscula seguida de 10 dígitos (ej: B0100000001)".into()
        ));
    }
    Ok(())
}
```

### `backend/src/services/plantillas.rs` (new)

```
listar()              — query plantillas_documento, optional entity_type filter
rellenar()            — load template, load entity data, resolve {{placeholders}}
```

Placeholder resolution strategy:
1. Load the template's `contenido` JSON
2. Load the target entity + related entities (e.g., for contrato: load propiedad + inquilino)
3. Walk the JSON tree, replacing `{{entity.field}}` patterns with actual values
4. Unresolved placeholders remain as `{{campo}}` and are flagged in the response

### `backend/src/services/documento_editor.rs` (new)

```
digitalizar()         — accept file, call OcrClient::extract(), convert OcrResult to
                        editor JSON format, store original as documento, return editable content
guardar_contenido()   — update documento.contenido_editable, set updated_at, audit
exportar_pdf()        — load contenido_editable JSON, render to PDF using genpdf, return bytes
```

#### Editor Content JSON Schema

The `contenido_editable` JSONB column stores a structured document:

```json
{
  "version": 1,
  "blocks": [
    {
      "type": "heading",
      "level": 1,
      "text": "CONTRATO DE ARRENDAMIENTO"
    },
    {
      "type": "paragraph",
      "text": "Entre las partes: {{arrendador}} y {{arrendatario}}..."
    },
    {
      "type": "paragraph",
      "text": "Monto mensual: RD$ 25,000.00",
      "confidence": 0.72,
      "source": "ocr"
    },
    {
      "type": "list",
      "ordered": true,
      "items": ["Primera cláusula...", "Segunda cláusula..."]
    },
    {
      "type": "table",
      "headers": ["Concepto", "Monto"],
      "rows": [["Alquiler", "RD$ 25,000.00"], ["Depósito", "RD$ 50,000.00"]]
    },
    {
      "type": "page_break"
    }
  ]
}
```

Blocks with `"source": "ocr"` and `"confidence" < 0.80` are highlighted in the editor.

#### OCR-to-Editor Conversion Flow

```
1. User uploads scanned file to POST /documentos/digitalizar/{entity_type}/{entity_id}
2. Handler saves original file as a Documento (tipo_documento = "otro")
3. Handler calls OcrClient::extract() with the file
4. documento_editor::convertir_ocr_a_editor() maps OcrResult → editor JSON:
   a. Group OCR lines into paragraphs (by vertical proximity using bbox)
   b. Set confidence per block (min confidence of constituent lines)
   c. If document_type is known, attempt to match a Plantilla and merge fields
5. Return DigitalizarResponse with editor content + low-confidence field list
6. Frontend loads content into DocumentEditor component
```

## Frontend Components

### New Files

| File | Description | Req |
|------|-------------|-----|
| `frontend/src/components/common/document_editor.rs` | Rich-text editor component | R12 |
| `frontend/src/components/common/compliance_badge.rs` | Compliance percentage badge | R6,R10 |
| `frontend/src/components/common/verification_badge.rs` | Verification status badge | R4 |
| `frontend/src/pages/documento_editor.rs` | Full-page editor view | R12,R13 |

### `DocumentEditor` Component

A `contenteditable`-based rich-text editor that renders the `contenido_editable` JSON blocks.

```
Props:
  contenido: Option<serde_json::Value>   — initial content (from template or OCR)
  readonly: bool                          — true for visualizador role
  entity_type: String
  entity_id: String
  tipo_documento: Option<String>          — from template
  on_save: Callback<serde_json::Value>    — called when user clicks "Guardar"

State:
  blocks: Vec<EditorBlock>                — parsed from contenido JSON
  dirty: bool                             — unsaved changes indicator
  exporting: bool                         — PDF export in progress

Toolbar (when !readonly):
  [H1] [H2] [B] [I] [U] [OL] [UL] [Table] [Page Break] | [Guardar] [Exportar PDF]

Rendering:
  - Each block renders as a contenteditable div with appropriate styling
  - OCR blocks with confidence < 0.80 get a yellow highlight + tooltip "Confianza baja: XX%"
  - Unresolved {{placeholders}} render with a blue highlight and are editable
  - Tables render as editable HTML tables
  - Page breaks render as a dashed horizontal line
```

Implementation approach: Use `web_sys::Document::exec_command()` for formatting (bold, italic, etc.) on `contenteditable` divs. This avoids pulling in a JS rich-text library and keeps the WASM-only stack. The editor serializes back to the block JSON format on save.

### Extended `DocumentGallery` Component

Add to the existing gallery:
- `tipo_documento` dropdown in the upload form
- Verification status badge per document
- "Editar" button that navigates to the document editor page
- "Digitalizar" button that triggers OCR → editor flow
- Filter controls for tipo_documento and estado_verificacion

### Extended `DocumentGallery` Upload Flow

```
1. User selects file + tipo_documento from dropdown
2. If tipo_documento is comprobante_fiscal_ncf, show numero_documento input
3. If tipo_documento is cedula (for inquilino), optionally show numero_documento input
4. POST multipart to /documentos/{entity_type}/{entity_id} with file + tipo_documento + optional fields
5. On success, refresh document list
```

## Dashboard Integration

Extend `backend/src/services/dashboard.rs` stats response:

```rust
pub documentos_vencidos: i64,      // count where estado_verificacion = 'vencido'
pub documentos_por_vencer: i64,    // count where fecha_vencimiento within 30 days
pub entidades_incompletas: i64,    // count of entities below 100% compliance
```

New endpoint: `GET /api/v1/documentos/cumplimiento/resumen` returns the 10 entities with lowest compliance scores.

## File Structure Summary

### New Backend Files

```
backend/migrations/m20250430_000001_extend_documentos_legal.rs
backend/migrations/m20250430_000002_add_documentos_editor.rs
backend/migrations/m20250430_000003_create_plantillas_documento.rs
backend/src/entities/plantilla_documento.rs
backend/src/services/plantillas.rs
backend/src/services/documento_editor.rs
```

### Modified Backend Files

```
backend/migrations/mod.rs                    — register new migrations
backend/src/entities/documento.rs            — add new columns
backend/src/entities/mod.rs                  — re-export plantilla_documento
backend/src/models/documento.rs              — add new DTOs
backend/src/services/documentos.rs           — extend upload/list, add verify/delete/compliance/expiry
backend/src/services/validacion_fiscal.rs    — add validar_ncf()
backend/src/services/mod.rs                  — re-export new services
backend/src/handlers/documentos.rs           — add new handler functions
backend/src/handlers/mod.rs                  — (no change needed, already exports documentos)
backend/src/routes.rs                        — add new routes
backend/src/services/dashboard.rs            — add compliance stats
```

### New Frontend Files

```
frontend/src/components/common/document_editor.rs
frontend/src/components/common/compliance_badge.rs
frontend/src/components/common/verification_badge.rs
frontend/src/pages/documento_editor.rs
```

### Modified Frontend Files

```
frontend/src/types/documento.rs              — extend response types, add new types
frontend/src/components/common/document_gallery.rs — add tipo_documento, verification, editor links
frontend/src/components/common/mod.rs        — re-export new components
frontend/src/pages/mod.rs                    — re-export documento_editor page
frontend/src/app.rs                          — add Route::DocumentoEditor
```

## Implementation Order

Tasks are ordered by dependency. Each task is independently verifiable.

### Task 1: Database Migrations
Create the three migration files and register them in `mod.rs`. Verify: `cargo test` passes, migrations apply cleanly.
**Files:** `backend/migrations/m20250430_000001_extend_documentos_legal.rs`, `m20250430_000002_add_documentos_editor.rs`, `m20250430_000003_create_plantillas_documento.rs`, `backend/migrations/mod.rs`
**Requirements:** R2, R14

### Task 2: Entity Updates
Update `documento.rs` entity with new columns. Create `plantilla_documento.rs` entity. Update `entities/mod.rs`.
**Files:** `backend/src/entities/documento.rs`, `backend/src/entities/plantilla_documento.rs`, `backend/src/entities/mod.rs`, `backend/src/entities/prelude.rs`
**Requirements:** R2, R11, R14

### Task 3: Backend Models (DTOs)
Extend `DocumentoResponse` and add all new request/response DTOs in `models/documento.rs`.
**Files:** `backend/src/models/documento.rs`
**Requirements:** R1-R14

### Task 4: NCF Validation
Add `validar_ncf()` to `validacion_fiscal.rs`.
**Files:** `backend/src/services/validacion_fiscal.rs`
**Requirements:** R8

### Task 5: Document Service — Classification & Validation
Extend `upload()` with tipo_documento validation, NCF validation, cedula cross-check. Add document type catalog constants.
**Files:** `backend/src/services/documentos.rs`
**Requirements:** R1, R3, R8

### Task 6: Document Service — Verification & Deletion
Add `verificar()`, `eliminar()`, `marcar_vencidos()` functions.
**Files:** `backend/src/services/documentos.rs`
**Requirements:** R4, R5, R9

### Task 7: Document Service — Compliance & Filtering
Add `cumplimiento()`, `por_vencer()`. Extend `listar_documentos()` with filters.
**Files:** `backend/src/services/documentos.rs`
**Requirements:** R5, R6, R7

### Task 8: Plantillas Service
Create `plantillas.rs` with `listar()` and `rellenar()`.
**Files:** `backend/src/services/plantillas.rs`, `backend/src/services/mod.rs`
**Requirements:** R11

### Task 9: Document Editor Service
Create `documento_editor.rs` with `digitalizar()`, `guardar_contenido()`, `exportar_pdf()`.
**Files:** `backend/src/services/documento_editor.rs`, `backend/src/services/mod.rs`
**Requirements:** R12, R13, R14

### Task 10: Handlers & Routes
Add all new handler functions and register routes.
**Files:** `backend/src/handlers/documentos.rs`, `backend/src/routes.rs`
**Requirements:** R1-R14

### Task 11: Dashboard Integration
Extend dashboard stats with compliance counts.
**Files:** `backend/src/services/dashboard.rs`
**Requirements:** R10

### Task 12: Frontend Types
Extend `documento.rs` types with all new DTOs.
**Files:** `frontend/src/types/documento.rs`
**Requirements:** R1-R14

### Task 13: Frontend — DocumentGallery Extension
Add tipo_documento upload, verification badges, filter controls, editor/digitalizar buttons.
**Files:** `frontend/src/components/common/document_gallery.rs`
**Requirements:** R1, R3, R4, R7

### Task 14: Frontend — Document Editor Component & Page
Create the `DocumentEditor` component and `documento_editor` page. Add route in `app.rs`.
**Files:** `frontend/src/components/common/document_editor.rs`, `frontend/src/components/common/compliance_badge.rs`, `frontend/src/components/common/verification_badge.rs`, `frontend/src/pages/documento_editor.rs`, `frontend/src/components/common/mod.rs`, `frontend/src/pages/mod.rs`, `frontend/src/app.rs`
**Requirements:** R12, R13


## Frontend Design — Visual Specification

Follows the "Tropical Professional" design system from `.impeccable.md`. All new components use existing design tokens (`--color-*`, `--space-*`, `--text-*`, `--surface-*`, `--border-*`) and component classes (`gi-*`).

### New CSS Classes (`frontend/styles/tailwind.css`)

```css
/* ============================================
   Document Management — Legal Document System
   ============================================ */

/* Verification status badges — extend gi-badge */
.gi-badge-verificado {
  background-color: var(--color-success-light);
  color: var(--color-success-dark);
}

.gi-badge-pendiente {
  background-color: var(--color-warning-light);
  color: var(--color-warning-dark);
}

.gi-badge-rechazado {
  background-color: var(--color-error-light);
  color: var(--color-error-dark);
}

.gi-badge-vencido {
  background-color: oklch(92% 0.04 50);
  color: oklch(42% 0.10 50);
}

.gi-badge-faltante {
  background-color: var(--color-sand-100);
  color: var(--color-sand-500);
  border: 1px dashed var(--color-sand-300);
}

[data-theme="dark"] .gi-badge-verificado {
  background-color: oklch(25% 0.04 155);
  color: oklch(80% 0.08 155);
}

[data-theme="dark"] .gi-badge-pendiente {
  background-color: oklch(25% 0.04 85);
  color: oklch(80% 0.10 85);
}

[data-theme="dark"] .gi-badge-rechazado {
  background-color: oklch(25% 0.04 25);
  color: oklch(80% 0.10 25);
}

[data-theme="dark"] .gi-badge-vencido {
  background-color: oklch(25% 0.04 50);
  color: oklch(80% 0.10 50);
}

[data-theme="dark"] .gi-badge-faltante {
  background-color: oklch(20% 0.01 80);
  color: oklch(55% 0.008 80);
  border-color: oklch(30% 0.01 80);
}

/* Compliance meter — horizontal bar */
.gi-compliance-meter {
  height: 6px;
  border-radius: 3px;
  background-color: var(--color-sand-200);
  overflow: hidden;
}

.gi-compliance-meter-fill {
  height: 100%;
  border-radius: 3px;
  transition: width var(--duration-normal) var(--ease-out);
}

.gi-compliance-meter-fill[data-level="high"] {
  background-color: var(--color-success);
}

.gi-compliance-meter-fill[data-level="medium"] {
  background-color: var(--color-warning);
}

.gi-compliance-meter-fill[data-level="low"] {
  background-color: var(--color-error);
}

[data-theme="dark"] .gi-compliance-meter {
  background-color: oklch(22% 0.01 185);
}

/* Document type selector — pill group */
.gi-doc-type-group {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.gi-doc-type-pill {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3);
  border-radius: 9999px;
  font-size: var(--text-xs);
  font-weight: 500;
  border: 1px solid var(--border-default);
  background-color: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.gi-doc-type-pill:hover {
  border-color: var(--color-primary-300);
  color: var(--color-primary-500);
}

.gi-doc-type-pill.selected {
  background-color: var(--color-primary-50);
  border-color: var(--color-primary-400);
  color: var(--color-primary-600);
  font-weight: 600;
}

[data-theme="dark"] .gi-doc-type-pill.selected {
  background-color: oklch(22% 0.03 185);
  border-color: var(--color-primary-400);
  color: var(--color-primary-300);
}

/* Document card — enhanced from gallery */
.gi-doc-card {
  background-color: var(--surface-raised);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  overflow: hidden;
  transition: box-shadow var(--duration-fast) var(--ease-out);
}

.gi-doc-card:hover {
  box-shadow: var(--shadow-md);
}

.gi-doc-card-preview {
  height: 100px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--color-sand-100);
  position: relative;
}

[data-theme="dark"] .gi-doc-card-preview {
  background-color: oklch(18% 0.01 185);
}

.gi-doc-card-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.gi-doc-card-body {
  padding: var(--space-2) var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.gi-doc-card-name {
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gi-doc-card-meta {
  font-size: 0.7rem;
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.gi-doc-card-actions {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3) var(--space-2);
}

/* Editor — contenteditable surface */
.gi-editor {
  background-color: var(--surface-overlay);
  border: 1px solid var(--border-default);
  border-radius: 12px;
  min-height: 500px;
  max-width: 800px;
  margin: 0 auto;
  box-shadow: var(--shadow-sm);
}

.gi-editor-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border-subtle);
  flex-wrap: wrap;
}

.gi-editor-toolbar-btn {
  background: none;
  border: none;
  padding: var(--space-1) var(--space-2);
  border-radius: 6px;
  font-size: var(--text-sm);
  color: var(--text-secondary);
  cursor: pointer;
  min-width: 32px;
  min-height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--duration-fast) var(--ease-out);
}

.gi-editor-toolbar-btn:hover {
  background-color: var(--color-sand-100);
  color: var(--text-primary);
}

.gi-editor-toolbar-btn.active {
  background-color: var(--color-primary-50);
  color: var(--color-primary-600);
}

[data-theme="dark"] .gi-editor-toolbar-btn:hover {
  background-color: oklch(22% 0.012 185);
}

[data-theme="dark"] .gi-editor-toolbar-btn.active {
  background-color: oklch(22% 0.03 185);
  color: var(--color-primary-300);
}

.gi-editor-toolbar-sep {
  width: 1px;
  height: 20px;
  background-color: var(--border-subtle);
  margin: 0 var(--space-1);
}

.gi-editor-content {
  padding: var(--space-6) var(--space-7);
  font-family: var(--font-body);
  font-size: var(--text-base);
  line-height: 1.7;
  color: var(--text-primary);
  outline: none;
  min-height: 400px;
}

.gi-editor-content h1 {
  font-family: var(--font-display);
  font-size: var(--text-xl);
  font-weight: 700;
  margin-bottom: var(--space-4);
  color: var(--text-primary);
}

.gi-editor-content h2 {
  font-family: var(--font-display);
  font-size: var(--text-lg);
  font-weight: 600;
  margin-top: var(--space-5);
  margin-bottom: var(--space-3);
  color: var(--text-primary);
}

.gi-editor-content p {
  margin-bottom: var(--space-3);
}

.gi-editor-content table {
  width: 100%;
  border-collapse: collapse;
  margin: var(--space-4) 0;
}

.gi-editor-content table td,
.gi-editor-content table th {
  border: 1px solid var(--border-default);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
}

.gi-editor-content table th {
  background-color: var(--color-sand-100);
  font-weight: 600;
}

[data-theme="dark"] .gi-editor-content table th {
  background-color: oklch(20% 0.012 185);
}

/* OCR low-confidence highlight */
.gi-editor-ocr-low {
  background-color: oklch(92% 0.08 85);
  border-radius: 3px;
  padding: 1px 3px;
  cursor: help;
}

[data-theme="dark"] .gi-editor-ocr-low {
  background-color: oklch(30% 0.06 85);
}

/* Unresolved placeholder highlight */
.gi-editor-placeholder {
  background-color: var(--color-info-light);
  border-radius: 3px;
  padding: 1px 3px;
  font-style: italic;
  cursor: text;
}

[data-theme="dark"] .gi-editor-placeholder {
  background-color: oklch(25% 0.04 230);
}

/* Page break indicator in editor */
.gi-editor-page-break {
  border: none;
  border-top: 2px dashed var(--border-default);
  margin: var(--space-6) 0;
  position: relative;
}

.gi-editor-page-break::after {
  content: "Salto de página";
  position: absolute;
  top: -10px;
  left: 50%;
  transform: translateX(-50%);
  background-color: var(--surface-overlay);
  padding: 0 var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

/* Compliance checklist */
.gi-compliance-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.gi-compliance-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-radius: 8px;
  font-size: var(--text-sm);
  transition: background-color var(--duration-fast) var(--ease-out);
}

.gi-compliance-item:hover {
  background-color: var(--color-sand-50);
}

[data-theme="dark"] .gi-compliance-item:hover {
  background-color: oklch(18% 0.01 185);
}

.gi-compliance-item-icon {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
}

.gi-compliance-item-icon.presente {
  background-color: var(--color-success-light);
  color: var(--color-success-dark);
}

.gi-compliance-item-icon.faltante {
  background-color: var(--color-sand-100);
  color: var(--color-sand-400);
  border: 1px dashed var(--color-sand-300);
}

.gi-compliance-item-icon.pendiente {
  background-color: var(--color-warning-light);
  color: var(--color-warning-dark);
}

.gi-compliance-item-icon.vencido {
  background-color: oklch(92% 0.04 50);
  color: oklch(42% 0.10 50);
}

.gi-compliance-item-icon.rechazado {
  background-color: var(--color-error-light);
  color: var(--color-error-dark);
}

/* Template selector cards */
.gi-template-card {
  background-color: var(--surface-raised);
  border: 1px solid var(--border-subtle);
  border-radius: 10px;
  padding: var(--space-4);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out);
}

.gi-template-card:hover {
  border-color: var(--color-primary-300);
  box-shadow: var(--shadow-md);
}

.gi-template-card.selected {
  border-color: var(--color-primary-400);
  background-color: var(--color-primary-50);
}

[data-theme="dark"] .gi-template-card:hover {
  border-color: var(--color-primary-400);
}

[data-theme="dark"] .gi-template-card.selected {
  border-color: var(--color-primary-400);
  background-color: oklch(20% 0.02 185);
}

.gi-template-card-icon {
  font-size: 1.5rem;
  margin-bottom: var(--space-2);
}

.gi-template-card-name {
  font-family: var(--font-display);
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.gi-template-card-desc {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  line-height: 1.4;
}

/* Expiring documents alert strip */
.gi-expiry-strip {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background-color: oklch(95% 0.04 50);
  border: 1px solid oklch(80% 0.08 50);
  border-radius: 10px;
  font-size: var(--text-sm);
  color: oklch(35% 0.08 50);
}

[data-theme="dark"] .gi-expiry-strip {
  background-color: oklch(22% 0.04 50);
  border-color: oklch(35% 0.06 50);
  color: oklch(80% 0.08 50);
}

.gi-expiry-strip-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
}

/* Print styles for editor export */
@media print {
  .gi-editor-toolbar,
  .gi-editor-toolbar-btn {
    display: none !important;
  }

  .gi-editor {
    border: none;
    box-shadow: none;
    border-radius: 0;
  }

  .gi-editor-content {
    padding: 0;
  }

  .gi-editor-page-break {
    page-break-after: always;
    border: none;
    margin: 0;
  }

  .gi-editor-page-break::after {
    display: none;
  }

  .gi-editor-ocr-low {
    background-color: transparent;
  }

  .gi-editor-placeholder {
    background-color: transparent;
    font-style: normal;
  }
}
```

### Component Layout Specifications

#### 1. Enhanced Document Gallery (within entity detail views)

```
┌─────────────────────────────────────────────────────────┐
│ Documentos Legales                    [📎 Subir] [📷 OCR] │
│                                                          │
│ Tipo: [Todos ▾] Estado: [Todos ▾]                       │
│                                                          │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐    │
│ │  preview  │ │  preview  │ │  preview  │ │  preview  │    │
│ │          │ │          │ │          │ │          │    │
│ ├──────────┤ ├──────────┤ ├──────────┤ ├──────────┤    │
│ │ filename │ │ filename │ │ filename │ │ filename │    │
│ │ ●Verif.  │ │ ○Pend.   │ │ ✕Rech.   │ │ ⏰Venc.   │    │
│ │ 1.2 MB   │ │ 340 KB   │ │ 2.1 MB   │ │ 890 KB   │    │
│ │ [✏️][🗑️] │ │ [✏️][🗑️] │ │ [✏️][🗑️] │ │ [✏️][🗑️] │    │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘    │
└─────────────────────────────────────────────────────────┘
```

- Grid: `repeat(auto-fill, minmax(160px, 1fr))` with `gap: var(--space-3)`
- Each card uses `.gi-doc-card` with preview, body, and actions sections
- Verification badge uses `.gi-badge` + status variant
- Filter dropdowns use `.gi-input` styled as `<select>`

#### 2. Compliance Profile (sidebar panel or inline section)

```
┌─────────────────────────────────────┐
│ Cumplimiento Legal           78%    │
│ ████████████████░░░░░                │
│                                      │
│ ● Cédula                  Verificado │
│ ● Comprobante ingresos    Pendiente  │
│ ○ Carta referencia        Faltante   │
│ ○ Contrato trabajo        Faltante   │
│ ○ Carta no antecedentes   Faltante   │
└─────────────────────────────────────┘
```

- Uses `.gi-compliance-meter` for the progress bar
- Level thresholds: `high` ≥ 80%, `medium` ≥ 50%, `low` < 50%
- Each row uses `.gi-compliance-item` with icon + name + badge
- Required items shown first, optional items below a collapsible

#### 3. Document Editor Page

```
┌─────────────────────────────────────────────────────────────┐
│ ← Volver                                    [Guardar] [PDF] │
│                                                              │
│ ┌──────────────────────────────────────────────────────────┐ │
│ │ [H1][H2] [B][I][U] │ [OL][UL] │ [Table] [—] │          │ │
│ ├──────────────────────────────────────────────────────────┤ │
│ │                                                          │ │
│ │  CONTRATO DE ARRENDAMIENTO                               │ │
│ │                                                          │ │
│ │  Entre las partes: Juan Pérez (arrendador) y             │ │
│ │  María García (arrendataria), se acuerda lo siguiente:   │ │
│ │                                                          │ │
│ │  Monto mensual: ██RD$ 25,000.00██  ← OCR low confidence │ │
│ │                                                          │ │
│ │  1. Primera cláusula...                                  │ │
│ │  2. Segunda cláusula...                                  │ │
│ │                                                          │ │
│ │  ┌──────────┬──────────────┐                             │ │
│ │  │ Concepto │ Monto        │                             │ │
│ │  ├──────────┼──────────────┤                             │ │
│ │  │ Alquiler │ RD$ 25,000   │                             │ │
│ │  │ Depósito │ RD$ 50,000   │                             │ │
│ │  └──────────┴──────────────┘                             │ │
│ │                                                          │ │
│ │  - - - - - Salto de página - - - - -                     │ │
│ │                                                          │ │
│ └──────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

- Editor container: `.gi-editor` with max-width 800px, centered
- Toolbar: `.gi-editor-toolbar` with icon buttons
- Content area: `.gi-editor-content` with `contenteditable="true"`
- OCR low-confidence: `.gi-editor-ocr-low` (warm yellow highlight)
- Unresolved placeholders: `.gi-editor-placeholder` (info blue highlight)
- Page breaks: `.gi-editor-page-break` (dashed line with label)
- Read-only mode: remove `contenteditable`, hide toolbar save button, keep PDF export

#### 4. Template Selector (shown before editor opens)

```
┌─────────────────────────────────────────────────────────────┐
│ Seleccionar Plantilla                                        │
│                                                              │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ │
│ │ 📋               │ │ 🧾               │ │ 📝               │ │
│ │ Contrato de      │ │ Recibo de       │ │ Acta             │ │
│ │ Arrendamiento    │ │ Pago            │ │ Notarial         │ │
│ │                  │ │                  │ │                  │ │
│ │ Contrato estándar│ │ Recibo para     │ │ Acta de entrega  │ │
│ │ de alquiler con  │ │ pagos de        │ │ o devolución de  │ │
│ │ cláusulas DR     │ │ alquiler        │ │ propiedad        │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────┘ │
│                                                              │
│ ┌─────────────────┐ ┌─────────────────┐                     │
│ │ ✉️               │ │ 📄               │                     │
│ │ Carta de        │ │ Addendum        │                     │
│ │ Referencia      │ │                  │                     │
│ │                  │ │ Modificación a  │                     │
│ │ Carta de ref.   │ │ contrato        │                     │
│ │ para inquilino  │ │ existente       │                     │
│ └─────────────────┘ └─────────────────┘                     │
│                                                              │
│                              [Crear Documento en Blanco]     │
└─────────────────────────────────────────────────────────────┘
```

- Grid: `repeat(auto-fill, minmax(200px, 1fr))` with `gap: var(--space-4)`
- Each card uses `.gi-template-card`
- Click selects and navigates to editor with pre-filled content

#### 5. Dashboard Compliance Widget

```
┌─────────────────────────────────────┐
│ 📋 Documentos                        │
│                                      │
│ 3 vencidos  ·  5 por vencer         │
│ 8 entidades incompletas             │
│                                      │
│ Ver detalle →                        │
└─────────────────────────────────────┘
```

- Uses existing `.gi-stat` card pattern
- Counts use `.tabular-nums` for alignment
- "vencidos" count in error color, "por vencer" in warning color
- Link to compliance overview page

### Route Addition (`frontend/src/app.rs`)

```rust
#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    // ... existing routes ...
    #[at("/documentos/editor/:entity_type/:entity_id")]
    DocumentoEditor {
        entity_type: String,
        entity_id: String,
    },
    #[at("/documentos/editor/:entity_type/:entity_id/:documento_id")]
    DocumentoEditorExisting {
        entity_type: String,
        entity_id: String,
        documento_id: String,
    },
}
```

### Accessibility Notes

- All interactive elements have `focus-visible` outlines via existing `.gi-btn:focus-visible` etc.
- Editor toolbar buttons have `aria-label` attributes (e.g., `aria-label="Negrita"`)
- Compliance status icons have `aria-label` with the status text
- OCR low-confidence highlights have `title` tooltips with confidence percentage
- Color is never the sole indicator — badges include text labels, compliance items include icons
- Editor content area has `role="textbox"` and `aria-multiline="true"`
- Template cards are keyboard-navigable with `tabindex="0"` and `Enter`/`Space` activation
