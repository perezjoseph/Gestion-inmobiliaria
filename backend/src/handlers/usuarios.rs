use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::usuario::CambiarRolRequest;
use crate::services::usuarios;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsuarioListQuery {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    query: web::Query<UsuarioListQuery>,
) -> Result<HttpResponse, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let result = usuarios::listar(db.get_ref(), page, per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn cambiar_rol(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
    body: web::Json<CambiarRolRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = usuarios::cambiar_rol(db.get_ref(), id, &body.nuevo_rol).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn activar(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = usuarios::activar(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn desactivar(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = usuarios::desactivar(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}
