use serde::{Deserialize, Serialize};

use crate::types::{deserialize_f64_from_any, deserialize_option_f64_from_any};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Contrato {
    pub id: String,
    pub propiedad_id: String,
    pub inquilino_id: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_mensual: f64,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub deposito: Option<f64>,
    pub moneda: String,
    pub estado: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateContrato {
    pub propiedad_id: String,
    pub inquilino_id: String,
    pub fecha_inicio: String,
    pub fecha_fin: String,
    pub monto_mensual: f64,
    pub deposito: Option<f64>,
    pub moneda: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateContrato {
    pub fecha_fin: Option<String>,
    pub monto_mensual: Option<f64>,
    pub deposito: Option<f64>,
    pub estado: Option<String>,
}
