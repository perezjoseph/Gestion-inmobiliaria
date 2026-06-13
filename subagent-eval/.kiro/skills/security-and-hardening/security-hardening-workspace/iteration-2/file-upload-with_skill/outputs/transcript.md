# File Upload Endpoint — Security Reasoning Transcript

## Task

Create `POST /api/v1/documentos/upload` accepting multipart file upload with `entity_type` and `entity_id` as form fields. Store under `./uploads/{uuid}.{ext}`, create a `Documento` DB record. Allow only PDF, JPG, PNG, DOCX up to 10 MB.

## Security Considerations Applied (from SKILL.md)

### 1. Authentication & Authorization

- **Extractor used**: `WriteAccess` — this is a write endpoint (POST creating data). Per the skill's decision quick-reference, write endpoints require `WriteAccess`, which enforces `admin` or `gerente` role before the handler runs.
- The user's `sub` (UUID) from claims is used as `uploaded_by` — never taken from the request body.

### 2. File Upload Safety (SKILL.md § File Upload Safety)

| Check | Implementation |
|-------|---------------|
| Validate extension against allowlist | `ALLOWED_EXTENSIONS: &["pdf", "jpg", "jpeg", "png", "docx"]` — explicit allowlist |
| Validate file size (reject before reading full body) | Early rejection in handler: checks `data.len() > MAX_FILE_SIZE` during chunk accumulation, not after |
| Validate magic bytes | Custom `validate_magic_bytes()` checks PDF header (`%PDF`), JPEG (`FF D8 FF`), PNG (8-byte signature), DOCX/ZIP (`PK\x03\x04`) |
| Prevent path traversal | `sanitize_filename()` strips `/`, `\`, `..`; stored with UUID-based name; canonical path verification post-write |
| Store with UUID-based names | File stored as `{uuid}.{ext}` — user-provided filename never used in path |
| Serve through authenticated handler | No static file serving added; existing pattern requires auth |

### 3. Input Validation (SKILL.md § Input Validation)

- `entity_type`: validated against explicit allowlist of 6 valid types
- `entity_id`: parsed as UUID via `Uuid::parse_str()` — rejects anything that isn't a valid UUID
- `filename`: sanitized, length-capped at 255 chars
- Unknown multipart fields: consumed and discarded (no error, no processing)
- Empty file: explicitly rejected

### 4. Error Response Safety (SKILL.md § Error Response Safety)

- Internal errors (filesystem, DB) use `AppError::Internal(anyhow::anyhow!(...))` which returns generic "Error interno del servidor" to client
- Validation errors give user-actionable messages without leaking internals
- No stack traces, file paths, or DB schema info in responses

### 5. Rate Limiting (SKILL.md § When to add rate limiting?)

- The `/documentos` scope already has `write_governor_conf` applied (2s/req, burst 20)
- Per the skill: "Write-heavy or expensive operations (uploads, imports, exports) → moderate" — this matches

### 6. Audit Logging (SKILL.md § When to log security events?)

- `tracing::info!` on successful upload with structured fields (documento_id, entity_type, entity_id, uploaded_by, file_size)
- Audit trail record created via `auditoria::registrar_best_effort` — consistent with existing pattern

### 7. Defense-in-Depth: Framework-Level Size Limit

- Recommended adding `MultipartFormConfig` with `total_limit(10 MB)` in app.rs
- This rejects oversized payloads at the framework level before handler code runs, reducing DoS surface

### 8. MIME Type Determination

- MIME type is derived from the validated extension, NOT from the client-provided `Content-Type` header
- Client-provided content type is ignored for security — attackers can set any content type
- Magic bytes verify the actual file content matches the extension

## Design Decisions

1. **Form fields vs path params**: The task specifies `entity_type` and `entity_id` as form fields (not URL path segments). This differs from the existing `/{entity_type}/{entity_id}` pattern but matches the task requirements exactly.

2. **Storage path**: `./uploads/{uuid}.{ext}` as specified. Simpler than the existing pattern (`./uploads/{entity_type}/{entity_id}/{uuid}-{filename}`) but matches the task requirements.

3. **No `infer` crate**: The skill mentions the `infer` crate for magic byte validation, but the project doesn't have it as a dependency. Rather than adding a new dependency, I implemented inline magic byte checks for the 4 specific file types. This is simpler and avoids supply chain risk for a small, well-defined set of formats.

4. **`tipo_documento` default**: Set to `"adjunto"` since the endpoint doesn't accept a `tipo_documento` field (task only specifies file + entity_type + entity_id). The existing upload endpoint requires this field, but our simplified endpoint uses a sensible default.

## Checklist Verification (SKILL.md § Security Review Checklist)

- [x] Handler uses `WriteAccess` extractor
- [x] Input validated at handler boundary (size, extension, magic bytes, entity_type allowlist)
- [x] No secrets in code or logs
- [x] Error responses don't expose internals
- [x] File uploads validated (size, extension, path traversal, magic bytes)
- [x] Rate limiting applied (via scope-level governor)
- [x] UUID-based storage names (no user-controlled paths)
- [x] Audit trail on upload
