---
name: maintainability-reviewer
description: >
  Reviews and fixes Rust code maintainability issues by directly editing source files.
  Refactors long functions, reduces deep nesting, fixes error handling anti-patterns,
  and enforces SOLID principles. References the project's layered architecture
  (handlers → services → entities) and error patterns in errors.rs. Runs cargo fmt,
  cargo clippy, and cargo test after changes. Use when refactoring code, fixing
  maintainability issues, or enforcing SOLID principles.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "1.0.0"
  domain: maintainability
  triggers: maintainability, SOLID, error handling, code organization, refactor
  role: specialist
  scope: analysis
  output-format: report
  related-skills: perf-optimizer, algorithm-advisor, code-reviewer
---

# Maintainability Reviewer

Specialist skill for reviewing and fixing Rust code maintainability in Actix-web + SeaORM + tokio codebases. Actively refactors long functions, reduces nesting, fixes error handling patterns, and enforces module boundaries. Validates changes with cargo fmt and cargo clippy.

## Core Workflow

1. **Check function length** — Flag functions exceeding 50 lines; suggest extraction of logical sub-steps into helper functions or service methods
2. **Check module public API surface** — Flag modules exporting more than 10 public items (`pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub const`); suggest `pub(crate)` or re-export consolidation
3. **Detect deeply nested control flow** — Flag control flow nested more than 3 levels deep (`if`/`match`/`for`/`while`); suggest early returns, guard clauses, or extraction into named functions
4. **Verify error handling follows `AppError` patterns** — Ensure handlers return `Result<HttpResponse, AppError>`, services propagate errors via `?` with `.context()`, and `AppError` variants (`NotFound`, `Unauthorized`, `Forbidden`, `Validation`, `Conflict`, `Internal`) are used correctly per `backend/src/errors.rs`
5. **Check `thiserror` vs `anyhow` consistency** — Verify `thiserror` is used for defining custom error enums (like `AppError`) and `anyhow` is used for application-level propagation with `.context()`; flag mixed usage within the same layer
6. **Evaluate trait design and module boundary clarity** — Check that handlers only do HTTP parsing and response building, services contain business logic, and entities remain data-only; flag cross-layer concerns and overly broad traits

## Reference Guide

Load detailed guidance based on context:

| Topic | Reference | Load When |
|-------|-----------|-----------|
| SOLID Principles | `references/solid-principles.md` | Trait design, module boundaries, SRP |
| Error Handling | `references/error-handling.md` | thiserror, anyhow, AppError patterns |
| Code Organization | `references/code-organization.md` | Module structure, visibility, API surface |

## Detection Rules

### Excessive Function Length

Flag functions exceeding 50 lines. Long functions often violate the Single Responsibility Principle and are harder to test in isolation.

```rust
// Anti-pattern: handler doing HTTP parsing, validation, business logic, and response building
async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    // 60+ lines mixing validation, overlap checks, DB queries, and response formatting
    let request = body.into_inner();
    if request.fecha_inicio >= request.fecha_fin {
        return Err(AppError::Validation("Fecha inicio debe ser anterior a fecha fin".into()));
    }
    // ... 50 more lines of interleaved logic ...
    Ok(HttpResponse::Created().json(response))
}

// Fix: delegate business logic to the service layer
async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
    user: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let request = body.into_inner();
    let contrato = contratos::create(&db, request, user.id).await?;
    Ok(HttpResponse::Created().json(contrato))
}
```

### Excessive Public API Surface

Flag modules exporting more than 10 public items. A large public surface increases coupling and makes refactoring harder.

```rust
// Anti-pattern: module exposing too many internals
pub fn list_propiedades() { ... }
pub fn get_propiedad() { ... }
pub fn create_propiedad() { ... }
pub fn update_propiedad() { ... }
pub fn delete_propiedad() { ... }
pub fn validate_propiedad() { ... }
pub fn format_propiedad_response() { ... }
pub fn parse_propiedad_filters() { ... }
pub fn check_propiedad_availability() { ... }
pub fn calculate_propiedad_stats() { ... }
pub fn export_propiedad_report() { ... }

// Fix: expose only the public API, keep helpers as pub(crate) or private
pub fn list_propiedades() { ... }
pub fn get_propiedad() { ... }
pub fn create_propiedad() { ... }
pub fn update_propiedad() { ... }
pub fn delete_propiedad() { ... }
fn validate_propiedad() { ... }
fn format_propiedad_response() { ... }
pub(crate) fn parse_propiedad_filters() { ... }
fn check_propiedad_availability() { ... }
pub(crate) fn calculate_propiedad_stats() { ... }
fn export_propiedad_report() { ... }
```

### Deep Nesting

Flag control flow nested more than 3 levels. Deep nesting increases cognitive load and signals that logic should be extracted or restructured with early returns.

