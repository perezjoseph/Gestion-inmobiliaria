use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::{invitacion, organizacion, usuario};
use crate::errors::AppError;
use crate::models::usuario::{LoginRequest, LoginResponse, RegisterRequest, UserResponse};
use crate::services::{invitaciones, validacion_fiscal};

pub use crate::services::user_security_cache::UserSecurityCache;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum RegisterResult {
    User(UserResponse),
    Login(LoginResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub rol: String,
    pub organizacion_id: Uuid,
    pub jti: Uuid,
    pub iat: i64,
    pub exp: usize,
    pub iss: String,
    pub aud: String,
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
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "sub", "iat"]);
    validation.set_audience(&["realestate-api"]);
    validation.set_issuer(&["realestate-api"]);
    validation.leeway = 30;
    validation.validate_nbf = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized(None))?;
    Ok(token_data.claims)
}

fn is_duplicate_key_error(err: &sea_orm::DbErr) -> bool {
    let msg = err.to_string();
    msg.contains("duplicate key") || msg.contains("unique constraint")
}

async fn check_email_unique<C: ConnectionTrait>(db: &C, email: &str) -> Result<(), AppError> {
    let existing = usuario::Entity::find()
        .filter(usuario::Column::Email.eq(email))
        .one(db)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "El email ya está registrado".to_string(),
        ));
    }
    Ok(())
}

fn build_login_response(
    user: &usuario::Model,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    let now = Utc::now();
    let exp = (now + chrono::Duration::hours(8)).timestamp() as usize;
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        rol: user.rol.clone(),
        organizacion_id: user.organizacion_id,
        jti: Uuid::new_v4(),
        iat: now.timestamp(),
        exp,
        iss: "realestate-api".to_string(),
        aud: "realestate-api".to_string(),
    };
    let token = encode_jwt(&claims, jwt_secret)?;

    Ok(LoginResponse {
        token,
        user: UserResponse {
            id: user.id,
            nombre: user.nombre.clone(),
            email: user.email.clone(),
            rol: user.rol.clone(),
            activo: user.activo,
            organizacion_id: user.organizacion_id,
            created_at: user.created_at.into(),
        },
    })
}

fn validate_auth_input(email: &str, password: &str, nombre: Option<&str>) -> Result<(), AppError> {
    if email.len() > 255 {
        return Err(AppError::Validation(
            "El email no puede exceder 255 caracteres".to_string(),
        ));
    }
    if !email.contains('@') || !email.contains('.') {
        return Err(AppError::Validation(
            "Formato de email inválido".to_string(),
        ));
    }

    if password.len() < 8 {
        return Err(AppError::Validation(
            "La contraseña debe tener al menos 8 caracteres".to_string(),
        ));
    }
    if password.len() > 128 {
        return Err(AppError::Validation(
            "La contraseña no puede exceder 128 caracteres".to_string(),
        ));
    }

    if let Some(name) = nombre {
        if name.trim().is_empty() {
            return Err(AppError::Validation(
                "El nombre no puede estar vacío".to_string(),
            ));
        }
        if name.len() > 255 {
            return Err(AppError::Validation(
                "El nombre no puede exceder 255 caracteres".to_string(),
            ));
        }
    }

    Ok(())
}

pub async fn register(
    db: &DatabaseConnection,
    input: RegisterRequest,
    jwt_secret: &str,
) -> Result<RegisterResult, AppError> {
    validate_auth_input(&input.email, &input.password, Some(&input.nombre))?;

    if let Some(token_inv) = input.token_invitacion.clone() {
        let login = register_with_invitation(db, input, &token_inv, jwt_secret).await?;
        Ok(RegisterResult::Login(login))
    } else {
        let login = register_new_org(db, input, jwt_secret).await?;
        Ok(RegisterResult::Login(login))
    }
}

