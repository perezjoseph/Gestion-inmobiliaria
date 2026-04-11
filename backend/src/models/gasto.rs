use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGastoRequest {
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_gasto: NaiveDate,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGastoRequest {
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub monto: Option<Decimal>,
    pub moneda: Option<String>,
    pub fecha_gasto: Option<NaiveDate>,
    pub unidad_id: Option<Uuid>,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GastoListQuery {
    pub propiedad_id: Option<Uuid>,
    pub unidad_id: Option<Uuid>,
    pub categoria: Option<String>,
    pub estado: Option<String>,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumenCategoriasQuery {
    pub propiedad_id: Uuid,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
}
