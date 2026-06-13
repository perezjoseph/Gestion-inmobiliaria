// No new service code needed.
//
// The handler reuses the existing `perfil::cambiar_password` function located at
// backend/src/services/perfil.rs which already:
//   1. Validates password length constraints (min 8, max 128)
//   2. Fetches the user record by ID
//   3. Verifies the current password against the stored argon2 hash
//   4. Hashes the new password with argon2 + random salt
//   5. Updates the password_hash column
//
// There is no reason to duplicate this logic. The authorization check
// (user can only change their own password) is enforced in the handler
// by comparing claims.sub == path id before calling the service.
