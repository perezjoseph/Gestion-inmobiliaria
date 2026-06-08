use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DgiiConsulta {
    pub cedula_rnc: String,
    pub nombre_razon_social: String,
    pub nombre_comercial: Option<String>,
    pub estado: String,
    pub regimen_de_pagos: Option<String>,
    pub actividad_economica: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DgiiNombreResult {
    pub resultados: Vec<DgiiNombreItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DgiiNombreItem {
    pub cedula_rnc: String,
    pub nombre_razon_social: String,
    pub nombre_comercial: Option<String>,
    pub estado: String,
}
