use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::notificacion::{
    ConteoNoLeidasResponse, MarcarTodasResponse, NotificacionListQuery,
};
use crate::services::auth::Claims;
use crate::services::notificaciones;

pub async fn pagos_vencidos(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let results =
        notificaciones::listar_pagos_vencidos(db.get_ref(), claims.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(results))
}

pub async fn listar(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    query: web::Query<NotificacionListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = notificaciones::listar(db.get_ref(), claims.sub, query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn conteo_no_leidas(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let count = notificaciones::conteo_no_leidas(db.get_ref(), claims.sub).await?;
    Ok(HttpResponse::Ok().json(ConteoNoLeidasResponse { count }))
}

pub async fn marcar_leida(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let result = notificaciones::marcar_leida(db.get_ref(), id, claims.sub).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn marcar_todas_leidas(
    db: web::Data<DatabaseConnection>,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    let actualizadas = notificaciones::marcar_todas_leidas(db.get_ref(), claims.sub).await?;
    Ok(HttpResponse::Ok().json(MarcarTodasResponse { actualizadas }))
}

pub async fn generar(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let result =
        notificaciones::generar_notificaciones(db.get_ref(), access.0.organizacion_id).await?;
    Ok(HttpResponse::Ok().json(result))
}
