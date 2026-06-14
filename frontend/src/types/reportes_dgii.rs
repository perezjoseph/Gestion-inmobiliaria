use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeriodoRequest {
    pub periodo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistroPreview {
    pub campos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistroExcluido {
    pub razon: String,
    pub referencia: String,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoRequest {
    pub estado: String,
}

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
