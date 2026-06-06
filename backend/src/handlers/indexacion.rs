#![allow(clippy::doc_markdown)]

use actix_web::{HttpResponse, web};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::rbac::WriteAccess;
use crate::models::indexacion::AprobarRenovacionRequest;
use crate::services::indexacion;

/// `GET /api/v1/indexacion/propuesta/{contrato_id}`
///
/// Returns a renewal proposal for the given contract based on IPC and Ley 85-25 cap.
pub async fn obtener_propuesta(
    db: web::Data<sea_orm::DatabaseConnection>,
    _user: WriteAccess,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    let propuesta = indexacion::calcular_propuesta_renovacion(db.get_ref(), contrato_id).await?;
    Ok(HttpResponse::Ok().json(propuesta))
}

/// `POST /api/v1/indexacion/aprobar/{contrato_id}`
///
/// Approves a renewal with the specified amount. Validates against the 10% legal cap.
pub async fn aprobar_renovacion_handler(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
    path: web::Path<Uuid>,
    body: web::Json<AprobarRenovacionRequest>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    let admin_id = user.0.sub;
    let req = body.into_inner();
    let new_contrato =
        indexacion::aprobar_renovacion(db.get_ref(), contrato_id, req.monto_aprobado, admin_id)
            .await?;
    Ok(HttpResponse::Ok().json(new_contrato))
}

/// `GET /api/v1/indexacion/proximos-vencer`
///
/// Lists active contracts expiring within 60 days for the user's organization.
pub async fn proximos_vencer(
    db: web::Data<sea_orm::DatabaseConnection>,
    user: WriteAccess,
) -> Result<HttpResponse, AppError> {
    let org_id = user.0.organizacion_id;
    let contratos = indexacion::contratos_proximos_vencer(db.get_ref(), org_id, 60).await?;
    Ok(HttpResponse::Ok().json(contratos))
}
