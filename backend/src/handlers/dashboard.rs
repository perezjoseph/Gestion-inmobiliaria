use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::models::dashboard::{OcupacionTendenciaQuery, PagosProximosQuery};
use crate::services::auth::Claims;
use crate::services::dashboard;

pub async fn stats(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = dashboard::get_stats(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn ocupacion_tendencia(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<OcupacionTendenciaQuery>,
) -> Result<HttpResponse, AppError> {
    let meses = query.meses.unwrap_or(12);
    let result = dashboard::ocupacion_tendencia(db.get_ref(), meses).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn ingresos_comparacion(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = dashboard::ingreso_comparacion(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn pagos_proximos(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PagosProximosQuery>,
) -> Result<HttpResponse, AppError> {
    let dias = query.dias.unwrap_or(30);
    let result = dashboard::pagos_proximos(db.get_ref(), dias).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn contratos_calendario(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = dashboard::contratos_calendario(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn gastos_comparacion(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = dashboard::gastos_comparacion(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}
