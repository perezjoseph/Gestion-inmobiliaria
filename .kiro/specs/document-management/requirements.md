# Requirements Document

## Introduction

This feature extends the existing polymorphic document attachment system to support Dominican Republic legal document management for property management. Currently, the `documentos` table stores generic file attachments (filename, path, MIME type) linked to entities via `entity_type`/`entity_id`, but has no concept of document categories, legal requirements, expiration tracking, or compliance status. Dominican Republic property management requires specific legal documents per entity type: cédula and proof of income for inquilinos, título de propiedad and certificación de no gravamen for propiedades, notarized lease agreements for contratos, and comprobantes fiscales (NCF) for pagos. This feature adds document type classification, expiration tracking, compliance validation, a completeness dashboard, a template-based document editor for generating legal documents with OCR-prefilled fields, and an OCR-to-editable-text converter for digitizing scanned documents — so property managers can ensure all legally required documents are on file, up to date, and editable.

## Glossary

- **Sistema_Documentos**: The document management subsystem responsible for classifying, tracking, validating, and reporting on legal documents attached to platform entities.
- **Documento**: An existing file attachment record in the `documentos` table, extended with legal metadata (document type, expiration, verification status).
- **Tipo_Documento**: The legal classification of a document. Each entity type has its own set of valid document types (see Requirement 1 for the full catalog).
- **Estado_Verificacion**: The verification status of a document. Valid values: `pendiente`, `verificado`, `rechazado`, `vencido`.
- **Propiedad**: An existing rental property entity in the system.
- **Inquilino**: An existing tenant entity in the system, identified by cédula.
- **Contrato**: An existing lease contract entity linking a Propiedad and an Inquilino.
- **Pago**: An existing payment entity linked to a Contrato.
- **NCF**: Número de Comprobante Fiscal — a tax receipt number issued by the DGII (Dirección General de Impuestos Internos) required on all fiscal transactions in the Dominican Republic.
- **DGII**: Dirección General de Impuestos Internos — the Dominican Republic tax authority.
- **RNC**: Registro Nacional del Contribuyente — the taxpayer identification number validated by the existing `validacion_fiscal.rs` module.
- **Certificacion_No_Gravamen**: A certificate from the Registro de Títulos confirming a property has no liens or encumbrances.
- **Plano_Catastral**: A cadastral survey map registered with the Dirección Nacional de Mensuras Catastrales.
- **Usuario_Autorizado**: A user with role `admin` or `gerente`.
- **Usuario_Visualizador**: A user with role `visualizador`.
- **Perfil_Cumplimiento**: A per-entity summary showing which required documents are present, missing, expired, or pending verification.
- **Plantilla_Documento**: A reusable document template with placeholder fields that can be auto-filled from entity data or OCR results. Templates are defined per Tipo_Documento.
- **Editor_Documento**: The in-browser document editor component that allows users to create, edit, and export legal documents based on Plantilla_Documento definitions.
- **Documento_Editable**: A document record created from a template or from OCR extraction, stored as structured JSON content that can be edited and exported to PDF.
- **OCR_Digitalizacion**: The process of converting a scanned image or PDF into an editable Documento_Editable by extracting text via the existing OCR service and mapping it to a structured document format.

## Requirements

### Requirement 1: Document Type Catalog

**User Story:** As a property manager, I want each uploaded document to be classified by its legal type, so that I can track which specific legal documents are on file for each entity.

#### Acceptance Criteria

