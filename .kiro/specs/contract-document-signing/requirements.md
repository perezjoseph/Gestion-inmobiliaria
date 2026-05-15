# Requirements Document

## Introduction

Legal document editor enhancement for the Dominican Republic property management platform. This feature adds DOCX export capability, full template CRUD management, and a digital signature workflow compliant with DR Ley 126-02. Property managers can create/manage document templates, export documents as DOCX files, and collect legally-binding digital signatures from both property managers (authenticated) and tenants (via presigned public links with password protection).

## Glossary

- **Document_Editor_Service**: The backend service (`documento_editor`) responsible for document content manipulation, export, and rendering.
- **Template_Service**: The backend service (`plantillas`) responsible for template CRUD operations and placeholder resolution.
- **Signature_Service**: The backend service responsible for digital signature creation, verification, token management, and document sealing.
- **DOCX_Exporter**: The backend component that converts block-based JSON content to Microsoft Word (.docx) format using the `docx-rs` crate.
- **Block_JSON**: The structured JSON format used by the document editor, containing a `version` field and a `blocks` array with typed elements (heading, paragraph, list, table, page_break).
- **Placeholder**: A template variable in `{{entity.field}}` syntax that is resolved against entity data during template filling.
- **Firma_Documento**: The database entity storing signature metadata including signer identity, signature image, IP address, user agent, timestamp, token, and expiration.
- **Presigned_Link**: A temporary public URL containing a unique token that allows an unauthenticated tenant to review and sign a document after password verification.
- **Document_Sealing**: The process of making a document readonly and generating a finalized PDF with embedded signature images once all required parties have signed.
- **WriteAccess**: The Actix-web extractor that enforces admin or gerente role for write operations.
- **Claims**: The JWT claims extracted from the authenticated user's token, containing sub (user_id), organizacion_id, and rol.

## Requirements

### Requirement 1: DOCX Export

**User Story:** As a property manager, I want to export editable documents as DOCX files, so that I can share them with parties who use Microsoft Word.

#### Acceptance Criteria

1.1 WHEN a GET request is received at `/documentos/{id}/exportar-docx`, THE DOCX_Exporter SHALL convert the document's `contenido_editable` Block_JSON into a valid `.docx` file and return it with content type `application/vnd.openxmlformats-officedocument.wordprocessingml.document`.

1.2 THE DOCX_Exporter SHALL render heading blocks with font sizes matching the PDF export (18pt for level 1, 15pt for level 2, 13pt for level 3).

1.3 THE DOCX_Exporter SHALL render paragraph blocks using Arial 11pt font with the same margins as the PDF export (15mm).

1.4 THE DOCX_Exporter SHALL render list blocks as numbered lists (ordered) or bulleted lists (unordered) preserving all item text.

1.5 THE DOCX_Exporter SHALL render table blocks with headers in bold and cell borders matching the PDF table layout.

1.6 THE DOCX_Exporter SHALL render page_break blocks as explicit page breaks in the DOCX output.

1.7 IF the document has no `contenido_editable` field, THEN THE DOCX_Exporter SHALL return HTTP 400 with error message "El documento no tiene contenido editable para exportar".

1.8 IF the document does not exist, THEN THE DOCX_Exporter SHALL return HTTP 404 with error message "Documento {id} no encontrado".

1.9 THE DOCX_Exporter SHALL set the Content-Disposition header to `attachment; filename="documento-{id}.docx"`.

1.10 THE DOCX_Exporter SHALL require a valid JWT (Claims extractor) to access the endpoint.

### Requirement 2: Template CRUD

**User Story:** As an admin or property manager, I want to create, update, and delete document templates, so that I can maintain a library of reusable legal document formats.

#### Acceptance Criteria

2.1 WHEN a POST request is received at `/documentos/plantillas` with a valid body containing nombre, tipo_documento, entity_type, and contenido, THE Template_Service SHALL create a new plantilla_documento record and return HTTP 201 with the created template.

2.2 WHEN a PUT request is received at `/documentos/plantillas/{id}` with updated fields, THE Template_Service SHALL update the matching plantilla_documento record and return HTTP 200 with the updated template.

