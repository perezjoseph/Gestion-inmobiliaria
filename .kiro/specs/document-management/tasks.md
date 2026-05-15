# Implementation Plan: Document Management

## Overview

Extends the existing `documentos` system with legal document classification, verification workflows, expiration tracking, compliance profiles, a template-based document editor, and OCR-to-editable-document conversion. Implementation uses the established layered architecture (handlers → services → entities) with Rust (Actix-web backend, Yew/WASM frontend).

## Tasks

- [x] 1. Database Migrations
  - [x] 1.1 Create `backend/migrations/m20250430_000001_extend_documentos_legal.rs`: Add `tipo_documento` (VARCHAR(50), NOT NULL, DEFAULT 'otro'), `estado_verificacion` (VARCHAR(20), NOT NULL, DEFAULT 'pendiente'), `fecha_vencimiento` (DATE, nullable), `verificado_por` (UUID, FK to usuarios, nullable), `fecha_verificacion` (TIMESTAMPTZ, nullable), `notas_verificacion` (TEXT, nullable), `numero_documento` (VARCHAR(100), nullable) to `documentos` table. Create indexes on `tipo_documento`, `estado_verificacion`, `fecha_vencimiento`.
    - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - [x] 1.2 Create `backend/migrations/m20250430_000002_add_documentos_editor.rs`: Add `contenido_editable` (JSONB, nullable) and `updated_at` (TIMESTAMPTZ, nullable) to `documentos` table.
    - _Requirements: 14.1, 14.2, 14.4_
  - [x] 1.3 Create `backend/migrations/m20250430_000003_create_plantillas_documento.rs`: Create `plantillas_documento` table with id (UUID PK), nombre (VARCHAR(100)), tipo_documento (VARCHAR(50)), entity_type (VARCHAR(50)), contenido (JSONB), activo (BOOLEAN DEFAULT true), created_at, updated_at. Create indexes on entity_type and tipo_documento. Seed built-in templates for contrato_arrendamiento, recibo_pago, acta_notarial, carta_referencia, addendum.
    - _Requirements: 11.1, 11.2_
  - [x] 1.4 Register all three migrations in `backend/migrations/mod.rs` Migrator.
    - _Requirements: 2.2, 14.2_

- [x] 2. Entity Updates
  - [x] 2.1 Update `backend/src/entities/documento.rs`: Add new columns to Model — tipo_documento (String), estado_verificacion (String), fecha_vencimiento (Option<Date>), verificado_por (Option<Uuid>), fecha_verificacion (Option<DateTimeWithTimeZone>), notas_verificacion (Option<String>), numero_documento (Option<String>), contenido_editable (Option<Json>), updated_at (Option<DateTimeWithTimeZone>).
    - _Requirements: 2.1, 14.1, 14.4_
  - [x] 2.2 Create `backend/src/entities/plantilla_documento.rs`: New SeaORM entity with Model (id, nombre, tipo_documento, entity_type, contenido as JsonBinary, activo, created_at, updated_at), empty Relation enum, ActiveModelBehavior impl.
    - _Requirements: 11.2_
  - [x] 2.3 Update `backend/src/entities/mod.rs` and `backend/src/entities/prelude.rs` to re-export plantilla_documento.
    - _Requirements: 11.2_

