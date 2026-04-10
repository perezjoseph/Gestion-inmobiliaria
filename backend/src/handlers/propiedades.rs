use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::propiedad::{
    CreatePropiedadRequest, PropiedadListQuery, UpdatePropiedadRequest,
};
use crate::services::auth::Claims;
use crate::services::propiedades;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PropiedadListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::list(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = propiedades::get_by_id(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = propiedades::update(db.get_ref(), id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    propiedades::delete(db.get_ref(), id).await?;
    Ok(HttpResponse::NoContent().finish())
}
