use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuotaCondominio {
    pub id: String,
    pub propiedad_id: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub frecuencia: String,
    pub fecha_inicio: String,
    pub fecha_fin: Option<String>,
    pub es_passthrough: bool,
    pub contrato_id: Option<String>,
    pub organizacion_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateCuotaCondominio {
    pub monto: f64,
    pub moneda: String,
    pub frecuencia: String,
    pub fecha_inicio: String,
    pub es_passthrough: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCuotaCondominio {
    pub monto: Option<f64>,
    pub moneda: Option<String>,
    pub frecuencia: Option<String>,
    pub fecha_inicio: Option<String>,
    pub fecha_fin: Option<String>,
    pub es_passthrough: Option<bool>,
}

/// Billing preview response from the API
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillingPreview {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_base: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub cuota_condominio: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_base: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_cuota: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total: f64,
    pub moneda: String,
}
