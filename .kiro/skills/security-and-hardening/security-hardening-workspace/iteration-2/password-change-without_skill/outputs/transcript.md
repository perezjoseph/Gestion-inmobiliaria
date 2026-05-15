# Password Change Endpoint — Implementation Transcript

## Task

Add `POST /api/v1/usuarios/{id}/cambiar-password` that lets an authenticated user change their own password by providing `current_password` and `new_password`.

## Reasoning & Decisions

### 1. Reuse existing service logic

The project already has `perfil::cambiar_password` in `backend/src/services/perfil.rs` which does exactly what's needed:
- Validates password length (min 8, max 128 chars)
- Fetches user by ID
- Verifies current password via `auth::verify_password` (argon2)
- Hashes new password via `auth::hash_password` (argon2 + random salt)
- Updates the DB record

No new service code is needed. Duplicating this logic would violate DRY.

### 2. Authorization: claims.sub == path id

The endpoint accepts a `{id}` path parameter but must only allow users to change their own password. The handler extracts `Claims` from the request (via the existing auth middleware/extractor) and compares `claims.sub` to the path `id`. If they don't match, it returns `403 Forbidden`.

This is the simplest secure approach — no admin override for password changes.

### 3. Request body uses camelCase

Following the project convention (`#[serde(rename_all = "camelCase")]`), the JSON body fields are:
```json
{
  "currentPassword": "...",
  "newPassword": "..."
}
```

The struct fields are named `current_password` and `new_password` as requested in the task description.

### 4. Route registration

Added to the existing `/usuarios` scope which already has the `write_governor_conf` rate limiter. Using `POST` since this is a command/action (not a resource update). The route sits alongside the existing `/{id}/rol`, `/{id}/activar`, `/{id}/desactivar` routes.

### 5. Security considerations

- **Rate limiting**: Inherited from the `/usuarios` scope's `Governor` wrapper (2 seconds per request, burst 20).
- **Password length cap**: The service rejects passwords > 128 chars before hashing, preventing argon2 DoS.
- **Timing-safe comparison**: argon2's `verify_password` uses constant-time comparison internally.
- **No information leakage**: The error message for wrong current password is generic ("La contraseña actual es incorrecta") — same whether user exists or not (though the auth middleware already ensures the user exists).
- **Audit logging**: `tracing::info!` logs the user_id on successful password change.

### 6. What already exists

There's already `PUT /api/v1/perfil/password` that does the same thing using the authenticated user's own ID from claims. The new endpoint at `/api/v1/usuarios/{id}/cambiar-password` is an alternative path that takes the ID explicitly — useful for API consistency with the usuarios resource namespace.
