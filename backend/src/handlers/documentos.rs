use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::documentos;

#[derive(serde::Deserialize)]
pub struct DocumentoPath {
    pub entity_type: String,
    pub entity_id: Uuid,
}

pub async fn upload(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<DocumentoPath>,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| AppError::Validation(format!("Error procesando multipart: {}", e)))?;

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
                .map(|f| f.to_string());
            content_type = field.content_type().map(|ct| ct.to_string());

            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk.map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {}", e))
                })?;
                data.extend_from_slice(&chunk);
            }
            file_data = Some(data);
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;
    let filename = filename
        .ok_or_else(|| AppError::Validation("Nombre de archivo no proporcionado".to_string()))?;
    let mime_type = content_type
        .ok_or_else(|| AppError::Validation("Tipo de contenido no proporcionado".to_string()))?;

    let result = documentos::upload(
        db.get_ref(),
        &path.entity_type,
        path.entity_id,
        &file_data,
        &filename,
        &mime_type,
        claims.sub,
    )
    .await?;

    Ok(HttpResponse::Created().json(result))
}

pub async fn listar(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<DocumentoPath>,
) -> Result<HttpResponse, AppError> {
    let path = path.into_inner();
    let docs =
        documentos::listar_documentos(db.get_ref(), &path.entity_type, path.entity_id).await?;
    Ok(HttpResponse::Ok().json(docs))
}
