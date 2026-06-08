use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResponsabilidadEfectiva {
    pub proveedor_servicio: String,
    pub responsable: String,
    pub es_override_contrato: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResponsabilidad {
    pub responsabilidades: Vec<ResponsabilidadItem>,
    pub unidad_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResponsabilidadItem {
    pub proveedor_servicio: String,
    pub responsable: String,
}