1. THE Sistema_Documentos SHALL define the following valid Tipo_Documento values for entity type `inquilino`: `cedula`, `comprobante_ingresos`, `carta_referencia`, `contrato_trabajo`, `carta_no_antecedentes`.
2. THE Sistema_Documentos SHALL define the following valid Tipo_Documento values for entity type `propiedad`: `titulo_propiedad`, `certificacion_no_gravamen`, `plano_catastral`, `certificacion_uso_suelo`, `poliza_seguro`.
3. THE Sistema_Documentos SHALL define the following valid Tipo_Documento values for entity type `contrato`: `contrato_arrendamiento`, `acta_notarial`, `registro_dgii`, `addendum`.
4. THE Sistema_Documentos SHALL define the following valid Tipo_Documento values for entity type `pago`: `recibo_pago`, `comprobante_fiscal_ncf`, `comprobante_transferencia`.
5. THE Sistema_Documentos SHALL define the following valid Tipo_Documento values for entity type `gasto`: `factura_proveedor`, `comprobante_fiscal_ncf`, `recibo_pago`.
6. WHEN a document is uploaded, THE Sistema_Documentos SHALL require a `tipo_documento` field matching a valid Tipo_Documento for the specified `entity_type`.
7. IF a `tipo_documento` value is not valid for the given `entity_type`, THEN THE Sistema_Documentos SHALL return HTTP status 422 with a descriptive error message in Spanish listing the valid types for that entity.

### Requirement 2: Document Metadata Extension

**User Story:** As a property manager, I want to record expiration dates and verification status on documents, so that I can track document validity over time.

#### Acceptance Criteria

1. THE Sistema_Documentos SHALL extend the `documentos` table with the following columns: `tipo_documento` (VARCHAR, NOT NULL), `estado_verificacion` (VARCHAR, NOT NULL, default `pendiente`), `fecha_vencimiento` (DATE, nullable), `verificado_por` (UUID, nullable, FK to `usuarios`), `fecha_verificacion` (TIMESTAMP WITH TIME ZONE, nullable), `notas_verificacion` (TEXT, nullable), `numero_documento` (VARCHAR, nullable).
2. THE Sistema_Documentos SHALL implement the schema changes as a SeaORM migration file.
3. THE Sistema_Documentos SHALL create database indexes on `tipo_documento`, `estado_verificacion`, and `fecha_vencimiento` columns.
4. THE Sistema_Documentos SHALL set `estado_verificacion` to `pendiente` for all existing Documento records during migration.
5. WHEN a document is uploaded, THE Sistema_Documentos SHALL accept optional fields: `fecha_vencimiento`, `numero_documento`, and `notas_verificacion`.

### Requirement 3: Document Upload with Classification

**User Story:** As a property manager, I want to upload documents with their legal classification in a single step, so that documents are properly categorized from the moment they enter the system.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado uploads a document with a valid `tipo_documento`, THE Sistema_Documentos SHALL create a Documento record with the classification metadata and return the created Documento with HTTP status 201.
2. THE Sistema_Documentos SHALL continue to accept JPEG, PNG, and PDF file types with a maximum file size of 10 MB.
3. WHEN a document of Tipo_Documento `comprobante_fiscal_ncf` is uploaded, THE Sistema_Documentos SHALL require the `numero_documento` field containing the NCF number.
4. WHEN a document of Tipo_Documento `cedula` is uploaded for an Inquilino, THE Sistema_Documentos SHALL validate that the `numero_documento` field, if provided, matches the Inquilino's `cedula` field using the existing cédula validation logic.
5. IF a Usuario_Visualizador attempts to upload a document, THEN THE Sistema_Documentos SHALL return HTTP status 403.
6. WHEN a document is uploaded, THE Sistema_Documentos SHALL record an audit trail entry with the action, entity reference, and the uploading user's identity.

### Requirement 4: Document Verification Workflow

**User Story:** As a property manager, I want to mark documents as verified or rejected, so that I can track which documents have been reviewed and approved.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado submits a verification update for a Documento, THE Sistema_Documentos SHALL update the `estado_verificacion` to the specified value and return the updated Documento with HTTP status 200.
2. THE Sistema_Documentos SHALL allow transitions to the following Estado_Verificacion values: `verificado`, `rechazado`, `pendiente`.
3. WHEN a Documento is verified, THE Sistema_Documentos SHALL record the `verificado_por` user ID and `fecha_verificacion` timestamp.
4. WHEN a Documento is rejected, THE Sistema_Documentos SHALL require a `notas_verificacion` field explaining the reason for rejection.
5. WHEN a Documento is reset to `pendiente`, THE Sistema_Documentos SHALL clear the `verificado_por`, `fecha_verificacion`, and `notas_verificacion` fields.
6. WHEN a Documento verification status changes, THE Sistema_Documentos SHALL record an audit trail entry with the old status, new status, and the verifying user's identity.
7. IF a Usuario_Visualizador attempts to change verification status, THEN THE Sistema_Documentos SHALL return HTTP status 403.
8. IF the Documento with the requested ID does not exist, THEN THE Sistema_Documentos SHALL return HTTP status 404 with an error message in Spanish.

