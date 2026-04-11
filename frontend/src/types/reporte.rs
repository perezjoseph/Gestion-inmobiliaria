use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IngresoReportRow {
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub estado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IngresoReportSummary {
    pub rows: Vec<IngresoReportRow>,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_pagado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_pendiente: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_atrasado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub tasa_ocupacion: f64,
    pub generated_at: String,
    pub generated_by: String,
}
