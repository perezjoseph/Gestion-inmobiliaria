use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmarRequest {
    pub firma_imagen: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitarFirmaRequest {
    pub firmante_nombre: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitarFirmaResponse {
    pub firma_id: Uuid,
    pub token: String,
    pub expira_at: DateTime<Utc>,
    pub email_enviado: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmaResponse {
    pub id: Uuid,
    pub documento_id: Uuid,
    pub firmante_tipo: String,
    pub firmante_nombre: String,
    pub estado: String,
    pub firmado_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificarTokenRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoFirmaResponse {
    pub documento_id: Uuid,
    pub contenido: serde_json::Value,
    pub firmante_nombre: String,
    pub estado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmarConTokenRequest {
    pub password: String,
    pub firma_imagen: String,
}
