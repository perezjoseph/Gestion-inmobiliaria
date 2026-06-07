use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::ncf::ConfigurarRangoRequest;
use crate::services::ncf;

/// GET /api/v1/ncf/secuencias — list NCF sequences for the organization.
pub async fn listar_secuencias(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let responses = ncf::listar_secuencias(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(responses))
}

/// POST /api/v1/ncf/configurar-rango — configure an authorized NCF range.
pub async fn configurar_rango_handler(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<ConfigurarRangoRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let result = ncf::configurar_rango_con_acceso(db.get_ref(), org_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// GET /api/v1/ncf/alertas — check range consumption alerts.
pub async fn obtener_alertas(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let alertas = ncf::obtener_alertas(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(alertas))
}
