use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoVencido {
    pub pago_id: Uuid,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub inquilino_apellido: String,
    pub monto: Decimal,
    pub moneda: String,
    pub dias_vencido: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificacionResponse {
    pub id: Uuid,
    pub tipo: String,
    pub titulo: String,
    pub mensaje: String,
    pub leida: bool,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub usuario_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificacionListQuery {
    pub leida: Option<bool>,
    pub tipo: Option<String>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConteoNoLeidasResponse {
    pub count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarcarTodasResponse {
    pub actualizadas: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerarNotificacionesResponse {
    pub pago_vencido: u64,
    pub contrato_por_vencer: u64,
    pub documento_vencido: u64,
    pub total: u64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn pago_vencido_serializes_to_camel_case() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Apartamento Centro".into(),
            inquilino_nombre: "Juan".into(),
            inquilino_apellido: "Perez".into(),
            monto: Decimal::new(25000, 0),
            moneda: "DOP".into(),
            dias_vencido: 15,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert!(json.get("pagoId").is_some());
        assert!(json.get("propiedadTitulo").is_some());
        assert!(json.get("inquilinoNombre").is_some());
        assert!(json.get("inquilinoApellido").is_some());
        assert!(json.get("diasVencido").is_some());
        assert_eq!(json["diasVencido"], 15);
    }

    #[test]
    fn pago_vencido_serializes_zero_days_overdue() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Local Comercial".into(),
            inquilino_nombre: "Maria".into(),
            inquilino_apellido: "Lopez".into(),
            monto: Decimal::new(5000, 2),
            moneda: "USD".into(),
            dias_vencido: 0,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert_eq!(json["diasVencido"], 0);
        assert_eq!(json["monto"], "50.00");
        assert_eq!(json["moneda"], "USD");
    }

    #[test]
    fn pago_vencido_serializes_large_days_overdue() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Casa Playa".into(),
            inquilino_nombre: "Carlos".into(),
            inquilino_apellido: "Ramirez".into(),
            monto: Decimal::new(150_000, 0),
            moneda: "DOP".into(),
            dias_vencido: 365,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert_eq!(json["diasVencido"], 365);
        assert_eq!(json["propiedadTitulo"], "Casa Playa");
        assert_eq!(json["inquilinoNombre"], "Carlos");
        assert_eq!(json["inquilinoApellido"], "Ramirez");
    }

    #[test]
    fn notificacion_response_serializes_to_camel_case() {
        let id = Uuid::new_v4();
        let entity_id = Uuid::new_v4();
        let usuario_id = Uuid::new_v4();
        let now = Utc::now();

        let resp = NotificacionResponse {
            id,
            tipo: "pago_vencido".into(),
            titulo: "Pago vencido - Apartamento Centro".into(),
            mensaje: "El pago de 25000 DOP tiene 15 días de vencido".into(),
            leida: false,
            entity_type: "pago".into(),
            entity_id,
            usuario_id,
            created_at: now,
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["id"], id.to_string());
        assert_eq!(json["tipo"], "pago_vencido");
        assert_eq!(json["titulo"], "Pago vencido - Apartamento Centro");
        assert_eq!(json["mensaje"], "El pago de 25000 DOP tiene 15 días de vencido");
        assert_eq!(json["leida"], false);
        assert!(json.get("entityType").is_some());
        assert_eq!(json["entityType"], "pago");
        assert!(json.get("entityId").is_some());
        assert_eq!(json["entityId"], entity_id.to_string());
        assert!(json.get("usuarioId").is_some());
        assert_eq!(json["usuarioId"], usuario_id.to_string());
        assert!(json.get("createdAt").is_some());
        // Verify snake_case keys are absent
        assert!(json.get("entity_type").is_none());
        assert!(json.get("entity_id").is_none());
        assert!(json.get("usuario_id").is_none());
        assert!(json.get("created_at").is_none());
    }

    #[test]
    fn notificacion_list_query_deserializes_all_fields() {
        let json = r#"{"leida": false, "tipo": "pago_vencido", "page": 2, "perPage": 10}"#;
        let query: NotificacionListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.leida, Some(false));
        assert_eq!(query.tipo.as_deref(), Some("pago_vencido"));
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(10));
    }

    #[test]
    fn notificacion_list_query_deserializes_empty_object() {
        let json = "{}";
        let query: NotificacionListQuery = serde_json::from_str(json).unwrap();
        assert!(query.leida.is_none());
        assert!(query.tipo.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn notificacion_list_query_deserializes_partial_fields() {
        let json = r#"{"leida": true}"#;
        let query: NotificacionListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.leida, Some(true));
        assert!(query.tipo.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn conteo_no_leidas_response_serializes_to_camel_case() {
        let resp = ConteoNoLeidasResponse { count: 42 };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["count"], 42);
    }

    #[test]
    fn marcar_todas_response_serializes_to_camel_case() {
        let resp = MarcarTodasResponse { actualizadas: 7 };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["actualizadas"], 7);
        assert!(json.get("actualizadas").is_some());
    }

    #[test]
    fn generar_notificaciones_response_serializes_to_camel_case() {
        let resp = GenerarNotificacionesResponse {
            pago_vencido: 3,
            contrato_por_vencer: 2,
            documento_vencido: 1,
            total: 6,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("pagoVencido").is_some());
        assert_eq!(json["pagoVencido"], 3);
        assert!(json.get("contratoPorVencer").is_some());
        assert_eq!(json["contratoPorVencer"], 2);
        assert!(json.get("documentoVencido").is_some());
        assert_eq!(json["documentoVencido"], 1);
        assert_eq!(json["total"], 6);
        // Verify snake_case keys are absent
        assert!(json.get("pago_vencido").is_none());
        assert!(json.get("contrato_por_vencer").is_none());
        assert!(json.get("documento_vencido").is_none());
    }
}
