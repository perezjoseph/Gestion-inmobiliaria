use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;
use uuid::Uuid;

use crate::config::ChatbotEnvConfig;
use crate::errors::AppError;

#[derive(Clone)]
pub struct BaileysClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResponse {
    pub qr: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub success: bool,
    pub message_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSessionResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    phone: String,
    content: String,
}

impl BaileysClient {
    pub fn new(config: &ChatbotEnvConfig) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!(
                    "Error creando cliente HTTP para Baileys: {e}"
                ))
            })?;

        Ok(Self {
            base_url: config.baileys_service_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    #[instrument(name = "baileys.start_session", skip(self), fields(realm_id = %realm_id))]
    pub async fn start_session(&self, realm_id: Uuid) -> Result<StartSessionResponse, AppError> {
        let url = format!("{}/sessions/{}/start", self.base_url, realm_id);

        let response = self.client.post(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error conectando al servicio Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    #[instrument(name = "baileys.send_message", skip(self, content), fields(realm_id = %realm_id))]
    pub async fn send_message(
        &self,
        realm_id: Uuid,
        phone: &str,
        content: &str,
    ) -> Result<SendMessageResponse, AppError> {
        let url = format!("{}/sessions/{}/send", self.base_url, realm_id);

        let body = SendMessageRequest {
            phone: phone.to_string(),
            content: content.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error enviando mensaje via Baileys: {e}"))
            })?;

        self.handle_response(response).await
    }

    #[instrument(name = "baileys.stop_session", skip(self), fields(realm_id = %realm_id))]
    pub async fn stop_session(&self, realm_id: Uuid) -> Result<StopSessionResponse, AppError> {
        let url = format!("{}/sessions/{}/stop", self.base_url, realm_id);

        let response = self.client.post(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error desconectando sesión Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    #[instrument(name = "baileys.get_status", skip(self), fields(realm_id = %realm_id))]
    pub async fn get_status(&self, realm_id: Uuid) -> Result<SessionStatusResponse, AppError> {
        let url = format!("{}/sessions/{}/status", self.base_url, realm_id);

        let response = self.client.get(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error consultando estado de Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, AppError> {
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!(
                "Baileys respondió con estado {status}: {body}"
            )));
        }

        response.json::<T>().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "Error procesando respuesta de Baileys: {e}"
            ))
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_client_with_trimmed_url() {
        let config = ChatbotEnvConfig {
            baileys_service_url: "http://baileys:3100/".to_string(),
            ..ChatbotEnvConfig::for_testing()
        };

        let client = BaileysClient::new(&config).unwrap();
        assert_eq!(client.base_url, "http://baileys:3100");
    }

    #[test]
    fn new_preserves_url_without_trailing_slash() {
        let config = ChatbotEnvConfig {
            baileys_service_url: "http://baileys:3100".to_string(),
            ..ChatbotEnvConfig::for_testing()
        };

        let client = BaileysClient::new(&config).unwrap();
        assert_eq!(client.base_url, "http://baileys:3100");
    }
}
