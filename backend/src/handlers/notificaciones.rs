use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::notificaciones;

pub async fn pagos_vencidos(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let results = notificaciones::listar_pagos_vencidos(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(results))
}
