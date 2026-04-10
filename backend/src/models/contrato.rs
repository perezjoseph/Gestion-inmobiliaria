use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContratoRequest {
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub deposito: Option<Decimal>,
    pub moneda: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContratoRequest {
    pub fecha_fin: Option<NaiveDate>,
    pub monto_mensual: Option<Decimal>,
    pub deposito: Option<Decimal>,
    pub estado: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContratoResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub deposito: Option<Decimal>,
    pub moneda: String,
    pub estado: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
