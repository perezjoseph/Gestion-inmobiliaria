use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsabilidadEfectivaResponse {
    pub proveedor_servicio: String,
    pub responsable: String,
    pub es_override_contrato: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResponsabilidadRequest {
    pub responsabilidades: Vec<ResponsabilidadItem>,
    /// Required for contract-level updates to identify the unit.
    pub unidad_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsabilidadItem {
    pub proveedor_servicio: String,
    pub responsable: String,
}
