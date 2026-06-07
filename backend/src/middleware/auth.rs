use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use sea_orm::DatabaseConnection;
use std::future::Future;
use std::pin::Pin;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::services::auth::{Claims, UserSecurityCache, decode_jwt};

impl FromRequest for Claims {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let claims_result = extract_claims(req);
        let db = req.app_data::<web::Data<DatabaseConnection>>().cloned();
        let cache = req.app_data::<web::Data<UserSecurityCache>>().cloned();

        Box::pin(async move {
            let claims = claims_result?;

            // If cache and db are available, validate user security state
            if let (Some(db), Some(cache)) = (db, cache) {
                let valid = cache
                    .is_token_valid(db.get_ref(), claims.sub, claims.iat)
                    .await?;
                if !valid {
                    return Err(AppError::Unauthorized(Some("Sesión inválida".to_string())));
                }
            }

            Ok(claims)
        })
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
