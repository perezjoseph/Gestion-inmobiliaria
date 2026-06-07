use actix_web::{HttpRequest, HttpResponse, web};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::rate_limit::extract_client_ip_from_request;
use crate::models::usuario::{LoginRequest, RegisterRequest};
use crate::services::auth::{self, RegisterResult};
use crate::services::login_lockout::LoginLockout;

pub async fn register(
    db: web::Data<DatabaseConnection>,
    config: web::Data<AppConfig>,
    body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, AppError> {
    let result = auth::register(db.get_ref(), body.into_inner(), &config.jwt_secret).await?;
    match result {
        RegisterResult::User(user) => Ok(HttpResponse::Created().json(user)),
        RegisterResult::Login(login) => Ok(HttpResponse::Created().json(login)),
    }
}

pub async fn login(
    req: HttpRequest,
    db: web::Data<DatabaseConnection>,
    config: web::Data<AppConfig>,
    lockout: web::Data<LoginLockout>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let input = body.into_inner();
    let email = input.email.clone();
    let client_ip = extract_client_ip_from_request(&req);

    // Check lockout BEFORE attempting login (Req 2.7: does not reveal if email exists)
    if let Err(info) = lockout.check(&email) {
        tracing::warn!(
            event = "login_blocked_lockout",
            email = %email,
            client_ip = %client_ip,
            retry_after_seconds = info.retry_after_seconds,
            "Login attempt blocked — account locked"
        );
        return Ok(HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "account_locked",
            "retry_after_seconds": info.retry_after_seconds
        })));
    }

    match auth::login(db.get_ref(), input, &config.jwt_secret).await {
        Ok(response) => {
            lockout.record_success(&email);
            tracing::info!(user_id = %response.user.id, "Successful login");
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let lockout_info = lockout.record_failure(&email);

            tracing::warn!(
                event = "login_failed",
                email = %email,
                client_ip = %client_ip,
                "Failed login attempt"
            );

            if let Some(info) = lockout_info {
                tracing::warn!(
                    event = "account_locked",
                    email = %email,
                    client_ip = %client_ip,
                    retry_after_seconds = info.retry_after_seconds,
                    "Account locked due to repeated failures"
                );
            }

            Err(e)
        }
    }
}