- [x] 3. Backend Models (DTOs)
  - [x] 3.1 Extend `DocumentoResponse` in `backend/src/models/documento.rs` with: tipo_documento, estado_verificacion, fecha_vencimiento (Option<NaiveDate>), verificado_por (Option<Uuid>), fecha_verificacion (Option<DateTime<Utc>>), notas_verificacion (Option<String>), numero_documento (Option<String>), contenido_editable (Option<serde_json::Value>), updated_at (Option<DateTime<Utc>>).
    - _Requirements: 2.1, 2.5, 14.1, 14.5_
  - [x] 3.2 Add `DocumentoListQuery` struct: tipo_documento (Option<String>), estado_verificacion (Option<String>), fecha_vencimiento_desde (Option<NaiveDate>), fecha_vencimiento_hasta (Option<NaiveDate>). Deserialize with camelCase.
    - _Requirements: 7.2_
  - [x] 3.3 Add `VerificarDocumentoRequest` struct: estado_verificacion (String), notas_verificacion (Option<String>). Deserialize with camelCase.
    - _Requirements: 4.1, 4.4_
  - [x] 3.4 Add `PorVencerQuery` struct: dias (Option<i64>). Deserialize with camelCase.
    - _Requirements: 5.3_
  - [x] 3.5 Add `CumplimientoResponse` and `CumplimientoItem` structs. CumplimientoResponse: entity_type, entity_id (Uuid), documentos (Vec<CumplimientoItem>), porcentaje (u8). CumplimientoItem: tipo_documento, nombre, requerido (bool), estado. Serialize with camelCase.
    - _Requirements: 6.1, 6.3, 6.4_
  - [x] 3.6 Add `GuardarEditorRequest` struct: contenido_editable (serde_json::Value). Deserialize with camelCase.
    - _Requirements: 14.3_
  - [x] 3.7 Add `PlantillaResponse` and `PlantillaRellenadaResponse` structs. PlantillaResponse: id, nombre, tipo_documento, entity_type, contenido (serde_json::Value). PlantillaRellenadaResponse: plantilla_id, nombre, tipo_documento, contenido. Serialize with camelCase.
    - _Requirements: 11.4, 11.5_
  - [x] 3.8 Add `DigitalizarResponse` struct: document_type, contenido_editable (serde_json::Value), campos_baja_confianza (Vec<String>), documento_original_id (Uuid). Serialize with camelCase.
    - _Requirements: 13.1, 13.4, 13.8_

- [x] 4. NCF Validation
  - [x] 4.1 Add `validar_ncf(ncf: &str) -> Result<(), AppError>` to `backend/src/services/validacion_fiscal.rs`. Validate NCF format: single uppercase letter followed by exactly 10 digits (regex `^[A-Z]\d{10}$`). Return AppError::Validation with Spanish error message on failure.
    - _Requirements: 8.1, 8.2_
  - [x] 4.2 Add unit tests for validar_ncf: valid NCF (e.g., B0100000001), invalid format (lowercase, wrong length, no letter prefix), empty string, NCF with hyphens.
    - _Requirements: 8.1, 8.2_

- [x] 5. Document Service — Classification & Validation
  - [x] 5.1 Add document type catalog constants to `backend/src/services/documentos.rs`: TIPOS_INQUILINO, TIPOS_PROPIEDAD, TIPOS_CONTRATO, TIPOS_PAGO, TIPOS_GASTO arrays, and REQUERIDOS_INQUILINO, REQUERIDOS_PROPIEDAD, REQUERIDOS_CONTRATO arrays.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 6.2_
  - [x] 5.2 Add `validate_tipo_documento(entity_type, tipo_documento) -> Result<(), AppError>` function that checks tipo_documento against the catalog for the given entity_type. Return 422 with valid types listed on failure.
    - _Requirements: 1.6, 1.7_
  - [x] 5.3 Extend `upload()` signature to accept tipo_documento, fecha_vencimiento, numero_documento, notas_verificacion parameters. Validate tipo_documento, validate NCF for comprobante_fiscal_ncf (require numero_documento, call validar_ncf, check uniqueness within organization), validate cedula match for inquilino cedula docs. Record audit trail via auditoria::registrar_best_effort().
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 8.1, 8.3_
  - [x] 5.4 Update the From<documento::Model> conversion to map all new fields into DocumentoResponse.
    - _Requirements: 2.5, 7.1, 14.5_

