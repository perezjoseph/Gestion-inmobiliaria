# Actix-Web Security Patterns

Detailed implementation patterns for security middleware and configuration in Actix-web 4. Read this when implementing security features or reviewing security-related code.

## Table of Contents

1. [Security Headers Middleware](#security-headers-middleware)
2. [Rate Limiting Configuration](#rate-limiting-configuration)
3. [CORS Configuration](#cors-configuration)
4. [Request Size Limits](#request-size-limits)
5. [Custom Auth Extractors](#custom-auth-extractors)
6. [Error Handler Middleware](#error-handler-middleware)
7. [Request Logging for Security](#request-logging-for-security)
8. [File Upload Security](#file-upload-security)

---

## Security Headers Middleware

Actix-web provides `DefaultHeaders` middleware for adding security headers to all responses.

### Basic Setup

```rust
use actix_web::middleware::DefaultHeaders;

// Add to create_app() in app.rs, BEFORE the cors middleware
.wrap(DefaultHeaders::new()
    .add(("X-Content-Type-Options", "nosniff"))
    .add(("X-Frame-Options", "DENY"))
    .add(("X-XSS-Protection", "0"))
    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
    .add(("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"))
    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
    .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"))
)
```

### Header Details

- **X-Content-Type-Options** = `nosniff` — Prevents MIME-type sniffing; browser won't execute files with wrong content-type
- **X-Frame-Options** = `DENY` — Prevents clickjacking; page cannot be embedded in iframes
- **X-XSS-Protection** = `0` — Disabled; legacy XSS filter can introduce vulnerabilities. Use CSP instead
- **Strict-Transport-Security** = `max-age=31536000; includeSubDomains` — Forces HTTPS for 1 year. Only set when behind TLS
- **Content-Security-Policy** = `default-src 'self'` — Restricts resource loading to same origin; primary XSS defense
- **Referrer-Policy** = `strict-origin-when-cross-origin` — Sends origin only for cross-origin requests, full URL for same-origin
- **Permissions-Policy** = `camera=(), microphone=(), geolocation=()` — Disables browser APIs the app doesn't need

### Environment-Aware Headers

HSTS should only be set in production (behind TLS). Make it conditional:

```rust
fn security_headers(config: &AppConfig) -> DefaultHeaders {
    let mut headers = DefaultHeaders::new()
        .add(("X-Content-Type-Options", "nosniff"))
        .add(("X-Frame-Options", "DENY"))
        .add(("X-XSS-Protection", "0"))
        .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
        .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=()"));

    // Only add HSTS in production (when CORS_ORIGIN is set, implying production config)
    if config.cors_origin.is_some() {
        headers = headers
            .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
            .add(("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"));
    }

    headers
}
```

### Note on actix-web-helmet

The `actix-web-helmet` crate exists but has very low adoption and unclear maintenance status. Prefer the explicit `DefaultHeaders` approach shown above — it's zero-dependency, fully auditable, and gives complete control over which headers are set in which environments.

---

## Rate Limiting Configuration

### actix-governor (Current — In-Memory)

Already in use. Configuration reference:

```rust
use actix_governor::{Governor, GovernorConfigBuilder};

// Auth endpoints: strict (prevents brute force)
// 6 seconds between requests, burst allows 10 rapid requests
let auth_governor = GovernorConfigBuilder::default()
    .seconds_per_request(6)   // Sustained rate: 10 req/min
    .burst_size(10)           // Initial burst allowance
    .finish()
    .expect("valid config");

// General write endpoints: moderate
let write_governor = GovernorConfigBuilder::default()
    .seconds_per_request(2)   // Sustained rate: 30 req/min
    .burst_size(20)           // Initial burst allowance
    .finish()
    .expect("valid config");

// Apply per-scope
web::scope("/auth").wrap(Governor::new(&auth_governor))
web::scope("/documentos").wrap(Governor::new(&write_governor))
```

**Limitation**: In-memory only. Each backend instance has its own counter. An attacker can bypass by hitting different instances.

### actix-limitation (Redis-Backed — For Distributed)

For multi-instance deployments, use `actix-limitation` from actix-extras.

**Note**: This crate's last publish was September 2023 and depends on `redis 0.23` (current is 0.27). It works with actix-web 4 but may require a version bump in the future. An alternative is `actix-extensible-rate-limit` which supports multiple backends.

```toml
# Cargo.toml
actix-limitation = "0.5"
```

```rust
use actix_limitation::{Limiter, RateLimiter};
use actix_web::{dev::ServiceRequest, web, App, HttpServer};
use std::time::Duration;

let limiter = web::Data::new(
    Limiter::builder("redis://127.0.0.1")
        .key_by(|req: &ServiceRequest| {
            // Rate limit by IP address
            req.peer_addr().map(|addr| addr.ip().to_string())
        })
        .limit(100)
        .period(Duration::from_secs(60))
        .build()
        .expect("valid limiter config"),
);

App::new()
    .wrap(RateLimiter::default())
    .app_data(limiter.clone())
```

### Custom Rate Limit Keys

For authenticated endpoints, rate limit by user ID instead of IP:

```rust
.key_by(|req: &ServiceRequest| {
    // Try to extract user ID from JWT
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .and_then(|token| decode_jwt_sub(token).ok())
        .map(|user_id| user_id.to_string())
        // Fallback to IP if no valid token
        .or_else(|| req.peer_addr().map(|a| a.ip().to_string()))
})
```

---

## CORS Configuration

### Current Implementation (app.rs)

```rust
fn build_cors(config: &AppConfig) -> Cors {
    config.cors_origin.as_deref().map_or_else(
        || {
            tracing::warn!("CORS_ORIGIN no configurado — usando política permisiva");
            Cors::permissive()
        },
        |origin| {
            Cors::default()
                .allowed_origin(origin)
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
                .max_age(3600)
        },
    )
}
```

### Production Hardening

```rust
fn build_cors(config: &AppConfig) -> Cors {
    match config.cors_origin.as_deref() {
        Some(origin) => {
            Cors::default()
                .allowed_origin(origin)
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                // Don't include OPTIONS — actix-cors handles preflight automatically
                .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
                .expose_headers(vec![header::CONTENT_DISPOSITION]) // For file downloads
                .supports_credentials()  // If using cookies
                .max_age(3600)
        }
        None => {
            // In production, this should be a hard error
            if std::env::var("ENVIRONMENT").unwrap_or_default() == "production" {
                panic!("CORS_ORIGIN must be set in production");
            }
            tracing::warn!("CORS_ORIGIN no configurado — dev mode permissive");
            Cors::permissive()
        }
    }
}
```

### Multiple Origins

If you need to support multiple frontend origins (e.g., admin panel + tenant app):

```rust
// CORS_ORIGIN=https://app.example.com,https://admin.example.com
let origins: Vec<&str> = origin_str.split(',').collect();
let mut cors = Cors::default();
for origin in &origins {
    cors = cors.allowed_origin(origin.trim());
}
```

---

## Request Size Limits

### JSON Body Limit (Already Configured)

```rust
let json_cfg = JsonConfig::default()
    .limit(1_048_576) // 1 MB — prevents memory exhaustion from huge payloads
    .error_handler(|err, _req| {
        let message = err.to_string();
        actix_web::error::InternalError::from_response(
            err,
            AppError::Validation(message).error_response(),
        ).into()
    });
```

### Multipart Upload Limits

For file uploads via `actix-multipart`:

```rust
use actix_multipart::form::MultipartFormConfig;

let multipart_cfg = MultipartFormConfig::default()
    .total_limit(10 * 1024 * 1024)  // 10 MB total
    .memory_limit(2 * 1024 * 1024); // 2 MB in memory before spilling to disk
```

### Per-Endpoint Payload Limits

For endpoints that need different limits:

```rust
use actix_web::web::PayloadConfig;

web::resource("/api/v1/importar/propiedades")
    .app_data(PayloadConfig::new(5 * 1024 * 1024)) // 5 MB for imports
    .route(web::post().to(handlers::importacion::importar_propiedades))
```

---

## Custom Auth Extractors

### Current Pattern (middleware/rbac.rs)

```rust
pub struct AdminOnly(pub Claims);
pub struct WriteAccess(pub Claims);

impl FromRequest for AdminOnly {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let claims = Claims::from_request(req, payload).into_inner();
        ready(match claims {
            Ok(c) if c.rol == "admin" => Ok(Self(c)),
            Ok(_) => Err(AppError::Forbidden),
            Err(e) => Err(e),
        })
    }
}
```

### Adding Resource Ownership Verification

For endpoints where you need to verify the user owns the resource:

```rust
/// Extractor that verifies the user belongs to the same org as the resource
pub struct OrgScoped {
    pub claims: Claims,
    pub org_id: Uuid,
}

impl OrgScoped {
    /// Call this in handlers to verify a resource belongs to the user's org
    pub fn verify_ownership(&self, resource_org_id: Uuid) -> Result<(), AppError> {
        if self.claims.organizacion_id != resource_org_id {
            tracing::warn!(
                user_id = %self.claims.sub,
                user_org = %self.claims.organizacion_id,
                resource_org = %resource_org_id,
                "Cross-tenant access attempt"
            );
            return Err(AppError::NotFound("Recurso no encontrado".into()));
        }
        Ok(())
    }
}
```

Note: Return `NotFound` instead of `Forbidden` for cross-tenant access — don't reveal that the resource exists.

---

## Error Handler Middleware

### Catching Unhandled Errors

Use `ErrorHandlers` middleware to ensure no raw error leaks:

```rust
use actix_web::middleware::ErrorHandlers;
use actix_web::http::StatusCode;

.wrap(
    ErrorHandlers::new()
        .default_handler_server(|res| {
            // Log the original error
            tracing::error!(status = %res.status(), "Unhandled server error");
            // Replace body with generic message
            let (req, _res) = res.into_parts();
            let new_res = HttpResponse::InternalServerError()
                .json(serde_json::json!({
                    "error": "internal",
                    "message": "Error interno del servidor"
                }));
            Ok(ErrorHandlerResponse::Response(
                ServiceResponse::new(req, new_res).map_into_right_body()
            ))
        })
)
```

---

## Request Logging for Security

### Logging Failed Auth Attempts

In the auth handler or service:

```rust
pub async fn login(db: &DatabaseConnection, input: LoginInput) -> Result<TokenResponse, AppError> {
    let user = find_user_by_email(db, &input.email).await?;

    match user {
        None => {
            tracing::warn!(email = %input.email, "Login attempt for non-existent user");
            Err(AppError::Unauthorized(None))
        }
        Some(u) if !u.activo => {
            tracing::warn!(user_id = %u.id, "Login attempt for deactivated account");
            Err(AppError::Unauthorized(None))
        }
        Some(u) => {
            let valid = verify_password(&u.password_hash, &input.password)?;
            if !valid {
                tracing::warn!(user_id = %u.id, "Failed login attempt — wrong password");
                Err(AppError::Unauthorized(None))
            } else {
                tracing::info!(user_id = %u.id, "Successful login");
                generate_token(&u)
            }
        }
    }
}
```

### Structured Security Events

Use consistent field names for security-relevant logs so they're searchable:

```rust
// Security event fields:
// - event_type: "auth_failure", "auth_success", "access_denied", "cross_tenant_attempt"
// - user_id: UUID of the acting user (if known)
// - target_id: UUID of the resource being accessed
// - ip: client IP address

tracing::warn!(
    event_type = "access_denied",
    user_id = %claims.sub,
    target_id = %resource_id,
    "User attempted to access resource without permission"
);
```

---

## File Upload Security

### Complete Upload Handler Pattern

```rust
use actix_multipart::form::{MultipartForm, tempfile::TempFile, text::Text};
use uuid::Uuid;

const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "jpg", "jpeg", "png", "docx", "xlsx"];
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

#[derive(MultipartForm)]
pub struct UploadForm {
    #[multipart(limit = "10 MiB")]
    file: TempFile,
    entity_type: Text<String>,
    entity_id: Text<Uuid>,
}

pub async fn upload_documento(
    access: WriteAccess,
    form: MultipartForm<UploadForm>,
) -> Result<HttpResponse, AppError> {
    let form = form.into_inner();
    let filename = form.file.file_name.unwrap_or_default();

    // 1. Validate extension
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Validation(format!("Extensión no permitida: {ext}")));
    }

    // 2. Validate size
    let size = form.file.size;
    if size > MAX_FILE_SIZE {
        return Err(AppError::Validation("Archivo excede 10 MB".into()));
    }

    // 3. Prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(AppError::Validation("Nombre de archivo inválido".into()));
    }

    // 4. Generate safe storage name (never use user-provided name for storage)
    let storage_name = format!("{}.{}", Uuid::new_v4(), ext);

    // 5. Move to upload directory
    let upload_dir = get_upload_dir();
    let dest_path = std::path::Path::new(&upload_dir).join(&storage_name);
    form.file.file.persist(&dest_path)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error guardando archivo: {e}")))?;

    // 6. Store metadata in DB (original filename for display, storage name for retrieval)
    // ...

    Ok(HttpResponse::Created().json(serde_json::json!({
        "filename": filename,
        "storage_path": storage_name,
    })))
}
```

### Serving Uploaded Files Securely

The current `app.rs` serves uploads as static files:
```rust
.service(Files::new("/uploads", &upload_dir))
```

This bypasses auth. For production, serve through an authenticated handler:

```rust
pub async fn descargar_documento(
    req: HttpRequest,  // Required for NamedFile::into_response
    claims: Claims,
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let doc_id = path.into_inner();

    // Verify the document belongs to the user's org
    let doc = documentos::Entity::find_by_id(doc_id)
        .filter(documentos::Column::OrganizacionId.eq(claims.organizacion_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Documento no encontrado".into()))?;

    let file_path = std::path::Path::new(&get_upload_dir()).join(&doc.file_path);
    let file = actix_files::NamedFile::open(&file_path)
        .map_err(|_| AppError::NotFound("Archivo no encontrado".into()))?;

    Ok(file
        .set_content_disposition(actix_web::http::header::ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Attachment,
            parameters: vec![
                actix_web::http::header::DispositionParam::Filename(doc.filename.clone())
            ],
        })
        .into_response(&req))
}
```
