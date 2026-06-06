use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;
use crate::types::deserialize_option_f64_from_any;

/// Per-property comparison row in the comparative dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyComparison {
    pub propiedad_id: String,
    pub titulo: String,
    pub tipo_propiedad: String,
    pub moneda: String,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub ingresos: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub gastos: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub cuotas_condominio: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub rentabilidad_neta: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub tasa_ocupacion: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub morosidad_pct: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub itbis_total: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub valor_catastral: Option<f64>,
    #[serde(default)]
    pub rentabilidad_unreliable: bool,
}

/// Response for the comparative dashboard endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DashboardComparativoResponse {
    pub propiedades: Vec<PropertyComparison>,
    pub moneda_display: String,
}
