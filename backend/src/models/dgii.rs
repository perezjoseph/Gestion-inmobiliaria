use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DgiiConsultaResponse {
    pub cedula_rnc: String,
    pub nombre_razon_social: String,
    pub nombre_comercial: Option<String>,
    pub estado: String,
    pub regimen_de_pagos: Option<String>,
    pub actividad_economica: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DgiiNombreResponse {
    pub resultados: Vec<DgiiNombreItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DgiiNombreItem {
    pub cedula_rnc: String,
    pub nombre_razon_social: String,
    pub nombre_comercial: Option<String>,
    pub estado: String,
}

#[derive(Debug, Deserialize)]
pub struct ConsultaRncQuery {
    pub rnc: String,
}

#[derive(Debug, Deserialize)]
pub struct ConsultaNombreQuery {
    pub buscar: String,
}
