use actix_web::{HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::usuario::{LoginRequest, RegisterRequest};
use crate::services::auth;

pub async fn register(
    db: web::Data<DatabaseConnection>,
    config: web::Data<AppConfig>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    let response = auth::register(db.get_ref(), body.into_inner(), &config.jwt_secret).await?;
    Ok(HttpResponse::Created().json(response))
}

pub async fn login(
    db: web::Data<DatabaseConnection>,
    config: web::Data<AppConfig>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let input = body.into_inner();
    let email = input.email.clone();
    match auth::login(db.get_ref(), input, &config.jwt_secret).await {
        Ok(response) => {
            tracing::info!(user_id = %response.user.id, "Successful login");
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            tracing::warn!(email = %email, "Failed login attempt");
            Err(e)
        }
    }
}
