use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMantenimientoProgramadoRequest {
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_estimado: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub frecuencia: String,
    pub proxima_fecha: NaiveDate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMantenimientoProgramadoRequest {
    pub titulo: Option<String>,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_estimado: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub frecuencia: Option<String>,
    pub proxima_fecha: Option<NaiveDate>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MantenimientoProgramadoListQuery {
    pub propiedad_id: Option<Uuid>,
    pub activo: Option<bool>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MantenimientoProgramadoResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub prioridad: String,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_estimado: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub frecuencia: String,
    pub proxima_fecha: NaiveDate,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
