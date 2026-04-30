use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrearInvitacionRequest {
    pub email: String,
    pub rol: String, // "gerente" | "visualizador"
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitacionResponse {
    pub id: Uuid,
    pub email: String,
    pub rol: String,
    pub token: String,
    pub usado: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn invitacion_response_serializes_to_camel_case() {
        let resp = InvitacionResponse {
            id: Uuid::nil(),
            email: "test@example.com".to_string(),
            rol: "gerente".to_string(),
            token: "abc-123".to_string(),
            usado: false,
            expires_at: DateTime::from_timestamp(0, 0).unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("expiresAt").is_some());
        assert!(json.get("expires_at").is_none());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("created_at").is_none());
        assert_eq!(json["email"], "test@example.com");
        assert_eq!(json["rol"], "gerente");
        assert_eq!(json["usado"], false);
    }

    #[test]
    fn crear_request_deserializes_camel_case() {
        let json = r#"{"email": "invite@example.com", "rol": "visualizador"}"#;
        let req: CrearInvitacionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "invite@example.com");
        assert_eq!(req.rol, "visualizador");
    }

    #[test]
    fn crear_request_deserializes_gerente_role() {
        let json = r#"{"email": "manager@example.com", "rol": "gerente"}"#;
        let req: CrearInvitacionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "manager@example.com");
        assert_eq!(req.rol, "gerente");
    }
}
