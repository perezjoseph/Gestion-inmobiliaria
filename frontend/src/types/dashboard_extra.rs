#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcupacionMensual {
    pub mes: u32,
    pub anio: i32,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub tasa: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IngresoComparacion {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub esperado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub cobrado: f64,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub diferencia: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PagoProximo {
    pub pago_id: String,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContratoCalendario {
    pub contrato_id: String,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub fecha_fin: String,
    pub dias_restantes: i64,
    pub color: String,
}
