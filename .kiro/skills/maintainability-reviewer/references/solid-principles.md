# SOLID Principles Reference Guide

## Project Architecture Overview

This project follows a layered architecture with clear separation of concerns:

```
handlers/   → Parse HTTP requests, validate input, call services, return responses
services/   → Business logic only, no HTTP concerns
entities/   → SeaORM data models, never manually edited
models/     → Request/response DTOs (serde-based)
errors.rs   → Centralized AppError enum with thiserror
```

Each SOLID principle maps naturally to this layering. Violations typically manifest as logic leaking across layer boundaries.


## Single Responsibility Principle (SRP)

**Each module, struct, and function should have one reason to change.**

In Rust, SRP maps to module boundaries: handlers own HTTP concerns, services own business logic, entities own data shape.

### Correct — Handler delegates to service

```rust
pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}
```

The handler's only job: extract inputs, call the service, build the HTTP response.

### Anti-pattern — Handler contains business logic

```rust
pub async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let input = body.into_inner();

    // SRP violation: overlap detection belongs in the service layer
    let existing = contrato::Entity::find()
        .filter(contrato::Column::PropiedadId.eq(input.propiedad_id))
        .filter(contrato::Column::Estado.eq("activo"))
        .all(db.get_ref())
        .await?;

    for c in &existing {
        if c.fecha_fin >= input.fecha_inicio && input.fecha_fin >= c.fecha_inicio {
            return Err(AppError::Conflict(
                "Contrato se solapa con uno existente".into(),
            ));
        }
    }

    let now = Utc::now().into();
    let model = contrato::ActiveModel {
        id: Set(Uuid::new_v4()),
        propiedad_id: Set(input.propiedad_id),
        // ... more fields
        created_at: Set(now),
        updated_at: Set(now),
    };
    let record = model.insert(db.get_ref()).await?;
    Ok(HttpResponse::Created().json(ContratoResponse::from(record)))
}
```

### Fix — Move logic to service

```rust
// handler — HTTP only
pub async fn create_contrato(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateContratoRequest>,
) -> Result<HttpResponse, AppError> {
    let contrato = contratos::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(contrato))
}

// service — business logic only
pub async fn create(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    validate_no_overlap(db, input.propiedad_id, input.fecha_inicio, input.fecha_fin, None).await?;
    // ... insert and return
}
```

### SRP Detection Heuristics

| Signal | Likely Violation |
|--------|-----------------|
| Handler imports `sea_orm::ColumnTrait` or `QueryFilter` | Handler is querying the DB directly instead of calling a service |
| Service imports `actix_web::HttpResponse` | Service is building HTTP responses instead of returning domain types |
| Function exceeds 50 lines | Multiple responsibilities mixed in one function |
| Module has more than 10 `pub` items | Module is doing too many things; split or reduce API surface |


## Open-Closed Principle (OCP)

**Types should be open for extension but closed for modification.**

In Rust, OCP is achieved through trait objects (`dyn Trait`) and generics (`impl Trait` / `<T: Trait>`). New behavior is added by implementing existing traits, not by modifying existing code.

### Correct — Extensible via trait

```rust
trait PaymentProcessor {
    fn process(&self, pago: &pago::Model) -> Result<(), AppError>;
}

struct TransferenciaProcessor;
struct EfectivoProcessor;

impl PaymentProcessor for TransferenciaProcessor {
    fn process(&self, pago: &pago::Model) -> Result<(), AppError> {
        // bank transfer logic
        Ok(())
    }
}

impl PaymentProcessor for EfectivoProcessor {
    fn process(&self, pago: &pago::Model) -> Result<(), AppError> {
        // cash payment logic
        Ok(())
    }
}

// Adding a new payment method requires only a new struct + impl,
// no changes to existing code
fn process_pago(
    processor: &dyn PaymentProcessor,
    pago: &pago::Model,
) -> Result<(), AppError> {
    processor.process(pago)
}
```

### Anti-pattern — Match on string type

```rust
fn process_pago(pago: &pago::Model) -> Result<(), AppError> {
    match pago.metodo_pago.as_deref() {
        Some("transferencia") => { /* ... */ }
        Some("efectivo") => { /* ... */ }
        // Adding a new method requires modifying this function
        _ => Err(AppError::Validation("Método de pago no soportado".into())),
    }
}
```

### OCP with Generics

Generics provide compile-time dispatch (zero-cost abstraction) when the concrete type is known at compile time:

