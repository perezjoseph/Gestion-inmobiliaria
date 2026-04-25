use serde::{Deserialize, Serialize};

use crate::types::{deserialize_f64_from_any, deserialize_option_f64_from_any};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Propiedad {
    pub id: String,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub direccion: String,
    pub ciudad: String,
    pub provincia: String,
    pub tipo_propiedad: String,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub area_m2: Option<f64>,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub precio: f64,
    pub moneda: String,
    pub estado: String,
    pub imagenes: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePropiedad {
    pub titulo: String,
    pub descripcion: Option<String>,
    pub direccion: String,
    pub ciudad: String,
    pub provincia: String,
    pub tipo_propiedad: String,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<f64>,
    pub precio: f64,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub imagenes: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePropiedad {
    pub titulo: Option<String>,
    pub descripcion: Option<String>,
    pub direccion: Option<String>,
    pub ciudad: Option<String>,
    pub provincia: Option<String>,
    pub tipo_propiedad: Option<String>,
    pub habitaciones: Option<i32>,
    pub banos: Option<i32>,
    pub area_m2: Option<f64>,
    pub precio: Option<f64>,
    pub moneda: Option<String>,
    pub estado: Option<String>,
    pub imagenes: Option<serde_json::Value>,
}
