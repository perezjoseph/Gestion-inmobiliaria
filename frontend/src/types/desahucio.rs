use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Desahucio {
    pub id: String,
    pub contrato_id: String,
    pub estado: String,
    pub fecha_inicio: String,
    pub fecha_resolucion: Option<String>,
    pub motivo: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesahucio {
    pub contrato_id: String,
    pub motivo: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDesahucio {
    pub estado: Option<String>,
    pub fecha_resolucion: Option<String>,
    pub motivo: Option<String>,
}
