use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    // User fields
    pub nombre: String,
    pub email: String,
    pub password: String,
    // Organization type discriminator
    pub tipo: Option<String>, // "persona_fisica" | "persona_juridica"
    // persona_fisica fields
    pub cedula: Option<String>,
    pub telefono: Option<String>,
    pub nombre_organizacion: Option<String>,
    // persona_juridica fields
    pub rnc: Option<String>,
    pub razon_social: Option<String>,
    pub nombre_comercial: Option<String>,
    pub direccion_fiscal: Option<String>,
    pub representante_legal: Option<String>,
    // Invitation flow (mutually exclusive with tipo)
    pub token_invitacion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub activo: bool,
    pub organizacion_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsuarioResponse {
    pub id: Uuid,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub activo: bool,
    pub organizacion_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarRolRequest {
    pub nuevo_rol: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn cambiar_rol_request_deserializes_camel_case() {
        let json = r#"{"nuevoRol": "admin"}"#;
        let req: CambiarRolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.nuevo_rol, "admin");
    }

    #[test]
    fn cambiar_rol_request_rejects_missing_field() {
        let json = r"{}";
        let result = serde_json::from_str::<CambiarRolRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn usuario_response_serializes_to_camel_case() {
        let resp = UsuarioResponse {
            id: Uuid::nil(),
            nombre: "Juan".to_string(),
            email: "juan@example.com".to_string(),
            rol: "admin".to_string(),
            activo: true,
            organizacion_id: Uuid::nil(),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("createdAt").is_some());
        assert!(json.get("created_at").is_none());
        assert_eq!(json["nombre"], "Juan");
        assert_eq!(json["activo"], true);
    }
}
