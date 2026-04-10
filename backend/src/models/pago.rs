use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePagoRequest {
    pub contrato_id: Uuid,
    pub monto: Decimal,
    pub moneda: Option<String>,
    pub fecha_pago: Option<NaiveDate>,
    pub fecha_vencimiento: NaiveDate,
    pub metodo_pago: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePagoRequest {
    pub monto: Option<Decimal>,
    pub fecha_pago: Option<NaiveDate>,
    pub metodo_pago: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoListQuery {
    pub contrato_id: Option<Uuid>,
    pub estado: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoResponse {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_pago: Option<NaiveDate>,
    pub fecha_vencimiento: NaiveDate,
    pub metodo_pago: Option<String>,
    pub estado: String,
    pub notas: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
