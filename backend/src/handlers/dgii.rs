use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::{AdminOnly, WriteAccess};
use crate::models::dgii::{ConsultaNombreQuery, ConsultaRncQuery};
use crate::services::dgii;

pub async fn consultar_rnc(
    db: web::Data<DatabaseConnection>,
    access: WriteAccess,
    query: web::Query<ConsultaRncQuery>,
) -> Result<HttpResponse, AppError> {
    let org_id = access.0.organizacion_id;

    let result = dgii::consultar_rnc(db.get_ref(), org_id, &query.rnc).await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn consultar_nombre(
    _access: WriteAccess,
    query: web::Query<ConsultaNombreQuery>,
) -> Result<HttpResponse, AppError> {
    let result = dgii::consultar_nombre(&query.buscar).await?;

    Ok(HttpResponse::Ok().json(result))
}

pub async fn invalidar_cache(
    db: web::Data<DatabaseConnection>,
    admin: AdminOnly,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let org_id = admin.0.organizacion_id;
    let rnc = path.into_inner();

    dgii::invalidar_cache(db.get_ref(), org_id, &rnc).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Cache invalidado exitosamente"
    })))
}
