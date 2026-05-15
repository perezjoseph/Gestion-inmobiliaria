# Code Organization Reference Guide

## Project Module Structure

The project uses a flat, layered module layout under `backend/src/`:

```
backend/src/
├── main.rs          → Crate root: declares modules, boots the server
├── app.rs           → App builder (Actix-web App configuration)
├── config.rs        → Environment-based configuration (AppConfig)
├── errors.rs        → Centralized AppError enum (thiserror)
├── routes.rs        → Route registration (maps paths to handlers)
├── entities/        → SeaORM generated models (never manually edited)
│   ├── mod.rs       → Re-exports all entity submodules
│   ├── prelude.rs   → Convenience re-exports for common entity types
│   ├── contrato.rs
│   ├── inquilino.rs
│   ├── pago.rs
│   ├── propiedad.rs
│   └── usuario.rs
├── handlers/        → HTTP layer: parse requests, call services, return responses
│   ├── mod.rs
│   ├── auth.rs
│   ├── contratos.rs
│   ├── dashboard.rs
│   ├── inquilinos.rs
│   ├── pagos.rs
│   └── propiedades.rs
├── services/        → Business logic layer: no HTTP concerns
│   ├── mod.rs
│   ├── auth.rs
│   ├── contratos.rs
│   ├── dashboard.rs
│   ├── inquilinos.rs
│   ├── pagos.rs
│   └── propiedades.rs
├── models/          → Request/response DTOs (serde-based)
│   ├── mod.rs       → Shared types like PaginatedResponse<T>
│   ├── contrato.rs
│   ├── inquilino.rs
│   ├── pago.rs
│   ├── propiedad.rs
│   └── usuario.rs
└── middleware/       → Auth and RBAC extractors
    ├── mod.rs
    ├── auth.rs
    └── rbac.rs
```

Each domain area (propiedades, inquilinos, contratos, pagos) has a matching file in `handlers/`, `services/`, `models/`, and `entities/`. This 1:1 mapping makes navigation predictable: to find the business logic for contracts, go to `services/contratos.rs`.


## Module Declaration Conventions

### File-Based Modules vs `mod.rs`

Rust supports two styles for declaring modules with child files:

| Style | Structure | When to Use |
|-------|-----------|-------------|
| `mod.rs` inside directory | `handlers/mod.rs` + `handlers/auth.rs` | Directories with multiple sibling files (current project pattern) |
| Named file + directory | `handlers.rs` + `handlers/auth.rs` | Preferred by Rust 2018+ conventions for new code |

The project currently uses the `mod.rs` pattern. Both styles are functionally identical — the compiler treats them the same way. Consistency within the project matters more than which style is chosen.

### Crate Root Declarations

All top-level modules are declared in `main.rs`:

```rust
mod app;
mod config;
mod entities;
mod errors;
mod handlers;
mod middleware;
mod models;
mod routes;
mod services;
```

These are private to the crate by default — no `pub` keyword. This is correct for a binary crate where nothing is exported to external consumers.

### Directory Module Re-exports

Each directory's `mod.rs` re-exports its submodules:

```rust
// handlers/mod.rs
pub mod auth;
pub mod contratos;
pub mod dashboard;
pub mod inquilinos;
pub mod pagos;
pub mod propiedades;
```

The `pub` here makes the submodules visible to the rest of the crate. Since the parent (`handlers`) is declared as `mod handlers` (private) in `main.rs`, these `pub` submodules are effectively `pub(crate)` — visible within the crate but not outside it.

### Anti-pattern — Forgetting to declare a submodule

```rust
// handlers/mod.rs — missing dashboard
pub mod auth;
pub mod contratos;
pub mod inquilinos;
pub mod pagos;
pub mod propiedades;
// dashboard.rs exists but is never compiled!
```

The file `handlers/dashboard.rs` would be ignored by the compiler. Every `.rs` file in a module directory must be declared in `mod.rs` to be part of the crate.


## Visibility: `pub` vs `pub(crate)` vs Private

### Visibility Levels

| Keyword | Visible To | Use When |
|---------|-----------|----------|
| (none) | Current module and its children only | Internal helpers, implementation details |
| `pub(crate)` | Entire crate | Types/functions used across modules but not exported |
| `pub(super)` | Parent module | Helpers shared between sibling submodules |
| `pub` | External consumers (if crate is a library) | Public API of a library crate |

### Binary Crate Nuance

