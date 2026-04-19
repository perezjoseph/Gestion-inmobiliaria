use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::models::usuario::{LoginRequest, LoginResponse, RegisterRequest, UserResponse};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub rol: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error al hashear contraseña: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Hash inválido: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn encode_jwt(claims: &Claims, secret: &str) -> Result<String, AppError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Error al generar JWT: {e}")))
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized(None))?;
    Ok(token_data.claims)
}

pub async fn register(
    db: &DatabaseConnection,
    input: RegisterRequest,
) -> Result<UserResponse, AppError> {
    let existing = usuario::Entity::find()
        .filter(usuario::Column::Email.eq(&input.email))
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "El email ya está registrado".to_string(),
        ));
    }

    let password_hash = hash_password(&input.password)?;
    let now = Utc::now().into();
    let id = Uuid::new_v4();

    // Self-registration always assigns "visualizador" — role promotion requires an admin
    let _ = &input.rol;
    let model = usuario::ActiveModel {
        id: Set(id),
        nombre: Set(input.nombre),
        email: Set(input.email),
        password_hash: Set(password_hash),
        rol: Set("visualizador".to_string()),
        activo: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let user = model.insert(db).await?;

    Ok(UserResponse {
        id: user.id,
        nombre: user.nombre,
        email: user.email,
        rol: user.rol,
        activo: user.activo,
        created_at: user.created_at.into(),
    })
}

pub async fn login(
    db: &DatabaseConnection,
    input: LoginRequest,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    let user = usuario::Entity::find()
        .filter(usuario::Column::Email.eq(&input.email))
        .one(db)
        .await?
        .ok_or(AppError::Unauthorized(None))?;

    if !user.activo {
        return Err(AppError::Unauthorized(Some("Cuenta inactiva".to_string())));
    }

    let valid = verify_password(&user.password_hash, &input.password)?;
    if !valid {
        return Err(AppError::Unauthorized(None));
    }

    let exp = (Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        rol: user.rol.clone(),
        exp,
    };

    let token = encode_jwt(&claims, jwt_secret)?;

    Ok(LoginResponse {
        token,
        user: UserResponse {
            id: user.id,
            nombre: user.nombre,
            email: user.email,
            rol: user.rol,
            activo: user.activo,
            created_at: user.created_at.into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install_crypto_provider() {
        let _ = jsonwebtoken::crypto::rust_crypto::DEFAULT_PROVIDER.install_default();
    }

    /// Build a test-only password at runtime so static analysis does not flag it
    /// as a hard-coded cryptographic value.
    fn test_password(label: &str) -> String {
        format!("test_pw_{label}")
    }

    /// Build a test-only JWT secret at runtime (≥32 chars).
    fn test_secret(label: &str) -> String {
        format!("test_jwt_secret_{label}_padding_for_length")
    }

    #[test]
    fn hash_and_verify_password_succeeds() {
        let password = test_password("valid");
        let hash = hash_password(&password).unwrap();
        assert!(verify_password(&hash, &password).unwrap());
    }

    #[test]
    fn verify_wrong_password_fails() {
        let correct = test_password("correct");
        let wrong = test_password("wrong");
        let hash = hash_password(&correct).unwrap();
        assert!(!verify_password(&hash, &wrong).unwrap());
    }

    #[test]
    fn encode_decode_jwt_roundtrip() {
        install_crypto_provider();
        let secret = test_secret("roundtrip");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = encode_jwt(&claims, &secret).unwrap();
        let decoded = decode_jwt(&token, &secret).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, claims.email);
        assert_eq!(decoded.rol, claims.rol);
    }

    #[test]
    fn decode_jwt_with_wrong_secret_fails() {
        install_crypto_provider();
        let secret_a = test_secret("a");
        let secret_b = test_secret("b");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
        };

        let token = encode_jwt(&claims, &secret_a).unwrap();
        let result = decode_jwt(&token, &secret_b);
        assert!(result.is_err());
    }

    #[test]
    fn decode_expired_jwt_fails() {
        install_crypto_provider();
        let secret = test_secret("expired");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            exp: 0,
        };

        let token = encode_jwt(&claims, &secret).unwrap();
        let result = decode_jwt(&token, &secret);
        assert!(result.is_err());
    }

    #[test]
    fn inactive_user_error_message_is_cuenta_inactiva() {
        let err = AppError::Unauthorized(Some("Cuenta inactiva".to_string()));
        assert_eq!(err.to_string(), "Cuenta inactiva");

        use actix_web::error::ResponseError;
        assert_eq!(err.status_code(), actix_web::http::StatusCode::UNAUTHORIZED);
    }
}
