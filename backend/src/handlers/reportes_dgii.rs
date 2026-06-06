use actix_web::{HttpResponse, web};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::entities::{organizacion, reporte_dgii};
use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::services::fiscal::verificar_acceso_fiscal;
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
    let org = organizacion::Entity::find_by_id(user.0.organizacion_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

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
    let org = organizacion::Entity::find_by_id(user.0.organizacion_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

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
    let org = organizacion::Entity::find_by_id(user.0.organizacion_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let (tipo, periodo) = path.into_inner();

    // Validate tipo
    if tipo != "606" && tipo != "607" {
        return Err(AppError::Validation(
            "Tipo de reporte debe ser '606' o '607'".to_string(),
        ));
    }

    // Look for existing borrador report
    let reporte = reporte_dgii::Entity::find()
        .filter(reporte_dgii::Column::OrganizacionId.eq(user.0.organizacion_id))
        .filter(reporte_dgii::Column::TipoReporte.eq(&tipo))
        .filter(reporte_dgii::Column::Periodo.eq(&periodo))
        .filter(reporte_dgii::Column::Estado.eq("borrador"))
        .one(db.get_ref())
        .await?;

    reporte.map_or_else(
        || {
            Err(AppError::NotFound(
                "Reporte no encontrado. Genere el reporte primero.".to_string(),
            ))
        },
        |r| Ok(HttpResponse::Ok().json(r)),
    )
}

/// PUT /api/v1/reportes-dgii/{id}/estado — mark report as enviado
pub async fn actualizar_estado(
    db: web::Data<DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<EstadoRequest>,
) -> Result<HttpResponse, AppError> {
    let org = organizacion::Entity::find_by_id(user.0.organizacion_id)
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;
    verificar_acceso_fiscal(&org)?;

    let id = path.into_inner();

    if body.estado != "enviado" {
        return Err(AppError::Validation(
            "Estado solo puede ser 'enviado'".to_string(),
        ));
    }

    let reporte = reporte_dgii::Entity::find_by_id(id)
        .filter(reporte_dgii::Column::OrganizacionId.eq(user.0.organizacion_id))
        .one(db.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Reporte no encontrado".to_string()))?;

    if reporte.estado == "enviado" {
        return Err(AppError::Conflict(
            "Reporte ya fue enviado a DGII para este período".to_string(),
        ));
    }

    let now = chrono::Utc::now().into();
    let mut active: reporte_dgii::ActiveModel = reporte.into();
    active.estado = Set("enviado".to_string());
    active.submitted_at = Set(Some(now));
    active.updated_at = Set(now);

    let updated = active.update(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(updated))
}
