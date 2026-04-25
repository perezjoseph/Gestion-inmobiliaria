use actix_multipart::Multipart;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use serde_json::json;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::ocr::{ExtractField, ExtractResponse};
use crate::services::ocr_client::OcrClient;
use crate::services::ocr_mapping;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

const ALLOWED_CONTENT_TYPES: &[&str] = &["image/jpeg", "image/png", "application/pdf"];

fn content_type_from_filename(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename).extension()?;
    if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
        Some("image/jpeg")
    } else if ext.eq_ignore_ascii_case("png") {
        Some("image/png")
    } else if ext.eq_ignore_ascii_case("pdf") {
        Some("application/pdf")
    } else {
        None
    }
}

fn is_valid_content_type(ct: &str) -> bool {
    ALLOWED_CONTENT_TYPES.contains(&ct)
}

#[allow(clippy::future_not_send)]
pub async fn ocr_extract(
    _access: WriteAccess,
    mut payload: Multipart,
) -> Result<HttpResponse, AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut document_type: Option<String> = None;

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

                let mut data = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}"))
                    })?;
                    data.extend_from_slice(&chunk);
                    if data.len() > MAX_FILE_SIZE {
                        return Err(AppError::Validation(
                            "El archivo excede el tamaño máximo de 10 MB".to_string(),
                        ));
                    }
                }
                file_data = Some(data);
            }
            "document_type" => {
                let mut value = String::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Error leyendo chunk: {e}"))
                    })?;
                    value.push_str(
                        std::str::from_utf8(&chunk)
                            .map_err(|_| AppError::Validation("Valor no válido".to_string()))?,
                    );
                }
                let trimmed = value.trim().to_string();
                if !trimmed.is_empty() {
                    document_type = Some(trimmed);
                }
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No se encontró archivo en la solicitud".to_string())
    })?;

    let fname = filename.as_deref().unwrap_or("upload.bin");

    let content_type = content_type_from_filename(fname).ok_or_else(|| {
        AppError::Validation("Formato no soportado. Use archivos JPEG, PNG o PDF".to_string())
    })?;

    if !is_valid_content_type(content_type) {
        return Err(AppError::Validation(
            "Formato no soportado. Use archivos JPEG, PNG o PDF".to_string(),
        ));
    }

    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el tamaño máximo de 10 MB".to_string(),
        ));
    }

    let client = OcrClient::new()?;
    let ocr_result = match client
        .extract(&file_data, fname, content_type, document_type.as_deref())
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Servicio OCR no disponible")
                || msg.contains("Error del servicio OCR")
                || msg.contains("Error procesando respuesta OCR")
            {
                return Ok(HttpResponse::ServiceUnavailable().json(json!({
                    "error": "service_unavailable",
                    "message": "Servicio OCR no disponible"
                })));
            }
            return Err(e);
        }
    };

    let doc_type = document_type
        .as_deref()
        .unwrap_or(&ocr_result.document_type);

    let fields = match doc_type {
        "deposito_bancario" => ocr_mapping::map_deposito_extract(&ocr_result),
        "recibo_gasto" => ocr_mapping::map_gasto_extract(&ocr_result),
        "cedula" => ocr_mapping::map_cedula(&ocr_result),
        "contrato" => ocr_mapping::map_contrato(&ocr_result),
        _ => Ok(raw_fields_from_result(&ocr_result)),
    }
    .unwrap_or_else(|_| raw_fields_from_result(&ocr_result));

    let raw_lines = ocr_result.lines.iter().map(|l| l.text.clone()).collect();

    let response = ExtractResponse {
        document_type: doc_type.to_string(),
        fields,
        raw_lines,
    };

    Ok(HttpResponse::Ok().json(response))
}

fn raw_fields_from_result(result: &crate::models::ocr::OcrResult) -> Vec<ExtractField> {
    result
        .structured_fields
        .iter()
        .map(|(key, value)| ExtractField {
            name: key.clone(),
            value: value.clone(),
            label: key.clone(),
            confidence: 0.0,
        })
        .collect()
}