### Requirement 5: Document Expiration Tracking

**User Story:** As a property manager, I want to be alerted when documents are about to expire or have expired, so that I can request updated documents before they become invalid.

#### Acceptance Criteria

1. THE Sistema_Documentos SHALL automatically set `estado_verificacion` to `vencido` for any Documento where `fecha_vencimiento` is earlier than the current date and `estado_verificacion` is `verificado`.
2. WHEN an authenticated user requests documents expiring soon, THE Sistema_Documentos SHALL return a list of Documento records where `fecha_vencimiento` falls within the next 30 days and `estado_verificacion` is `verificado`.
3. THE Sistema_Documentos SHALL support a query parameter `dias` to override the default 30-day expiration window (minimum 1, maximum 365).
4. THE Sistema_Documentos SHALL return expiring documents sorted by `fecha_vencimiento` in ascending order (soonest expiring first).
5. THE Sistema_Documentos SHALL include the parent entity information (entity type, entity ID) in the expiration response so the user can identify which Propiedad, Inquilino, or Contrato is affected.

### Requirement 6: Entity Compliance Profile

**User Story:** As a property manager, I want to see a compliance summary for each entity showing which required documents are present and which are missing, so that I can ensure legal completeness.

#### Acceptance Criteria

1. WHEN an authenticated user requests the Perfil_Cumplimiento for an entity, THE Sistema_Documentos SHALL return a list of all required Tipo_Documento values for that entity type, each annotated with its status: `presente` (document exists and is verified), `pendiente` (document exists but is not yet verified), `vencido` (document exists but has expired), `rechazado` (document exists but was rejected), or `faltante` (no document of this type exists).
2. THE Sistema_Documentos SHALL define the following required document types per entity: for `inquilino`: `cedula`, `comprobante_ingresos`; for `propiedad`: `titulo_propiedad`; for `contrato`: `contrato_arrendamiento`.
3. THE Sistema_Documentos SHALL calculate a compliance percentage as the count of documents with status `presente` divided by the total count of required document types for that entity, expressed as an integer from 0 to 100.
4. THE Sistema_Documentos SHALL return the Perfil_Cumplimiento with HTTP status 200 including the entity reference, the list of document statuses, and the compliance percentage.
5. IF the entity type is not valid, THEN THE Sistema_Documentos SHALL return HTTP status 422 with a descriptive error message in Spanish.
6. IF the entity with the requested ID does not exist, THEN THE Sistema_Documentos SHALL return HTTP status 404 with an error message in Spanish.

### Requirement 7: Document Listing with Filters

**User Story:** As a property manager, I want to filter and search documents by type, status, and date range, so that I can quickly find specific legal documents.

#### Acceptance Criteria

1. WHEN an authenticated user requests the document list for an entity, THE Sistema_Documentos SHALL return a list of Documento records with all metadata fields including `tipo_documento`, `estado_verificacion`, `fecha_vencimiento`, and `numero_documento`.
2. THE Sistema_Documentos SHALL support filtering the document list by: `tipo_documento`, `estado_verificacion`, `fecha_vencimiento_desde`, and `fecha_vencimiento_hasta`.
3. WHEN a `tipo_documento` filter is provided, THE Sistema_Documentos SHALL return only Documento records matching that document type.
4. WHEN a `estado_verificacion` filter is provided, THE Sistema_Documentos SHALL return only Documento records matching that verification status.
5. WHEN `fecha_vencimiento_desde` and `fecha_vencimiento_hasta` filters are provided, THE Sistema_Documentos SHALL return only Documento records where `fecha_vencimiento` falls within the specified date range (inclusive).
6. THE Sistema_Documentos SHALL return documents sorted by `created_at` in descending order (newest first) by default.

### Requirement 8: NCF Validation for Fiscal Documents

