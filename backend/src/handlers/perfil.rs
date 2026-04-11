use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::perfil;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualizarPerfilRequest {
    pub nombre: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarPasswordRequest {
    pub password_actual: String,
    pub password_nuevo: String,
}

pub async fn obtener(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = perfil::obtener_perfil(db.get_ref(), claims.sub).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn actualizar(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    body: web::Json<ActualizarPerfilRequest>,
) -> Result<HttpResponse, AppError> {
    let input = body.into_inner();
    let result =
        perfil::actualizar_perfil(db.get_ref(), claims.sub, input.nombre, input.email).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn cambiar_password(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    body: web::Json<CambiarPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let input = body.into_inner();
    perfil::cambiar_password(
        db.get_ref(),
        claims.sub,
        &input.password_actual,
        &input.password_nuevo,
    )
    .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Contraseña actualizada exitosamente"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actualizar_perfil_request_deserializes_camel_case() {
        let json = r#"{"nombre": "Juan", "email": "juan@example.com"}"#;
        let req: ActualizarPerfilRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.nombre.as_deref(), Some("Juan"));
        assert_eq!(req.email.as_deref(), Some("juan@example.com"));
    }

    #[test]
    fn actualizar_perfil_request_allows_partial_fields() {
        let json = r#"{"nombre": "Juan"}"#;
        let req: ActualizarPerfilRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.nombre.as_deref(), Some("Juan"));
        assert!(req.email.is_none());
    }

    #[test]
    fn actualizar_perfil_request_allows_empty_object() {
        let json = r#"{}"#;
        let req: ActualizarPerfilRequest = serde_json::from_str(json).unwrap();
        assert!(req.nombre.is_none());
        assert!(req.email.is_none());
    }

    #[test]
    fn cambiar_password_request_deserializes_camel_case() {
        let json = r#"{"passwordActual": "old123", "passwordNuevo": "new456"}"#;
        let req: CambiarPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.password_actual, "old123");
        assert_eq!(req.password_nuevo, "new456");
    }

    #[test]
    fn cambiar_password_request_rejects_missing_fields() {
        let json = r#"{"passwordActual": "old123"}"#;
        let result = serde_json::from_str::<CambiarPasswordRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn cambiar_password_request_rejects_empty_object() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<CambiarPasswordRequest>(json);
        assert!(result.is_err());
    }
}