In a binary crate (like this project's backend), `pub` and `pub(crate)` are functionally equivalent — there are no external consumers. However, using `pub(crate)` communicates intent: "this is internal to the crate, not part of a public API."

### Correct — Explicit visibility intent

```rust
// services/contratos.rs

// Used by handlers — needs to be visible across modules
pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    validate_no_overlap(db, input.propiedad_id, input.fecha_inicio, input.fecha_fin, None).await?;
    // ...
}

// Only used within this module — keep private
async fn validate_no_overlap(
    db: &DatabaseConnection,
    propiedad_id: Uuid,
    fecha_inicio: NaiveDate,
    fecha_fin: NaiveDate,
    exclude_id: Option<Uuid>,
) -> Result<(), AppError> {
    // ...
}
```

The service's public functions (`create`, `list`, `get_by_id`, `update`, `delete`) are the module's API. Internal helpers like `validate_no_overlap` stay private.

### Anti-pattern — Everything is `pub`

```rust
// services/contratos.rs — over-exposed API surface

pub async fn create(...) -> Result<ContratoResponse, AppError> { /* ... */ }
pub async fn list(...) -> Result<Vec<ContratoResponse>, AppError> { /* ... */ }
pub async fn get_by_id(...) -> Result<ContratoResponse, AppError> { /* ... */ }
pub async fn update(...) -> Result<ContratoResponse, AppError> { /* ... */ }
pub async fn delete(...) -> Result<(), AppError> { /* ... */ }

// These should be private — they're implementation details
pub async fn validate_no_overlap(...) -> Result<(), AppError> { /* ... */ }
pub fn build_overlap_condition(...) -> Condition { /* ... */ }
pub fn calculate_late_fee(...) -> Decimal { /* ... */ }
```

Making internal helpers `pub` expands the module's API surface unnecessarily. Other modules might start depending on these helpers, creating tight coupling.

### When to Use `pub(crate)`

Use `pub(crate)` for types that are shared across module boundaries but aren't part of any module's primary API:

```rust
// errors.rs
pub(crate) enum AppError {
    NotFound(String),
    Unauthorized,
    Forbidden,
    Validation(String),
    Conflict(String),
    Internal(#[from] anyhow::Error),
}
```

```rust
// models/mod.rs
pub(crate) struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}
```

In practice, the project uses `pub` for these since it's a binary crate. Either works — the key is consistency within the codebase.


## Re-exports via `prelude.rs`

### Purpose

A `prelude.rs` module provides convenience re-exports so consumers can import commonly used types with a single `use` statement instead of importing from multiple submodules.

### Project Example — Entity Prelude

```rust
// entities/prelude.rs
pub use sea_orm::Database;
pub use sea_orm::DatabaseConnection;
pub use sea_orm::DbBackend;
pub use sea_orm::DbErr;

pub use super::contrato::Entity as Contrato;
pub use super::inquilino::Entity as Inquilino;
pub use super::pago::Entity as Pago;
pub use super::propiedad::Entity as Propiedad;
pub use super::usuario::Entity as Usuario;
```

This allows consumers to write:

```rust
use crate::entities::prelude::*;

// Instead of:
use crate::entities::contrato::Entity as Contrato;
use crate::entities::inquilino::Entity as Inquilino;
use crate::entities::pago::Entity as Pago;
// ... etc.
```

### Prelude Design Rules

| Rule | Rationale |
|------|-----------|
| Only re-export frequently used types | Preludes that re-export everything defeat the purpose |
| Use `as` aliases for clarity | `Entity as Contrato` is clearer than bare `Entity` when multiple entities exist |
| Keep the prelude small (under 15 items) | Large preludes pollute the namespace and make it hard to trace where types come from |
| Don't re-export implementation details | Only types that consumers actually need in their `use` statements |

### Anti-pattern — Prelude re-exports everything

```rust
// entities/prelude.rs — too much
pub use super::contrato::*;
pub use super::inquilino::*;
pub use super::pago::*;
pub use super::propiedad::*;
pub use super::usuario::*;
pub use sea_orm::*;
```

Glob re-exports from every submodule dump hundreds of symbols into scope. This causes name collisions (every entity has `Entity`, `Model`, `ActiveModel`, `Column`) and makes it impossible to tell where a type comes from.

### When to Create a Prelude

Create a `prelude.rs` when:
- A module has many submodules with similarly-named types (like SeaORM entities)
- Consumers consistently need the same set of imports from the module
- The re-exported set is small and stable

Don't create a prelude when:
- The module has few submodules (just use direct imports)
- Consumers need different subsets of types (no single prelude fits all)
- Types are used in only one or two places (direct imports are clearer)


## Documentation Standards

### Module-Level Documentation

Every module should have a top-level doc comment explaining its purpose and relationship to other modules:

```rust
//! Service layer for contract management.
//!
//! Handles business logic for creating, updating, and querying contracts.
//! Validates contract date ranges to prevent overlapping active contracts
//! for the same property.
//!
//! Called by `handlers::contratos`. Queries `entities::contrato`.

pub async fn create(...) -> Result<ContratoResponse, AppError> { /* ... */ }
```

### Public Function Documentation

Public functions that form a module's API should have doc comments explaining parameters, return values, and error conditions:

```rust
/// Creates a new contract after validating no date overlap exists.
///
/// Returns `AppError::NotFound` if the property or tenant doesn't exist.
/// Returns `AppError::Conflict` if an active contract overlaps the date range.
pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    // ...
}
```

### What to Document vs What to Skip

| Item | Document? | Rationale |
|------|-----------|-----------|
| Public service functions | Yes | They form the module's API contract |
| Handler functions | Briefly | The route path and HTTP method are usually sufficient context |
| Private helper functions | Only if non-obvious | Self-documenting names are preferred over comments |
| Struct fields | Only if the name isn't self-explanatory | `propiedad_id: Uuid` needs no comment; `monto_mensual: Decimal` might |
| Module (`//!` doc) | Yes | Explains the module's role in the architecture |
| Generated entity files | No | Auto-generated by SeaORM; comments would be overwritten |

### Anti-pattern — Redundant documentation

```rust
/// Gets a property by ID.
///
/// # Arguments
///
/// * `db` - The database connection
/// * `id` - The UUID of the property
///
/// # Returns
///
/// A `PropiedadResponse` wrapped in a `Result`
pub async fn get_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    // ...
}
```

The doc comment restates what the function signature already says. Better:

```rust
/// Returns `AppError::NotFound` if no property exists with the given ID.
pub async fn get_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<PropiedadResponse, AppError> {
    // ...
}
```

Document the non-obvious: error conditions, side effects, invariants. The type signature documents the obvious.


## API Surface Minimization

### The 10-Item Rule

A module with more than 10 public items is likely doing too much. This is a heuristic, not a hard rule, but it signals that the module should be reviewed for splitting or reducing its public API.

### Counting Public Items

Count all `pub` functions, types, traits, and constants in a module:

```rust
// services/contratos.rs — 5 public items (good)
pub async fn create(...) { }
pub async fn list(...) { }
pub async fn get_by_id(...) { }
pub async fn update(...) { }
pub async fn delete(...) { }

// Plus private helpers (don't count)
async fn validate_no_overlap(...) { }
fn build_date_condition(...) { }
```

### Strategies for Reducing API Surface

| Strategy | Example |
|----------|---------|
| Make helpers private | `validate_no_overlap` doesn't need to be `pub` |
| Use `pub(crate)` for cross-module internals | Shared utility types that aren't part of the domain API |
| Group related items into submodules | Split a large `models/contrato.rs` into `models/contrato/request.rs` and `models/contrato/response.rs` |
| Re-export only what consumers need | `mod.rs` re-exports the module, not individual items |
| Consolidate similar functions | If `list_active` and `list_inactive` differ only by a filter, use `list` with a query parameter |

### Anti-pattern — Leaking internal types

```rust
// models/contrato.rs — leaks internal types

pub struct CreateContratoRequest { /* ... */ }    // needed by handlers ✓
pub struct UpdateContratoRequest { /* ... */ }    // needed by handlers ✓
pub struct ContratoResponse { /* ... */ }         // needed by handlers ✓
pub struct ContratoListQuery { /* ... */ }        // needed by handlers ✓

// These should be private or pub(crate) — only used within services
pub struct OverlapCheckParams { /* ... */ }       // internal to services ✗
pub struct ContratoWithRelations { /* ... */ }    // internal to services ✗
pub fn format_contrato_dates(...) { }            // utility, not API ✗
```

### Correct — Minimal public surface

```rust
// models/contrato.rs — only what handlers need

pub struct CreateContratoRequest { /* ... */ }
pub struct UpdateContratoRequest { /* ... */ }
pub struct ContratoResponse { /* ... */ }
pub struct ContratoListQuery { /* ... */ }

// Internal types stay in the service module or are private here
struct OverlapCheckParams { /* ... */ }
```


## Layer Boundary Rules

The project's layered architecture imposes strict import direction rules:

```
handlers  →  services  →  entities
    ↓            ↓            ↓
  models       models      (generated)
    ↓
middleware
```

### Allowed Imports by Layer

| Module | Can Import From | Must NOT Import From |
|--------|----------------|---------------------|
| `handlers/` | `services/`, `models/`, `middleware/`, `errors` | `entities/` directly (go through services) |
| `services/` | `entities/`, `models/`, `errors` | `handlers/`, `middleware/` |
| `entities/` | `sea_orm` only | `handlers/`, `services/`, `models/`, `middleware/` |
| `models/` | `serde`, `chrono`, `uuid`, `rust_decimal` | `handlers/`, `services/`, `entities/` |
| `middleware/` | `errors`, `jsonwebtoken`, `models/` (for Claims) | `handlers/`, `services/` |
| `routes.rs` | `handlers/` | `services/`, `entities/` |

### Anti-pattern — Handler imports entities directly

```rust
// handlers/propiedades.rs — bypasses the service layer
use crate::entities::propiedad;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    // Handler is doing the service's job
    let records = propiedad::Entity::find()
        .filter(propiedad::Column::Estado.eq("disponible"))
        .all(db.get_ref())
        .await?;
    Ok(HttpResponse::Ok().json(records))
}
```

### Correct — Handler delegates to service

```rust
// handlers/propiedades.rs
use crate::services::propiedades;

pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PropiedadListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::list(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}
```

### Detection Heuristics

| Signal | Likely Issue |
|--------|-------------|
| `use crate::entities::` in a handler file | Handler bypasses service layer |
| `use crate::handlers::` in a service file | Inverted dependency direction |
| `use sea_orm::ColumnTrait` in a handler | Handler is building queries directly |
| `use actix_web::HttpResponse` in a service | Service is building HTTP responses |
| `use crate::services::` in an entity file | Entity depends on higher layer |
| `use crate::middleware::` in a service | Service depends on HTTP middleware |


## Module Naming Conventions

### File and Module Names

| Convention | Example | Rationale |
|------------|---------|-----------|
| `snake_case` for all module names | `contratos.rs`, `auth.rs` | Rust convention |
| Plural for collection-oriented modules | `handlers/`, `services/`, `entities/`, `models/` | Contains multiple domain items |
| Singular for domain files within | `contrato.rs`, `inquilino.rs`, `pago.rs` | Each file represents one domain entity |
| Descriptive names for cross-cutting | `errors.rs`, `config.rs`, `routes.rs` | Purpose is immediately clear |

### Matching Names Across Layers

The project maintains consistent naming across layers:

```
entities/contrato.rs   → models/contrato.rs   → services/contratos.rs   → handlers/contratos.rs
entities/inquilino.rs  → models/inquilino.rs  → services/inquilinos.rs  → handlers/inquilinos.rs
entities/pago.rs       → models/pago.rs       → services/pagos.rs       → handlers/pagos.rs
entities/propiedad.rs  → models/propiedad.rs  → services/propiedades.rs → handlers/propiedades.rs
```

Note: entities and models use singular (one entity definition per file), while services and handlers use plural (they operate on collections of entities). This is a project convention — consistency matters more than which form is chosen.


## Detection Heuristics Summary

| Signal | Likely Issue | Category |
|--------|-------------|----------|
| Module with >10 `pub` items | API surface too large; split or reduce visibility | API surface |
| `pub` on internal helper functions | Over-exposed API; make private or `pub(crate)` | Visibility |
| Glob re-export (`pub use module::*`) in prelude | Namespace pollution; re-export specific types | Re-exports |
| Missing `mod` declaration for existing `.rs` file | Dead code; file is never compiled | Module structure |
| Handler importing from `entities/` | Layer boundary violation; use services | Architecture |
| Service importing from `handlers/` | Inverted dependency direction | Architecture |
| No module-level `//!` doc comment | Missing context for module's role | Documentation |
| Doc comment restating the type signature | Redundant; document error conditions and invariants instead | Documentation |
| Prelude with >15 re-exports | Too large; defeats the purpose of focused imports | Re-exports |
| Inconsistent naming across layers | Breaks navigation predictability | Naming |
