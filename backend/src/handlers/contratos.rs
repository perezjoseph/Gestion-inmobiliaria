use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::contrato::{
    ContratoListQuery, CreateContratoRequest, PorVencerQuery, RenovarContratoRequest,
    TerminarContratoRequest, UpdateContratoRequest,
};
use crate::services::auth::Claims;
use crate::services::contratos;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<ContratoListQuery>,
) -> Result<HttpResponse, AppError> {
    let q = query.into_inner();
    let result = contratos::list(db.get_ref(), q.page, q.per_page).await?;
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
    access: WriteAccess,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let result = contratos::create(db.get_ref(), body.into_inner(), usuario_id, access.0.organizacion_id).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::update(db.get_ref(), id, body.into_inner(), usuario_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let id = path.into_inner();
    contratos::delete(db.get_ref(), id, usuario_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn renovar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<RenovarContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::renovar(db.get_ref(), id, body.into_inner(), usuario_id).await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn terminar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<TerminarContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let result = contratos::terminar(db.get_ref(), id, body.into_inner(), usuario_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn por_vencer(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PorVencerQuery>,
) -> Result<HttpResponse, AppError> {
    let dias = query.into_inner().dias;
    let result = contratos::listar_por_vencer(db.get_ref(), dias).await?;
    Ok(HttpResponse::Ok().json(result))
}
