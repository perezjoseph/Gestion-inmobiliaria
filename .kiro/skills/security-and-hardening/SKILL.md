---
name: security-and-hardening
description: Hardens Rust/Actix-web backend code against vulnerabilities. Use when handling user input, authentication, authorization, file uploads, database queries, or external integrations. Use when building any handler that accepts untrusted data, manages JWT sessions, touches passwords, or interacts with third-party services. Use when creating any new endpoint, adding or updating dependencies, or modifying application configuration (app.rs, config.rs, .env). Also use when reviewing code for security issues, adding rate limiting, configuring CORS, or implementing audit logging. Trigger this skill whenever security, auth, permissions, input validation, secrets, OWASP, XSS, injection, CSRF, hardening, export, or data access are mentioned — even tangentially.
---

# Security and Hardening

Security-first patterns for Rust/Actix-web 4/SeaORM backends. Every external input is hostile, every secret is sacred, every authorization check is mandatory.

## Decision Quick-Reference

### Which extractor?

- Read endpoint → `Claims`
- Write endpoint (POST/PUT/DELETE on data) → `WriteAccess`
- User management (roles, activate/deactivate) → `AdminOnly`
- GET with side effects (e.g., mark-as-read) → `WriteAccess` (it mutates data)
- Public (health, login, register) → no extractor

`Claims` fields: `sub` (user UUID), `email`, `rol` (role string), `organizacion_id` (tenant UUID), `exp` (expiration).
`WriteAccess` and `AdminOnly` wrap `Claims` — access inner claims via `.0`.

### Which error type?

- User can fix it (bad input, wrong format, missing field) → `AppError::Validation("message")`
- Resource doesn't exist → `AppError::NotFound("...")`
- Cross-tenant access attempt → `AppError::NotFound("...")` (never reveal the resource exists to another tenant)
- Wrong role for this action → `AppError::Forbidden`
- Duplicate unique value (email, cedula) → `AppError::Conflict("...")`
- Missing or invalid JWT token → `AppError::Unauthorized(None)` (handled by middleware)
- Malformed request body (invalid JSON) → handled by framework via `JsonConfig` error handler
- Unexpected internal failure → `AppError::Internal(err.into())`

### Which validation approach?

