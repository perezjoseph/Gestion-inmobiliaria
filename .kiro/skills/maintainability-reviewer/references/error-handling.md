# Error Handling Reference Guide

## Project Error Architecture

The project uses a two-crate error strategy:

```
thiserror  → Define structured error enums with variants (library-style)
anyhow     → Propagate errors ergonomically with .context() (application-style)
```

The central error type lives in `backend/src/errors.rs` and bridges both crates: `AppError` is defined with `thiserror` but accepts `anyhow::Error` via its `Internal` variant.


## AppError Enum — The Project's Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No encontrado: {0}")]
    NotFound(String),          // → 404 NOT FOUND

    #[error("No autorizado")]
    Unauthorized,              // → 401 UNAUTHORIZED

    #[error("Acceso denegado")]
    Forbidden,                 // → 403 FORBIDDEN

    #[error("{0}")]
    Validation(String),        // → 422 UNPROCESSABLE ENTITY

    #[error("Conflicto: {0}")]
    Conflict(String),          // → 409 CONFLICT

    #[error(transparent)]
    Internal(#[from] anyhow::Error), // → 500 INTERNAL SERVER ERROR
}
```

Key design decisions:

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Derive macro | `thiserror::Error` | Generates `Display` and `Error` impls from `#[error(...)]` attributes |
| Internal variant | `#[from] anyhow::Error` | Allows `?` to auto-convert any `anyhow::Error` into `AppError::Internal` |
| Transparent display | `#[error(transparent)]` on `Internal` | Delegates `Display` to the inner `anyhow::Error` |
| Message hiding | `Internal` returns generic message in HTTP response | Prevents leaking stack traces or DB details to clients |


## Error Response Format

`AppError` implements `actix_web::error::ResponseError` to produce JSON error responses:

```rust
impl actix_web::error::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (error_type, message) = match self {
            AppError::NotFound(msg) => ("not_found", msg.clone()),
            AppError::Unauthorized => ("unauthorized", self.to_string()),
            AppError::Forbidden => ("forbidden", self.to_string()),
            AppError::Validation(msg) => ("validation", msg.clone()),
            AppError::Conflict(msg) => ("conflict", msg.clone()),
            AppError::Internal(_) => ("internal", "Error interno del servidor".to_string()),
        };

        HttpResponse::build(self.status_code()).json(json!({
            "error": error_type,
            "message": message,
        }))
    }
}
```

Every error response follows this shape:

```json
{
  "error": "not_found",
  "message": "Propiedad con id 123 no encontrada"
}
```

The `Internal` variant always returns a generic message (`"Error interno del servidor"`) to avoid leaking sensitive details like database errors or stack traces.


## thiserror vs anyhow — When to Use Each

### thiserror — Defining Error Types

Use `thiserror` when defining error enums that callers need to match on. This is the "library" pattern: structured, typed errors with explicit variants.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No encontrado: {0}")]
    NotFound(String),
    // ... other variants
}
```

`thiserror` generates:
- `impl Display` from `#[error("...")]` attributes
- `impl std::error::Error` with optional `source()` from `#[from]` or `#[source]`
- `From<T>` conversions for variants annotated with `#[from]`

### anyhow — Propagating Errors

Use `anyhow` for ergonomic error propagation in application code where callers don't need to match on specific error types. The `.context()` method adds human-readable context to error chains.

```rust
use anyhow::Context;

pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    let propiedad = propiedad::Entity::find_by_id(input.propiedad_id)
        .one(db)
        .await
        .context("Error buscando propiedad para contrato")?;
    // .context() wraps the DbErr in anyhow::Error
    // The ? operator then converts anyhow::Error → AppError::Internal via #[from]

    let propiedad = propiedad
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".into()))?;

    validate_no_overlap(db, input.propiedad_id, input.fecha_inicio, input.fecha_fin, None)
        .await
        .context("Error validando solapamiento de contratos")?;

    // ... insert and return
}
```

### Layer Rules

| Layer | Error Crate | Return Type | Rationale |
|-------|-------------|-------------|-----------|
| `errors.rs` | `thiserror` | Defines `AppError` | Callers (handlers) need to match variants for HTTP status codes |
| Handlers | Neither directly | `Result<HttpResponse, AppError>` | Handlers call services and let `?` propagate `AppError` |
| Services | `anyhow` (`.context()`) | `Result<T, AppError>` | Services add context to errors before propagation |
| Entities | N/A | Generated by SeaORM | Entity code is auto-generated; errors come from `sea_orm::DbErr` |