2.3 WHEN a DELETE request is received at `/documentos/plantillas/{id}`, THE Template_Service SHALL set the template's `activo` field to false (soft-delete) and return HTTP 204.

2.4 WHEN a GET request is received at `/documentos/plantillas/{id}`, THE Template_Service SHALL return the single template matching the given id with HTTP 200.

2.5 IF the template id does not exist for GET, PUT, or DELETE operations, THEN THE Template_Service SHALL return HTTP 404 with error message "Plantilla {id} no encontrada".

2.6 THE Template_Service SHALL require WriteAccess (admin or gerente role) for POST, PUT, and DELETE operations.

2.7 THE Template_Service SHALL validate that `nombre` is non-empty and `tipo_documento` is non-empty before creating or updating a template.

2.8 THE Template_Service SHALL support `{{placeholder}}` syntax in template contenido blocks, where placeholders follow the pattern `entity.field` (e.g., `{{inquilino.nombre}}`, `{{contrato.fecha_inicio}}`).

2.9 WHEN a template is created or updated, THE Template_Service SHALL set the `updated_at` timestamp to the current UTC time.

2.10 WHEN a GET request is received at `/documentos/plantillas` without an id, THE Template_Service SHALL return a list of all active templates (where `activo` is true) with HTTP 200.

2.11 THE Template_Service SHALL support optional query parameters `tipo_documento` and `entity_type` to filter the template list.

### Requirement 3: Template Management Frontend

**User Story:** As an admin or property manager, I want a template management interface, so that I can visually create and edit document templates with the block editor.

#### Acceptance Criteria

3.1 THE Frontend SHALL provide a template management page accessible from the main navigation for users with admin or gerente role.

3.2 THE Frontend SHALL display a list of active templates showing nombre, tipo_documento, and entity_type for each template.

3.3 WHEN the user clicks "Crear Plantilla", THE Frontend SHALL display a form with fields for nombre, tipo_documento (dropdown), entity_type (dropdown), and the block editor for contenido.

3.4 WHEN the user submits the template creation form, THE Frontend SHALL send a POST request to `/documentos/plantillas` and display a success toast on completion.

3.5 WHEN the user clicks edit on a template, THE Frontend SHALL load the template data into the form and block editor for modification.

3.6 WHEN the user clicks delete on a template, THE Frontend SHALL display a confirmation dialog and send a DELETE request upon confirmation.

3.7 IF an API error occurs during template operations, THEN THE Frontend SHALL display the error message in an error banner.

### Requirement 4: Digital Signature - Manager Signing

**User Story:** As a property manager, I want to sign documents while authenticated, so that I can add my legally-binding signature without leaving the application.

#### Acceptance Criteria

4.1 WHEN an authenticated user with admin or gerente role submits a signature via POST `/documentos/{id}/firmar`, THE Signature_Service SHALL create a firma_documento record with firmante_tipo set to "propietario" (for both admin and gerente roles), the provided firma_imagen, the request IP address, user agent, and current timestamp.

4.2 THE Signature_Service SHALL store the firma_imagen as bytea data in the firma_documento table.

4.3 THE Signature_Service SHALL capture and store the signer's IP address from the request headers (X-Forwarded-For or peer address).

4.4 THE Signature_Service SHALL capture and store the signer's User-Agent header value.

4.5 THE Signature_Service SHALL set the firma_documento estado to "firmado" and firmado_at to the current UTC timestamp upon successful signing.

4.6 THE Signature_Service SHALL require WriteAccess for the authenticated signing endpoint.

### Requirement 5: Digital Signature - Tenant Presigned Link

**User Story:** As a property manager, I want to send a secure signing link to tenants, so that they can review and sign documents without needing an account.

#### Acceptance Criteria

5.1 WHEN a POST request is received at `/documentos/{id}/solicitar-firma` with firmante_nombre and email, THE Signature_Service SHALL generate a unique token, a random password (minimum 16 characters), create a firma_documento record with estado "pendiente" and expira_at set to 72 hours from creation. IF email delivery fails, THE Signature_Service SHALL still persist the firma_documento record and return the response with `email_enviado` set to false.

5.2 THE Signature_Service SHALL hash the generated password using argon2 before storing it in the firma_documento password_hash field.

