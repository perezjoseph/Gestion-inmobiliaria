use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IpiLiabilityResponse {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub valor_total: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub umbral: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub exceso: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub ipi_anual: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub pago_semestral: f64,
    pub proxima_fecha: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CopropietarioResponse {
    pub id: String,
    pub propiedad_id: String,
    pub nombre: String,
    pub cedula_rnc: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub porcentaje_propiedad: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrearCopropietarioRequest {
    pub propiedad_id: String,
    pub nombre: String,
    pub cedula_rnc: String,
    pub porcentaje_propiedad: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguracionIpiRequest {
    pub umbral_ipi: f64,
    pub anio: i32,
    pub fecha_pago_1: String,
    pub fecha_pago_2: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropiedadIpiInfo {
    pub id: String,
    pub titulo: String,
    #[serde(
        default,
        deserialize_with = "crate::types::deserialize_option_f64_from_any"
    )]
    pub valor_catastral: Option<f64>,
    #[serde(default)]
    pub exento_ipi: bool,
    pub motivo_exencion: Option<String>,
}