```rust
async fn validate_entity<C: ConnectionTrait>(
    db: &C,
    entity_id: Uuid,
) -> Result<propiedad::Model, AppError> {
    propiedad::Entity::find_by_id(entity_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".into()))
}
```

This function works with `&DatabaseConnection`, `&DatabaseTransaction`, or any future `ConnectionTrait` implementor — no modification needed.

### When to Use Each

| Approach | Use When |
|----------|----------|
| `dyn Trait` (trait object) | Runtime polymorphism, heterogeneous collections, plugin-style extensibility |
| `impl Trait` / generics | Compile-time dispatch, known types, zero-cost abstraction |
| Enum variants | Closed set of variants that rarely changes (like `AppError`) |


## Liskov Substitution Principle (LSP)

**Subtypes must be substitutable for their base types without altering correctness.**

In Rust, LSP applies to trait implementations: any type implementing a trait must honor the trait's contract. Callers should work correctly regardless of which concrete type they receive.

### Correct — Consistent trait behavior

```rust
trait Notifier {
    fn notify(&self, inquilino_id: Uuid, message: &str) -> Result<(), AppError>;
}

struct EmailNotifier;
struct SmsNotifier;

impl Notifier for EmailNotifier {
    fn notify(&self, inquilino_id: Uuid, message: &str) -> Result<(), AppError> {
        // sends email — always returns Ok or a meaningful error
        Ok(())
    }
}

impl Notifier for SmsNotifier {
    fn notify(&self, inquilino_id: Uuid, message: &str) -> Result<(), AppError> {
        // sends SMS — same contract: Ok on success, Err on failure
        Ok(())
    }
}

// Caller works identically with any Notifier implementation
fn notify_late_payment(
    notifier: &dyn Notifier,
    inquilino_id: Uuid,
) -> Result<(), AppError> {
    notifier.notify(inquilino_id, "Su pago está vencido")
}
```

### Anti-pattern — Implementation violates trait contract

```rust
impl Notifier for NoOpNotifier {
    fn notify(&self, _inquilino_id: Uuid, _message: &str) -> Result<(), AppError> {
        // LSP violation: silently panics instead of returning Result
        panic!("NoOpNotifier should not be called in production");
    }
}
```

A `NoOpNotifier` that panics violates LSP because callers expect `notify` to return a `Result`, not abort. A correct no-op implementation returns `Ok(())`.

### LSP in SeaORM's ConnectionTrait

The project already benefits from LSP through SeaORM's `ConnectionTrait`:

```rust
async fn validate_no_overlap<C: ConnectionTrait>(
    db: &C,
    propiedad_id: Uuid,
    fecha_inicio: chrono::NaiveDate,
    fecha_fin: chrono::NaiveDate,
    exclude_id: Option<Uuid>,
) -> Result<(), AppError> {
    // Works with DatabaseConnection or DatabaseTransaction
    // Both honor the same query semantics — LSP in action
    let overlapping = contrato::Entity::find()
        .filter(condition)
        .one(db)
        .await?;
    // ...
}
```

Both `DatabaseConnection` and `DatabaseTransaction` implement `ConnectionTrait` and behave consistently for queries. The transaction variant adds atomicity guarantees without breaking the query contract.

### LSP Detection Heuristics

| Signal | Likely Violation |
|--------|-----------------|
| Trait impl that panics or returns hardcoded errors | Breaks caller expectations |
| Trait impl that ignores required parameters | Semantic contract violation |
| Downcasting (`Any`) to check concrete type before calling | Caller doesn't trust the trait abstraction |
| Different error semantics across implementations | Callers can't handle errors uniformly |


## Interface Segregation Principle (ISP)

**Clients should not be forced to depend on methods they don't use.**

In Rust, ISP means designing fine-grained traits rather than large monolithic ones. Types implement only the traits they need.

### Correct — Fine-grained traits

```rust
trait Readable {
    fn get_by_id(db: &DatabaseConnection, id: Uuid) -> impl Future<Output = Result<Self, AppError>>
    where
        Self: Sized;
}

trait Listable {
    type Query;
    fn list(
        db: &DatabaseConnection,
        query: Self::Query,
    ) -> impl Future<Output = Result<Vec<Self>, AppError>>
    where
        Self: Sized;
}

trait Writable {
    type Input;
    fn create(
        db: &DatabaseConnection,
        input: Self::Input,
    ) -> impl Future<Output = Result<Self, AppError>>
    where
        Self: Sized;
}

trait Deletable {
    fn delete(db: &DatabaseConnection, id: Uuid) -> impl Future<Output = Result<(), AppError>>;
}
```

