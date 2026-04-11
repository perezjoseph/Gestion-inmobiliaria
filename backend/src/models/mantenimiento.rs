use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSolicitudRequest {
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub inquilino_id: Option<Uuid>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_monto: Option<Decimal>,
    pub costo_moneda: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSolicitudRequest {
    pub titulo: Option<String>,
    pub descripcion: Option<String>,
    pub prioridad: Option<String>,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_monto: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub unidad_id: Option<Uuid>,
    pub inquilino_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarEstadoRequest {
    pub estado: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNotaRequest {
    pub contenido: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitudListQuery {
    pub estado: Option<String>,
    pub prioridad: Option<String>,
    pub propiedad_id: Option<Uuid>,
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolicitudResponse {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub unidad_id: Option<Uuid>,
    pub inquilino_id: Option<Uuid>,
    pub titulo: String,
    pub descripcion: Option<String>,
    pub estado: String,
    pub prioridad: String,
    pub nombre_proveedor: Option<String>,
    pub telefono_proveedor: Option<String>,
    pub email_proveedor: Option<String>,
    pub costo_monto: Option<Decimal>,
    pub costo_moneda: Option<String>,
    pub fecha_inicio: Option<DateTime<Utc>>,
    pub fecha_fin: Option<DateTime<Utc>>,
    pub notas: Option<Vec<NotaResponse>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotaResponse {
    pub id: Uuid,
    pub solicitud_id: Uuid,
    pub autor_id: Uuid,
    pub contenido: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_solicitud_request_deserializes_camel_case() {
        let json = serde_json::json!({
            "propiedadId": "550e8400-e29b-41d4-a716-446655440000",
            "unidadId": "660e8400-e29b-41d4-a716-446655440000",
            "inquilinoId": "770e8400-e29b-41d4-a716-446655440000",
            "titulo": "Fuga de agua en baño",
            "descripcion": "Se detectó una fuga en el baño principal",
            "prioridad": "alta",
            "nombreProveedor": "Plomería Express",
            "telefonoProveedor": "809-555-1234",
            "emailProveedor": "plomeria@example.com",
            "costoMonto": "1500.50",
            "costoMoneda": "DOP"
        });
        let req: CreateSolicitudRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.titulo, "Fuga de agua en baño");
        assert_eq!(req.prioridad.as_deref(), Some("alta"));
        assert_eq!(req.costo_monto, Some(Decimal::new(150050, 2)));
        assert_eq!(req.costo_moneda.as_deref(), Some("DOP"));
        assert!(req.unidad_id.is_some());
        assert!(req.inquilino_id.is_some());
    }

    #[test]
    fn solicitud_list_query_deserializes_optional_fields() {
        let json = serde_json::json!({});
        let query: SolicitudListQuery = serde_json::from_value(json).unwrap();
        assert!(query.estado.is_none());
        assert!(query.prioridad.is_none());
        assert!(query.propiedad_id.is_none());
        assert!(query.page.is_none());
        assert!(query.per_page.is_none());
    }

    #[test]
    fn solicitud_list_query_deserializes_with_filters() {
        let json = serde_json::json!({
            "estado": "pendiente",
            "prioridad": "alta",
            "page": 2,
            "perPage": 10
        });
        let query: SolicitudListQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.estado.as_deref(), Some("pendiente"));
        assert_eq!(query.prioridad.as_deref(), Some("alta"));
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(10));
    }

    #[test]
    fn solicitud_response_serializes_camel_case() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let propiedad_id = Uuid::new_v4();
        let response = SolicitudResponse {
            id,
            propiedad_id,
            unidad_id: None,
            inquilino_id: None,
            titulo: "Reparar techo".to_string(),
            descripcion: Some("Goteras en el techo".to_string()),
            estado: "pendiente".to_string(),
            prioridad: "media".to_string(),
            nombre_proveedor: None,
            telefono_proveedor: None,
            email_proveedor: None,
            costo_monto: None,
            costo_moneda: None,
            fecha_inicio: None,
            fecha_fin: None,
            notas: None,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json.get("propiedadId").is_some());
        assert!(json.get("unidadId").is_some());
        assert!(json.get("inquilinoId").is_some());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("fechaInicio").is_some());
        assert!(json.get("fechaFin").is_some());
        assert!(json.get("costoMonto").is_some());
        assert!(json.get("costoMoneda").is_some());
        assert!(json.get("nombreProveedor").is_some());
        assert_eq!(json["titulo"], "Reparar techo");
        assert_eq!(json["estado"], "pendiente");
        assert_eq!(json["prioridad"], "media");
    }
}
