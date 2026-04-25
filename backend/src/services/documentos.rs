use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::documento;
use crate::errors::AppError;
use crate::models::documento::DocumentoResponse;

const MAX_FILE_SIZE: i64 = 10 * 1024 * 1024;
const ALLOWED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "application/pdf"];
const ALLOWED_ENTITY_TYPES: &[&str] = &[
    "propiedad",
    "inquilino",
    "contrato",
    "pago",
    "gasto",
    "mantenimiento",
];

fn validate_file_size(size: i64) -> Result<(), AppError> {
    if size > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el tamaño máximo de 10 MB".to_string(),
        ));
    }
    Ok(())
}

fn validate_mime_type(mime_type: &str) -> Result<(), AppError> {
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        return Err(AppError::Validation(
            "Tipo de archivo no permitido. Tipos permitidos: JPEG, PNG, PDF".to_string(),
        ));
    }
    Ok(())
}

fn validate_entity_type(entity_type: &str) -> Result<(), AppError> {
    if !ALLOWED_ENTITY_TYPES.contains(&entity_type) {
        return Err(AppError::Validation(format!(
            "Tipo de entidad no válido: {entity_type}"
        )));
    }
    Ok(())
}

/// Sanitize a filename by stripping path separators and directory traversal sequences.
fn sanitize_filename(name: &str) -> Result<String, AppError> {
    let sanitized: String = name.replace(['/', '\\'], "_").replace("..", "_");

    // After sanitization the name must still be non-empty
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
    let file_size = file_data.len() as i64;
    validate_file_size(file_size)?;
    validate_mime_type(mime_type)?;
    validate_entity_type(entity_type)?;
    let safe_filename = sanitize_filename(filename)?;

    let upload_dir = get_upload_dir();
    // entity_type is validated against an allowlist; entity_id is a Uuid (no traversal possible)
    let dir_path = format!("{upload_dir}/{entity_type}/{entity_id}");
    std::fs::create_dir_all(&dir_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando directorio: {e}")))?;

    let file_uuid = Uuid::new_v4();
    let stored_filename = format!("{file_uuid}-{safe_filename}");
    let full_path = format!("{dir_path}/{stored_filename}");

    // Verify the resolved path stays within the upload directory
    let canonical_dir = std::fs::canonicalize(&upload_dir)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo directorio: {e}")))?;
    // Write the file first so canonicalize can resolve it
    std::fs::write(&full_path, file_data)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error escribiendo archivo: {e}")))?;
    let canonical_file = std::fs::canonicalize(&full_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error resolviendo ruta: {e}")))?;
    if !canonical_file.starts_with(&canonical_dir) {
        // Remove the file that escaped the upload directory
        let _ = std::fs::remove_file(&full_path);
        return Err(AppError::Validation(
            "Ruta de archivo fuera del directorio permitido".to_string(),
        ));
    }

    let relative_path = format!("{entity_type}/{entity_id}/{stored_filename}");
    let id = Uuid::new_v4();
    let now = Utc::now().into();

    let model = documento::ActiveModel {
        id: Set(id),
        entity_type: Set(entity_type.to_string()),
        entity_id: Set(entity_id),
        filename: Set(safe_filename),
        file_path: Set(relative_path.clone()),
        mime_type: Set(mime_type.to_string()),
        file_size: Set(file_size),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
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
    })
}

pub async fn listar_documentos(
    db: &DatabaseConnection,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<Vec<DocumentoResponse>, AppError> {
    let docs = documento::Entity::find()
        .filter(documento::Column::EntityType.eq(entity_type))
        .filter(documento::Column::EntityId.eq(entity_id))
        .all(db)
        .await?;

    Ok(docs
        .into_iter()
        .map(|d| DocumentoResponse {
            id: d.id,
            entity_type: d.entity_type,
            entity_id: d.entity_id,
            filename: d.filename,
            file_path: d.file_path,
            mime_type: d.mime_type,
            file_size: d.file_size,
            uploaded_by: d.uploaded_by,
            created_at: d.created_at.into(),
        })
        .collect())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_file_size_rejects_over_10mb() {
        let size = 11 * 1024 * 1024;
        let result = validate_file_size(size);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "El archivo excede el tamaño máximo de 10 MB"
        );
    }

    #[test]
    fn validate_file_size_accepts_exactly_10mb() {
        let size = 10 * 1024 * 1024;
        assert!(validate_file_size(size).is_ok());
    }

    #[test]
    fn validate_file_size_accepts_small_file() {
        assert!(validate_file_size(1024).is_ok());
    }

    #[test]
    fn validate_file_size_accepts_zero() {
        assert!(validate_file_size(0).is_ok());
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
    fn validate_mime_type_accepts_pdf() {
        assert!(validate_mime_type("application/pdf").is_ok());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn validate_mime_type_rejects_gif() {
        let result = validate_mime_type("image/gif");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Tipo de archivo no permitido. Tipos permitidos: JPEG, PNG, PDF"
        );
    }

    #[test]
    fn validate_mime_type_rejects_text() {
        assert!(validate_mime_type("text/plain").is_err());
    }

    #[test]
    fn validate_mime_type_rejects_empty() {
        assert!(validate_mime_type("").is_err());
    }

    #[test]
    fn validate_entity_type_accepts_allowed() {
        for t in ALLOWED_ENTITY_TYPES {
            assert!(validate_entity_type(t).is_ok(), "should accept {t}");
        }
    }

    #[test]
    fn validate_entity_type_rejects_unknown() {
        assert!(validate_entity_type("unknown").is_err());
    }

    #[test]
    fn validate_entity_type_rejects_traversal() {
        assert!(validate_entity_type("../etc").is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_strips_path_separators() {
        let result = sanitize_filename("../../etc/passwd").unwrap();
        assert!(!result.contains('/'));
        assert!(!result.contains(".."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_strips_backslashes() {
        let result = sanitize_filename("..\\..\\windows\\system32").unwrap();
        assert!(!result.contains('\\'));
        assert!(!result.contains(".."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn sanitize_filename_preserves_normal_name() {
        assert_eq!(sanitize_filename("photo.jpg").unwrap(), "photo.jpg");
    }

    #[test]
    fn sanitize_filename_rejects_empty() {
        assert!(sanitize_filename("").is_err());
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    #[allow(clippy::unwrap_used)]
    #[allow(unsafe_code)]
    fn get_upload_dir_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPLOAD_DIR") };
        assert_eq!(get_upload_dir(), "./uploads");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    #[allow(unsafe_code)]
    fn get_upload_dir_returns_env_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("UPLOAD_DIR", "/tmp/test-uploads") };
        assert_eq!(get_upload_dir(), "/tmp/test-uploads");
        unsafe { std::env::remove_var("UPLOAD_DIR") };
    }
}