5.3 THE Signature_Service SHALL send an email to the provided address containing the presigned link URL and the plaintext password.

5.4 THE Signature_Service SHALL require WriteAccess for the solicitar-firma endpoint.

5.5 WHEN a POST request is received at `/firmas/{token}/verificar` with a valid password in the JSON body, THE Signature_Service SHALL verify the password against the stored hash and return the document content for review.

5.6 IF the token does not exist, THEN THE Signature_Service SHALL return HTTP 404.

5.7 IF the token has expired (current time > expira_at), THEN THE Signature_Service SHALL return HTTP 410 with error message "El enlace de firma ha expirado".

5.8 IF the password verification fails, THEN THE Signature_Service SHALL return HTTP 401 with error message "Contraseña incorrecta".

5.9 WHEN a POST request is received at `/firmas/{token}/firmar` with firma_imagen and valid password, THE Signature_Service SHALL verify the password, store the signature image, capture IP and user agent metadata, set estado to "firmado", and set firmado_at to current UTC timestamp.

5.10 IF the firma_documento estado is not "pendiente" when attempting to sign, THEN THE Signature_Service SHALL return HTTP 409 with error message "Esta firma ya fue procesada".

### Requirement 6: Document Sealing

**User Story:** As a property manager, I want documents to be automatically sealed once all parties sign, so that the final version is tamper-proof and legally valid.

#### Acceptance Criteria

6.1 WHEN a signature is recorded and all required parties have signed the same document, THE Signature_Service SHALL seal the document by setting contenido_editable to readonly status. A document requires signatures from both a propietario (manager/admin) and an inquilino (tenant) — at least one firma_documento with firmante_tipo="propietario" and estado="firmado" AND at least one with firmante_tipo="inquilino" and estado="firmado" must exist for the same documento_id.

6.2 WHEN a document is sealed, THE Signature_Service SHALL generate a finalized PDF containing the document content with embedded signature images positioned at the end of the document.

6.3 THE Signature_Service SHALL store the finalized PDF as a new documento record linked to the original document via a `documento_origen_id` field referencing the sealed document's id.

6.4 WHILE a document is sealed, THE Document_Editor_Service SHALL reject any PUT requests to `/{id}/contenido` with HTTP 403 and error message "El documento está sellado y no puede ser modificado".

6.5 WHILE a document is sealed, THE system SHALL reject any DELETE requests to the document with HTTP 403 and error message "El documento está sellado y no puede ser eliminado".

6.6 THE Signature_Service SHALL record the sealing timestamp in the document metadata.

6.7 THE Signature_Service SHALL allow both parties to sign independently in any order without requiring a specific signing sequence.

### Requirement 7: Signature Database Schema

**User Story:** As a developer, I want a well-structured firma_documento table, so that signature data is stored securely and supports the signing workflow.

#### Acceptance Criteria

7.1 THE firma_documento table SHALL contain columns: id (UUID PK), documento_id (UUID FK to documento), firmante_tipo (VARCHAR), firmante_nombre (VARCHAR), firma_imagen (BYTEA), ip_address (VARCHAR), user_agent (TEXT), firmado_at (TIMESTAMPTZ nullable), token (VARCHAR UNIQUE nullable), password_hash (VARCHAR nullable), expira_at (TIMESTAMPTZ nullable), estado (VARCHAR), created_at (TIMESTAMPTZ).

7.2 THE firma_documento table SHALL have a foreign key constraint on documento_id referencing the documento table.

7.3 THE firma_documento table SHALL have a unique index on the token column.

7.4 THE firma_documento table SHALL have an index on documento_id for efficient lookups of all signatures for a document.

7.5 THE firma_documento estado column SHALL accept values: "pendiente", "firmado", "expirado", "cancelado".

7.6 THE firma_documento firmante_tipo column SHALL accept values: "propietario", "inquilino".

7.7 THE documentos table SHALL be extended with: `sellado` (BOOLEAN NOT NULL DEFAULT FALSE), `sellado_at` (TIMESTAMPTZ nullable), and `documento_origen_id` (UUID nullable FK to documentos) to link sealed PDF records back to their source document.

