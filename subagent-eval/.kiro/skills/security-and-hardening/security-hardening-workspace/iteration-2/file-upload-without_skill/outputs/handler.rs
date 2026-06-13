use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::services::auth::Claims;
use crate::services::documentos_upload;

/// POST /api/v1/documentos/upload
///
/// Accepts a multipart form with:
/// - `file`: the uploaded file (required)
/// - `entity_type`: string identifying the parent entity type (required)
/// - `entity_id`: UUID of the parent entity (required)
///
/// Allowed file types: PDF, JPG, PNG, DOCX (max 10 MB).
/// Stores the file on disk at `./uploads/{uuid}.{ext}` and creates a Documento record.
#[allow(clippy::future_not_send)]
pub async fn upload_documento(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    _access: WriteAccess,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut entity_type: Option<String> = None;
    let mut entity_id: Option<uuid::Uuid> = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| AppError::Validation(format!("Error procesando multipart: {e}")))?;

        let disposition = field.content_disposition();
        let field_name = disposition
            .as_ref()
            .and_then(|d| d.get_name())
            .unwrap_or("")
            .to_string();

        if field_name == "file" {
            filename = disposition
                .as_ref()
                .and_then(|d| d.get_filename())
                .map(ToString::to_string);
            content_type = field.content_type().map(ToString::to_string);

            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}")))?;
                data.extend_from_slice(&chunk);

                // Early rejection: stop reading if file exceeds 10 MB
                if data.len() > 10 * 1024 * 1024 {
                    return Err(AppError::Validation(
                        "El archivo excede el tamaño máximo de 10 MB".to_string(),
                    ));
                }
            }
            file_data = Some(data);
        } else {
            // Read text field value
            let mut value = String::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo campo: {e}")))?;
                value.push_str(&String::from_utf8_lossy(&chunk));
            }
            match field_name.as_str() {
                "entity_type" => entity_type = Some(value),
                "entity_id" => {
                    entity_id = Some(value.parse::<uuid::Uuid>().map_err(|_| {
                        AppError::Validation(
                            "entity_id debe ser un UUID válido".to_string(),
                        )
                    })?);
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // Validate required fields
    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;
    let filename = filename
        .ok_or_else(|| AppError::Validation("Nombre de archivo no proporcionado".to_string()))?;
    let content_type = content_type
        .ok_or_else(|| AppError::Validation("Tipo de contenido no proporcionado".to_string()))?;
    let entity_type = entity_type.ok_or_else(|| {
        AppError::Validation("El campo 'entity_type' es requerido".to_string())
    })?;
    let entity_id = entity_id.ok_or_else(|| {
        AppError::Validation("El campo 'entity_id' es requerido".to_string())
    })?;

    let result = documentos_upload::upload(
        db.get_ref(),
        &entity_type,
        entity_id,
        &file_data,
        &filename,
        &content_type,
        claims.sub,
    )
    .await?;

    Ok(HttpResponse::Created().json(result))
}
