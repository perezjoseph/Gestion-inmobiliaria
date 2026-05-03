use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::ejecucion_tarea;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EjecucionTareaResponse {
    pub id: Uuid,
    pub nombre_tarea: String,
    pub iniciado_en: DateTime<Utc>,
    pub duracion_ms: i64,
    pub exitosa: bool,
    pub registros_afectados: i64,
    pub mensaje_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorialQuery {
    pub nombre_tarea: Option<String>,
    pub exitosa: Option<bool>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EjecutarTareaResponse {
    pub ejecucion: EjecucionTareaResponse,
}

impl From<ejecucion_tarea::Model> for EjecucionTareaResponse {
    fn from(model: ejecucion_tarea::Model) -> Self {
        Self {
            id: model.id,
            nombre_tarea: model.nombre_tarea,
            iniciado_en: model.iniciado_en.with_timezone(&Utc),
            duracion_ms: model.duracion_ms,
            exitosa: model.exitosa,
            registros_afectados: model.registros_afectados,
            mensaje_error: model.mensaje_error,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn historial_query_with_all_filters() {
        let json = serde_json::json!({
            "nombreTarea": "marcar_pagos_atrasados",
            "exitosa": true,
            "page": 1,
            "perPage": 20
        });
        let query: HistorialQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.nombre_tarea.as_deref(), Some("marcar_pagos_atrasados"));
        assert_eq!(query.exitosa, Some(true));
        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(20));
    }

    #[test]
    fn historial_query_with_no_filters() {
        let json = serde_json::json!({});
        let query: HistorialQuery = serde_json::from_value(json).unwrap();
        assert!(query.nombre_tarea.is_none());
        assert!(query.exitosa.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn ejecucion_tarea_response_serializes_camel_case() {
        let response = EjecucionTareaResponse {
            id: Uuid::nil(),
            nombre_tarea: "marcar_pagos_atrasados".to_string(),
            iniciado_en: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            duracion_ms: 150,
            exitosa: true,
            registros_afectados: 5,
            mensaje_error: None,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("nombreTarea").is_some());
        assert!(json.get("iniciadoEn").is_some());
        assert!(json.get("duracionMs").is_some());
        assert!(json.get("registrosAfectados").is_some());
        assert!(json.get("mensajeError").is_some());
    }

    #[test]
    fn ejecutar_tarea_response_serializes_camel_case() {
        let response = EjecutarTareaResponse {
            ejecucion: EjecucionTareaResponse {
                id: Uuid::nil(),
                nombre_tarea: "marcar_pagos_atrasados".to_string(),
                iniciado_en: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
                duracion_ms: 100,
                exitosa: true,
                registros_afectados: 3,
                mensaje_error: None,
            },
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("ejecucion").is_some());
        let ejecucion = json.get("ejecucion").unwrap();
        assert!(ejecucion.get("nombreTarea").is_some());
    }

    #[test]
    fn from_entity_model_converts_correctly() {
        use chrono::FixedOffset;

        let model = ejecucion_tarea::Model {
            id: Uuid::nil(),
            nombre_tarea: "marcar_contratos_vencidos".to_string(),
            iniciado_en: DateTime::<FixedOffset>::from(
                DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            ),
            duracion_ms: 250,
            exitosa: false,
            registros_afectados: 0,
            mensaje_error: Some("db connection error".to_string()),
        };

        let response = EjecucionTareaResponse::from(model);
        assert_eq!(response.nombre_tarea, "marcar_contratos_vencidos");
        assert_eq!(response.duracion_ms, 250);
        assert!(!response.exitosa);
        assert_eq!(response.registros_afectados, 0);
        assert_eq!(response.mensaje_error.as_deref(), Some("db connection error"));
    }
}
