use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::services::documentos_upload;

/// POST /api/v1/documentos/upload
///
/// Accepts a multipart form with:
/// - `file`: the uploaded file (PDF, JPG, PNG, DOCX; max 10 MB)
/// - `entity_type`: string form field (e.g. "propiedad", "inquilino", "contrato")
/// - `entity_id`: UUID form field identifying the parent entity
///
/// Returns 201 with the created Documento record on success.
#[allow(clippy::future_not_send)]
pub async fn upload(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut entity_type: Option<String> = None;
    let mut entity_id: Option<uuid::Uuid> = None;

    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| AppError::Validation(format!("Error procesando multipart: {e}")))?;

        let disposition = field.content_disposition();
        let field_name = disposition
            .as_ref()
            .and_then(|d| d.get_name())
            .unwrap_or("")
            .to_string();

        match field_name.as_str() {
            "file" => {
                filename = disposition
                    .as_ref()
                    .and_then(|d| d.get_filename())
                    .map(ToString::to_string);
                content_type = field.content_type().map(ToString::to_string);

                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}"))
                    })?;
                    data.extend_from_slice(&chunk);

                    // Reject early if file exceeds size limit (avoid buffering entire payload)
                    if data.len() > MAX_FILE_SIZE {
                        return Err(AppError::Validation(
                            "El archivo excede el tamaño máximo de 10 MB".to_string(),
                        ));
                    }
                }
                file_data = Some(data);
            }
            "entity_type" => {
                let mut value = String::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Error leyendo campo: {e}"))
                    })?;
                    value.push_str(&String::from_utf8_lossy(&chunk));
                }
                if value.len() > 50 {
                    return Err(AppError::Validation(
                        "entity_type excede longitud máxima".to_string(),
                    ));
                }
                entity_type = Some(value);
            }
            "entity_id" => {
                let mut value = String::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Error leyendo campo: {e}"))
                    })?;
                    value.push_str(&String::from_utf8_lossy(&chunk));
                }
                let parsed = uuid::Uuid::parse_str(value.trim()).map_err(|_| {
                    AppError::Validation("entity_id debe ser un UUID válido".to_string())
                })?;
                entity_id = Some(parsed);
            }
            _ => {
                // Ignore unknown fields — consume and discard
                while field.next().await.is_some() {}
            }
        }
    }

    // ── Validate required fields ───────────────────────────────

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;
    let filename = filename
        .ok_or_else(|| AppError::Validation("Nombre de archivo no proporcionado".to_string()))?;
    let _content_type = content_type
        .ok_or_else(|| AppError::Validation("Tipo de contenido no proporcionado".to_string()))?;
    let entity_type = entity_type
        .ok_or_else(|| AppError::Validation("El campo 'entity_type' es requerido".to_string()))?;
    let entity_id = entity_id
        .ok_or_else(|| AppError::Validation("El campo 'entity_id' es requerido".to_string()))?;

    // ── Delegate to service layer ──────────────────────────────

    let result = documentos_upload::upload(
        db.get_ref(),
        &entity_type,
        entity_id,
        &file_data,
        &filename,
        access.0.sub,
    )
    .await?;

    Ok(HttpResponse::Created().json(result))
}
