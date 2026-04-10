use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::usuario::{LoginRequest, RegisterRequest};
use crate::services::auth;

pub async fn register(
    db: web::Data<DatabaseConnection>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    let user = auth::register(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(user))
}

pub async fn login(
    db: web::Data<DatabaseConnection>,
    config: web::Data<AppConfig>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let response = auth::login(db.get_ref(), body.into_inner(), &config.jwt_secret).await?;
    Ok(HttpResponse::Ok().json(response))
}
