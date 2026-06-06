use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGastoRecurrenteRequest {
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: Decimal,
    pub moneda: String,
    pub proveedor: Option<String>,
    pub frecuencia: String,
    pub dia_del_mes: Option<i32>,
    pub proxima_fecha: NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGastoRecurrenteRequest {
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub monto: Option<Decimal>,
    pub moneda: Option<String>,
    pub proveedor: Option<String>,
    pub frecuencia: Option<String>,
    pub dia_del_mes: Option<i32>,
    pub proxima_fecha: Option<NaiveDate>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GastoRecurrenteListQuery {
    pub propiedad_id: Option<Uuid>,
    pub activo: Option<bool>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GastoRecurrenteResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: Decimal,
    pub moneda: String,
    pub proveedor: Option<String>,
    pub frecuencia: String,
    pub dia_del_mes: Option<i32>,
    pub proxima_fecha: NaiveDate,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
