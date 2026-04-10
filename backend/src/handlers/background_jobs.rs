use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::background_jobs::{EjecutarTareaResponse, HistorialQuery};
use crate::services::background_jobs;

pub async fn ejecutar_tarea(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let nombre = path.into_inner();
    let ejecucion = background_jobs::ejecutar_tarea_por_nombre(db.get_ref(), &nombre).await?;
    Ok(HttpResponse::Ok().json(EjecutarTareaResponse { ejecucion }))
}

pub async fn historial(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    query: web::Query<HistorialQuery>,
) -> Result<HttpResponse, AppError> {
    let result = background_jobs::historial(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