async fn register_new_org(
    db: &DatabaseConnection,
    input: RegisterRequest,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    let tipo = input
        .tipo
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("El campo 'tipo' es requerido".to_string()))?;

    if tipo != "persona_fisica" && tipo != "persona_juridica" {
        return Err(AppError::BadRequest(
            "El tipo debe ser 'persona_fisica' o 'persona_juridica'".to_string(),
        ));
    }

    let (
        org_nombre,
        org_cedula,
        org_telefono,
        org_rnc,
        org_razon_social,
        org_nombre_comercial,
        org_direccion_fiscal,
        org_representante_legal,
    );

    if tipo == "persona_fisica" {
        let cedula = input.cedula.as_deref().ok_or_else(|| {
            AppError::BadRequest("La cédula es requerida para persona física".to_string())
        })?;
        let telefono = input.telefono.as_deref().ok_or_else(|| {
            AppError::BadRequest("El teléfono es requerido para persona física".to_string())
        })?;
        let nombre_org = input.nombre_organizacion.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "El nombre de la organización es requerido para persona física".to_string(),
            )
        })?;

        validacion_fiscal::validar_cedula(cedula)?;

        org_nombre = nombre_org.to_string();
        org_cedula = Some(cedula.to_string());
        org_telefono = Some(telefono.to_string());
        org_rnc = None;
        org_razon_social = None;
        org_nombre_comercial = None;
        org_direccion_fiscal = None;
        org_representante_legal = None;
    } else {
        let rnc = input.rnc.as_deref().ok_or_else(|| {
            AppError::BadRequest("El RNC es requerido para persona jurídica".to_string())
        })?;
        let razon_social = input.razon_social.as_deref().ok_or_else(|| {
            AppError::BadRequest("La razón social es requerida para persona jurídica".to_string())
        })?;
        let nombre_comercial = input.nombre_comercial.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "El nombre comercial es requerido para persona jurídica".to_string(),
            )
        })?;
        let direccion_fiscal = input.direccion_fiscal.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "La dirección fiscal es requerida para persona jurídica".to_string(),
            )
        })?;
        let representante_legal = input.representante_legal.as_deref().ok_or_else(|| {
            AppError::BadRequest(
                "El representante legal es requerido para persona jurídica".to_string(),
            )
        })?;

        validacion_fiscal::validar_rnc(rnc)?;

        org_nombre = nombre_comercial.to_string();
        org_cedula = None;
        org_telefono = None;
        org_rnc = Some(rnc.to_string());
        org_razon_social = Some(razon_social.to_string());
        org_nombre_comercial = Some(nombre_comercial.to_string());
        org_direccion_fiscal = Some(direccion_fiscal.to_string());
        org_representante_legal = Some(representante_legal.to_string());
    }

    let password_hash = hash_password(&input.password)?;

    let txn = db.begin().await?;

    check_email_unique(&txn, &input.email).await?;

    if let Some(ref cedula) = org_cedula {
        let dup = organizacion::Entity::find()
            .filter(organizacion::Column::Cedula.eq(cedula.as_str()))
            .one(&txn)
            .await?;
        if dup.is_some() {
            return Err(AppError::Conflict(
                "La cédula ya está registrada en otra organización".to_string(),
            ));
        }
    }

    if let Some(ref rnc) = org_rnc {
        let dup = organizacion::Entity::find()
            .filter(organizacion::Column::Rnc.eq(rnc.as_str()))
            .one(&txn)
            .await?;
        if dup.is_some() {
            return Err(AppError::Conflict(
                "El RNC ya está registrado en otra organización".to_string(),
            ));
        }
    }

    let now = Utc::now().into();
    let org_id = Uuid::new_v4();

    let org_model = organizacion::ActiveModel {
        id: Set(org_id),
        tipo: Set(tipo.to_string()),
        nombre: Set(org_nombre),
        estado: Set("activo".to_string()),
        cedula: Set(org_cedula),
        telefono: Set(org_telefono),
        email_organizacion: Set(None),
        rnc: Set(org_rnc),
        razon_social: Set(org_razon_social),
        nombre_comercial: Set(org_nombre_comercial),
        direccion_fiscal: Set(org_direccion_fiscal),
        representante_legal: Set(org_representante_legal),
        dgii_data: Set(None),
        tipo_fiscal: Set("informal".to_string()),
        regimen_pagos: Set(None),
        fecha_inicio_operaciones: Set(None),
        is_ecf_certificado: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };
    org_model.insert(&txn).await?;

    let user_id = Uuid::new_v4();
    let user_model = usuario::ActiveModel {
        id: Set(user_id),
        nombre: Set(input.nombre.clone()),
        email: Set(input.email.clone()),
        password_hash: Set(password_hash),
        rol: Set("admin".to_string()),
        activo: Set(true),
        organizacion_id: Set(org_id),
        created_at: Set(now),
        updated_at: Set(now),
        password_changed_at: Set(now),
    };
    let user = user_model.insert(&txn).await.map_err(|e| {
        if is_duplicate_key_error(&e) {
            AppError::Conflict("El email ya está registrado".to_string())
        } else {
            AppError::from(e)
        }
    })?;

    txn.commit().await?;

    build_login_response(&user, jwt_secret)
}