A read-only dashboard service only needs `Readable + Listable`. A full CRUD handler uses all four. Neither is forced to depend on unused capabilities.

### Anti-pattern — God trait

```rust
trait CrudRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Self, AppError> where Self: Sized;
    async fn list(&self) -> Result<Vec<Self>, AppError> where Self: Sized;
    async fn create(&self, db: &DatabaseConnection) -> Result<Self, AppError> where Self: Sized;
    async fn update(&self, db: &DatabaseConnection) -> Result<Self, AppError> where Self: Sized;
    async fn delete(&self, db: &DatabaseConnection) -> Result<(), AppError>;
    async fn validate(&self) -> Result<(), AppError>;
    async fn notify_created(&self) -> Result<(), AppError>;
    async fn export_csv(&self) -> Result<String, AppError>;
}
```

Types that only need read access are forced to provide stub implementations for `create`, `update`, `delete`, `notify_created`, and `export_csv`.

### ISP in the Project's Middleware

The project already applies ISP through its middleware extractors:

```rust
// Claims — any authenticated user
pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
    query: web::Query<PropiedadListQuery>,
) -> Result<HttpResponse, AppError> { /* ... */ }

// WriteAccess — users with write permissions (admin, gerente)
pub async fn create(
    db: web::Data<DatabaseConnection>,
    _access: WriteAccess,
    body: web::Json<CreatePropiedadRequest>,
) -> Result<HttpResponse, AppError> { /* ... */ }

// AdminOnly — only admin users
pub async fn delete(
    db: web::Data<DatabaseConnection>,
    _admin: AdminOnly,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> { /* ... */ }
```

Each extractor (`Claims`, `WriteAccess`, `AdminOnly`) is a focused interface. Handlers declare exactly the authorization level they need — no more, no less.

### ISP Detection Heuristics

| Signal | Likely Violation |
|--------|-----------------|
| Trait with more than 5 methods | Consider splitting into focused traits |
| Implementations with `unimplemented!()` or `todo!()` stubs | Type doesn't need all trait methods |
| Trait used in only one place | May be over-abstracted; consider a concrete type |
| Multiple `where` clauses combining many trait bounds | Callers may need a subset; split the traits |


## Dependency Inversion Principle (DIP)

**High-level modules should depend on abstractions, not on low-level details.**

In Rust, DIP is achieved by depending on traits rather than concrete types. The project's layered architecture naturally supports this: handlers depend on service function signatures, services depend on `ConnectionTrait` (not a specific database), and error types use `From` trait conversions.

### Correct — Service depends on trait, not concrete DB

```rust
pub async fn create(
    db: &DatabaseConnection,
    input: CreatePropiedadRequest,
) -> Result<PropiedadResponse, AppError> {
    // DatabaseConnection implements ConnectionTrait
    // The service doesn't know or care about the underlying driver
    let model = propiedad::ActiveModel { /* ... */ };
    let record = model.insert(db).await?;
    Ok(PropiedadResponse::from(record))
}
```

### DIP with Trait-Based Dependency Injection

For testability and flexibility, inject dependencies as trait objects:

```rust
trait ContratoRepository {
    async fn find_overlapping(
        &self,
        propiedad_id: Uuid,
        fecha_inicio: chrono::NaiveDate,
        fecha_fin: chrono::NaiveDate,
    ) -> Result<Option<contrato::Model>, AppError>;

    async fn insert(
        &self,
        model: contrato::ActiveModel,
    ) -> Result<contrato::Model, AppError>;
}

struct SeaOrmContratoRepo<'a> {
    db: &'a DatabaseConnection,
}

impl ContratoRepository for SeaOrmContratoRepo<'_> {
    async fn find_overlapping(
        &self,
        propiedad_id: Uuid,
        fecha_inicio: chrono::NaiveDate,
        fecha_fin: chrono::NaiveDate,
    ) -> Result<Option<contrato::Model>, AppError> {
        let condition = Condition::all()
            .add(contrato::Column::PropiedadId.eq(propiedad_id))
            .add(contrato::Column::Estado.eq("activo"))
            .add(contrato::Column::FechaInicio.lt(fecha_fin))
            .add(contrato::Column::FechaFin.gt(fecha_inicio));

        Ok(contrato::Entity::find().filter(condition).one(self.db).await?)
    }

    async fn insert(
        &self,
        model: contrato::ActiveModel,
    ) -> Result<contrato::Model, AppError> {
        Ok(model.insert(self.db).await?)
    }
}

// Service depends on the trait, not the concrete implementation
async fn create_contrato(
    repo: &dyn ContratoRepository,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    let overlap = repo
        .find_overlapping(input.propiedad_id, input.fecha_inicio, input.fecha_fin)
        .await?;

    if overlap.is_some() {
        return Err(AppError::Conflict(
            "Contrato se solapa con uno existente".into(),
        ));
    }

    let model = contrato::ActiveModel { /* ... */ };
    let record = repo.insert(model).await?;
    Ok(ContratoResponse::from(record))
}
```