### Anti-pattern — Using anyhow::Result in services

```rust
use anyhow::Result;

pub async fn create_contrato(
    db: &DatabaseConnection,
    req: CreateContratoRequest,
) -> Result<Contrato> {
    // Returns anyhow::Error — handler cannot match on specific AppError variants
    // Handler loses the ability to return 404 vs 409 vs 422
    let propiedad = find_propiedad(db, req.propiedad_id).await?;
    // ...
}
```

Services should return `Result<T, AppError>` so handlers can rely on the variant-to-status-code mapping in `ResponseError`.

### Anti-pattern — Using thiserror to define one-off errors in services

```rust
#[derive(Debug, thiserror::Error)]
enum ContratoServiceError {
    #[error("Propiedad no encontrada")]
    PropiedadNotFound,
    #[error("Contrato se solapa")]
    Overlap,
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
}
```

Defining per-service error enums adds unnecessary indirection. The project centralizes all error variants in `AppError`. Services should use `AppError` directly.


## Mapping sea_orm::DbErr to AppError

The project defines a `From<sea_orm::DbErr>` implementation that wraps all database errors as `AppError::Internal`:

```rust
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::Internal(anyhow::anyhow!(err))
    }
}
```

This means the `?` operator on any SeaORM query automatically converts database errors to 500 responses:

```rust
pub async fn list(db: &DatabaseConnection) -> Result<Vec<PropiedadResponse>, AppError> {
    let records = propiedad::Entity::find()
        .all(db)
        .await?; // DbErr → AppError::Internal → 500
    Ok(records.into_iter().map(PropiedadResponse::from).collect())
}
```

### Adding Context to Database Errors

Use `.context()` from `anyhow::Context` to add meaningful descriptions before the `?` conversion:

```rust
use anyhow::Context;

pub async fn get_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    let record = propiedad::Entity::find_by_id(id)
        .one(db)
        .await
        .context(format!("Error consultando propiedad {}", id))?;
    // The error chain: DbErr → anyhow::Error (with context) → AppError::Internal

    record
        .map(PropiedadResponse::from)
        .ok_or_else(|| AppError::NotFound(format!("Propiedad con id {} no encontrada", id)))
}
```

Without `.context()`, the 500 response logs only the raw `DbErr`. With `.context()`, the error chain includes the operation that failed, making debugging easier.

### When DbErr Should NOT Be Internal

Some `DbErr` variants represent client errors, not server errors. A unique constraint violation typically means a conflict (409), not an internal error (500):

```rust
pub async fn create(
    db: &DatabaseConnection,
    input: CreateInquilinoRequest,
) -> Result<InquilinoResponse, AppError> {
    let model = inquilino::ActiveModel { /* ... */ };

    match model.insert(db).await {
        Ok(record) => Ok(InquilinoResponse::from(record)),
        Err(sea_orm::DbErr::Query(RuntimeErr::SqlxError(sqlx_err)))
            if is_unique_violation(&sqlx_err) =>
        {
            Err(AppError::Conflict(
                "Inquilino con esa cédula ya existe".into(),
            ))
        }
        Err(err) => Err(AppError::Internal(anyhow::anyhow!(err))),
    }
}
```

This pattern intercepts specific database errors and maps them to the appropriate `AppError` variant instead of letting the blanket `From<DbErr>` conversion turn everything into a 500.


## Correct Variant Selection

Choosing the wrong `AppError` variant produces misleading HTTP status codes. Use this guide:

| Situation | Correct Variant | Status Code |
|-----------|----------------|-------------|
| Entity not found by ID | `NotFound("Propiedad con id {id} no encontrada")` | 404 |
| Missing or invalid JWT | `Unauthorized` | 401 |
| Valid JWT but insufficient role | `Forbidden` | 403 |
| Invalid request body fields | `Validation("Campo fecha_inicio es requerido")` | 422 |
| Business rule violation (overlap, duplicate) | `Conflict("Contrato se solapa con uno existente")` | 409 |
| Database error, unexpected failure | `Internal(anyhow::anyhow!(err))` | 500 |

### Anti-pattern — Wrong variant for the situation

```rust
// WRONG: DB errors are Internal, not Validation
let propiedad = propiedad::Entity::find_by_id(id)
    .one(db)
    .await
    .map_err(|e| AppError::Validation(e.to_string()))?;

// WRONG: "not found" is NotFound, not Internal
propiedad.ok_or_else(|| AppError::Internal(anyhow::anyhow!("not found")))

// WRONG: business rule violation is Conflict, not Validation
if has_overlap {
    return Err(AppError::Validation("Contrato se solapa".into()));
}
```