### Requirement 8: Ley 126-02 Compliance

**User Story:** As a property manager operating in the Dominican Republic, I want the digital signature process to comply with Ley 126-02, so that signed documents are legally valid.

#### Acceptance Criteria

8.1 THE Signature_Service SHALL capture and store a timestamp (firmado_at) for each signature event using UTC timezone.

8.2 THE Signature_Service SHALL capture and store the signer's IP address for each signature event.

8.3 THE Signature_Service SHALL capture and store the signer's user agent for each signature event.

8.4 THE Signature_Service SHALL ensure signature tokens are cryptographically random with minimum 128 bits of entropy (e.g., UUID v4 provides 122 bits which is acceptable, or a 32-byte CSPRNG value encoded as hex/base64).

8.5 THE Signature_Service SHALL ensure signing links expire after exactly 72 hours from creation.

8.6 THE Signature_Service SHALL protect tenant signing links with password verification using argon2 hashing.

8.7 WHILE a document is sealed, THE system SHALL preserve the original signed content immutably (no edits allowed to contenido_editable).

8.8 THE Signature_Service SHALL enforce rate limiting on public signing endpoints (`/firmas/{token}/verificar` and `/firmas/{token}/firmar`) to prevent brute-force password attacks, allowing a maximum of 5 attempts per 30 seconds per IP address.

### Requirement 9: Frontend DOCX Export Button

**User Story:** As a property manager, I want a DOCX export button in the document editor, so that I can download documents in Word format alongside the existing PDF export.

#### Acceptance Criteria

9.1 THE Frontend document editor toolbar SHALL display an "Exportar DOCX" button next to the existing "Exportar PDF" button.

9.2 WHEN the user clicks "Exportar DOCX", THE Frontend SHALL initiate a download from `/documentos/{id}/exportar-docx`.

9.3 WHILE the DOCX export is in progress, THE Frontend SHALL display the button text as "Exportando..." and disable the button.

9.4 IF the document has not been saved (no documento_id), THEN THE Frontend SHALL disable the "Exportar DOCX" button.

### Requirement 10: Frontend Signature Interface

**User Story:** As a property manager, I want a signature interface in the document view, so that I can sign documents and request tenant signatures.

#### Acceptance Criteria

10.1 THE Frontend SHALL display a signature panel below the document editor showing the current signature status for each party.

10.2 WHEN the user clicks "Firmar Documento", THE Frontend SHALL display a signature canvas where the user can draw their signature.

10.3 WHEN the user submits their drawn signature, THE Frontend SHALL send the signature image data to POST `/documentos/{id}/firmar`.

10.4 WHEN the user clicks "Solicitar Firma del Inquilino", THE Frontend SHALL display a form requesting the tenant's name and email address.

10.5 WHEN the user submits the signature request form, THE Frontend SHALL send a POST request to `/documentos/{id}/solicitar-firma` and display a success toast with confirmation.

10.6 THE Frontend SHALL display a "Sellado" badge on documents that have been sealed after all signatures are collected.

10.7 WHILE a document is sealed, THE Frontend SHALL set the document editor to readonly mode.

### Requirement 11: Public Signing Page

**User Story:** As a tenant, I want a public signing page accessible via the presigned link, so that I can review and sign documents without creating an account.

#### Acceptance Criteria

11.1 THE Frontend SHALL provide a public route at `/firmas/{token}` that does not require authentication.

11.2 WHEN the tenant navigates to the signing page, THE Frontend SHALL display a password input form.

11.3 WHEN the tenant submits the correct password, THE Frontend SHALL display the document content in readonly mode with a signature canvas.

11.4 WHEN the tenant draws and submits their signature, THE Frontend SHALL send the signature data to POST `/firmas/{token}/firmar` with the password.

11.5 IF the link has expired, THEN THE Frontend SHALL display a message "Este enlace de firma ha expirado. Contacte al administrador de la propiedad."

11.6 IF the password is incorrect, THEN THE Frontend SHALL display an error message "Contraseña incorrecta" and allow retry.

11.7 WHEN the signature is submitted successfully, THE Frontend SHALL display a confirmation message "Documento firmado exitosamente".
