use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPagos {
    pub contrato_id: String,
    pub pagos: Vec<PagoPreviewItem>,
    pub total_pagos: u64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_total: f64,
    pub pagos_existentes: u64,
    pub pagos_nuevos: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PagoPreviewItem {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenerarPagosResponse {
    pub contrato_id: String,
    pub pagos_generados: u64,
}
