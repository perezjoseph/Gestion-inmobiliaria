use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Inquilino {
    pub id: String,
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateInquilino {
    pub nombre: String,
    pub apellido: String,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: String,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInquilino {
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub email: Option<String>,
    pub telefono: Option<String>,
    pub cedula: Option<String>,
    pub contacto_emergencia: Option<String>,
    pub notas: Option<String>,
}
