use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePropiedadRequest {
    pub titulo: String,
    pub descripcion: Option<String>,
    pub direccion: String,
    pub ciudad: String,
    pub provincia: String,
    pub tipo_propiedad: String,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Decimal,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub imagenes: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePropiedadRequest {
    pub titulo: Option<String>,
    pub descripcion: Option<String>,
    pub direccion: Option<String>,
    pub ciudad: Option<String>,
    pub provincia: Option<String>,
    pub tipo_propiedad: Option<String>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Option<Decimal>,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub imagenes: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropiedadResponse {
    pub id: Uuid,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub direccion: String,
    pub ciudad: String,
    pub provincia: String,
    pub tipo_propiedad: String,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<Decimal>,
    pub precio: Decimal,
    pub moneda: String,
    pub estado: String,
    pub imagenes: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropiedadListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub ciudad: Option<String>,
    pub provincia: Option<String>,
    pub tipo_propiedad: Option<String>,
    pub estado: Option<String>,
    pub precio_min: Option<Decimal>,
    pub precio_max: Option<Decimal>,
}
