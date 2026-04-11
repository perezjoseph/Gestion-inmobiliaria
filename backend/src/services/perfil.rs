use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::models::usuario::UsuarioResponse;
use crate::services::auth::{hash_password, verify_password};

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
) -> Result<(), AppError> {
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
    active.update(db).await?;

    Ok(())
}