- [x] 6. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 7. Document Service — Verification & Deletion
  - [x] 7.1 Add `verificar(db, documento_id, request, usuario_id) -> Result<DocumentoResponse, AppError>` to `backend/src/services/documentos.rs`. Validate estado_verificacion is verificado/rechazado/pendiente. On verificado: set verificado_por and fecha_verificacion. On rechazado: require notas_verificacion. On pendiente: clear verificado_por, fecha_verificacion, notas_verificacion. Record audit trail. Return 404 if not found.
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.8_
  - [x] 7.2 Add `eliminar(db, documento_id, usuario_id) -> Result<(), AppError>`. Find document, delete file from disk, delete DB record, record audit trail. Return 404 if not found.
    - _Requirements: 9.1, 9.2, 9.3_
  - [x] 7.3 Add `marcar_vencidos(db) -> Result<u64, AppError>`. Batch update: set estado_verificacion='vencido' where fecha_vencimiento < today AND estado_verificacion='verificado'. Return count of updated records.
    - _Requirements: 5.1_

- [x] 8. Document Service — Compliance & Filtering
  - [x] 8.1 Add `cumplimiento(db, entity_type, entity_id) -> Result<CumplimientoResponse, AppError>`. Get required types for entity_type, query existing docs, determine status per type (presente/pendiente/vencido/rechazado/faltante), calculate percentage. Return 422 for invalid entity_type, 404 if entity doesn't exist.
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_
  - [x] 8.2 Add `por_vencer(db, dias) -> Result<Vec<DocumentoResponse>, AppError>`. Query docs where fecha_vencimiento within dias days (default 30, min 1, max 365) and estado_verificacion='verificado'. Sort by fecha_vencimiento ascending.
    - _Requirements: 5.2, 5.3, 5.4, 5.5_
  - [x] 8.3 Extend `listar_documentos()` to accept DocumentoListQuery filter params. Filter by tipo_documento, estado_verificacion, fecha_vencimiento date range. Sort by created_at descending. Call marcar_vencidos() before listing.
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [x] 9. Plantillas Service
  - [x] 9.1 Create `backend/src/services/plantillas.rs` with `listar(db, entity_type_filter) -> Result<Vec<PlantillaResponse>, AppError>`. Query plantillas_documento where activo=true, optionally filter by entity_type.
    - _Requirements: 11.4_
  - [x] 9.2 Add `rellenar(db, plantilla_id, entity_type, entity_id) -> Result<PlantillaRellenadaResponse, AppError>`. Load template, load target entity + related entities, walk contenido JSON replacing {{entity.field}} placeholders with actual values. Return 404 if template or entity not found.
    - _Requirements: 11.3, 11.5, 11.6_
  - [x] 9.3 Re-export plantillas in `backend/src/services/mod.rs`.
    - _Requirements: 11.4_

- [x] 10. Document Editor Service
  - [x] 10.1 Create `backend/src/services/documento_editor.rs` with `digitalizar(db, entity_type, entity_id, file_data, filename, mime_type, uploaded_by) -> Result<DigitalizarResponse, AppError>`. Store original file as Documento (tipo_documento="otro"), call OcrClient::extract(), convert OcrResult to editor JSON via convertir_ocr_a_editor() (group lines into paragraphs by bbox proximity, set confidence per block, match Plantilla if document_type known), flag fields with confidence < 0.80.
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 13.7_
  - [x] 10.2 Add `guardar_contenido(db, documento_id, contenido, usuario_id) -> Result<DocumentoResponse, AppError>`. Update contenido_editable and updated_at. Record audit trail. Return 404 if not found.
    - _Requirements: 14.3, 14.6_
  - [x] 10.3 Add `exportar_pdf(db, documento_id) -> Result<Vec<u8>, AppError>`. Load contenido_editable, render to PDF using genpdf. Map editor blocks to genpdf elements: headings, paragraphs, lists, tables, page breaks.
    - _Requirements: 12.5_
  - [x] 10.4 Re-export documento_editor in `backend/src/services/mod.rs`.
    - _Requirements: 13.1_

