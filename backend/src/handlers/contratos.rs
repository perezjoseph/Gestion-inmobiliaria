use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::contrato::{CreateContratoRequest, UpdateContratoRequest};
use crate::services::auth::Claims;
use crate::services::contratos;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = contratos::list(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = contratos::get_by_id(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let result = contratos::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = contratos::update(db.get_ref(), id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    contratos::delete(db.get_ref(), id).await?;
    Ok(HttpResponse::NoContent().finish())
}
