use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrResult {
    pub document_type: String,
    pub lines: Vec<OcrLine>,
    pub structured_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub confidence: f64,
    pub bbox: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub preview_id: Uuid,
    pub document_type: String,
    pub fields: Vec<PreviewField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewField {
    pub name: String,
    pub value: String,
    pub confidence: f64,
    pub label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPreviewRequest {
    pub preview_id: Uuid,
    pub corrections: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResponse {
    pub document_type: String,
    pub fields: Vec<ExtractField>,
    pub raw_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractField {
    pub name: String,
    pub value: String,
    pub label: String,
    pub confidence: f64,
}
