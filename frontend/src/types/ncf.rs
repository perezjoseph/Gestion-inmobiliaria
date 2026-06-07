use serde::{Deserialize, Serialize};

use super::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecuenciaNcf {
    pub id: String,
    pub tipo_ncf: String,
    pub prefijo: String,
    pub siguiente_numero: i32,
    pub rango_desde: i32,
    pub rango_hasta: i32,
    pub is_active: bool,
    pub is_ecf: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurarRango {
    pub tipo_ncf: String,
    pub prefijo: String,
    pub rango_desde: i32,
    pub rango_hasta: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlertaRango {
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub consumo_porcentaje: f64,
    pub tipo_ncf: String,
    pub restantes: i32,
}
