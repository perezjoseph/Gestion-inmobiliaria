use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::usuarios;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/v1/usuarios/{id}/cambiar-password
///
/// Lets an authenticated user change their own password.
/// Verifies the path `{id}` matches the authenticated user's ID (from JWT claims).
/// Verifies the current password before updating.
pub async fn cambiar_password(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
    body: web::Json<CambiarPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();

    // Authorization: user can only change their own password
    if target_id != claims.sub {
        return Err(AppError::Forbidden);
    }

    let input = body.into_inner();

    // Input validation: prevent DoS via Argon2 on huge inputs
    if input.current_password.len() > 128 {
        return Err(AppError::Validation(
            "La contraseña actual es incorrecta".to_string(),
        ));
    }
    if input.new_password.len() < 8 {
        return Err(AppError::Validation(
            "La contraseña debe tener al menos 8 caracteres".to_string(),
        ));
    }
    if input.new_password.len() > 128 {
        return Err(AppError::Validation(
            "La contraseña no puede exceder 128 caracteres".to_string(),
        ));
    }

    usuarios::cambiar_password(
        db.get_ref(),
        claims.sub,
        &input.current_password,
        &input.new_password,
    )
    .await?;

    tracing::info!(user_id = %claims.sub, "Password changed via /usuarios/{id}/cambiar-password");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Contraseña actualizada exitosamente"
    })))
}
