use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizacionResponse {
    pub id: Uuid,
    pub tipo: String,
    pub nombre: String,
    pub estado: String,
    pub cedula: Option<String>,
    pub telefono: Option<String>,
    pub email_organizacion: Option<String>,
    pub rnc: Option<String>,
    pub razon_social: Option<String>,
    pub nombre_comercial: Option<String>,
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOrganizacionRequest {
    pub nombre: Option<String>,
    pub telefono: Option<String>,
    pub email_organizacion: Option<String>,
    pub nombre_comercial: Option<String>,
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    pub dgii_data: Option<serde_json::Value>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn organizacion_response_serializes_to_camel_case() {
        let resp = OrganizacionResponse {
            id: Uuid::nil(),
            tipo: "persona_fisica".to_string(),
            nombre: "Test Org".to_string(),
            estado: "activo".to_string(),
            cedula: Some("00112345678".to_string()),
            telefono: Some("809-555-1234".to_string()),
            email_organizacion: Some("org@example.com".to_string()),
            rnc: None,
            razon_social: None,
            nombre_comercial: None,
            direccion_fiscal: None,
            representante_legal: None,
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("createdAt").is_some());
        assert!(json.get("created_at").is_none());
        assert!(json.get("emailOrganizacion").is_some());
        assert!(json.get("email_organizacion").is_none());
        assert_eq!(json["tipo"], "persona_fisica");
        assert_eq!(json["estado"], "activo");
    }

    #[test]
    fn update_request_deserializes_camel_case() {
        let json = r#"{"nombre": "Nuevo Nombre", "telefono": "809-555-0000"}"#;
        let req: UpdateOrganizacionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.nombre.as_deref(), Some("Nuevo Nombre"));
        assert_eq!(req.telefono.as_deref(), Some("809-555-0000"));
        assert!(req.email_organizacion.is_none());
        assert!(req.dgii_data.is_none());
    }

    #[test]
    fn update_request_deserializes_empty_object() {
        let json = r"{}";
        let req: UpdateOrganizacionRequest = serde_json::from_str(json).unwrap();
        assert!(req.nombre.is_none());
        assert!(req.telefono.is_none());
        assert!(req.email_organizacion.is_none());
        assert!(req.nombre_comercial.is_none());
        assert!(req.direccion_fiscal.is_none());
        assert!(req.representante_legal.is_none());
        assert!(req.dgii_data.is_none());
    }

    #[test]
    fn update_request_deserializes_dgii_data() {
        let json = r#"{"dgiiData": {"status": "active", "name": "Test Corp"}}"#;
        let req: UpdateOrganizacionRequest = serde_json::from_str(json).unwrap();
        assert!(req.dgii_data.is_some());
        let data = req.dgii_data.unwrap();
        assert_eq!(data["status"], "active");
    }

    #[test]
    fn update_request_does_not_include_immutable_fields() {
        // tipo, cedula, rnc should NOT be in UpdateOrganizacionRequest
        let json = r#"{"tipo": "persona_juridica", "cedula": "12345678901", "rnc": "123456789"}"#;
        let req: UpdateOrganizacionRequest = serde_json::from_str(json).unwrap();
        // These fields are silently ignored since they don't exist on the struct
        assert!(req.nombre.is_none());
    }
}
