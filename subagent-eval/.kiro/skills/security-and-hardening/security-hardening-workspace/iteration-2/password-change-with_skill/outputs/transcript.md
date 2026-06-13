# Password Change Endpoint — Security Reasoning

## Task

Add `POST /api/v1/usuarios/{id}/cambiar-password` that lets an authenticated user change their own password by providing `current_password` and `new_password`.

## Approach

I followed the existing project patterns closely:

1. **Existing precedent**: The project already has `PUT /api/v1/perfil/password` in `handlers/perfil.rs` + `services/perfil.rs` with the same logic. The new endpoint at `/usuarios/{id}/cambiar-password` adds the path-based variant under the usuarios scope.

2. **Layered architecture**: Handler validates input and authorization → service performs password verification and update → entities handle persistence.

3. **Auth extractor**: Used `Claims` (not `AdminOnly` or `WriteAccess`) because this is a self-service operation — any authenticated user can change their own password. The handler enforces `path_id == claims.sub`.

## Security Considerations Applied (from SKILL.md)

### 1. Authorization — User can only change their own password
- The `{id}` path parameter is compared against `claims.sub` from the JWT.
- If they don't match, returns `AppError::Forbidden`.
- This prevents any user from changing another user's password via this endpoint.

### 2. Input validation at handler boundary
- `current_password` capped at 128 chars — prevents DoS via Argon2 on huge inputs.
- `new_password` minimum 8 chars — enforces password strength floor.
- `new_password` maximum 128 chars — same DoS prevention.
- Validation happens before any database or crypto operations.

### 3. Constant-time password verification
- Uses `argon2::verify_password` which performs constant-time comparison internally.
- Safe from timing attacks — an attacker cannot determine if the current password is partially correct.

### 4. Generic error messages
- Wrong current password returns "La contraseña actual es incorrecta" (same message whether user exists or password is wrong from the service perspective).
- No information leakage about password hash format or internal state.

### 5. Password hashing with OWASP-recommended params
- Uses `Argon2::default()` which provides Argon2id with m=19456 KiB, t=2, p=1.
- Fresh random salt generated per hash via `SaltString::generate(&mut OsRng)`.

### 6. Rate limiting
- The `/usuarios` scope already has `write_governor_conf` (2s/req, burst 20).
- This protects the password change endpoint from brute-force attempts on `current_password`.

### 7. Audit logging
- `tracing::info!` logs the user_id when password is changed (security event per SKILL.md).
- Never logs password values or hashes.

### 8. No internal details in error responses
- All errors use `AppError` variants that map to safe JSON responses.
- Internal errors (hash failures) are wrapped in `AppError::Internal` which returns a generic 500.

### 9. serde camelCase for request body
- Matches project convention: `#[serde(rename_all = "camelCase")]`.
- Client sends `{ "currentPassword": "...", "newPassword": "..." }`.

## What I did NOT add (simplicity first)

- No password complexity rules beyond minimum length (not requested, matches existing pattern).
- No "confirm new password" field (that's a frontend concern).
- No session invalidation after password change (no refresh token infrastructure exists).
- No notification email (not part of the request).

## Files

| File | Purpose |
|------|---------|
| `handler.rs` | Handler function added to `handlers/usuarios.rs` |
| `service.rs` | Service function added to `services/usuarios.rs` |
| `routes.rs` | Route registration added to the `/usuarios` scope in `routes.rs` |
