use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::inquilino::InquilinoSearchQuery;
use crate::models::inquilino::{CreateInquilinoRequest, UpdateInquilinoRequest};
use crate::services::auth::Claims;
use crate::services::inquilinos;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<InquilinoSearchQuery>,
) -> Result<HttpResponse, AppError> {
    let result = inquilinos::list(db.get_ref(), query.into_inner().search).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = inquilinos::get_by_id(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreateInquilinoRequest>,
) -> Result<HttpResponse, AppError> {
    let result = inquilinos::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateInquilinoRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = inquilinos::update(db.get_ref(), id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    inquilinos::delete(db.get_ref(), id).await?;
    Ok(HttpResponse::NoContent().finish())
}
