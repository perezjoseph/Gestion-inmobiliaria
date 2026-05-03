use serde::{Deserialize, Serialize};

use crate::types::{deserialize_f64_from_any, deserialize_option_f64_from_any};

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Unidad {
    pub id: String,
    pub propiedad_id: String,
    pub numero_unidad: String,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    #[serde(deserialize_with = "deserialize_option_f64_from_any", default)]
    pub area_m2: Option<f64>,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub precio: f64,
    pub moneda: String,
    pub estado: String,
    pub descripcion: Option<String>,
    pub gastos_count: Option<u64>,
    pub mantenimiento_count: Option<u64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateUnidad {
    pub numero_unidad: String,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<f64>,
    pub precio: f64,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub descripcion: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUnidad {
    pub numero_unidad: Option<String>,
    pub piso: Option<i32>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<f64>,
    pub precio: Option<f64>,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub descripcion: Option<String>,
}