**User Story:** As a property manager, I want NCF numbers on fiscal documents to be validated, so that I can ensure compliance with DGII requirements.

#### Acceptance Criteria

1. WHEN a document of Tipo_Documento `comprobante_fiscal_ncf` is uploaded or updated, THE Sistema_Documentos SHALL validate that the `numero_documento` field matches the NCF format: a single uppercase letter prefix followed by exactly 10 digits (e.g., `B0100000001`).
2. IF the `numero_documento` does not match the NCF format, THEN THE Sistema_Documentos SHALL return HTTP status 422 with a descriptive error message in Spanish indicating the expected format.
3. THE Sistema_Documentos SHALL enforce uniqueness of `numero_documento` for documents of Tipo_Documento `comprobante_fiscal_ncf` within the same organization to prevent duplicate NCF entries.
4. IF a duplicate NCF number is detected, THEN THE Sistema_Documentos SHALL return HTTP status 409 with a descriptive error message in Spanish.

### Requirement 9: Document Deletion

**User Story:** As a property manager, I want to delete documents that were uploaded by mistake, so that my document records stay accurate.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado requests deletion of a Documento, THE Sistema_Documentos SHALL delete the Documento record and the associated file from storage, and return HTTP status 204.
2. WHEN a Documento is deleted, THE Sistema_Documentos SHALL record an audit trail entry with the action, the deleted document's metadata, and the deleting user's identity.
3. IF the Documento with the requested ID does not exist, THEN THE Sistema_Documentos SHALL return HTTP status 404 with an error message in Spanish.
4. IF a Usuario_Visualizador attempts to delete a Documento, THEN THE Sistema_Documentos SHALL return HTTP status 403.

### Requirement 10: Compliance Dashboard Integration

**User Story:** As a property manager, I want to see a compliance overview on the dashboard, so that I can quickly identify entities with missing or expired documents.

#### Acceptance Criteria

1. WHEN the dashboard stats are requested, THE Sistema_Documentos SHALL include `documentos_vencidos` (count of documents with `estado_verificacion` equal to `vencido`) in the dashboard statistics response.
2. WHEN the dashboard stats are requested, THE Sistema_Documentos SHALL include `documentos_por_vencer` (count of documents expiring within the next 30 days) in the dashboard statistics response.
3. WHEN the dashboard stats are requested, THE Sistema_Documentos SHALL include `entidades_incompletas` (count of entities where the compliance percentage is below 100) in the dashboard statistics response.
4. THE Sistema_Documentos SHALL expose an endpoint returning a list of entities with the lowest compliance percentages, limited to the 10 entities with the lowest scores, sorted in ascending order by compliance percentage.

### Requirement 11: Document Templates

**User Story:** As a property manager, I want predefined templates for common legal documents, so that I can generate properly formatted documents with entity data pre-filled.

#### Acceptance Criteria

1. THE Sistema_Documentos SHALL provide built-in Plantilla_Documento definitions for the following document types: `contrato_arrendamiento` (lease agreement), `recibo_pago` (payment receipt), `acta_notarial` (notarial act), `carta_referencia` (reference letter), `addendum` (contract addendum).
2. EACH Plantilla_Documento SHALL define: a `nombre` (display name in Spanish), a `tipo_documento` (matching a valid Tipo_Documento), an `entity_type` (the entity this template applies to), and a `contenido` (structured JSON content with placeholder fields).
3. THE Sistema_Documentos SHALL define placeholder fields using the syntax `{{campo}}` within the template content, where `campo` maps to an entity field (e.g., `{{inquilino.nombre}}`, `{{contrato.monto_mensual}}`, `{{propiedad.direccion}}`).
4. WHEN a Usuario_Autorizado requests the list of available templates, THE Sistema_Documentos SHALL return all Plantilla_Documento definitions filtered by `entity_type` if specified, with HTTP status 200.
5. WHEN a Usuario_Autorizado requests a template pre-filled for a specific entity, THE Sistema_Documentos SHALL resolve all placeholder fields from the entity's current data and return the populated content with HTTP status 200.
6. IF a referenced entity does not exist, THEN THE Sistema_Documentos SHALL return HTTP status 404 with an error message in Spanish.