- Single field, single handler → inline check in handler
- Field reused across multiple handlers → newtype with constructor validation
- Complex multi-field business rule → service layer validation
- Enum/state values → explicit allowlist check (don't rely on serde alone)

### When to add rate limiting?

- Authentication endpoints → strict (6s/req, burst 10)
- Write-heavy or expensive operations (uploads, imports, exports) → moderate (2s/req, burst 20)
- New expensive read endpoint (reports, bulk exports) → wrap with write_governor

### When to log security events?

- `tracing::warn!` — failed auth, authorization denied, cross-tenant attempt, suspicious input
- `tracing::error!` — unexpected token failure, data integrity violation
- `tracing::info!` — successful login, data export, role change, account creation

## Actix-web 4 Security Patterns

### Authentication via FromRequest Extractors

The idiomatic Actix-web pattern for auth is custom `FromRequest` extractors — not middleware. This gives compile-time enforcement: handlers declare their auth requirements in the function signature.

```rust
impl FromRequest for Claims {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(extract_claims_from_header(req))
    }
}
```

Role-based extractors wrap `Claims` and check the role before the handler runs:
```rust
impl FromRequest for WriteAccess {
    // Extracts Claims, then verifies rol == "admin" || "gerente"
    // Returns AppError::Forbidden if role doesn't match
}
```

### Security Headers (DefaultHeaders middleware)

Add before CORS in the middleware chain. Actix-web processes `.wrap()` in reverse order — last registered runs first on request:

```rust
use actix_web::middleware::DefaultHeaders;

.wrap(DefaultHeaders::new()
    .add(("X-Content-Type-Options", "nosniff"))
    .add(("X-Frame-Options", "DENY"))
    .add(("X-XSS-Protection", "0"))
    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
    .add(("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'"))
    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
    .add(("Cache-Control", "no-store"))  // API routes only — exclude static file serving
)
```

Notes:
- `X-XSS-Protection: 0` — OWASP recommends disabling (legacy XSS auditors can introduce vulnerabilities)
- `Cache-Control: no-store` — prevents caching of API responses with sensitive data. Apply only to `/api/*` routes, NOT to static file serving (images, documents need caching for performance)
- `Content-Security-Policy: default-src 'none'` — for API-only backends; adjust for frontends
- Only set HSTS when behind TLS termination

### Rate Limiting (actix-governor 0.6)

In-memory token bucket per worker. Effective burst = `burst_size × N workers`.

```rust
use actix_governor::{Governor, GovernorConfigBuilder};

let auth_governor = GovernorConfigBuilder::default()
    .seconds_per_request(6)
    .burst_size(10)
    .finish()
    .expect("valid config");

web::scope("/auth").wrap(Governor::new(&auth_governor))
```

For production with multiple instances, rate limit at the reverse proxy layer (nginx `limit_req`, AWS WAF) or use Redis-backed `actix-limitation`.

### Request Size Limits

```rust
let json_cfg = JsonConfig::default()
    .limit(1_048_576) // 1 MB max body
    .error_handler(|err, _req| {
        actix_web::error::InternalError::from_response(
            err,
            AppError::Validation(err.to_string()).error_response(),
        ).into()
    });
```

## jsonwebtoken (v10) Security Configuration

The `jsonwebtoken` crate v10 requires explicit crypto provider installation and supports enhanced validation:

```rust
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};

fn secure_validation() -> Validation {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_required_spec_claims(&["exp", "sub"]);
    v.leeway = 30;                              // Clock skew tolerance (seconds)
    v.reject_tokens_expiring_in_less_than = 60; // Reject tokens about to expire
    v.validate_nbf = true;                      // Validate "not before" if present
    // For multi-service: v.set_issuer(&["https://auth.example.com"]);
    // For audience: v.set_audience(&["https://api.example.com"]);
    v
}

pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &secure_validation())
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthorized(None))
}
```

Key points:
- Always restrict algorithm (`Validation::new(Algorithm::HS256)`) — prevents algorithm confusion attacks
- `reject_tokens_expiring_in_less_than` is new in v10 — optional, only useful when token expiry is short (≤1 hour). Skip if using long-lived tokens.
- JWT secret must be ≥ 32 bytes for HS256
- Token expiration depends on whether refresh tokens are implemented:
  - With refresh tokens: access token ≤ 30 minutes, refresh token 7 days
  - Without refresh tokens: ≤ 8 hours is a reasonable balance of security vs. UX
  - Never reduce token expiry without implementing refresh tokens first — it will log out all users

## argon2 (v0.5) Password Hashing

OWASP-recommended parameters for Argon2id (2025): m=19456 KiB, t=2, p=1. The `Argon2::default()` in the `argon2` 0.5 crate already uses these values.

```rust
use argon2::{Argon2, Algorithm, Params, Version};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng};

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()  // Argon2id, m=19456, t=2, p=1
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Hash error: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(hash: &str, password: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid hash: {e}")))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}
```

- `verify_password` uses constant-time comparison internally — safe from timing attacks
- Hash verification should take 50–200ms on production hardware
- Never log password hashes or plaintext passwords

## SeaORM Security

### SQL Injection Prevention

| Method | Safe? |
|--------|-------|
| Query builder (`.find()`, `.filter()`, `.insert()`) | ✅ Always parameterized |
| `Statement::from_sql_and_values()` | ✅ Uses `$1`, `$2` placeholders |
| `Statement::from_string()` with static SQL | ✅ If no user input interpolated |
| `Statement::from_string()` with `format!()` user input | ❌ SQL injection |
| `db.execute_unprepared()` | ❌ Raw SQL, no parameterization |

**Rule**: Use the query builder for all application queries. Reserve `Statement::from_string()` only for static DDL (migrations, extensions).

### Multi-Tenant Query Scoping

Every query must filter by the tenant identifier from the authenticated user's claims. A missing filter = cross-tenant data leak.

```rust
Entity::find()
    .filter(Column::OrganizacionId.eq(claims.organizacion_id))
    .all(db)
    .await?
```

For nested resources, verify parent ownership before creating children. This includes polymorphic attachments (documents, notes, files) — before attaching to an entity, confirm the entity belongs to the user's organization:

```rust
// Before creating a child record (e.g., Documento for a Propiedad):
let parent = propiedad::Entity::find_by_id(entity_id)
    .filter(propiedad::Column::OrganizacionId.eq(claims.organizacion_id))
    .one(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Entidad no encontrada".into()))?;
// Now safe to create the child record referencing parent.id
```

For polymorphic entity references (where `entity_type` + `entity_id` can point to different tables), validate ownership by dispatching on the entity type:

```rust
match entity_type {
    "propiedad" => {
        propiedad::Entity::find_by_id(entity_id)
            .filter(propiedad::Column::OrganizacionId.eq(org_id))
            .one(db).await?
            .ok_or_else(|| AppError::NotFound("Entidad no encontrada".into()))?;
    }
    "contrato" => {
        contrato::Entity::find_by_id(entity_id)
            .filter(contrato::Column::OrganizacionId.eq(org_id))
            .one(db).await?
            .ok_or_else(|| AppError::NotFound("Entidad no encontrada".into()))?;
    }
    // ... other entity types
    _ => return Err(AppError::Validation("Tipo de entidad no válido".into())),
}
```

This prevents IDOR attacks where a user provides a valid `entity_id` belonging to another organization.

## Input Validation

Validate at the handler boundary, not in the service layer. The handler is the first code you control after deserialization — reject invalid input here before it reaches business logic or crypto operations. Even if a service also validates, the handler must independently check. This prevents DoS (e.g., hashing a 1MB password with argon2) and gives clear, immediate error responses.

```rust
pub async fn create_handler(
    _access: WriteAccess,
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateDto>,
) -> Result<HttpResponse, AppError> {
    let dto = body.into_inner();

    if dto.name.trim().is_empty() {
        return Err(AppError::Validation("Name is required".into()));
    }
    if dto.name.len() > 100 {
        return Err(AppError::Validation("Name exceeds 100 characters".into()));
    }

    let result = service::create(&*db, dto).await?;
    Ok(HttpResponse::Created().json(result))
}
```

For password endpoints specifically, always validate length bounds in the handler before calling any hashing function:
```rust
// In the handler — BEFORE calling the service:
if input.new_password.len() < 8 {
    return Err(AppError::Validation("La contraseña debe tener al menos 8 caracteres".into()));
}
if input.new_password.len() > 128 || input.current_password.len() > 128 {
    return Err(AppError::Validation("Contraseña excede longitud máxima".into()));
}
// Now safe to call the service which will hash with argon2
```

Key validations:
- String max lengths matching DB VARCHAR constraints
- Required fields non-empty after `.trim()`
- Email format: reject if missing `@` or domain part is empty
- Enum values against explicit allowlist:
  ```rust
  const VALID_STATES: &[&str] = &["pending", "active", "cancelled"];
  if !VALID_STATES.contains(&dto.state.as_str()) {
      return Err(AppError::Validation(format!("Invalid state: {}", dto.state)));
  }
  ```
- Monetary amounts: positive, ≤ 2 decimal places (`decimal.scale() <= 2`)
- Dates: start < end, reject unreasonable ranges (>10 years past, >5 years future)
- UUIDs in path params: handled by `web::Path<Uuid>`
- `#[serde(deny_unknown_fields)]` — use cautiously. Prevents unknown fields but breaks forward compatibility during rolling deployments. Rust's `ActiveModel` already prevents mass assignment (you explicitly `Set()` each field). Only use on DTOs where accepting unknown fields is a security risk, not as a blanket rule.

## Error Response Safety

Never leak internal details. Log server-side, return generic to client.

```rust
// GOOD
tracing::error!(?err, "Database operation failed");
Err(AppError::Internal(err.into()))

// BAD — leaks DB schema info
Err(AppError::BadRequest(format!("DB error: {err}")))
```

## Data Export Security

Export endpoints (CSV, PDF, XLSX) return bulk data and need additional protection:

- Wrap with rate limiter (expensive operation)
- Require at least one date boundary — default to last 12 months if none provided
- Cap date ranges (max 2 years per request) or cap row count at 50k
- Log every export with structured fields
- Set `Content-Disposition: attachment` and expose via CORS

## File Upload Safety

- Validate extension against allowlist
- Validate file size (reject before reading full body)
- Validate magic bytes with `infer` crate (don't trust extension alone)
- Prevent path traversal: reject `..`, `/`, `\` in filenames
- Store with UUID-based names (never user-provided filenames)
- Serve through authenticated handler — `Files::new()` without auth exposes all uploads to anyone with the URL. Migration: add authenticated endpoint first, update all clients, then remove static serving.

## Supply Chain Security

- `cargo deny check` on every PR (advisories, licenses, bans, sources)
- `cargo audit` via scheduled CI
- Pin exact versions for security-critical deps (`argon2`, `jsonwebtoken`)
- Document advisory ignores with justification in `deny.toml`
- `gitleaks` for secret detection in pre-commit and CI

## Common AI Mistakes to Avoid

- Do NOT create endpoints without an auth extractor unless explicitly asked for a public endpoint
- Do NOT use `format!("error: {err}")` in error responses — use `AppError::Internal(err.into())`
- Do NOT forget tenant-scoping filter on queries
- Do NOT skip entity ownership verification on polymorphic attachments (documents, files, notes) — always confirm the target entity belongs to the user's org before creating the child record
- Do NOT return `Forbidden` for cross-tenant access — use `NotFound` (don't reveal resource exists)
- Do NOT add `#[derive(Debug)]` to structs containing secrets
- Do NOT use `Statement::from_string()` with user input
- Do NOT rely on service-layer validation alone — validate input bounds (string lengths, password lengths) in the handler before calling services that perform expensive operations (hashing, DB writes)
- Do NOT buffer unbounded query results — cap with pagination or date range limits
- Do NOT create export/download endpoints without a default date boundary and row count cap
- Do NOT serve files via `Files::new()` without authentication

## References

For routine handler work (validation + tenant-scoping + error handling), this file is sufficient. Load references only when implementing security infrastructure.

- **`references/actix-security-patterns.md`** — Load when: security headers setup, rate limiting changes, CORS hardening, file upload handler, or custom extractors.
- **`references/attack-prevention.md`** — Load when: auth flows, timing attacks, IDOR patterns, enumeration mitigation, or DoS prevention.
- **`references/supply-chain.md`** — Load when: adding dependencies, triaging advisories, CI security jobs, or deny.toml review.
- **`references/security-checklist.md`** — Load when: PR security review, deployment prep, periodic audits, or incident response.

---

## Security Review Checklist (Quick)

- [ ] All handlers use appropriate extractor (`Claims`, `WriteAccess`, `AdminOnly`)
- [ ] Queries filter by tenant ID
- [ ] Input validated at handler boundary (lengths, formats, required fields)
- [ ] No secrets in code or logs
- [ ] Error responses don't expose internals
- [ ] File uploads validated (size, extension, path traversal, magic bytes)
- [ ] Rate limiting on auth and expensive endpoints
- [ ] JWT validation uses restricted algorithm + required claims
- [ ] New dependencies audited (`cargo deny check`)
- [ ] No new `unsafe` blocks in production code
- [ ] Data exports logged to audit trail
- [ ] CORS origin explicitly set for production
