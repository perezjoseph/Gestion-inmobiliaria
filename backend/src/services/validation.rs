use crate::errors::AppError;

pub fn validate_enum(field_name: &str, value: &str, allowed: &[&str]) -> Result<(), AppError> {
    if !allowed.contains(&value) {
        return Err(AppError::Validation(format!(
            "Valor inválido para {field_name}: '{value}'. Valores permitidos: {}",
            allowed.join(", ")
        )));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use actix_web::error::ResponseError;
    use actix_web::http::StatusCode;

    const ROLES: &[&str] = &["admin", "gerente", "visualizador"];

    #[test]
    fn validate_enum_accepts_valid_role_admin() {
        assert!(validate_enum("rol", "admin", ROLES).is_ok());
    }

    #[test]
    fn validate_enum_accepts_valid_role_gerente() {
        assert!(validate_enum("rol", "gerente", ROLES).is_ok());
    }

    #[test]
    fn validate_enum_accepts_valid_role_visualizador() {
        assert!(validate_enum("rol", "visualizador", ROLES).is_ok());
    }

    #[test]
    fn validate_enum_rejects_invalid_role() {
        let result = validate_enum("rol", "superadmin", ROLES);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.to_string().contains("superadmin"));
    }

    #[test]
    fn validate_enum_rejects_empty_string() {
        let result = validate_enum("rol", "", ROLES);
        assert!(result.is_err());
    }

    #[test]
    fn validate_enum_is_case_sensitive() {
        let result = validate_enum("rol", "Admin", ROLES);
        assert!(result.is_err());
    }
}