### Requirement 12: Document Editor

**User Story:** As a property manager, I want an in-browser document editor where I can create and edit legal documents, so that I can customize generated documents before saving or exporting them.

#### Acceptance Criteria

1. THE Editor_Documento SHALL render a rich-text editing interface in the browser that supports: headings, bold, italic, underline, numbered lists, bulleted lists, tables, and page breaks.
2. THE Editor_Documento SHALL allow the user to load a Plantilla_Documento pre-filled with entity data as the starting content for editing.
3. THE Editor_Documento SHALL allow the user to load OCR-extracted content (from Requirement 13) as the starting content for editing.
4. WHEN a Usuario_Autorizado saves a document from the Editor_Documento, THE Sistema_Documentos SHALL store the document content as structured JSON in a new `contenido_editable` column (JSONB, nullable) on the `documentos` table.
5. THE Editor_Documento SHALL provide an "Exportar PDF" button that generates a PDF from the current editor content and downloads it to the user's browser.
6. THE Editor_Documento SHALL provide a "Guardar como Documento" button that saves the editor content as a Documento record attached to the current entity, with the appropriate `tipo_documento` from the template.
7. THE Editor_Documento SHALL display placeholder fields that could not be resolved from entity data as highlighted editable fields, so the user can fill them manually.
8. IF a Usuario_Visualizador opens the Editor_Documento, THEN the editor SHALL render in read-only mode with export capability but no save or edit functionality.

### Requirement 13: OCR to Editable Document

**User Story:** As a property manager, I want to scan a physical document and get an editable digital version, so that I can correct OCR errors and maintain a clean digital copy.

#### Acceptance Criteria

1. WHEN a Usuario_Autorizado uploads a scanned image or PDF for digitization, THE Sistema_Documentos SHALL send the file to the existing OCR service and receive the extracted text lines with confidence scores.
2. THE Sistema_Documentos SHALL convert the OCR result into a Documento_Editable by mapping extracted lines into structured JSON content compatible with the Editor_Documento format.
3. THE Sistema_Documentos SHALL preserve the original line order and paragraph structure from the OCR extraction in the generated Documento_Editable.
4. THE Sistema_Documentos SHALL flag low-confidence text segments (confidence below 0.80) with a visual highlight in the Editor_Documento so the user can review and correct them.
5. WHEN the OCR service detects a known document type (e.g., `cedula`, `contrato`, `deposito_bancario`, `recibo_gasto`), THE Sistema_Documentos SHALL attempt to map the extracted structured fields to the corresponding Plantilla_Documento, pre-filling matching fields.
6. WHEN the OCR service returns `document_type` as `unknown`, THE Sistema_Documentos SHALL still generate a Documento_Editable with the raw extracted text as free-form content.
7. THE Sistema_Documentos SHALL store the original scanned file as a Documento attachment alongside the generated Documento_Editable, linking them via the same `entity_type` and `entity_id`.
8. THE Sistema_Documentos SHALL return the generated Documento_Editable content with HTTP status 200, ready to be loaded into the Editor_Documento.

### Requirement 14: Document Editor Storage

**User Story:** As a property manager, I want my edited documents to be saved and versioned, so that I can retrieve previous versions if needed.

#### Acceptance Criteria

1. THE Sistema_Documentos SHALL extend the `documentos` table with a `contenido_editable` column (JSONB, nullable) to store the structured editor content.
2. THE Sistema_Documentos SHALL implement the schema change as a SeaORM migration file.
3. WHEN a Documento_Editable is saved, THE Sistema_Documentos SHALL store the current content in `contenido_editable` and update the `updated_at` timestamp.
4. THE Sistema_Documentos SHALL add an `updated_at` column (TIMESTAMP WITH TIME ZONE, nullable) to the `documentos` table to track the last edit time.
5. WHEN an authenticated user requests a Documento that has `contenido_editable` set, THE Sistema_Documentos SHALL include the editable content in the response so it can be loaded into the Editor_Documento.
6. WHEN a Documento_Editable is saved, THE Sistema_Documentos SHALL record an audit trail entry with the action and the editing user's identity.
