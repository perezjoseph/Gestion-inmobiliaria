use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContratoProximoVencer {
    pub contrato_id: String,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub fecha_fin: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_actual: f64,
    pub moneda: String,
    pub dias_restantes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropuestaRenovacion {
    pub contrato_id: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_actual: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_maximo: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub ipc_porcentaje: f64,
    pub tope_aplicado: bool,
    pub datos_stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AprobarRenovacionRequest {
    pub monto_aprobado: String,
}