async fn register_with_invitation(
    db: &DatabaseConnection,
    input: RegisterRequest,
    token_inv: &str,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    let inv = invitaciones::validar_token(db, token_inv).await?;

    let password_hash = hash_password(&input.password)?;

    let txn = db.begin().await?;

    check_email_unique(&txn, &input.email).await?;

    let now = Utc::now().into();
    let user_id = Uuid::new_v4();

    let user_model = usuario::ActiveModel {
        id: Set(user_id),
        nombre: Set(input.nombre),
        email: Set(input.email),
        password_hash: Set(password_hash),
        rol: Set(inv.rol.clone()),
        activo: Set(true),
        organizacion_id: Set(inv.organizacion_id),
        created_at: Set(now),
        updated_at: Set(now),
        password_changed_at: Set(now),
    };
    let user = user_model.insert(&txn).await.map_err(|e| {
        if is_duplicate_key_error(&e) {
            AppError::Conflict("El email ya está registrado".to_string())
        } else {
            AppError::from(e)
        }
    })?;

    let mut inv_active: invitacion::ActiveModel = inv.into();
    inv_active.usado = Set(true);
    inv_active.update(&txn).await?;

    txn.commit().await?;

    build_login_response(&user, jwt_secret)
}

pub async fn login(
    db: &DatabaseConnection,
    input: LoginRequest,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    if input.email.len() > 255 || input.password.len() > 128 {
        return Err(AppError::Unauthorized(None));
    }

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

    let now = Utc::now();
    let exp = (now + chrono::Duration::hours(8)).timestamp() as usize;
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        rol: user.rol.clone(),
        organizacion_id: user.organizacion_id,
        jti: Uuid::new_v4(),
        iat: now.timestamp(),
        exp,
        iss: "realestate-api".to_string(),
        aud: "realestate-api".to_string(),
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
            organizacion_id: user.organizacion_id,
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

    fn test_password(label: &str) -> String {
        format!("test_pw_{label}")
    }

    fn test_secret(label: &str) -> String {
        format!("test_jwt_secret_{label}_padding_for_length")
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn hash_and_verify_password_succeeds() {
        let password = test_password("valid");
        let hash = hash_password(&password).unwrap();
        assert!(verify_password(&hash, &password).unwrap());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn verify_wrong_password_fails() {
        let correct = test_password("correct");
        let wrong = test_password("wrong");
        let hash = hash_password(&correct).unwrap();
        assert!(!verify_password(&hash, &wrong).unwrap());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn encode_decode_jwt_roundtrip() {
        install_crypto_provider();
        let secret = test_secret("roundtrip");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            organizacion_id: Uuid::new_v4(),
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
        };

        let token = encode_jwt(&claims, &secret).unwrap();
        let decoded = decode_jwt(&token, &secret).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.email, claims.email);
        assert_eq!(decoded.rol, claims.rol);
        assert_eq!(decoded.organizacion_id, claims.organizacion_id);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn decode_jwt_with_wrong_secret_fails() {
        install_crypto_provider();
        let secret_a = test_secret("a");
        let secret_b = test_secret("b");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            organizacion_id: Uuid::new_v4(),
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
        };

        let token = encode_jwt(&claims, &secret_a).unwrap();
        let result = decode_jwt(&token, &secret_b);
        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn decode_expired_jwt_fails() {
        install_crypto_provider();
        let secret = test_secret("expired");
        let claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            rol: "admin".to_string(),
            organizacion_id: Uuid::new_v4(),
            jti: Uuid::new_v4(),
            iat: Utc::now().timestamp(),
            exp: 0,
            iss: "realestate-api".to_string(),
            aud: "realestate-api".to_string(),
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