### Anti-pattern — Service tightly coupled to database

```rust
use sea_orm::DatabaseConnection;

pub async fn create_contrato(
    db: &DatabaseConnection,
    input: CreateContratoRequest,
) -> Result<ContratoResponse, AppError> {
    // Directly uses DatabaseConnection — cannot substitute for testing
    // without a real database
    let overlapping = contrato::Entity::find()
        .filter(/* ... */)
        .one(db)
        .await?;
    // ...
}
```

While the project currently uses `&DatabaseConnection` directly (which is pragmatic for a small codebase), introducing repository traits becomes valuable when:
- Unit testing services without a database
- Swapping storage backends
- Adding caching layers transparently

### DIP via Actix-web's App Data

The project uses Actix-web's dependency injection through `web::Data`:

```rust
// main.rs — register the concrete dependency
let db = Database::connect(&config.database_url).await?;
HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(db.clone()))
        // handlers receive db via web::Data<DatabaseConnection>
})

// handler — depends on the injected abstraction
pub async fn list(
    db: web::Data<DatabaseConnection>,
    _claims: Claims,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::list(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(result))
}
```

The handler doesn't construct or configure the database connection — it receives it through Actix-web's DI container. This is DIP at the framework level.

### DIP via From Trait for Error Conversion

The `AppError` type uses `From` trait implementations to decouple error sources from error handling:

```rust
// errors.rs — defines conversions, not concrete handling
impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::Internal(anyhow::anyhow!(err))
    }
}

// service code — uses ? operator, doesn't know about HTTP
let record = propiedad::Entity::find_by_id(id)
    .one(db)
    .await?; // DbErr auto-converts to AppError::Internal
```

Services don't need to know how errors become HTTP responses. The `From` impl and `ResponseError` trait handle that inversion.

### DIP Detection Heuristics

| Signal | Likely Violation |
|--------|-----------------|
| Service function takes `&DatabaseConnection` and could take `&dyn ConnectionTrait` | Tight coupling to concrete DB type |
| Handler constructs its own dependencies (e.g., `Database::connect()`) | Should receive via `web::Data` |
| Service imports from `handlers::` module | Inverted dependency direction |
| Entity module imports from `services::` module | Inverted dependency direction |
| Hard-coded file paths or URLs in service logic | Should be injected via configuration |


## Quick Reference: SOLID in the Project's Layers

| Principle | Handler Layer | Service Layer | Entity Layer |
|-----------|--------------|---------------|--------------|
| SRP | Parse HTTP, validate input, return response | Business logic, validation rules, orchestration | Data shape only |
| OCP | New endpoints = new handler functions | New behavior via trait implementations | Generated by SeaORM, not modified |
| LSP | Middleware extractors are interchangeable (`Claims`, `WriteAccess`, `AdminOnly`) | `ConnectionTrait` works with connection or transaction | `Model` ↔ `ActiveModel` conversions are consistent |
| ISP | Each extractor checks one concern | Fine-grained service functions, not god-services | Entities expose only data fields |
| DIP | Receives `web::Data<T>` via DI | Depends on `ConnectionTrait`, not concrete DB | No upward dependencies |

## Common Violations by Layer

### Handlers

- Importing `sea_orm::ColumnTrait` or building queries directly (SRP)
- Containing business rules like overlap detection or payment calculations (SRP)
- Constructing service dependencies instead of receiving them (DIP)

### Services

- Importing `actix_web::HttpResponse` or `web::Json` (SRP)
- Using `anyhow::Result` as return type instead of `Result<T, AppError>` (DIP — callers can't match variants)
- Monolithic functions doing validation + DB queries + side effects (SRP)

### Entities

- Adding business methods to entity models (SRP — entities are data-only)
- Importing from `services::` or `handlers::` (DIP — entities are the lowest layer)
