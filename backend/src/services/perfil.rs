use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::models::usuario::UsuarioResponse;
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::auth::{hash_password, verify_password};
use crate::services::user_security_cache::UserSecurityCache;

pub async fn obtener_perfil(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<UsuarioResponse, AppError> {
    let record = usuario::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    Ok(UsuarioResponse::from(record))
}

pub async fn actualizar_perfil(
    db: &DatabaseConnection,
    user_id: Uuid,
    nombre: Option<String>,
    email: Option<String>,
) -> Result<UsuarioResponse, AppError> {
    if let Some(ref name) = nombre {
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
    if let Some(ref mail) = email {
        if mail.len() > 255 {
            return Err(AppError::Validation(
                "El email no puede exceder 255 caracteres".to_string(),
            ));
        }
        if !mail.contains('@') || !mail.contains('.') {
            return Err(AppError::Validation(
                "Formato de email inválido".to_string(),
            ));
        }
    }

    let record = usuario::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    if let Some(ref new_email) = email
        && *new_email != record.email
    {
        let existing = usuario::Entity::find()
            .filter(usuario::Column::Email.eq(new_email))
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "El email ya está registrado".to_string(),
            ));
        }
    }

    let mut active: usuario::ActiveModel = record.into();

    if let Some(nombre) = nombre {
        active.nombre = Set(nombre);
    }
    if let Some(email) = email {
        active.email = Set(email);
    }

    let updated = active.update(db).await?;
    Ok(UsuarioResponse::from(updated))
}

pub async fn cambiar_password(
    db: &DatabaseConnection,
    user_id: Uuid,
    password_actual: &str,
    password_nuevo: &str,
    cache: &UserSecurityCache,
) -> Result<(), AppError> {
    if password_actual.len() > 128 {
        return Err(AppError::Validation(
            "La contraseña actual es incorrecta".to_string(),
        ));
    }
    if password_nuevo.len() < 8 {
        return Err(AppError::Validation(
            "La contraseña debe tener al menos 8 caracteres".to_string(),
        ));
    }
    if password_nuevo.len() > 128 {
        return Err(AppError::Validation(
            "La contraseña no puede exceder 128 caracteres".to_string(),
        ));
    }

    let record = usuario::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    let valid = verify_password(&record.password_hash, password_actual)?;
    if !valid {
        return Err(AppError::Validation(
            "La contraseña actual es incorrecta".to_string(),
        ));
    }

    let new_hash = hash_password(password_nuevo)?;
    let mut active: usuario::ActiveModel = record.into();
    active.password_hash = Set(new_hash);
    active.password_changed_at = Set(chrono::Utc::now().into());
    active.update(db).await?;

    cache.invalidate(user_id);
    tracing::info!(event = "password_changed", user_id = %user_id, "Password changed");

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id: user_id,
            entity_type: "usuario".to_string(),
            entity_id: user_id,
            accion: "cambiar_password".to_string(),
            cambios: serde_json::json!({}),
        },
    )
    .await;

    Ok(())
}
