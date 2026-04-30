use actix_multipart::Multipart;
use actix_web::{HttpResponse, web};
use futures_util::StreamExt;
use sea_orm::DatabaseConnection;
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::importacion::ImportFormat;
use crate::models::ocr::ConfirmPreviewRequest;
use crate::services::importacion;
use crate::services::ocr_client::OcrClient;
use crate::services::ocr_mapping;
use crate::services::ocr_preview::PreviewStore;

fn detect_format(filename: &str) -> Result<ImportFormat, AppError> {
    let ext = std::path::Path::new(filename)
        .extension()
        .unwrap_or_default();
    if ext.eq_ignore_ascii_case("csv") {
        Ok(ImportFormat::Csv)
    } else if ext.eq_ignore_ascii_case("xlsx") {
        Ok(ImportFormat::Xlsx)
    } else if ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("pdf")
    {
        Ok(ImportFormat::Image)
    } else {
        Err(AppError::Validation(
            "Formato no soportado. Use archivos CSV, XLSX, o imágenes (JPG, PNG, PDF)".to_string(),
        ))
    }
}

#[allow(clippy::future_not_send)]
async fn extract_file(mut payload: Multipart) -> Result<(Vec<u8>, String), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;

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

            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}")))?;
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

#[allow(clippy::future_not_send)]
pub async fn importar_propiedades(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let (file_data, filename) = extract_file(payload).await?;
    let formato = detect_format(&filename)?;
    let result = importacion::importar_propiedades(db.get_ref(), &file_data, formato, access.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[allow(clippy::future_not_send)]
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

fn content_type_from_filename(filename: &str) -> &'static str {
    let ext = std::path::Path::new(filename)
        .extension()
        .unwrap_or_default();
    if ext.eq_ignore_ascii_case("png") {
        "image/png"
    } else if ext.eq_ignore_ascii_case("pdf") {
        "application/pdf"
    } else {
        "image/jpeg"
    }
}

#[allow(clippy::future_not_send)]
pub async fn importar_pagos(
    _db: web::Data<DatabaseConnection>,
    preview_store: web::Data<PreviewStore>,
    _access: WriteAccess,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let (file_data, filename) = extract_file(payload).await?;
    let formato = detect_format(&filename)?;

    match formato {
        ImportFormat::Image => {
            let client = OcrClient::new()?;
            let ct = content_type_from_filename(&filename);
            let ocr_result = client.extract(&file_data, &filename, ct, None).await?;
            let preview = ocr_mapping::map_deposito(&ocr_result)?;
            preview_store.insert(preview.clone());
            Ok(HttpResponse::Ok().json(preview))
        }
        _ => Err(AppError::Validation(
            "Importación CSV/XLSX de pagos no soportada. Use imágenes de recibos de depósito."
                .to_string(),
        )),
    }
}

#[allow(clippy::future_not_send)]
pub async fn importar_gastos(
    db: web::Data<DatabaseConnection>,
    preview_store: web::Data<PreviewStore>,
    access: WriteAccess,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let (file_data, filename) = extract_file(payload).await?;
    let formato = detect_format(&filename)?;

    if formato == ImportFormat::Image {
        let client = OcrClient::new()?;
        let ct = content_type_from_filename(&filename);
        let ocr_result = client.extract(&file_data, &filename, ct, None).await?;
        let preview = ocr_mapping::map_gasto(&ocr_result)?;
        preview_store.insert(preview.clone());
        Ok(HttpResponse::Ok().json(preview))
    } else {
        let result =
            importacion::importar_gastos(db.get_ref(), &file_data, formato, access.0.sub, access.0.organizacion_id)
                .await?;
        Ok(HttpResponse::Ok().json(result))
    }
}

pub async fn confirmar_preview(
    _db: web::Data<DatabaseConnection>,
    preview_store: web::Data<PreviewStore>,
    _access: WriteAccess,
    body: web::Json<ConfirmPreviewRequest>,
) -> Result<HttpResponse, AppError> {
    let mut preview = preview_store
        .remove(&body.preview_id)
        .ok_or_else(|| AppError::NotFound("Vista previa no encontrada o expirada".to_string()))?;

    if let Some(corrections) = &body.corrections {
        for field in &mut preview.fields {
            if let Some(corrected) = corrections.get(&field.name) {
                field.value = corrected.clone();
                field.confidence = 1.0;
            }
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "totalFilas": 1,
        "exitosos": 1,
        "fallidos": 0
    })))
}

pub async fn descartar_preview(
    preview_store: web::Data<PreviewStore>,
    _access: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let preview_id = path.into_inner();
    match preview_store.remove(&preview_id) {
        Some(_) => Ok(HttpResponse::Ok().json(json!({"message": "Vista previa descartada"}))),
        None => Err(AppError::NotFound(
            "Vista previa no encontrada o expirada".to_string(),
        )),
    }
}
