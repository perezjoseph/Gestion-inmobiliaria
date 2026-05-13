use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::desahucio::{
    CreateDesahucioRequest, DesahucioListQuery, UpdateDesahucioRequest,
};
use crate::services::desahucios;

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateDesahucioRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;

    let result = desahucios::create(db.get_ref(), body.into_inner(), usuario_id, org_id).await?;

    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateDesahucioRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();

    let result =
        desahucios::update(db.get_ref(), org_id, id, body.into_inner(), usuario_id).await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    query: web::Query<DesahucioListQuery>,
) -> Result<HttpResponse, AppError> {
    let org_id = access.0.organizacion_id;

    let result = desahucios::list(db.get_ref(), org_id, query.into_inner()).await?;

    Ok(HttpResponse::Ok().json(result))
}
