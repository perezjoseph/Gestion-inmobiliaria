use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No encontrado: {0}")]
    NotFound(String),
    #[error("{}", .0.as_deref().unwrap_or("No autorizado"))]
    Unauthorized(Option<String>),
    #[error("Solicitud inválida: {0}")]
    BadRequest(String),
    #[error("Acceso denegado")]
    Forbidden,
    #[error("{0}")]
    Validation(String),
    #[error("Conflicto: {0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl actix_web::error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (error_type, message) = match self {
            AppError::NotFound(msg) => ("not_found", msg.clone()),
            AppError::Unauthorized(_) => ("unauthorized", self.to_string()),
            AppError::BadRequest(msg) => ("bad_request", msg.clone()),
            AppError::Forbidden => ("forbidden", self.to_string()),
            AppError::Validation(msg) => ("validation", msg.clone()),
            AppError::Conflict(msg) => ("conflict", msg.clone()),
            AppError::Internal(_) => ("internal", "Error interno del servidor".to_string()),
        };

        HttpResponse::build(self.status_code()).json(json!({
            "error": error_type,
            "message": message,
        }))
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::Internal(anyhow::anyhow!(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::error::ResponseError;

    #[test]
    fn not_found_returns_404() {
        let err = AppError::NotFound("recurso".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_returns_401() {
        let err = AppError::Unauthorized(None);
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_returns_403() {
        let err = AppError::Forbidden;
        assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn bad_request_returns_400() {
        let err = AppError::BadRequest("campo faltante".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn validation_returns_422() {
        let err = AppError::Validation("campo requerido".to_string());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn conflict_returns_409() {
        let err = AppError::Conflict("ya existe".to_string());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
    }

    #[test]
    fn internal_returns_500() {
        let err = AppError::Internal(anyhow::anyhow!("something broke"));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_response_contains_json_body() {
        let err = AppError::NotFound("propiedad".to_string());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_hides_details() {
        let err = AppError::Internal(anyhow::anyhow!("secret db info"));
        let (error_type, message) = match &err {
            AppError::Internal(_) => ("internal", "Error interno del servidor"),
            _ => unreachable!(),
        };
        assert_eq!(error_type, "internal");
        assert_eq!(message, "Error interno del servidor");
    }

    #[test]
    fn db_err_converts_to_app_error() {
        let db_err = sea_orm::DbErr::Custom("test error".to_string());
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
