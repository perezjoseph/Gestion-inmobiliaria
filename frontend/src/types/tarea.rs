use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EjecucionTarea {
    pub id: String,
    pub nombre_tarea: String,
    pub iniciado_en: String,
    pub duracion_ms: i64,
    pub exitosa: bool,
    pub registros_afectados: i64,
    pub mensaje_error: Option<String>,
}
