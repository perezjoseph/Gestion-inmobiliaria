use serde::{Deserialize, Serialize};

use crate::types::deserialize_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Gasto {
    pub id: String,
    pub propiedad_id: String,
    pub unidad_id: Option<String>,
    pub categoria: String,
    pub descripcion: String,
    #[serde(deserialize_with = "deserialize_f64_from_any")]
    pub monto: f64,
    pub moneda: String,
    pub fecha_gasto: String,
    pub estado: String,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub notas: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateGasto {
    pub propiedad_id: String,
    pub unidad_id: Option<String>,
    pub categoria: String,
    pub descripcion: String,
    pub monto: f64,
    pub moneda: String,
    pub fecha_gasto: String,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGasto {
    pub categoria: Option<String>,
    pub descripcion: Option<String>,
    pub monto: Option<f64>,
    pub moneda: Option<String>,
    pub fecha_gasto: Option<String>,
    pub unidad_id: Option<String>,
    pub proveedor: Option<String>,
    pub numero_factura: Option<String>,
    pub estado: Option<String>,
    pub notas: Option<String>,
}
