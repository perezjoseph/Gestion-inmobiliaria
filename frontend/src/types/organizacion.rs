use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Organizacion {
    pub id: String,
    pub tipo: String,
    pub nombre: String,
    pub estado: String,
    pub cedula: Option<String>,
    pub telefono: Option<String>,
    pub email_organizacion: Option<String>,
    pub rnc: Option<String>,
    pub razon_social: Option<String>,
    pub nombre_comercial: Option<String>,
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrganizacion {
    pub nombre: Option<String>,
    pub telefono: Option<String>,
    pub email_organizacion: Option<String>,
    pub nombre_comercial: Option<String>,
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    pub dgii_data: Option<String>,
}
