use crate::errors::AppError;
use crate::models::ocr::OcrResult;
use std::time::Duration;

pub struct OcrClient {
    base_url: String,
    client: reqwest::Client,
    token: Option<String>,
}

impl OcrClient {
    pub fn new() -> Result<Self, AppError> {
        let base_url = std::env::var("OCR_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let token = std::env::var("OCR_SERVICE_TOKEN")
            .ok()
            .filter(|s| !s.is_empty());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando cliente HTTP: {e}")))?;
        Ok(Self {
            base_url,
            client,
            token,
        })
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

        let mut request = self.client.post(&url).multipart(form);

        if let Some(ref t) = self.token {
            request = request.header("Authorization", format!("Bearer {t}"));
        }

        let response = request
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
