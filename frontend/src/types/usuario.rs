use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub activo: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub nombre: String,
    pub email: String,
    pub password: String,
    // Organization type discriminator
    pub tipo: String,
    // persona_fisica fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cedula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telefono: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nombre_organizacion: Option<String>,
    // persona_juridica fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rnc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub razon_social: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nombre_comercial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direccion_fiscal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representante_legal: Option<String>,
}
