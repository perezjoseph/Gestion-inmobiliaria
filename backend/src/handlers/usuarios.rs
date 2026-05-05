use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::usuario::CambiarRolRequest;
use crate::services::usuarios;

const VALID_ROLES: &[&str] = &["admin", "gerente", "visualizador"];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsuarioListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    query: web::Query<UsuarioListQuery>,
) -> Result<HttpResponse, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let result = usuarios::listar(db.get_ref(), admin.0.organizacion_id, page, per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn cambiar_rol(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
    body: web::Json<CambiarRolRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let dto = body.into_inner();

    if !VALID_ROLES.contains(&dto.nuevo_rol.as_str()) {
        return Err(AppError::Validation(format!(
            "Rol inválido. Valores permitidos: {}",
            VALID_ROLES.join(", ")
        )));
    }

    let result =
        usuarios::cambiar_rol(db.get_ref(), id, admin.0.organizacion_id, &dto.nuevo_rol).await?;
    tracing::info!(admin_id = %admin.0.sub, target_user_id = %id, new_role = %dto.nuevo_rol, "Role changed");
    Ok(HttpResponse::Ok().json(result))
}

pub async fn activar(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = usuarios::activar(db.get_ref(), id, admin.0.organizacion_id).await?;
    tracing::info!(admin_id = %admin.0.sub, target_user_id = %id, action = "activar", "User activated");
    Ok(HttpResponse::Ok().json(result))
}

pub async fn desactivar(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = usuarios::desactivar(db.get_ref(), id, admin.0.organizacion_id).await?;
    tracing::info!(admin_id = %admin.0.sub, target_user_id = %id, action = "desactivar", "User deactivated");
    Ok(HttpResponse::Ok().json(result))
}
