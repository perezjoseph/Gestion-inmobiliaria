use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::importacion::ImportFormat;
use crate::services::importacion;

fn detect_format(filename: &str) -> Result<ImportFormat, AppError> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".csv") {
        Ok(ImportFormat::Csv)
    } else if lower.ends_with(".xlsx") {
        Ok(ImportFormat::Xlsx)
    } else {
        Err(AppError::Validation(
            "Formato no soportado. Use archivos CSV o XLSX".to_string(),
        ))
    }
}

async fn extract_file(mut payload: Multipart) -> Result<(Vec<u8>, String), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

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

    Ok((file_data, filename))
}

pub async fn importar_propiedades(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let (file_data, filename) = extract_file(payload).await?;
    let formato = detect_format(&filename)?;
    let result = importacion::importar_propiedades(db.get_ref(), &file_data, formato).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn importar_inquilinos(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let (file_data, filename) = extract_file(payload).await?;
    let formato = detect_format(&filename)?;
    let result = importacion::importar_inquilinos(db.get_ref(), &file_data, formato).await?;
    Ok(HttpResponse::Ok().json(result))
}
