// Addition to backend/src/handlers/usuarios.rs
// New imports needed:
// use crate::services::auth::Claims;

use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::perfil;

/// Request body for the password change endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// POST /api/v1/usuarios/{id}/cambiar-password
///
/// Allows an authenticated user to change their own password.
/// Verifies that the path `{id}` matches the authenticated user's ID,
/// then delegates to the existing perfil service for password verification and update.
pub async fn cambiar_password(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
    body: web::Json<CambiarPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let target_id = path.into_inner();

    // Users can only change their own password
    if target_id != claims.sub {
        return Err(AppError::Forbidden);
    }

    let input = body.into_inner();

    // Reuse existing service logic: validates lengths, verifies current password, hashes new one
    perfil::cambiar_password(
        db.get_ref(),
        claims.sub,
        &input.current_password,
        &input.new_password,
    )
    .await?;

    tracing::info!(user_id = %claims.sub, "Password changed via usuarios endpoint");

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Contraseña actualizada exitosamente"
    })))
}