- [x] 11. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 12. Handlers & Routes
  - [x] 12.1 Extend `upload` handler in `backend/src/handlers/documentos.rs` to parse tipo_documento, fecha_vencimiento, numero_documento, notas_verificacion from multipart fields. Extend `listar` handler to parse DocumentoListQuery from query params.
    - _Requirements: 3.1, 3.3, 7.1_
  - [x] 12.2 Add `verificar` handler: PUT /{id}/verificar, WriteAccess, parse VerificarDocumentoRequest from JSON body.
    - _Requirements: 4.1, 4.7_
  - [x] 12.3 Add `eliminar` handler: DELETE /{id}, WriteAccess.
    - _Requirements: 9.1, 9.4_
  - [x] 12.4 Add `por_vencer` handler: GET /por-vencer, Claims, parse PorVencerQuery.
    - _Requirements: 5.2_
  - [x] 12.5 Add `cumplimiento` handler: GET /cumplimiento/{entity_type}/{entity_id}, Claims.
    - _Requirements: 6.4_
  - [x] 12.6 Add `listar_plantillas` handler: GET /plantillas, Claims, optional entity_type query param. Add `rellenar_plantilla` handler: GET /plantillas/{id}/rellenar/{entity_type}/{entity_id}, Claims.
    - _Requirements: 11.4, 11.5_
  - [x] 12.7 Add `digitalizar` handler: POST /digitalizar/{entity_type}/{entity_id}, WriteAccess, multipart file.
    - _Requirements: 13.1_
  - [x] 12.8 Add `guardar_contenido` handler: PUT /{id}/contenido, WriteAccess, JSON body. Add `exportar_pdf` handler: GET /{id}/exportar-pdf, Claims, return application/pdf.
    - _Requirements: 12.5, 14.3_
  - [x] 12.9 Register all routes in `backend/src/routes.rs` under /documentos scope. Static paths before dynamic paths.
    - _Requirements: 1.6, 3.1, 4.1, 5.2, 6.4, 9.1, 11.4, 12.5, 13.1, 14.3_

- [x] 13. Dashboard Integration
  - [x] 13.1 Extend dashboard stats in `backend/src/services/dashboard.rs` with documentos_vencidos (count where estado_verificacion='vencido'), documentos_por_vencer (count where fecha_vencimiento within 30 days and estado_verificacion='verificado'), entidades_incompletas (count of entities below 100% compliance). Use batch queries with is_in() to avoid N+1.
    - _Requirements: 10.1, 10.2, 10.3_
  - [x] 13.2 Add compliance summary handler at GET /api/v1/documentos/cumplimiento/resumen returning the 10 entities with lowest compliance percentages sorted ascending.
    - _Requirements: 10.4_

- [x] 14. Checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 15. Frontend Types
  - [x] 15.1 Extend `DocumentoResponse` in `frontend/src/types/documento.rs` with: tipo_documento, estado_verificacion, fecha_vencimiento, verificado_por, notas_verificacion, numero_documento (all Option<String>), contenido_editable (Option<serde_json::Value>), updated_at (Option<String>).
    - _Requirements: 2.5, 7.1, 14.5_
  - [x] 15.2 Add CumplimientoResponse, CumplimientoItem, PlantillaResponse, DigitalizarResponse structs with camelCase serde rename.
    - _Requirements: 6.4, 11.4, 13.8_

- [x] 16. Frontend CSS — Document Management Styles
  - [x] 16.1 Add verification status badges to `frontend/styles/tailwind.css`: .gi-badge-verificado, .gi-badge-pendiente, .gi-badge-rechazado, .gi-badge-vencido, .gi-badge-faltante with light/dark mode variants using existing OKLCH tokens.
    - _Requirements: 4.1, 6.1_
  - [x] 16.2 Add compliance meter (.gi-compliance-meter, .gi-compliance-meter-fill with data-level attribute), document type pills (.gi-doc-type-group, .gi-doc-type-pill), and document cards (.gi-doc-card, .gi-doc-card-preview, .gi-doc-card-body, .gi-doc-card-name, .gi-doc-card-meta, .gi-doc-card-actions).
    - _Requirements: 6.3, 7.1_
  - [x] 16.3 Add editor styles: .gi-editor, .gi-editor-toolbar, .gi-editor-toolbar-btn, .gi-editor-toolbar-sep, .gi-editor-content (with h1/h2/p/table styles), .gi-editor-ocr-low, .gi-editor-placeholder, .gi-editor-page-break. Include print styles for editor.
    - _Requirements: 12.1, 13.4_
  - [x] 16.4 Add compliance list (.gi-compliance-list, .gi-compliance-item, .gi-compliance-item-icon with status variants), template cards (.gi-template-card, .gi-template-card-icon, .gi-template-card-name, .gi-template-card-desc), and expiry strip (.gi-expiry-strip).
    - _Requirements: 6.1, 11.4, 5.2_

