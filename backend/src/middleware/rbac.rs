use crate::errors::AppError;
use crate::services::auth::Claims;

#[allow(dead_code)]
pub fn check_role(claims: &Claims, allowed_roles: &[&str]) -> Result<(), AppError> {
    if allowed_roles.contains(&claims.rol.as_str()) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub struct AdminOnly(pub Claims);
pub struct WriteAccess(pub Claims);

impl actix_web::FromRequest for AdminOnly {
    type Error = AppError;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let claims = Claims::from_request(req, payload).into_inner();
        std::future::ready(match claims {
            Ok(c) if c.rol == "admin" => Ok(Self(c)),
            Ok(_) => Err(AppError::Forbidden),
            Err(e) => Err(e),
        })
    }
}

impl actix_web::FromRequest for WriteAccess {
    type Error = AppError;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let claims = Claims::from_request(req, payload).into_inner();
        std::future::ready(match claims {
            Ok(c) if c.rol == "admin" || c.rol == "gerente" => Ok(Self(c)),
            Ok(_) => Err(AppError::Forbidden),
            Err(e) => Err(e),
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
            exp: 9_999_999_999,
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
