# File Upload Endpoint — Implementation Without Security Skill

## Task

Create `POST /api/v1/documentos/upload` that accepts multipart file upload with `entity_type` and `entity_id` as form fields, stores the file on disk under `./uploads/{uuid}.{ext}`, creates a Documento record, and only allows PDF, JPG, PNG, DOCX up to 10 MB.

## Reasoning & Decisions

### 1. Handler Design

I followed the existing multipart handling pattern from `handlers/documentos.rs` (the existing `upload` handler) and `handlers/importacion.rs`. The key difference is that `entity_type` and `entity_id` come as **form fields** rather than path parameters.

- Used `actix_multipart::Multipart` with `futures_util::StreamExt` to iterate fields
- File field is identified by `field_name == "file"`
- Text fields are read as UTF-8 strings
- Added early rejection during chunk reading if file exceeds 10 MB (avoids buffering the entire oversized file)

### 2. Service Layer

Separated validation logic into small focused functions following the existing `documentos.rs` service pattern:

- **File size**: Rejects > 10 MB
- **MIME type**: Allowlist of `application/pdf`, `image/jpeg`, `image/png`, and the DOCX MIME type
- **Extension**: Double-checks the filename extension matches allowed types (defense in depth — don't trust MIME alone)
- **Entity type**: Allowlist validation
- **Filename sanitization**: Strips path separators and `..` sequences

### 3. File Storage

The task specifies `./uploads/{uuid}.{ext}` format:
- Generate a new UUID v4 for each upload
- Extract the validated extension from the original filename
- Store at `{UPLOAD_DIR}/{uuid}.{ext}` (UPLOAD_DIR defaults to `./uploads`)
- Added canonical path verification to prevent path traversal attacks

### 4. Database Record

Created a `documento::ActiveModel` with all required fields. Set `tipo_documento` to `"general"` since the task doesn't mention a specific document type field in the form. The existing upload endpoint requires `tipo_documento` as a form field, but this new endpoint is simpler — it's a generic upload.

### 5. Route Registration

Placed the `/upload` route inside the existing `/documentos` scope, among the static paths (before the dynamic `/{entity_type}/{entity_id}` catch-all) to avoid path conflicts.

### 6. Security Considerations Applied (from general knowledge)

- **File size limit**: 10 MB enforced both during streaming and in the service layer
- **MIME type allowlist**: Only 4 specific types accepted
- **Extension validation**: Defense-in-depth alongside MIME check
- **Filename sanitization**: Strips `../`, `/`, `\` to prevent path traversal
- **Canonical path check**: After writing, verifies the file is within the upload directory
- **WriteAccess extractor**: Ensures only admin/gerente roles can upload
- **UUID-based storage names**: Prevents filename collisions and information leakage

### 7. What's NOT included (intentionally)

- No virus/malware scanning (would require external service)
- No content-type sniffing/magic byte verification (could be added but wasn't requested)
- No rate limiting beyond what the existing `write_governor_conf` provides on the scope
- No async file I/O via `spawn_blocking` (the existing codebase uses sync `std::fs` for file writes in the documentos service, so I matched that pattern)
