# Rust Performance Reference Guide

## Iterator Adapter Patterns

### Chained Adapters vs Manual Loops

Iterator adapters (`.map()`, `.filter()`, `.collect()`) compile to the same machine code as hand-written loops thanks to Rust's zero-cost abstractions. Prefer adapters for clarity — they fuse into a single pass with no intermediate allocations.

```rust
// Preferred: chained adapters — single pass, no intermediate Vec
let active_totals: Vec<Decimal> = contratos
    .iter()
    .filter(|c| c.estado == "activo")
    .map(|c| c.monto_mensual)
    .collect();

// Avoid: manual loop doing the same thing with more noise
let mut active_totals = Vec::new();
for c in &contratos {
    if c.estado == "activo" {
        active_totals.push(c.monto_mensual);
    }
}
```

### Fold and Reduce for Aggregation

Use `.fold()` when accumulating a value from a collection. This avoids collecting into an intermediate `Vec` just to sum or aggregate.

```rust
// Preferred: fold directly — no intermediate collection
let ingreso_mensual = contratos_activos
    .iter()
    .fold(Decimal::ZERO, |acc, c| acc + c.monto_mensual);

// Avoid: collecting then summing
let montos: Vec<Decimal> = contratos_activos
    .iter()
    .map(|c| c.monto_mensual)
    .collect();
let ingreso_mensual = montos.iter().sum::<Decimal>();
```

### Collect-Then-Iterate Anti-Pattern

Collecting into a `Vec` only to iterate again wastes an allocation. Chain the operations instead.

```rust
// Anti-pattern: collect then iterate
let active: Vec<_> = pagos.iter().filter(|p| p.estado == "pendiente").collect();
for pago in active.iter() {
    process_pago(pago);
}

// Fix: iterate directly
for pago in pagos.iter().filter(|p| p.estado == "pendiente") {
    process_pago(pago);
}
```

### filter_map for Combined Filter + Map

When filtering and transforming in one step, `filter_map` is more concise and avoids an extra closure layer.

```rust
// Preferred: filter_map
let paid_amounts: Vec<Decimal> = pagos
    .iter()
    .filter_map(|p| {
        if p.estado == "pagado" { Some(p.monto) } else { None }
    })
    .collect();

// Also valid: filter + map (slightly more readable for simple predicates)
let paid_amounts: Vec<Decimal> = pagos
    .iter()
    .filter(|p| p.estado == "pagado")
    .map(|p| p.monto)
    .collect();
```

## `into_iter()` vs `iter()` vs `iter_mut()` Selection

### When to Use Each

| Method | Yields | Ownership | Use When |
|--------|--------|-----------|----------|
| `.iter()` | `&T` | Borrows | Reading without consuming the collection |
| `.iter_mut()` | `&mut T` | Mutable borrow | Modifying elements in place |
| `.into_iter()` | `T` | Consumes | Transforming into a new collection, done with the original |

### SeaORM Query Results

SeaORM `.all()` returns `Vec<Model>` which you own. Use `.into_iter()` when converting to response types — this moves each `Model` instead of cloning.

```rust
// Preferred: into_iter moves ownership — no clones needed
pub async fn list(
    db: &DatabaseConnection,
    query: PagoListQuery,
) -> Result<Vec<PagoResponse>, AppError> {
    let records = pago::Entity::find()
        .order_by_desc(pago::Column::FechaVencimiento)
        .all(db)
        .await?;

    Ok(records.into_iter().map(PagoResponse::from).collect())
}

// Avoid: iter() + clone when you don't need the original Vec
let responses: Vec<PagoResponse> = records
    .iter()
    .map(|r| PagoResponse::from(r.clone()))
    .collect();
```

### Actix-web Handler Extractors

Actix extractors like `web::Json<T>` and `web::Query<T>` provide `.into_inner()` which moves the value out. Use this instead of cloning.

