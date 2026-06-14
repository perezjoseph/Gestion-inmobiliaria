use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::ncf::ConfigurarRangoRequest;
use crate::services::ncf;

pub async fn listar_secuencias(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let responses = ncf::listar_secuencias(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(responses))
}

pub async fn configurar_rango_handler(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    body: web::Json<ConfigurarRangoRequest>,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let result = ncf::configurar_rango_con_acceso(db.get_ref(), org_id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

pub async fn obtener_alertas(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let alertas = ncf::obtener_alertas(db.get_ref(), org_id).await?;
    Ok(HttpResponse::Ok().json(alertas))
}
