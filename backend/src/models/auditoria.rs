use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditoriaQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub fecha_desde: Option<NaiveDate>,
    pub fecha_hasta: Option<NaiveDate>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditoriaResponse {
    pub id: Uuid,
    pub usuario_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub accion: String,
    pub cambios: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn auditoria_query_deserializes_all_fields() {
        let id = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let raw = format!(
            r#"{{
                "entityType": "propiedad",
                "entityId": "{id}",
                "usuarioId": "{uid}",
                "fechaDesde": "2025-01-01",
                "fechaHasta": "2025-12-31",
                "page": 2,
                "perPage": 50
            }}"#
        );
        let q: AuditoriaQuery = serde_json::from_str(&raw).unwrap();
        assert_eq!(q.entity_type.as_deref(), Some("propiedad"));
        assert_eq!(q.entity_id, Some(id));
        assert_eq!(q.usuario_id, Some(uid));
        assert_eq!(
            q.fecha_desde,
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        );
        assert_eq!(
            q.fecha_hasta,
            Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap())
        );
        assert_eq!(q.page, Some(2));
        assert_eq!(q.per_page, Some(50));
    }

    #[test]
    fn auditoria_query_deserializes_empty_filters() {
        let raw = r"{}";
        let q: AuditoriaQuery = serde_json::from_str(raw).unwrap();
        assert!(q.entity_type.is_none());
        assert!(q.entity_id.is_none());
        assert!(q.usuario_id.is_none());
        assert!(q.fecha_desde.is_none());
        assert!(q.fecha_hasta.is_none());
        assert!(q.page.is_none());
        assert!(q.per_page.is_none());
    }

    #[test]
    fn auditoria_query_deserializes_partial_filters() {
        let raw = r#"{"entityType": "contrato", "page": 1}"#;
        let q: AuditoriaQuery = serde_json::from_str(raw).unwrap();
        assert_eq!(q.entity_type.as_deref(), Some("contrato"));
        assert!(q.entity_id.is_none());
        assert_eq!(q.page, Some(1));
    }

    #[test]
    fn auditoria_response_serializes_to_camel_case() {
        let id = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let eid = Uuid::new_v4();
        let now = Utc::now();
        let resp = AuditoriaResponse {
            id,
            usuario_id: uid,
            entity_type: "pago".to_string(),
            entity_id: eid,
            accion: "crear".to_string(),
            cambios: json!({"monto": 5000}),
            created_at: now,
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["usuarioId"], uid.to_string());
        assert_eq!(serialized["entityType"], "pago");
        assert_eq!(serialized["entityId"], eid.to_string());
        assert_eq!(serialized["accion"], "crear");
        assert_eq!(serialized["cambios"]["monto"], 5000);
        assert!(serialized.get("createdAt").is_some());
        assert!(serialized.get("created_at").is_none());
    }

    #[test]
    fn auditoria_response_serializes_complex_cambios() {
        let resp = AuditoriaResponse {
            id: Uuid::new_v4(),
            usuario_id: Uuid::new_v4(),
            entity_type: "propiedad".to_string(),
            entity_id: Uuid::new_v4(),
            accion: "actualizar".to_string(),
            cambios: json!({
                "antes": {"precio": 1000},
                "despues": {"precio": 1500}
            }),
            created_at: Utc::now(),
        };
        let serialized = serde_json::to_value(&resp).unwrap();
        assert_eq!(serialized["cambios"]["antes"]["precio"], 1000);
        assert_eq!(serialized["cambios"]["despues"]["precio"], 1500);
    }
}
