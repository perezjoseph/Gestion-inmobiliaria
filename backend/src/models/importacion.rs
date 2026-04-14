use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub total_filas: usize,
    pub exitosos: usize,
    pub fallidos: Vec<ImportError>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub fila: usize,
    pub error: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Csv,
    Xlsx,
    Image,
}
