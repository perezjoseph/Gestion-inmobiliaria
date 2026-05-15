// No new service code needed.
//
// The handler reuses the existing `services::perfil::cambiar_password` function
// which already implements:
//
// 1. Input validation (password length limits to prevent Argon2 DoS)
// 2. User lookup by ID
// 3. Current password verification via argon2 `verify_password`
// 4. New password hashing via argon2 `hash_password`
// 5. Database update of the password_hash field
//
// Located at: backend/src/services/perfil.rs
//
// Signature:
//   pub async fn cambiar_password(
//       db: &DatabaseConnection,
//       user_id: Uuid,
//       password_actual: &str,
//       password_nuevo: &str,
//   ) -> Result<(), AppError>
//
// This avoids code duplication and keeps password change logic in one place.
