use crate::errors::AppError;
use crate::models::ocr::OcrResult;
use std::time::Duration;

pub struct OcrClient {
    base_url: String,
    client: reqwest::Client,
}

impl OcrClient {
    pub fn new() -> Result<Self, AppError> {
        let base_url = std::env::var("OCR_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando cliente HTTP: {e}")))?;
        Ok(Self { base_url, client })
    }

    pub async fn extract(
        &self,
        file_data: &[u8],
        filename: &str,
        content_type: &str,
        document_type: Option<&str>,
    ) -> Result<OcrResult, AppError> {
        let part = reqwest::multipart::Part::bytes(file_data.to_vec())
            .file_name(filename.to_string())
            .mime_str(content_type)
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error creando parte multipart: {e}"))
            })?;

        let mut form = reqwest::multipart::Form::new().part("image", part);

        if let Some(dt) = document_type {
            form = form.text("document_type", dt.to_string());
        }

        let url = format!("{}/ocr/extract", self.base_url);

        let response = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Servicio OCR no disponible: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(AppError::Internal(anyhow::anyhow!(
                "Error del servicio OCR: {status}"
            )));
        }

        response
            .json::<OcrResult>()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error procesando respuesta OCR: {e}")))
    }
}
