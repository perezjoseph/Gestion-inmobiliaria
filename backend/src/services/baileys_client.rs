use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::config::ChatbotEnvConfig;
use crate::errors::AppError;

/// HTTP client for communicating with the Baileys `WhatsApp` sidecar service.
///
/// Designed to be stored in Actix `app_data` and shared across handlers.
/// Uses a persistent `reqwest::Client` with connection pooling.
#[derive(Clone)]
pub struct BaileysClient {
    base_url: String,
    client: reqwest::Client,
}

/// Response from `POST /sessions/{realmId}/start`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResponse {
    /// Base64-encoded QR code data (present when status is `qr_pending`).
    pub qr: Option<String>,
    /// Current connection status after the start attempt.
    pub status: String,
}

/// Response from `POST /sessions/{realmId}/send`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    /// Whether the message was successfully queued/sent.
    pub success: bool,
    /// Optional message ID from `WhatsApp`.
    pub message_id: Option<String>,
}

/// Response from `GET /sessions/{realmId}/status`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusResponse {
    /// Current connection status: `disconnected`, `qr_pending`, `connected`, `logged_out`.
    pub status: String,
}

/// Response from `POST /sessions/{realmId}/stop`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSessionResponse {
    /// Whether the session was successfully stopped.
    pub success: bool,
}

/// Request body for `POST /sessions/{realmId}/send`.
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    phone: String,
    content: String,
}

impl BaileysClient {
    /// Creates a new `BaileysClient` from the chatbot environment configuration.
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

    /// Starts a `WhatsApp` session for the given organization.
    ///
    /// Calls `POST /sessions/{realmId}/start` on the Baileys sidecar.
    /// Returns QR code data or current connection status.
    pub async fn start_session(&self, realm_id: Uuid) -> Result<StartSessionResponse, AppError> {
        let url = format!("{}/sessions/{}/start", self.base_url, realm_id);

        let response = self.client.post(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error conectando al servicio Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    /// Sends a `WhatsApp` message to a recipient via the Baileys sidecar.
    ///
    /// Calls `POST /sessions/{realmId}/send` with the phone number and content.
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

    /// Stops the `WhatsApp` session for the given organization.
    ///
    /// Calls `POST /sessions/{realmId}/stop` on the Baileys sidecar.
    pub async fn stop_session(&self, realm_id: Uuid) -> Result<StopSessionResponse, AppError> {
        let url = format!("{}/sessions/{}/stop", self.base_url, realm_id);

        let response = self.client.post(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error desconectando sesión Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    /// Gets the current connection status for the given organization.
    ///
    /// Calls `GET /sessions/{realmId}/status` on the Baileys sidecar.
    pub async fn get_status(&self, realm_id: Uuid) -> Result<SessionStatusResponse, AppError> {
        let url = format!("{}/sessions/{}/status", self.base_url, realm_id);

        let response = self.client.get(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error consultando estado de Baileys: {e}"))
        })?;

        self.handle_response(response).await
    }

    /// Handles the HTTP response from the Baileys service.
    ///
    /// Returns the deserialized body on success, or an `AppError` on failure.
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
            baileys_internal_token: "a".repeat(32),
            ovms_endpoint: "http://ovms:8000/v1".to_string(),
            ovms_chat_model: "Qwen3.6-35B-A3B".to_string(),
            ai_chat_timeout_secs: 30,
        };

        let client = BaileysClient::new(&config).unwrap();
        assert_eq!(client.base_url, "http://baileys:3100");
    }

    #[test]
    fn new_preserves_url_without_trailing_slash() {
        let config = ChatbotEnvConfig {
            baileys_service_url: "http://baileys:3100".to_string(),
            baileys_internal_token: "a".repeat(32),
            ovms_endpoint: "http://ovms:8000/v1".to_string(),
            ovms_chat_model: "Qwen3.6-35B-A3B".to_string(),
            ai_chat_timeout_secs: 30,
        };

        let client = BaileysClient::new(&config).unwrap();
        assert_eq!(client.base_url, "http://baileys:3100");
    }
}
