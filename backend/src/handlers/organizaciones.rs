use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::organizacion::UpdateOrganizacionRequest;
use crate::services::auth::Claims;
use crate::services::organizaciones;

pub async fn get(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = organizaciones::get_by_id(db.get_ref(), claims.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<UpdateOrganizacionRequest>,
) -> Result<HttpResponse, AppError> {
    let result =
        organizaciones::update(db.get_ref(), admin.0.organizacion_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
