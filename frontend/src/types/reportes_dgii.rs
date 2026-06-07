use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

/// Request body for generating a 606 or 607 report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeriodoRequest {
    pub periodo: String,
}

/// A preview row where each field is rendered as a string for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistroPreview {
    pub campos: Vec<String>,
}

/// A record excluded from report generation due to incomplete fiscal data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistroExcluido {
    pub razon: String,
    pub referencia: String,
}

/// The generated report output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReporteGenerado {
    pub contenido: String,
    pub preview: Vec<RegistroPreview>,
    pub excluidos: Vec<RegistroExcluido>,
    pub cantidad_registros: u32,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_total: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_total: f64,
}

/// Preview response from GET /preview/{tipo}/{periodo}.
/// The backend returns the `reporte_dgii` entity model directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReportePreviewResponse {
    pub id: String,
    pub tipo_reporte: String,
    pub periodo: String,
    pub estado: String,
    pub cantidad_registros: i32,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto_total: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_total: f64,
    #[serde(default)]
    pub contenido: Option<String>,
    #[serde(default)]
    pub registros_excluidos: Option<Vec<RegistroExcluido>>,
}

/// Request body for updating report status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoRequest {
    pub estado: String,
}

/// Result of ITBIS neto calculation (607 ITBIS - 606 ITBIS).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct ItbisNetoResult {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_cobrado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_pagado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub itbis_neto: f64,
}
