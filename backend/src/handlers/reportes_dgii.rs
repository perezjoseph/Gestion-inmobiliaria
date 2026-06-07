use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::services::fiscal::obtener_org_con_acceso_fiscal;
use crate::services::reportes_dgii;

#[derive(Deserialize)]
pub struct PeriodoRequest {
    pub periodo: String,
}

#[derive(Deserialize)]
pub struct EstadoRequest {
    pub estado: String,
}

/// POST /api/v1/reportes-dgii/607 — generate 607 report
pub async fn generar_607_handler(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<PeriodoRequest>,
) -> Result<HttpResponse, AppError> {
    obtener_org_con_acceso_fiscal(db.get_ref(), user.0.organizacion_id).await?;

    let reporte = reportes_dgii::generar_607(
        db.get_ref(),
        user.0.organizacion_id,
        &body.periodo,
        user.0.sub,
    )
    .await?;

    Ok(HttpResponse::Ok().json(reporte))
}

/// POST /api/v1/reportes-dgii/606 — generate 606 report
pub async fn generar_606_handler(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    body: web::Json<PeriodoRequest>,
) -> Result<HttpResponse, AppError> {
    obtener_org_con_acceso_fiscal(db.get_ref(), user.0.organizacion_id).await?;

    let reporte = reportes_dgii::generar_606(
        db.get_ref(),
        user.0.organizacion_id,
        &body.periodo,
        user.0.sub,
    )
    .await?;

    Ok(HttpResponse::Ok().json(reporte))
}

/// GET /api/v1/reportes-dgii/preview/{tipo}/{periodo} — preview report as JSON
pub async fn preview_reporte(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let (tipo, periodo) = path.into_inner();
    let reporte =
        reportes_dgii::preview_reporte(db.get_ref(), user.0.organizacion_id, &tipo, &periodo)
            .await?;
    Ok(HttpResponse::Ok().json(reporte))
}

/// PUT /api/v1/reportes-dgii/{id}/estado — mark report as enviado
pub async fn actualizar_estado(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<EstadoRequest>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let updated = reportes_dgii::actualizar_estado_reporte(
        db.get_ref(),
        user.0.organizacion_id,
        id,
        &body.estado,
    )
    .await?;
    Ok(HttpResponse::Ok().json(updated))
}
