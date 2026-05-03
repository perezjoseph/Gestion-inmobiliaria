use actix_web::{HttpResponse, web};
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::gasto::{
    CreateGastoRequest, GastoListQuery, ResumenCategoriasQuery, UpdateGastoRequest,
};
use crate::services::auth::Claims;
use crate::services::gastos;

pub async fn create(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    body: web::Json<CreateGastoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let organizacion_id = access.0.organizacion_id;
    let txn = db.begin().await?;
    let result = gastos::create(&txn, body.into_inner(), usuario_id, organizacion_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Created().json(result))
}

pub async fn list(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<GastoListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = gastos::list(db.get_ref(), claims.organizacion_id, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn get_by_id(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = gastos::get_by_id(db.get_ref(), claims.organizacion_id, id).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn update(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<UpdateGastoRequest>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let txn = db.begin().await?;
    let result = gastos::update(&txn, org_id, id, body.into_inner(), usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn delete(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let usuario_id = access.0.sub;
    let org_id = access.0.organizacion_id;
    let id = path.into_inner();
    let txn = db.begin().await?;
    gastos::delete(&txn, org_id, id, usuario_id).await?;
    txn.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn resumen_categorias(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<ResumenCategoriasQuery>,
) -> Result<HttpResponse, AppError> {
    let result =
        gastos::resumen_categorias(db.get_ref(), claims.organizacion_id, query.into_inner())
            .await?;
    Ok(HttpResponse::Ok().json(result))
}
