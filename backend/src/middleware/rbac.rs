use super::Claims;
use crate::errors::AppError;

use std::future::Future;
use std::pin::Pin;

#[allow(dead_code)]
pub fn check_role(claims: &Claims, allowed_roles: &[&str]) -> Result<(), AppError> {
    if allowed_roles.contains(&claims.rol.as_str()) {
        Ok(())
    } else {
        Err(AppError::Forbidden("Acceso denegado".to_string()))
    }
}

pub struct AdminOnly(pub Claims);
pub struct WriteAccess(pub Claims);

impl actix_web::FromRequest for AdminOnly {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let claims_fut = Claims::from_request(req, payload);
        Box::pin(async move {
            let claims = claims_fut.await?;
            if claims.rol == "admin" {
                Ok(Self(claims))
            } else {
                Err(AppError::Forbidden("Acceso denegado".to_string()))
            }
        })
    }
}

impl actix_web::FromRequest for WriteAccess {
    type Error = AppError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let claims_fut = Claims::from_request(req, payload);
        Box::pin(async move {
            let claims = claims_fut.await?;
            if claims.rol == "admin" || claims.rol == "gerente" {
                Ok(Self(claims))
            } else {
                Err(AppError::Forbidden("Acceso denegado".to_string()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_claims(rol: &str) -> Claims {
        Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: rol.to_string(),
            organizacion_id: Uuid::nil(),
            jti: Uuid::new_v4(),
            iat: 0,
            exp: 9_999_999_999,
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
        }
    }

    #[test]
    fn check_role_allows_matching_role() {
        let claims = make_claims("admin");
        assert!(check_role(&claims, &["admin", "gerente"]).is_ok());
    }

    #[test]
    fn check_role_denies_non_matching_role() {
        let claims = make_claims("visualizador");
        assert!(check_role(&claims, &["admin", "gerente"]).is_err());
    }

    #[test]
    fn check_role_admin_has_full_access() {
        let claims = make_claims("admin");
        assert!(check_role(&claims, &["admin"]).is_ok());
    }

    #[test]
    fn check_role_gerente_write_access() {
        let claims = make_claims("gerente");
        assert!(check_role(&claims, &["admin", "gerente"]).is_ok());
    }

    #[test]
    fn check_role_visualizador_read_only() {
        let claims = make_claims("visualizador");
        assert!(check_role(&claims, &["admin", "gerente"]).is_err());
        assert!(check_role(&claims, &["admin", "gerente", "visualizador"]).is_ok());
    }
}