```rust
// Anti-pattern: 4+ levels of nesting
async fn process_pagos(pagos: &[Pago], contratos: &[Contrato]) -> Result<(), AppError> {
    for pago in pagos {
        if pago.estado == "pendiente" {
            if let Some(contrato) = contratos.iter().find(|c| c.id == pago.contrato_id) {
                if contrato.estado == "activo" {
                    if pago.monto > contrato.monto_mensual {
                        // deeply nested logic
                    }
                }
            }
        }
    }
    Ok(())
}

// Fix: use early continues and guard clauses
async fn process_pagos(pagos: &[Pago], contratos: &[Contrato]) -> Result<(), AppError> {
    for pago in pagos {
        if pago.estado != "pendiente" {
            continue;
        }
        let contrato = match contratos.iter().find(|c| c.id == pago.contrato_id) {
            Some(c) if c.estado == "activo" => c,
            _ => continue,
        };
        if pago.monto <= contrato.monto_mensual {
            continue;
        }
        // flat logic at one level of nesting
    }
    Ok(())
}
```

### Incorrect AppError Usage

Verify error handling follows the `AppError` enum defined in `backend/src/errors.rs`. The project defines six variants:

- `AppError::NotFound(String)` → 404
- `AppError::Unauthorized` → 401
- `AppError::Forbidden` → 403
- `AppError::Validation(String)` → 422
- `AppError::Conflict(String)` → 409
- `AppError::Internal(anyhow::Error)` → 500 (via `#[from] anyhow::Error`)

`sea_orm::DbErr` converts to `AppError::Internal` automatically via the `From<sea_orm::DbErr>` impl.

```rust
// Anti-pattern: returning raw strings or wrong variant
async fn get_propiedad(db: &DatabaseConnection, id: i32) -> Result<Propiedad, AppError> {
    let propiedad = Propiedad::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| AppError::Validation(e.to_string()))?; // wrong: DB errors are Internal, not Validation
    propiedad.ok_or_else(|| AppError::Internal(anyhow::anyhow!("not found"))) // wrong: use NotFound
}

// Fix: use correct AppError variants
async fn get_propiedad(db: &DatabaseConnection, id: i32) -> Result<Propiedad, AppError> {
    let propiedad = Propiedad::find_by_id(id)
        .one(db)
        .await?; // DbErr auto-converts to AppError::Internal via From impl
    propiedad.ok_or_else(|| AppError::NotFound(format!("Propiedad con id {} no encontrada", id)))
}
```

### Inconsistent thiserror vs anyhow Usage

`thiserror` defines structured error enums (like `AppError`). `anyhow` provides ergonomic error propagation with `.context()`. Mixing them within the same layer creates confusion.

```rust
// Anti-pattern: using anyhow::Result in a service that should return AppError
use anyhow::Result;

pub async fn create_contrato(db: &DatabaseConnection, req: CreateContratoRequest) -> Result<Contrato> {
    // returns anyhow::Error — handler cannot match on specific error variants
    let propiedad = find_propiedad(db, req.propiedad_id).await?;
    // ...
}

// Fix: services return Result<T, AppError> so handlers can match variants
use crate::errors::AppError;

pub async fn create_contrato(db: &DatabaseConnection, req: CreateContratoRequest) -> Result<Contrato, AppError> {
    let propiedad = find_propiedad(db, req.propiedad_id)
        .await
        .context("Error buscando propiedad para contrato")?; // anyhow context wraps into AppError::Internal
    // ...
}
```

### Layer Boundary Violations

The project follows a layered architecture: handlers → services → entities. Each layer has a clear responsibility.

```rust
// Anti-pattern: handler contains business logic (overlap detection)
async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let existing = Contrato::find()
        .filter(contrato::Column::PropiedadId.eq(body.propiedad_id))
        .all(db.get_ref())
        .await?;
    for c in &existing {
        if c.fecha_fin >= body.fecha_inicio && body.fecha_fin >= c.fecha_inicio {
            return Err(AppError::Conflict("Contrato se solapa con uno existente".into()));
        }
    }
    // ... more business logic in the handler
}

// Fix: handler delegates to service, service owns the business logic
async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let contrato = contratos::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(contrato))
}
```

## Constraints

### MUST DO
- Directly edit source files to apply maintainability improvements
- Run `cargo fmt` after changes to maintain formatting
- Run `cargo clippy --all-targets` to validate changes introduce no warnings
- Run `cargo test --workspace` to verify refactors don't break existing tests
- Log each change with file path, what was refactored, and why
- Reference the project's `AppError` variants and layered architecture in fixes
- Tailor refactors to Actix-web handlers, SeaORM queries, and the property management domain

### MUST NOT DO
- Suggest fixes without applying them — always edit the code directly
- Refactor short utility functions that are intentionally concise
- Touch `pub` items in `prelude.rs` modules designed for re-exports
- Make architectural changes that contradict the handler → service → entity layering
- Modify `#[derive]` macros or generated entity code in `backend/src/entities/`
