use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use serde_json::json;
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No encontrado: {0}")]
    NotFound(String),
    #[error("{}", .0.as_deref().unwrap_or("No autorizado"))]
    Unauthorized(Option<String>),
    #[error("Solicitud inválida: {0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Validation(String),
    #[error("{message}")]
    ValidationWithFields {
        message: String,
        fields: serde_json::Value,
    },
    #[error("Conflicto: {0}")]
    Conflict(String),
    #[error("Recurso expirado: {0}")]
    Gone(String),
    #[error("Bad Gateway: {0}")]
    BadGateway(String),
    #[error("Servicio no disponible: {0}")]
    ServiceUnavailable(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl actix_web::error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Validation(_) | Self::ValidationWithFields { .. } => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Gone(_) => StatusCode::GONE,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let Self::Internal(e) = self {
            error!(error = %e, error_debug = ?e, "Internal server error");
        }
        let body = if let Self::ValidationWithFields { message, fields } = self {
            let mut obj = json!({
                "error": "validation",
                "message": message,
            });
            if let Some(map) = fields.as_object() {
                for (k, v) in map {
                    obj[k] = v.clone();
                }
            }
            obj
        } else {
            let (error_type, message) = match self {
                Self::NotFound(msg) => ("not_found", msg.clone()),
                Self::Unauthorized(_) => ("unauthorized", self.to_string()),
                Self::BadRequest(msg) => ("bad_request", msg.clone()),
                Self::Forbidden(msg) => ("forbidden", msg.clone()),
                Self::Validation(msg) => ("validation", msg.clone()),
                Self::Conflict(msg) => ("conflict", msg.clone()),
                Self::Gone(msg) => ("gone", msg.clone()),
                Self::BadGateway(msg) => ("bad_gateway", msg.clone()),
                Self::ServiceUnavailable(msg) => ("service_unavailable", msg.clone()),
                Self::Internal(_) => ("internal", "Error interno del servidor".to_string()),
                Self::ValidationWithFields { .. } => unreachable!(),
            };
            json!({
                "error": error_type,
                "message": message,
            })
        };

        HttpResponse::build(self.status_code()).json(body)
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        let msg = err.to_string();
        if msg.contains("duplicate key") || msg.contains("unique constraint") {
            return Self::Conflict(extract_constraint_message(&msg));
        }
        if msg.contains("foreign key") || msg.contains("violates foreign key") {
            return Self::Validation("Referencia a un recurso inexistente".to_string());
        }
        if msg.contains("not-null constraint") || msg.contains("null value in column") {
            return Self::Validation(extract_null_column(&msg));
        }
        Self::Internal(anyhow::anyhow!(err))
    }
}

fn extract_constraint_message(msg: &str) -> String {
    if let Some(detail_start) = msg.find("Detail:") {
        let detail = &msg[detail_start..];
        if let Some(key_start) = detail.find("Key (") {
            if let Some(key_end) = detail[key_start..].find(')') {
                let column = &detail[key_start + 5..key_start + key_end];
                return format!("Ya existe un registro con el mismo valor de '{column}'");
            }
        }
    }
    "Ya existe un registro con valores duplicados".to_string()
}

fn extract_null_column(msg: &str) -> String {
    if let Some(start) = msg.find("null value in column \"") {
        let rest = &msg[start + 22..];
        if let Some(end) = rest.find('"') {
            let column = &rest[..end];
            return format!("El campo '{column}' es requerido");
        }
    }
    "Falta un campo requerido".to_string()
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
        let err = AppError::Forbidden("Acceso denegado".to_string());
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
    fn gone_returns_410() {
        let err = AppError::Gone("expirado".to_string());
        assert_eq!(err.status_code(), StatusCode::GONE);
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
    fn db_err_converts_to_internal_for_generic_errors() {
        let db_err = sea_orm::DbErr::Custom("test error".to_string());
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn db_err_converts_to_conflict_for_duplicate_key() {
        let db_err = sea_orm::DbErr::Custom(
            "duplicate key value violates unique constraint \"propiedades_titulo_key\" Detail: Key (titulo)=(Mi Casa) already exists.".to_string(),
        );
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::CONFLICT);
    }

    #[test]
    fn db_err_converts_to_conflict_for_unique_constraint() {
        let db_err =
            sea_orm::DbErr::Custom("unique constraint violation on column email".to_string());
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::CONFLICT);
    }

    #[test]
    fn db_err_converts_to_validation_for_not_null() {
        let db_err = sea_orm::DbErr::Custom(
            "null value in column \"titulo\" of relation \"propiedades\" violates not-null constraint".to_string(),
        );
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(app_err.to_string().contains("titulo"));
    }

    #[test]
    fn db_err_converts_to_validation_for_foreign_key() {
        let db_err = sea_orm::DbErr::Custom(
            "violates foreign key constraint on table propiedades".to_string(),
        );
        let app_err: AppError = db_err.into();
        assert_eq!(app_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn extract_constraint_message_parses_detail() {
        let msg = "duplicate key value violates unique constraint Detail: Key (titulo)=(test) already exists.";
        let result = super::extract_constraint_message(msg);
        assert!(result.contains("titulo"));
    }

    #[test]
    fn extract_null_column_parses_column_name() {
        let msg = "null value in column \"direccion\" violates not-null constraint";
        let result = super::extract_null_column(msg);
        assert!(result.contains("direccion"));
    }
}
