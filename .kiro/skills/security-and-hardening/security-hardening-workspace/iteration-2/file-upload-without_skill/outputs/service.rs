use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::documento::DocumentoResponse;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "jpg", "jpeg", "png", "docx"];

const ALLOWED_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "image/jpeg",
    "image/png",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
];

const ALLOWED_ENTITY_TYPES: &[&str] = &[
    "propiedad",
    "inquilino",
    "contrato",
    "pago",
    "gasto",
    "mantenimiento",
];

/// Validate file size does not exceed 10 MB.
fn validate_file_size(data: &[u8]) -> Result<(), AppError> {
    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el tamaño máximo de 10 MB".to_string(),
        ));
    }
    Ok(())
}

/// Validate the MIME type is in the allowlist.
fn validate_mime_type(mime_type: &str) -> Result<(), AppError> {
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        return Err(AppError::Validation(
            "Tipo de archivo no permitido. Tipos permitidos: PDF, JPG, PNG, DOCX".to_string(),
        ));
    }
    Ok(())
}

/// Extract and validate the file extension from the filename.
fn validate_and_extract_extension(filename: &str) -> Result<String, AppError> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext.is_empty() || !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(
            "Extensión de archivo no permitida. Extensiones permitidas: pdf, jpg, png, docx"
                .to_string(),
        ));
    }
    Ok(ext)
}

/// Validate entity_type is in the allowlist.
fn validate_entity_type(entity_type: &str) -> Result<(), AppError> {
    if !ALLOWED_ENTITY_TYPES.contains(&entity_type) {
        return Err(AppError::Validation(format!(
            "Tipo de entidad no válido: {entity_type}. Tipos válidos: {}",
            ALLOWED_ENTITY_TYPES.join(", ")
        )));
    }
    Ok(())
}

/// Sanitize a filename by stripping path separators and directory traversal sequences.
fn sanitize_filename(name: &str) -> Result<String, AppError> {
    let sanitized: String = name.replace(['/', '\\'], "_").replace("..", "_");
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "Nombre de archivo inválido".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn get_upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

/// Upload a file to disk and create a Documento record.
///
/// File is stored at `./uploads/{uuid}.{ext}` where uuid is a new random UUID
/// and ext is the validated file extension.
#[allow(clippy::cast_possible_wrap)]
pub async fn upload(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
    file_data: &[u8],
    filename: &str,
    mime_type: &str,
    uploaded_by: Uuid,
) -> Result<DocumentoResponse, AppError> {
    // ── Validation ─────────────────────────────────────────────
    validate_file_size(file_data)?;
    validate_mime_type(mime_type)?;
    validate_entity_type(entity_type)?;
    let ext = validate_and_extract_extension(filename)?;
    let safe_filename = sanitize_filename(filename)?;

    // ── Store file on disk ─────────────────────────────────────
    let file_uuid = Uuid::new_v4();
    let stored_filename = format!("{file_uuid}.{ext}");
    let upload_dir = get_upload_dir();

    // Create upload directory if it doesn't exist
    std::fs::create_dir_all(&upload_dir)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando directorio: {e}")))?;

    let full_path = format!("{upload_dir}/{stored_filename}");

    // Write file to disk
    std::fs::write(&full_path, file_data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo archivo: {e}")))?;

    // Verify the resolved path stays within the upload directory (path traversal protection)
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

    // ── Create database record ─────────────────────────────────
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().into();
    let file_size = file_data.len() as i64;

    use crate::entities::documento;

    let model = documento::ActiveModel {
        id: Set(id),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        filename: Set(safe_filename),
        file_path: Set(stored_filename),
        mime_type: Set(mime_type.to_string()),
        file_size: Set(file_size),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
        tipo_documento: Set("general".to_string()),
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

    Ok(DocumentoResponse {
        id: inserted.id,
        entity_type: inserted.entity_type,
        entity_id: inserted.entity_id,
        filename: inserted.filename,
        file_path: inserted.file_path,
        mime_type: inserted.mime_type,
        file_size: inserted.file_size,
        uploaded_by: inserted.uploaded_by,
        created_at: inserted.created_at.into(),
        tipo_documento: inserted.tipo_documento,
        estado_verificacion: inserted.estado_verificacion,
        fecha_vencimiento: inserted.fecha_vencimiento,
        verificado_por: inserted.verificado_por,
        fecha_verificacion: inserted.fecha_verificacion.map(Into::into),
        notas_verificacion: inserted.notas_verificacion,
        numero_documento: inserted.numero_documento,
        contenido_editable: inserted.contenido_editable,
        updated_at: inserted.updated_at.map(Into::into),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_file_size_rejects_over_10mb() {
        let data = vec![0u8; 11 * 1024 * 1024];
        assert!(validate_file_size(&data).is_err());
    }

    #[test]
    fn validate_file_size_accepts_exactly_10mb() {
        let data = vec![0u8; 10 * 1024 * 1024];
        assert!(validate_file_size(&data).is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_pdf() {
        assert!(validate_mime_type("application/pdf").is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_jpeg() {
        assert!(validate_mime_type("image/jpeg").is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_png() {
        assert!(validate_mime_type("image/png").is_ok());
    }

    #[test]
    fn validate_mime_type_accepts_docx() {
        assert!(validate_mime_type(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        )
        .is_ok());
    }

    #[test]
    fn validate_mime_type_rejects_gif() {
        assert!(validate_mime_type("image/gif").is_err());
    }

    #[test]
    fn validate_extension_accepts_allowed() {
        assert!(validate_and_extract_extension("doc.pdf").is_ok());
        assert!(validate_and_extract_extension("photo.jpg").is_ok());
        assert!(validate_and_extract_extension("photo.jpeg").is_ok());
        assert!(validate_and_extract_extension("image.png").is_ok());
        assert!(validate_and_extract_extension("report.docx").is_ok());
    }

    #[test]
    fn validate_extension_rejects_exe() {
        assert!(validate_and_extract_extension("malware.exe").is_err());
    }

    #[test]
    fn validate_extension_rejects_no_extension() {
        assert!(validate_and_extract_extension("noext").is_err());
    }

    #[test]
    fn validate_entity_type_accepts_valid() {
        assert!(validate_entity_type("propiedad").is_ok());
        assert!(validate_entity_type("inquilino").is_ok());
        assert!(validate_entity_type("contrato").is_ok());
    }

    #[test]
    fn validate_entity_type_rejects_invalid() {
        assert!(validate_entity_type("unknown").is_err());
        assert!(validate_entity_type("../etc").is_err());
    }

    #[test]
    fn sanitize_filename_strips_traversal() {
        let result = sanitize_filename("../../etc/passwd").unwrap();
        assert!(!result.contains('/'));
        assert!(!result.contains(".."));
    }

    #[test]
    fn sanitize_filename_preserves_normal() {
        assert_eq!(sanitize_filename("report.pdf").unwrap(), "report.pdf");
    }
}
