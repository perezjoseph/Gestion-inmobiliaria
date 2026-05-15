use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::perfil;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CambiarPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn cambiar_password(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
    body: web::Json<CambiarPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();

    // Only allow users to change their own password
    if claims.sub != id {
        return Err(AppError::Forbidden);
    }

    let input = body.into_inner();
    perfil::cambiar_password(
        db.get_ref(),
        id,
        &input.current_password,
        &input.new_password,
    )
    .await?;

    tracing::info!(user_id = %id, "Password changed via usuarios endpoint");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Contraseña actualizada exitosamente"
    })))
}
