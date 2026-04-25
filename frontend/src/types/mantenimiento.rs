use serde::{Deserialize, Serialize};

use crate::types::deserialize_option_f64_from_any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Solicitud {
    pub id: String,
    pub propiedad_id: String,
    pub unidad_id: Option<String>,
    pub inquilino_id: Option<String>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub estado: String,
    pub prioridad: String,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_f64_from_any")]
    pub costo_monto: Option<f64>,
    pub costo_moneda: Option<String>,
    pub fecha_inicio: Option<String>,
    pub fecha_fin: Option<String>,
    pub notas: Option<Vec<Nota>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Nota {
    pub id: String,
    pub solicitud_id: String,
    pub autor_id: String,
    pub contenido: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSolicitud {
    pub propiedad_id: String,
    pub unidad_id: Option<String>,
    pub inquilino_id: Option<String>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_monto: Option<f64>,
    pub costo_moneda: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSolicitud {
    pub titulo: Option<String>,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_monto: Option<f64>,
    pub costo_moneda: Option<String>,
    pub unidad_id: Option<String>,
    pub inquilino_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CambiarEstado {
    pub estado: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNota {
    pub contenido: String,
}
