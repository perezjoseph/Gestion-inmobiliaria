use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::middleware::rbac::AdminOnly;
use crate::models::auditoria::AuditoriaQuery;
use crate::services::auditoria;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    query: web::Query<AuditoriaQuery>,
) -> Result<HttpResponse, AppError> {
    let result = auditoria::listar(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
