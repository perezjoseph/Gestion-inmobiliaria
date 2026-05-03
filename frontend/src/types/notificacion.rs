use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PagoVencido {
    pub pago_id: String,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub inquilino_apellido: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub dias_vencido: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Notificacion {
    pub id: String,
    pub tipo: String,
    pub titulo: String,
    pub mensaje: String,
    pub leida: bool,
    pub entity_type: String,
    pub entity_id: String,
    pub usuario_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConteoNoLeidas {
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MarcarTodasResponse {
    pub actualizadas: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GenerarNotificacionesResponse {
    pub pago_vencido: u64,
    pub contrato_por_vencer: u64,
    pub documento_vencido: u64,
    pub total: u64,
}
