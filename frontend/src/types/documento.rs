use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentoResponse {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub filename: String,
    pub file_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub uploaded_by: String,
    pub created_at: String,
    // Document management fields
    pub tipo_documento: Option<String>,
    pub estado_verificacion: Option<String>,
    pub fecha_vencimiento: Option<String>,
    pub verificado_por: Option<String>,
    pub notas_verificacion: Option<String>,
    pub numero_documento: Option<String>,
    pub contenido_editable: Option<serde_json::Value>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoResponse {
    pub entity_type: String,
    pub entity_id: String,
    pub documentos: Vec<CumplimientoItem>,
    pub porcentaje: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CumplimientoItem {
    pub tipo_documento: String,
    pub nombre: String,
    pub requerido: bool,
    pub estado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlantillaResponse {
    pub id: String,
    pub nombre: String,
    pub tipo_documento: String,
    pub entity_type: String,
    pub contenido: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DigitalizarResponse {
    pub document_type: String,
    pub contenido_editable: serde_json::Value,
    pub campos_baja_confianza: Vec<String>,
    pub documento_original_id: String,
}
