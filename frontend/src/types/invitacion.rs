use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Invitacion {
    pub id: String,
    pub email: String,
    pub rol: String,
    pub token: String,
    pub usado: bool,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CrearInvitacion {
    pub email: String,
    pub rol: String,
}
