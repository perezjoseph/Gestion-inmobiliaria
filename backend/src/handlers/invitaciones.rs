use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::invitacion::{CrearInvitacionRequest, InvitacionListQuery};
use crate::services::invitaciones;

pub async fn crear(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<CrearInvitacionRequest>,
) -> Result<HttpResponse, AppError> {
    let result =
        invitaciones::crear(db.get_ref(), admin.0.organizacion_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn listar(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    query: web::Query<InvitacionListQuery>,
) -> Result<HttpResponse, AppError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let result =
        invitaciones::listar(db.get_ref(), admin.0.organizacion_id, page, per_page).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn revocar(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    invitaciones::revocar(db.get_ref(), admin.0.organizacion_id, id).await?;
    Ok(HttpResponse::NoContent().finish())
}
