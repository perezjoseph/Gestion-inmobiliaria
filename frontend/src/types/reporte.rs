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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadReportRow {
    pub propiedad_id: String,
    pub propiedad_titulo: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_ingresos: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_gastos: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub ingreso_neto: f64,
    pub moneda: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadReportSummary {
    pub rows: Vec<RentabilidadReportRow>,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_ingresos: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_gastos: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub total_neto: f64,
    pub mes: u32,
    pub anio: i32,
    pub generated_at: String,
    pub generated_by: String,
}
