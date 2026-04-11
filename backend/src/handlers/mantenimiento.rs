use actix_web::{HttpResponse, web};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::mantenimiento::{
    CambiarEstadoRequest, CreateNotaRequest, CreateSolicitudRequest, SolicitudListQuery,
    UpdateSolicitudRequest,
};
use crate::services::auth::Claims;
use crate::services::mantenimiento;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<SolicitudListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = mantenimiento::list(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = mantenimiento::get_by_id(db.get_ref(), id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateSolicitudRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let txn = db.begin().await?;
    let result = mantenimiento::create(&txn, body.into_inner(), usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateSolicitudRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let txn = db.begin().await?;
    let result = mantenimiento::update(&txn, id, body.into_inner(), usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn cambiar_estado(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CambiarEstadoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let id = path.into_inner();
    let txn = db.begin().await?;
    let result = mantenimiento::cambiar_estado(&txn, id, &body.estado, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let id = path.into_inner();
    let txn = db.begin().await?;
    mantenimiento::delete(&txn, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn agregar_nota(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CreateNotaRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let solicitud_id = path.into_inner();
    let txn = db.begin().await?;
    let result =
        mantenimiento::agregar_nota(&txn, solicitud_id, body.into_inner().contenido, usuario_id)
            .await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}