```rust
// Preferred: move out of the extractor
pub async fn create(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreatePropiedadRequest>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::create(db.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

// Avoid: cloning the inner value
let input = body.clone().into_inner();
```

## Zero-Copy Techniques

### `&str` Over `String`

Use `&str` for function parameters that only read string data. This avoids forcing callers to allocate a `String`.

```rust
// Preferred: accept &str — works with both String and &str callers
fn is_valid_estado(estado: &str) -> bool {
    matches!(estado, "disponible" | "ocupada" | "mantenimiento")
}

// Avoid: requiring String ownership when only reading
fn is_valid_estado(estado: String) -> bool {
    matches!(estado.as_str(), "disponible" | "ocupada" | "mantenimiento")
}
```

### `&[T]` Over `Vec<T>`

Accept slices for read-only access to collections. This works with `Vec`, arrays, and other contiguous storage.

```rust
// Preferred: accept a slice — caller keeps ownership
fn total_ingresos(pagos: &[pago::Model]) -> Decimal {
    pagos
        .iter()
        .filter(|p| p.estado == "pagado")
        .fold(Decimal::ZERO, |acc, p| acc + p.monto)
}

// Avoid: taking Vec by value when you only need to read
fn total_ingresos(pagos: Vec<pago::Model>) -> Decimal {
    pagos
        .iter()
        .filter(|p| p.estado == "pagado")
        .fold(Decimal::ZERO, |acc, p| acc + p.monto)
}
```

### String Comparisons Without Allocation

Compare against string literals directly — no need to create a `String` for comparison.

```rust
// Preferred: compare &str field against literal
if propiedad.estado == "disponible" {
    // ...
}

// Avoid: allocating a String for comparison
let target = String::from("disponible");
if propiedad.estado == target {
    // ...
}
```

### Borrowing SeaORM Filter Values

When building SeaORM queries with optional filters, borrow the `Option` contents with `ref` to avoid moving out of the query struct.

```rust
// Preferred: ref borrows the inner String without moving
if let Some(ref ciudad) = query.ciudad {
    select = select.filter(propiedad::Column::Ciudad.eq(ciudad));
}

// Avoid: moving out of query (won't compile if query is used later)
if let Some(ciudad) = query.ciudad {
    select = select.filter(propiedad::Column::Ciudad.eq(ciudad));
}
```

## `Cow<T>` for Conditional Ownership

### When to Use Cow

`Cow<'_, T>` (Clone on Write) is ideal when a function sometimes returns borrowed data and sometimes needs to allocate. It avoids allocation in the common (borrowed) path.

### Default Values Without Allocation

```rust
use std::borrow::Cow;

fn moneda_display(moneda: &str) -> Cow<'_, str> {
    match moneda {
        "DOP" => Cow::Borrowed("RD$"),
        "USD" => Cow::Borrowed("US$"),
        other => Cow::Owned(format!("{other} ")),
    }
}
```

Known values return a borrowed static string (zero allocation). Unknown values allocate only when needed.

### Normalizing Input Strings

```rust
use std::borrow::Cow;

fn normalize_cedula(cedula: &str) -> Cow<'_, str> {
    if cedula.contains('-') {
        Cow::Owned(cedula.replace('-', ""))
    } else {
        Cow::Borrowed(cedula)
    }
}
```

If the input is already clean, no allocation occurs. Only malformed input triggers a `String` allocation.

### Cow in Actix-web Error Responses

```rust
use std::borrow::Cow;

fn error_message_for_status(status: u16) -> Cow<'static, str> {
    match status {
        404 => Cow::Borrowed("Recurso no encontrado"),
        401 => Cow::Borrowed("No autorizado"),
        403 => Cow::Borrowed("Acceso denegado"),
        _ => Cow::Owned(format!("Error inesperado (código {})", status)),
    }
}
```

Static messages are borrowed from the binary. Dynamic messages allocate only for uncommon status codes.

### Cow in Function Parameters

Accept `Cow` when a function needs to optionally take ownership:

