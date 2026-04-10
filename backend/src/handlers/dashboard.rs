use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::errors::AppError;
use crate::services::auth::Claims;
use crate::services::dashboard;

pub async fn stats(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = dashboard::get_stats(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}
