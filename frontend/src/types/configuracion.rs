use serde::{Deserialize, Serialize};

use super::deserialize_option_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MonedaConfig {
    pub tasa: f64,
    pub actualizado: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecargoDefectoConfig {
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub porcentaje: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecargoDefectoRequest {
    pub porcentaje: f64,
}
