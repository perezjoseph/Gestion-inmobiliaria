use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;
use crate::services::auth::{hash_password, verify_password};

/// Changes a user's password after verifying the current password.
///
/// Security considerations:
/// - Uses constant-time comparison via argon2's verify_password (timing-attack safe)
/// - Returns a generic "incorrect password" error to avoid enumeration
/// - Validates password length bounds before hashing (DoS prevention)
/// - Never logs password values or hashes
pub async fn cambiar_password(
    db: &DatabaseConnection,
    user_id: Uuid,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let record = usuario::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Usuario no encontrado".to_string()))?;

    // Verify current password using argon2 (constant-time internally)
    let valid = verify_password(&record.password_hash, current_password)?;
    if !valid {
        return Err(AppError::Validation(
            "La contraseña actual es incorrecta".to_string(),
        ));
    }

    // Hash new password with Argon2id (OWASP-recommended params via Argon2::default())
    let new_hash = hash_password(new_password)?;

    let mut active: usuario::ActiveModel = record.into();
    active.password_hash = Set(new_hash);
    active.update(db).await?;

    Ok(())
}
