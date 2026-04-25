use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditoriaEntry {
    pub id: String,
    pub usuario_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub accion: String,
    pub cambios: serde_json::Value,
    pub created_at: String,
}
