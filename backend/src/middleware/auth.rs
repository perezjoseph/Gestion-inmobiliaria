use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use std::future::{Ready, ready};

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::services::auth::{Claims, decode_jwt};

impl FromRequest for Claims {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = extract_claims(req);
        ready(result)
    }
}

fn extract_claims(req: &HttpRequest) -> Result<Claims, AppError> {
    let config = req
        .app_data::<web::Data<AppConfig>>()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "AppConfig no disponible"
        )))?;

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized(None))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized(None))?;

    decode_jwt(token, &config.jwt_secret)
}
