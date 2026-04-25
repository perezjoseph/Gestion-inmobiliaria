use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Pago {
    pub id: String,
    pub contrato_id: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub fecha_pago: Option<String>,
    pub fecha_vencimiento: String,
    pub metodo_pago: Option<String>,
    pub estado: String,
    pub notas: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePago {
    pub contrato_id: String,
    pub monto: f64,
    pub moneda: Option<String>,
    pub fecha_pago: Option<String>,
    pub fecha_vencimiento: String,
    pub metodo_pago: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePago {
    pub monto: Option<f64>,
    pub fecha_pago: Option<String>,
    pub metodo_pago: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}