```rust
use std::borrow::Cow;

fn log_action(entity: &str, action: Cow<'_, str>) {
    tracing::info!("{entity}: {action}");
}

// Caller with static string — no allocation
log_action("propiedad", Cow::Borrowed("created"));

// Caller with dynamic string — allocation happens at call site
log_action("propiedad", Cow::Owned(format!("updated field {field_name}")));
```

## Pre-Allocation

### `Vec::with_capacity`

When the final size is known or estimable, pre-allocate to avoid repeated reallocations.

```rust
// Preferred: pre-allocate when size is known
let mut responses = Vec::with_capacity(records.len());
for record in records {
    responses.push(PropiedadResponse::from(record));
}

// Even better: use into_iter + collect (capacity is inferred from ExactSizeIterator)
let responses: Vec<PropiedadResponse> = records
    .into_iter()
    .map(PropiedadResponse::from)
    .collect();
```

`collect()` on an `ExactSizeIterator` (like `Vec::into_iter()`) automatically pre-allocates the correct capacity.

### `String::with_capacity`

For building strings in a loop, pre-allocate based on expected length.

```rust
// Preferred: estimate capacity
fn build_csv_line(fields: &[&str]) -> String {
    let capacity: usize = fields.iter().map(|f| f.len() + 1).sum();
    let mut line = String::with_capacity(capacity);
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            line.push(',');
        }
        line.push_str(field);
    }
    line
}
```

## Actix-web Specific Patterns

### Avoid Cloning `web::Data`

`web::Data<T>` is internally `Arc<T>`. Calling `.get_ref()` borrows the inner value without cloning the `Arc`.

```rust
// Preferred: borrow the inner DatabaseConnection
pub async fn list(
    db: web::Data<DatabaseConnection>,
    query: web::Query<PropiedadListQuery>,
) -> Result<HttpResponse, AppError> {
    let result = propiedades::list(db.get_ref(), query.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

// Avoid: cloning the Arc unnecessarily
let db_clone = db.clone();
let result = propiedades::list(&db_clone, query.into_inner()).await?;
```

### Response Serialization

`HttpResponse::Ok().json(value)` serializes directly. Avoid converting to a `String` first.

```rust
// Preferred: serialize directly
Ok(HttpResponse::Ok().json(result))

// Avoid: double serialization
let json_string = serde_json::to_string(&result)?;
Ok(HttpResponse::Ok().body(json_string))
```

## SeaORM Specific Patterns

### From Trait for Model Conversion

Implement `From<Model>` for response types. This enables zero-cost conversion with `into_iter().map(From::from).collect()`.

```rust
impl From<propiedad::Model> for PropiedadResponse {
    fn from(m: propiedad::Model) -> Self {
        Self {
            id: m.id,
            titulo: m.titulo,
            direccion: m.direccion,
            ciudad: m.ciudad,
            provincia: m.provincia,
            // ... fields moved, not cloned
        }
    }
}

// Usage: into_iter moves each Model into From::from
let responses: Vec<PropiedadResponse> = records
    .into_iter()
    .map(PropiedadResponse::from)
    .collect();
```

### Paginated Queries

SeaORM's `.paginate()` returns a `Paginator` that fetches only the requested page. Avoid fetching all records then slicing in memory.

```rust
// Preferred: database-level pagination
let paginator = propiedad::Entity::find()
    .order_by_desc(propiedad::Column::CreatedAt)
    .paginate(db, per_page);

let total = paginator.num_items().await?;
let records = paginator.fetch_page(page - 1).await?;

// Avoid: fetching everything then slicing
let all_records = propiedad::Entity::find().all(db).await?;
let page_records = &all_records[start..end]; // wastes memory
```

### Counting Without Fetching

Use `.count()` when you only need the count, not the actual records.

```rust
// Preferred: count at database level
let total_propiedades = propiedad::Entity::find().count(db).await?;

// Avoid: fetching all records just to count
let all = propiedad::Entity::find().all(db).await?;
let total = all.len();
```