- [x] 17. Frontend — DocumentGallery Extension
  - [x] 17.1 Add tipo_documento dropdown to upload form in `frontend/src/components/common/document_gallery.rs`, populated per entity_type from the catalog. Show numero_documento input when tipo_documento is comprobante_fiscal_ncf. Add tipo_documento and estado_verificacion as multipart fields to upload request.
    - _Requirements: 1.6, 3.3, 8.1_
  - [x] 17.2 Display verification status badge on each document card using .gi-badge + status variant. Add filter controls (tipo_documento and estado_verificacion dropdowns) above the document grid.
    - _Requirements: 4.1, 7.2_
  - [x] 17.3 Add "Editar" button per document navigating to /documentos/editor/{entity_type}/{entity_id}/{documento_id}. Add "Digitalizar" button in header triggering OCR-to-editor flow. Use .gi-doc-card styling. Split into sub-components to keep html! blocks under 150 lines.
    - _Requirements: 12.2, 13.1_

- [x] 18. Frontend — Compliance Badge & Verification Badge Components
  - [x] 18.1 Create `frontend/src/components/common/compliance_badge.rs`: ComplianceBadge component with porcentaje (u8) prop. Render .gi-compliance-meter with fill width and color based on level (high >= 80%, medium >= 50%, low < 50%). Show percentage text.
    - _Requirements: 6.3, 10.3_
  - [x] 18.2 Create `frontend/src/components/common/verification_badge.rs`: VerificationBadge component with estado (String) prop. Render .gi-badge with appropriate status variant class and Spanish label.
    - _Requirements: 4.1_
  - [x] 18.3 Re-export both components in `frontend/src/components/common/mod.rs`.
    - _Requirements: 6.3, 4.1_

- [x] 19. Frontend — Document Editor Component & Page
  - [x] 19.1 Create `frontend/src/components/common/document_editor.rs`: DocumentEditor component with props contenido (Option<serde_json::Value>), readonly (bool), entity_type, entity_id, tipo_documento (Option<String>), on_save (Callback<serde_json::Value>). Render .gi-editor with toolbar (H1, H2, B, I, U, OL, UL, Table, Page Break, Guardar, Exportar PDF) and contenteditable content area. Use web_sys::Document::exec_command() for formatting. Parse/serialize editor JSON blocks. Highlight OCR low-confidence (.gi-editor-ocr-low) and unresolved placeholders (.gi-editor-placeholder). Read-only mode hides save, keeps PDF export.
    - _Requirements: 12.1, 12.2, 12.3, 12.5, 12.7, 12.8, 13.4_
  - [x] 19.2 Create `frontend/src/pages/documento_editor.rs`: Editor page with routes /documentos/editor/{entity_type}/{entity_id} (new doc) and /documentos/editor/{entity_type}/{entity_id}/{documento_id} (existing). On mount: fetch document or show template selector (.gi-template-card grid). "Crear Documento en Blanco" option. Save calls PUT /documentos/{id}/contenido. Export PDF calls GET /documentos/{id}/exportar-pdf.
    - _Requirements: 12.2, 12.5, 12.6, 14.3, 14.5_
  - [x] 19.3 Add Route::DocumentoEditor and Route::DocumentoEditorExisting to `frontend/src/app.rs`. Re-export in `frontend/src/components/common/mod.rs` and `frontend/src/pages/mod.rs`.
    - _Requirements: 12.1_

- [x] 20. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are complete — the full document management feature has been implemented
- Each task references specific requirements for traceability
- The design does not include Correctness Properties, so no property-based test tasks were added
- Implementation uses Rust throughout (Actix-web backend, Yew/WASM frontend)
