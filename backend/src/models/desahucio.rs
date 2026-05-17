use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesahucioRequest {
    pub contrato_id: Uuid,
    pub motivo: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDesahucioRequest {
    pub estado: Option<String>,
    pub fecha_resolucion: Option<NaiveDate>,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesahucioResponse {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub estado: String,
    pub fecha_inicio: NaiveDate,
    pub fecha_resolucion: Option<NaiveDate>,
    pub motivo: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesahucioListQuery {
    pub contrato_id: Option<Uuid>,
    pub estado: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}