### Correct usage

```rust
let propiedad = propiedad::Entity::find_by_id(id)
    .one(db)
    .await?; // DbErr auto-converts to Internal

propiedad.ok_or_else(|| AppError::NotFound(format!("Propiedad con id {} no encontrada", id)))

if has_overlap {
    return Err(AppError::Conflict("Contrato se solapa con uno existente".into()));
}
```


## The .context() Pattern

`anyhow::Context` extends `Result` with `.context()` and `.with_context()` methods. These wrap the error in an `anyhow::Error` with an additional message, which then converts to `AppError::Internal` via `#[from]`.

### When to use .context()

Use `.context()` on operations where the raw error alone doesn't explain what went wrong:

```rust
use anyhow::Context;

let config = std::fs::read_to_string("config.toml")
    .context("Error leyendo archivo de configuración")?;

let parsed: Config = toml::from_str(&config)
    .context("Error parseando configuración TOML")?;
```

### When to use .with_context()

Use `.with_context()` when the context message requires computation (avoids allocating the string on the success path):

```rust
use anyhow::Context;

let record = contrato::Entity::find_by_id(id)
    .one(db)
    .await
    .with_context(|| format!("Error consultando contrato {}", id))?;
```

### When NOT to use .context()

Don't add context when the error is already descriptive or when you're converting to a specific `AppError` variant:

```rust
// No .context() needed — the error is already clear and we're using a specific variant
let propiedad = propiedad
    .ok_or_else(|| AppError::NotFound(format!("Propiedad con id {} no encontrada", id)))?;

// No .context() needed — DbErr auto-converts via From impl and the operation is obvious
let records = propiedad::Entity::find().all(db).await?;
```


## Error Handling by Layer — Complete Flow

### Handler → Service → Database

```rust
// Handler: parse HTTP, delegate to service, return response
pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let result = contratos::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
    // If the service returns Err(AppError::Conflict(...)), Actix-web calls
    // error_response() which produces {"error":"conflict","message":"..."} with 409
}

// Service: business logic, uses .context() for DB errors, returns AppError variants
pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    let propiedad = propiedad::Entity::find_by_id(input.propiedad_id)
        .one(db)
        .await
        .context("Error buscando propiedad")?; // DbErr → anyhow → AppError::Internal

    let propiedad = propiedad
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".into()))?;

    validate_no_overlap(db, input.propiedad_id, input.fecha_inicio, input.fecha_fin, None).await?;

    let model = contrato::ActiveModel { /* ... */ };
    let record = model.insert(db).await?; // DbErr → AppError::Internal
    Ok(ContratoResponse::from(record))
}
```

### Error Propagation Chain

```
sea_orm::DbErr
  → From<DbErr> for AppError     (auto via ? operator)
  → AppError::Internal(anyhow)
  → ResponseError::error_response()
  → {"error":"internal","message":"Error interno del servidor"} with 500

anyhow::Error (from .context())
  → #[from] anyhow::Error        (auto via ? operator)
  → AppError::Internal(anyhow)
  → ResponseError::error_response()
  → {"error":"internal","message":"Error interno del servidor"} with 500

AppError::NotFound(msg)
  → returned directly by service
  → ResponseError::error_response()
  → {"error":"not_found","message":"Propiedad no encontrada"} with 404
```


## Detection Heuristics

| Signal | Likely Issue |
|--------|-------------|
| Service returns `anyhow::Result<T>` instead of `Result<T, AppError>` | Handler loses ability to match error variants for correct HTTP status |
| `AppError::Validation` used for database errors | Wrong variant — DB errors should be `Internal` |
| `AppError::Internal` used for "not found" | Wrong variant — use `NotFound` for missing entities |
| `AppError::Validation` used for business rule violations (overlap, duplicate) | Wrong variant — use `Conflict` for constraint violations |
| `.unwrap()` or `.expect()` in handler or service code | Panics in production — use `?` with proper error conversion |
| Missing `.context()` on complex DB queries | Error logs will lack operation context for debugging |
| Per-service error enum defined with `thiserror` | Unnecessary indirection — use the centralized `AppError` |
| `map_err(\|e\| AppError::Internal(anyhow::anyhow!(e)))` on `DbErr` | Redundant — the `From<DbErr>` impl already handles this; just use `?` |
| `Internal` variant message exposed in HTTP response | Security risk — verify `error_response()` returns generic message for `Internal` |
