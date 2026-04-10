use actix_web::{HttpResponse, web};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::unidad::{CreateUnidadRequest, UnidadListQuery, UpdateUnidadRequest};
use crate::services::auth::Claims;
use crate::services::unidades;

#[derive(Debug, Deserialize)]
pub struct UnidadPath {
    pub propiedad_id: Uuid,
    pub id: Uuid,
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
    query: web::Query<UnidadListQuery>,
) -> Result<HttpResponse, AppError> {
    let propiedad_id = path.into_inner();
    let result = unidades::list(
        db.get_ref(),
        propiedad_id,
        claims.organizacion_id,
        query.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<UnidadPath>,
) -> Result<HttpResponse, AppError> {
    let p = path.into_inner();
    let result =
        unidades::get_by_id(db.get_ref(), p.propiedad_id, claims.organizacion_id, p.id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<CreateUnidadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let propiedad_id = path.into_inner();
    let txn = db.begin().await?;
    let result =
        unidades::create(&txn, propiedad_id, org_id, body.into_inner(), usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<UnidadPath>,
    body: web::Json<UpdateUnidadRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let p = path.into_inner();
    let txn = db.begin().await?;
    let result = unidades::update(
        &txn,
        p.propiedad_id,
        org_id,
        p.id,
        body.into_inner(),
        usuario_id,
    )
    .await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<UnidadPath>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = admin.0.sub;
    let org_id = admin.0.organizacion_id;
    let p = path.into_inner();
    let txn = db.begin().await?;
    unidades::delete(&txn, p.propiedad_id, org_id, p.id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
