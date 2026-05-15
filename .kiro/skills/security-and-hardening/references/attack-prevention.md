# Attack Prevention Patterns

Advanced security patterns for preventing specific attack vectors in Rust/Actix-web APIs. Read this when implementing authentication flows, handling sensitive comparisons, or defending against IDOR, timing attacks, and deserialization exploits.

## Table of Contents

1. [Timing Attacks](#timing-attacks)
2. [Insecure Direct Object References (IDOR)](#insecure-direct-object-references-idor)
3. [Insecure Deserialization](#insecure-deserialization)
4. [Enumeration Attacks](#enumeration-attacks)
5. [Request ID Tracing](#request-id-tracing)
6. [Content-Type Enforcement](#content-type-enforcement)
7. [Denial of Service Prevention](#denial-of-service-prevention)

---

## Timing Attacks

Timing attacks exploit measurable differences in execution time to infer secret data. In web APIs, the most common vector is token/password comparison.

### The Problem

Standard string comparison (`==`) short-circuits on the first mismatched byte. An attacker can measure response times to deduce secrets character by character.

```rust
// VULNERABLE — short-circuits on first mismatch
fn check_token(provided: &str, expected: &str) -> bool {
    provided == expected  // Returns faster when first chars differ
}
```

### The Solution: Constant-Time Comparison

Use the `subtle` crate for constant-time equality checks on security-sensitive values:

```rust
use subtle::ConstantTimeEq;

/// Compare two tokens in constant time — prevents timing attacks
fn secure_token_compare(provided: &[u8], expected: &[u8]) -> bool {
    if provided.len() != expected.len() {
        return false;
    }
    provided.ct_eq(expected).into()
}
```

### Where This Matters in This Codebase

- **Password verification**: Already safe — `argon2::verify_password` uses constant-time comparison internally
- **JWT validation**: Already safe — `jsonwebtoken` handles HMAC comparison securely
- **API keys or webhook secrets**: If you add these, use `subtle::ConstantTimeEq`
- **Password reset tokens**: If comparing raw tokens (not hashed), use constant-time comparison
- **Invitation tokens**: The `invitaciones::validar_token` should use constant-time comparison if comparing raw strings

### Adding the `subtle` Crate

```toml
# Cargo.toml
subtle = "2"
```

```rust
use subtle::ConstantTimeEq;

pub fn verify_reset_token(provided: &str, stored_hash: &str) -> bool {
    // If tokens are hashed (recommended), use argon2::verify
    // If tokens are raw strings, use constant-time comparison:
    let provided_bytes = provided.as_bytes();
    let stored_bytes = stored_hash.as_bytes();

    if provided_bytes.len() != stored_bytes.len() {
        return false;
    }
    provided_bytes.ct_eq(stored_bytes).into()
}
```

---

## Insecure Direct Object References (IDOR)

IDOR occurs when an API uses client-supplied identifiers (like UUIDs in path params) to access resources without verifying the caller is authorized to access that specific resource.

### The Problem

```rust
// VULNERABLE — no ownership check
pub async fn obtener_contrato(
    _claims: Claims,  // Only checks authentication, not authorization
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    // Any authenticated user can access ANY contrato by guessing UUIDs
    let contrato = contratos::Entity::find_by_id(contrato_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".into()))?;
    Ok(HttpResponse::Ok().json(contrato))
}
```

### The Solution: Always Scope by Organization

```rust
// SECURE — scoped to user's organization
pub async fn obtener_contrato(
    claims: Claims,
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let contrato_id = path.into_inner();
    let contrato = contratos::Entity::find_by_id(contrato_id)
        .filter(contratos::Column::OrganizacionId.eq(claims.organizacion_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".into()))?;
    Ok(HttpResponse::Ok().json(contrato))
}
```

### IDOR Prevention Rules

1. **Every query must filter by `organizacion_id`** — this is the primary tenant isolation mechanism
2. **Return 404, not 403** — don't reveal that a resource exists in another org
3. **Verify parent ownership for nested resources** — when creating a pago, verify the contrato belongs to the user's org
4. **Don't trust path params alone** — always cross-reference with the authenticated user's scope

### Nested Resource Verification Pattern

```rust
// When creating a pago, verify the contrato belongs to the user's org
pub async fn crear_pago(
    access: WriteAccess,
    db: web::Data<DatabaseConnection>,
    body: web::Json<CrearPagoDto>,
) -> Result<HttpResponse, AppError> {
    let dto = body.into_inner();

    // Verify the parent contrato belongs to this org
    let contrato = contratos::Entity::find_by_id(dto.contrato_id)
        .filter(contratos::Column::OrganizacionId.eq(access.0.organizacion_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Contrato no encontrado".into()))?;

    // Now safe to create the pago
    let pago = pagos_service::crear(db.as_ref(), dto, &contrato).await?;
    Ok(HttpResponse::Created().json(pago))
}
```

---

## Insecure Deserialization

Serde deserialization in Rust is safer than in dynamic languages (no arbitrary code execution from deserialization alone), but logic vulnerabilities remain.

### The Problem: Privilege Escalation via Unvalidated Fields

```rust
// VULNERABLE — attacker can set their own role
#[derive(Deserialize)]
pub struct UpdateUsuarioDto {
    pub nombre: Option<String>,
    pub email: Option<String>,
    pub rol: Option<String>,      // Attacker sends: {"rol": "admin"}
    pub activo: Option<bool>,     // Attacker sends: {"activo": true}
}
```

### The Solution: Separate DTOs by Permission Level

```rust
// For self-service profile updates (any authenticated user)
#[derive(Deserialize)]
pub struct UpdatePerfilDto {
    pub nombre: Option<String>,
    // No rol, no activo — users can't escalate themselves
}

// For admin-only user management
#[derive(Deserialize)]
pub struct AdminUpdateUsuarioDto {
    pub nombre: Option<String>,
    pub email: Option<String>,
    pub rol: Option<String>,
    pub activo: Option<bool>,
}
```

### Deny Unknown Fields

Prevent clients from sending unexpected fields that might be processed later:

```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrearContratoDto {
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub monto_mensual: Decimal,
    pub moneda: String,
}
```

### Validate Enum Values Explicitly

Don't rely solely on serde for business-critical enums:

```rust
#[derive(Deserialize)]
pub struct UpdateEstadoDto {
    pub estado: String,
}

// In the handler or service:
const VALID_ESTADOS_CONTRATO: &[&str] = &["activo", "vencido", "cancelado", "finalizado", "terminado"];

fn validate_estado_contrato(estado: &str) -> Result<(), AppError> {
    if !VALID_ESTADOS_CONTRATO.contains(&estado) {
        return Err(AppError::Validation(format!(
            "Estado inválido: {estado}. Valores permitidos: {}",
            VALID_ESTADOS_CONTRATO.join(", ")
        )));
    }
    Ok(())
}
```

---

## Enumeration Attacks

Attackers probe APIs to discover valid usernames, emails, or resource IDs by observing different responses.

### The Problem

```rust
// VULNERABLE — reveals whether email exists
pub async fn login(input: LoginInput) -> Result<TokenResponse, AppError> {
    let user = find_by_email(&input.email).await?;
    match user {
        None => Err(AppError::NotFound("Usuario no encontrado".into())),  // Reveals: email doesn't exist
        Some(u) => {
            if !verify_password(&u.password_hash, &input.password)? {
                Err(AppError::Unauthorized(Some("Contraseña incorrecta".into())))  // Reveals: email exists
            } else {
                Ok(generate_token(&u)?)
            }
        }
    }
}
```

### The Solution: Uniform Error Responses

```rust
// SECURE — same response regardless of failure reason
pub async fn login(input: LoginInput) -> Result<TokenResponse, AppError> {
    let user = find_by_email(&input.email).await?;

    let auth_failed = || AppError::Unauthorized(None);  // Generic "No autorizado"

    match user {
        None => {
            // Perform a dummy hash to prevent timing-based enumeration
            let argon2 = Argon2::default();
            let salt = SaltString::generate(&mut OsRng);
            let _ = argon2.hash_password(b"dummy", &salt);
            Err(auth_failed())
        }
        Some(u) if !u.activo => Err(auth_failed()),
        Some(u) => {
            if !verify_password(&u.password_hash, &input.password)? {
                Err(auth_failed())
            } else {
                Ok(generate_token(&u)?)
            }
        }
    }
}
```

Key principle: The response (status code, body, and timing) should be indistinguishable whether the email exists or not.

### Registration Enumeration

Same principle applies to registration:

```rust
// Don't say "email already exists" — say "check your email for verification"
// Then only send the verification email if the account is actually new
```

---

## Request ID Tracing

The project already uses `tracing-actix-web` (TracingLogger). Enhance it for security event correlation.

### Adding Request IDs

`tracing-actix-web` automatically generates a request ID for each request. Access it in handlers:

```rust
use tracing_actix_web::RequestId;

pub async fn some_handler(
    request_id: RequestId,
    claims: Claims,
) -> Result<HttpResponse, AppError> {
    tracing::info!(
        request_id = %request_id,
        user_id = %claims.sub,
        "Processing request"
    );
    // ...
}
```

### Correlating Security Events

When a security event occurs, include the request ID so you can trace the full request lifecycle:

```rust
tracing::warn!(
    request_id = %request_id,
    event_type = "cross_tenant_attempt",
    user_id = %claims.sub,
    user_org = %claims.organizacion_id,
    target_resource = %resource_id,
    "SECURITY: Cross-tenant access attempt detected"
);
```

### Returning Request IDs to Clients

Add the request ID to response headers for debugging (but not in error responses that might aid attackers):

```rust
use actix_web::HttpResponseBuilder;

// In success responses only
response.insert_header(("X-Request-Id", request_id.to_string()));
```

---

## Content-Type Enforcement

Ensure handlers only accept the content types they expect.

### The Problem

Without enforcement, an attacker might send `text/plain` or `application/xml` to a JSON endpoint, potentially bypassing WAF rules or triggering unexpected parsing behavior.

### The Solution

Actix-web's `web::Json<T>` extractor already rejects non-JSON content types with a 400 error. But for multipart endpoints, be explicit:

```rust
// The JsonConfig error handler (already in app.rs) catches content-type mismatches
let json_cfg = JsonConfig::default()
    .limit(1_048_576)
    .content_type(|mime| mime == mime::APPLICATION_JSON)  // Strict content-type check
    .error_handler(|err, _req| {
        // Returns 422 for content-type or parse errors
        let message = err.to_string();
        actix_web::error::InternalError::from_response(
            err,
            AppError::Validation(message).error_response(),
        ).into()
    });
```

---

## Denial of Service Prevention

Beyond rate limiting, protect against resource exhaustion attacks.

### Request Body Bombs

Already mitigated by `JsonConfig::limit(1_048_576)` (1 MB). For multipart:

```rust
// Limit total upload size
MultipartFormConfig::default()
    .total_limit(10 * 1024 * 1024)  // 10 MB max total
    .memory_limit(2 * 1024 * 1024)  // 2 MB in memory before disk spill
```

### Regex DoS (ReDoS)

If using regex for input validation, avoid patterns vulnerable to catastrophic backtracking:

```rust
// BAD — vulnerable to ReDoS with input like "aaaaaaaaaaaaaaaaaa!"
let re = Regex::new(r"^(a+)+$").unwrap();

// GOOD — use non-backtracking patterns or the `regex` crate (which is ReDoS-safe)
// Rust's `regex` crate uses finite automata, making it immune to ReDoS by design
let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
```

Note: Rust's standard `regex` crate is inherently safe from ReDoS because it uses Thompson's NFA construction, not backtracking. This is a significant advantage over regex engines in Python, JavaScript, or Java.

### Query Complexity Limits

Prevent expensive database queries from list endpoints:

```rust
// Limit pagination to prevent full-table scans
const MAX_PAGE_SIZE: u64 = 100;

pub fn validate_pagination(page: u64, page_size: u64) -> Result<(u64, u64), AppError> {
    let page = page.max(1);
    let page_size = page_size.min(MAX_PAGE_SIZE).max(1);
    Ok((page, page_size))
}
```

### Connection Pool Exhaustion

Already configured in `config.rs` with pool limits. Additional protection:

```rust
// Set query timeout to prevent long-running queries from holding connections
let mut opts = ConnectOptions::new(&self.database_url);
opts.sqlx_logging(false)
    .acquire_timeout(Duration::from_secs(5));  // Don't wait forever for a connection
```

### Slowloris Prevention

Actix-web has built-in protection via `HttpServer` configuration:

```rust
HttpServer::new(|| app)
    .client_request_timeout(Duration::from_secs(5))   // Max time to receive full request
    .client_disconnect_timeout(Duration::from_secs(5)) // Max time for client disconnect
    .keep_alive(Duration::from_secs(75))               // Keep-alive timeout
```
