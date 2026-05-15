use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

use crate::entities::documento;
use crate::errors::AppError;
use crate::models::documento::DocumentoResponse;
use crate::services::auditoria;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Allowed file extensions (lowercase).
const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "jpg", "jpeg", "png", "docx"];

/// Allowed MIME types mapped from extensions.
const ALLOWED_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "image/jpeg",
    "image/png",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
];

/// Allowed entity types that can have documents attached.
const ALLOWED_ENTITY_TYPES: &[&str] = &[
    "propiedad",
    "inquilino",
    "contrato",
    "pago",
    "gasto",
    "mantenimiento",
];

/// Magic byte signatures for allowed file types.
/// Used to verify file content matches the claimed extension.
const PDF_MAGIC: &[u8] = b"%PDF";
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
// DOCX is a ZIP file — starts with PK\x03\x04
const ZIP_MAGIC: &[u8] = &[0x50, 0x4B, 0x03, 0x04];

/// Upload a file, validate it, store on disk, and create a Documento record.
#[allow(clippy::cast_possible_wrap)]
pub async fn upload(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
    file_data: &[u8],
    filename: &str,
    uploaded_by: Uuid,
) -> Result<DocumentoResponse, AppError> {
    // ── 1. Validate file size ──────────────────────────────────
    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el tamaño máximo de 10 MB".to_string(),
        ));
    }
    if file_data.is_empty() {
        return Err(AppError::Validation(
            "El archivo está vacío".to_string(),
        ));
    }

    // ── 2. Validate entity_type ────────────────────────────────
    let entity_type_trimmed = entity_type.trim();
    if !ALLOWED_ENTITY_TYPES.contains(&entity_type_trimmed) {
        return Err(AppError::Validation(format!(
            "Tipo de entidad no válido: {entity_type_trimmed}. \
             Tipos permitidos: {}",
            ALLOWED_ENTITY_TYPES.join(", ")
        )));
    }

    // ── 3. Sanitize filename and extract extension ─────────────
    let safe_filename = sanitize_filename(filename)?;
    let extension = extract_extension(&safe_filename)?;

    // ── 4. Validate extension against allowlist ────────────────
    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::Validation(format!(
            "Extensión de archivo no permitida: .{extension}. \
             Extensiones permitidas: PDF, JPG, PNG, DOCX"
        )));
    }

    // ── 5. Validate magic bytes match the extension ────────────
    validate_magic_bytes(file_data, &extension)?;

    // ── 6. Determine MIME type from extension ──────────────────
    let mime_type = mime_from_extension(&extension);

    // ── 7. Store file on disk with UUID-based name ─────────────
    let file_uuid = Uuid::new_v4();
    let stored_filename = format!("{file_uuid}.{extension}");

    let upload_dir = get_upload_dir();
    let full_path = format!("{upload_dir}/{stored_filename}");

    // Create upload directory if it doesn't exist
    std::fs::create_dir_all(&upload_dir)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando directorio: {e}")))?;

    // Write file to disk
    std::fs::write(&full_path, file_data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo archivo: {e}")))?;

    // Verify the resolved path stays within the upload directory (path traversal defense-in-depth)
    let canonical_dir = std::fs::canonicalize(&upload_dir)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo directorio: {e}")))?;
    let canonical_file = std::fs::canonicalize(&full_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo ruta: {e}")))?;
    if !canonical_file.starts_with(&canonical_dir) {
        let _ = std::fs::remove_file(&full_path);
        return Err(AppError::Validation(
            "Ruta de archivo fuera del directorio permitido".to_string(),
        ));
    }

    // ── 8. Create Documento record in database ─────────────────
    let id = Uuid::new_v4();
    let now = Utc::now().into();
    let file_size = file_data.len() as i64;

    let model = documento::ActiveModel {
        id: Set(id),
        entity_type: Set(entity_type_trimmed.to_string()),
        entity_id: Set(entity_id),
        filename: Set(safe_filename),
        file_path: Set(stored_filename.clone()),
        mime_type: Set(mime_type.to_string()),
        file_size: Set(file_size),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
        tipo_documento: Set("adjunto".to_string()),
        estado_verificacion: Set("pendiente".to_string()),
        fecha_vencimiento: Set(None),
        verificado_por: Set(None),
        fecha_verificacion: Set(None),
        notas_verificacion: Set(None),
        numero_documento: Set(None),
        contenido_editable: Set(None),
        updated_at: Set(None),
    };

    let inserted = model.insert(db).await?;

    // ── 9. Audit trail (best-effort) ───────────────────────────
    tracing::info!(
        documento_id = %inserted.id,
        entity_type = %inserted.entity_type,
        entity_id = %inserted.entity_id,
        uploaded_by = %uploaded_by,
        file_size = file_size,
        "Documento subido exitosamente"
    );

    auditoria::registrar_best_effort(
        db,
        auditoria::CreateAuditoriaEntry {
            usuario_id: uploaded_by,
            entity_type: "documento".to_string(),
            entity_id: inserted.id,
            accion: "subir_documento".to_string(),
            cambios: serde_json::json!({
                "entity_type": inserted.entity_type,
                "entity_id": inserted.entity_id.to_string(),
                "filename": inserted.filename,
                "mime_type": inserted.mime_type,
                "file_size": file_size,
            }),
        },
    )
    .await;

    Ok(model_to_response(inserted))
}

// ── Private helpers ────────────────────────────────────────────

/// Sanitize a filename: strip path separators and directory traversal sequences.
fn sanitize_filename(name: &str) -> Result<String, AppError> {
    let sanitized: String = name.replace(['/', '\\'], "_").replace("..", "_");
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Nombre de archivo inválido".to_string(),
        ));
    }
    if trimmed.len() > 255 {
        return Err(AppError::Validation(
            "Nombre de archivo excede 255 caracteres".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Extract the file extension (lowercase) from a sanitized filename.
fn extract_extension(filename: &str) -> Result<String, AppError> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .ok_or_else(|| {
            AppError::Validation("El archivo debe tener una extensión válida".to_string())
        })?;
    Ok(ext)
}

/// Validate that the file's magic bytes match the claimed extension.
/// This prevents uploading a malicious file with a renamed extension.
fn validate_magic_bytes(data: &[u8], extension: &str) -> Result<(), AppError> {
    let valid = match extension {
        "pdf" => data.starts_with(PDF_MAGIC),
        "jpg" | "jpeg" => data.starts_with(JPEG_MAGIC),
        "png" => data.starts_with(PNG_MAGIC),
        "docx" => data.starts_with(ZIP_MAGIC),
        _ => false,
    };

    if !valid {
        return Err(AppError::Validation(
            "El contenido del archivo no coincide con la extensión declarada".to_string(),
        ));
    }
    Ok(())
}

/// Map a validated extension to its MIME type.
fn mime_from_extension(extension: &str) -> &'static str {
    match extension {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    }
}

fn get_upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

/// Convert a `documento::Model` into the public `DocumentoResponse` DTO.
fn model_to_response(d: documento::Model) -> DocumentoResponse {
    DocumentoResponse {
        id: d.id,
        entity_type: d.entity_type,
        entity_id: d.entity_id,
        filename: d.filename,
        file_path: d.file_path,
        mime_type: d.mime_type,
        file_size: d.file_size,
        uploaded_by: d.uploaded_by,
        created_at: d.created_at.into(),
        tipo_documento: d.tipo_documento,
        estado_verificacion: d.estado_verificacion,
        fecha_vencimiento: d.fecha_vencimiento,
        verificado_por: d.verificado_por,
        fecha_verificacion: d.fecha_verificacion.map(Into::into),
        notas_verificacion: d.notas_verificacion,
        numero_documento: d.numero_documento,
        contenido_editable: d.contenido_editable,
        updated_at: d.updated_at.map(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_strips_traversal() {
        let result = sanitize_filename("../../etc/passwd").unwrap();
        assert!(!result.contains('/'));
        assert!(!result.contains(".."));
    }

    #[test]
    fn sanitize_filename_rejects_empty() {
        assert!(sanitize_filename("").is_err());
    }

    #[test]
    fn sanitize_filename_rejects_too_long() {
        let long_name = "a".repeat(256);
        assert!(sanitize_filename(&long_name).is_err());
    }

    #[test]
    fn extract_extension_works() {
        assert_eq!(extract_extension("photo.JPG").unwrap(), "jpg");
        assert_eq!(extract_extension("doc.PDF").unwrap(), "pdf");
        assert_eq!(extract_extension("file.docx").unwrap(), "docx");
    }

    #[test]
    fn extract_extension_rejects_no_extension() {
        assert!(extract_extension("noextension").is_err());
    }

    #[test]
    fn validate_magic_bytes_accepts_valid_pdf() {
        let data = b"%PDF-1.4 rest of file";
        assert!(validate_magic_bytes(data, "pdf").is_ok());
    }

    #[test]
    fn validate_magic_bytes_rejects_mismatched() {
        let data = b"%PDF-1.4 rest of file";
        assert!(validate_magic_bytes(data, "png").is_err());
    }

    #[test]
    fn validate_magic_bytes_accepts_valid_jpeg() {
        let data = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00];
        assert!(validate_magic_bytes(data, "jpg").is_ok());
    }

    #[test]
    fn validate_magic_bytes_accepts_valid_png() {
        let data = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        assert!(validate_magic_bytes(data, "png").is_ok());
    }

    #[test]
    fn validate_magic_bytes_accepts_valid_docx() {
        let data = &[0x50, 0x4B, 0x03, 0x04, 0x14, 0x00];
        assert!(validate_magic_bytes(data, "docx").is_ok());
    }

    #[test]
    fn mime_from_extension_maps_correctly() {
        assert_eq!(mime_from_extension("pdf"), "application/pdf");
        assert_eq!(mime_from_extension("jpg"), "image/jpeg");
        assert_eq!(mime_from_extension("jpeg"), "image/jpeg");
        assert_eq!(mime_from_extension("png"), "image/png");
        assert_eq!(
            mime_from_extension("docx"),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
    }

    #[test]
    fn allowed_extensions_includes_required_types() {
        assert!(ALLOWED_EXTENSIONS.contains(&"pdf"));
        assert!(ALLOWED_EXTENSIONS.contains(&"jpg"));
        assert!(ALLOWED_EXTENSIONS.contains(&"png"));
        assert!(ALLOWED_EXTENSIONS.contains(&"docx"));
    }
}
