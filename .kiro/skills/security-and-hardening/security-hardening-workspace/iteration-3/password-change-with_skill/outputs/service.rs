use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::services::auth::{hash_password, verify_password};

/// Changes a user's password after verifying the current one.
///
/// This function assumes input validation (length bounds) has already been
/// performed at the handler boundary. It:
/// 1. Fetches the user record by ID.
/// 2. Verifies the current password against the stored argon2 hash.
/// 3. Hashes the new password with argon2id (OWASP defaults).
/// 4. Updates the record.
///
/// Security notes:
/// - Returns a generic "incorrect password" error for wrong current_password
///   (no timing oracle — argon2 verify is constant-time internally).
/// - Never logs password values or hashes.
pub async fn cambiar_password(
    db: &DatabaseConnection,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let record = usuario::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".into()))?;

    let valid = verify_password(&record.password_hash, current_password)?;
    if !valid {
        return Err(AppError::Validation(
            "La contraseña actual es incorrecta".into(),
        ));
    }

    let new_hash = hash_password(new_password)?;
    let mut active: usuario::ActiveModel = record.into();
    active.password_hash = Set(new_hash);
    active.update(db).await?;

    Ok(())
}
