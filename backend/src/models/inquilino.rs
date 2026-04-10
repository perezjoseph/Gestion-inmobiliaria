use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct InquilinoSearchQuery {
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInquilinoRequest {
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInquilinoRequest {
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: Option<String>,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InquilinoResponse {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
