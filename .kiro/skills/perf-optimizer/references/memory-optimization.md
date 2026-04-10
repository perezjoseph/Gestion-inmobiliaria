# Memory Optimization Reference Guide

## Stack vs Heap Allocation

### When Values Live on the Stack

Primitive types, fixed-size arrays, and small structs live on the stack by default. Stack allocation is essentially free — it's just a pointer bump.

```rust
// Stack-allocated: fixed size, known at compile time
let count: u32 = 42;
let coords: [f64; 3] = [1.0, 2.0, 3.0];

struct PagoSummary {
    total: Decimal,
    count: u32,
    has_late: bool,
}

// This struct is small and fixed-size — lives on the stack
let summary = PagoSummary {
    total: Decimal::ZERO,
    count: 0,
    has_late: false,
};
```

### When Values Move to the Heap

`String`, `Vec<T>`, `HashMap<K, V>`, and `Box<T>` allocate on the heap. Each heap allocation involves a system call and must be freed later. Minimize unnecessary heap allocations in hot paths.

```rust
// Heap-allocated: dynamic size
let name = String::from("Apartamento Centro");  // heap
let pagos: Vec<PagoResponse> = Vec::new();       // heap (buffer allocated on first push)
let lookup: HashMap<i32, String> = HashMap::new(); // heap
```

### Size Thresholds

As a rule of thumb, types larger than ~128 bytes should be boxed to avoid expensive stack copies when moved. Check with `std::mem::size_of`:

```rust
// Check struct size at compile time
const _: () = assert!(std::mem::size_of::<SmallStruct>() <= 128);

// Or at runtime during development
println!("Size of ContratoResponse: {} bytes", std::mem::size_of::<ContratoResponse>());
```

## `Box<T>` for Large Types

### Reducing Stack Frame Size

Use `Box<T>` to move large types to the heap, keeping the stack frame small (a `Box` is just a pointer — 8 bytes on 64-bit).

```rust
// Anti-pattern: large struct on the stack, expensive to move
struct FullReport {
    summary: String,
    findings: [Finding; 256],  // large fixed array
    metadata: ReportMetadata,
}

fn generate_report() -> FullReport {
    // This copies ~N KB on every return
    FullReport { /* ... */ }
}

// Fix: box the large type
fn generate_report() -> Box<FullReport> {
    Box::new(FullReport { /* ... */ })
}
```

### Recursive Types

Recursive types require `Box` because the compiler can't determine their size at compile time.

```rust
// Won't compile: infinite size
// enum FilterTree {
//     Leaf(String),
//     And(FilterTree, FilterTree),
// }

// Fix: Box the recursive variants
enum FilterTree {
    Leaf(String),
    And(Box<FilterTree>, Box<FilterTree>),
    Or(Box<FilterTree>, Box<FilterTree>),
}

// Usage: building a filter for property queries
let filter = FilterTree::And(
    Box::new(FilterTree::Leaf("ciudad = 'Santo Domingo'".into())),
    Box::new(FilterTree::Leaf("estado = 'disponible'".into())),
);
```

### Trait Objects

`Box<dyn Trait>` enables dynamic dispatch when you need to store different concrete types behind a common interface.

```rust
use std::fmt;

trait Validator: Send + Sync {
    fn validate(&self, value: &str) -> Result<(), String>;
}

// Store heterogeneous validators in a Vec
fn build_validators() -> Vec<Box<dyn Validator>> {
    vec![
        Box::new(CedulaValidator),
        Box::new(EmailValidator),
        Box::new(MontoValidator { min: Decimal::ZERO }),
    ]
}
```

## `Rc<T>` and `Arc<T>` for Shared Ownership

### `Rc<T>` — Single-Threaded Shared Ownership

Use `Rc<T>` when multiple parts of your code need to read the same data and no single owner is obvious. `Rc` uses reference counting — no heap copy, just a counter increment.

```rust
use std::rc::Rc;

// Shared configuration across multiple components (single-threaded)
let config = Rc::new(AppConfig::load()?);

let validator = PropertyValidator::new(Rc::clone(&config));
let formatter = ResponseFormatter::new(Rc::clone(&config));
// Both hold a reference to the same AppConfig — no clone of the data
```

### `Arc<T>` — Thread-Safe Shared Ownership

Use `Arc<T>` when shared data crosses thread or task boundaries. This is common in Actix-web handlers where `web::Data<T>` is internally `Arc<T>`.

```rust
use std::sync::Arc;

// Actix-web already wraps Data in Arc — don't double-wrap
// Anti-pattern: Arc inside Arc
let db = Arc::new(create_connection().await?);
let data = web::Data::new(db); // web::Data<Arc<DatabaseConnection>> — double Arc!

// Fix: let web::Data handle the Arc
let db = create_connection().await?;
let data = web::Data::new(db); // web::Data<DatabaseConnection> — single Arc
```

### Choosing Between `Rc` and `Arc`

| Scenario | Use | Why |
|----------|-----|-----|
| Single-threaded local sharing | `Rc<T>` | No atomic overhead |
| Shared across `tokio::spawn` tasks | `Arc<T>` | Requires `Send + Sync` |
| Actix-web shared state | `web::Data<T>` (uses `Arc`) | Framework handles it |
| Read-heavy, rare writes | `Arc<RwLock<T>>` | Concurrent reads, exclusive writes |
| Frequent writes from multiple tasks | `Arc<Mutex<T>>` | Exclusive access |

### Cloning `Arc` is Cheap

`Arc::clone()` increments an atomic counter — it does not clone the inner data. This is idiomatic and should not be flagged as an unnecessary clone.

```rust
// This is fine — only increments the reference count
let db = Arc::clone(&shared_db);
tokio::spawn(async move {
    let result = some_query(&db).await;
});
```

## Clone Detection Heuristics

### Unnecessary `String` Clones

Flag `.clone()` on `String` when the cloned value is passed to a function accepting `&str` or used only for reading.

```rust
// Anti-pattern: clone String to pass as &str
fn log_property(name: &str) {
    tracing::info!("Processing property: {name}");
}

let prop_name = propiedad.titulo.clone(); // unnecessary!
log_property(&prop_name);

// Fix: borrow directly
log_property(&propiedad.titulo);
```

### Unnecessary `Vec<T>` Clones

Flag `.clone()` on `Vec<T>` when the clone is only iterated or passed to a function accepting `&[T]`.

```rust
// Anti-pattern: clone Vec just to iterate
fn count_active(contratos: &[contrato::Model]) -> usize {
    contratos.iter().filter(|c| c.estado == "activo").count()
}

let active_count = count_active(&contratos.clone()); // unnecessary clone!

// Fix: pass a reference
let active_count = count_active(&contratos);
```

### Unnecessary `HashMap` Clones

Flag `.clone()` on `HashMap` when the clone is only used for lookups or iteration.

```rust
// Anti-pattern: clone HashMap for read-only access
fn find_tenant(
    lookup: &HashMap<i32, inquilino::Model>,
    id: i32,
) -> Option<&inquilino::Model> {
    lookup.get(&id)
}

let tenant = find_tenant(&tenant_map.clone(), tenant_id); // unnecessary!

// Fix: borrow the map
let tenant = find_tenant(&tenant_map, tenant_id);
```

### When Cloning IS Necessary

Do not flag these patterns — cloning is required here:

```rust
// 1. Moving into a closure that crosses a task boundary
let name = propiedad.titulo.clone();
tokio::spawn(async move {
    process(name).await; // needs owned String for Send
});

// 2. Storing in a struct that requires ownership
let response = PropiedadResponse {
    titulo: propiedad.titulo.clone(), // struct owns its fields
    ciudad: propiedad.ciudad.clone(),
};

// 3. Mutating a copy while preserving the original
let mut updated = contrato.clone();
updated.estado = "finalizado".to_string();
// original `contrato` is still needed later
```

### Clone Detection Decision Tree

```
Is .clone() called?
├── Is the cloned value passed to a fn accepting &T / &str / &[T]?
│   └── YES → Flag: pass a reference instead
├── Is the cloned value only read (no mutation)?
│   └── YES → Flag: borrow with & instead
├── Is the clone moved into a tokio::spawn or thread::spawn?
│   └── NO flag: clone needed for Send bound
├── Is the clone stored in a struct field that requires ownership?
│   └── NO flag: ownership transfer is intentional
└── Is the original used after the clone is mutated?
    └── NO flag: defensive copy is correct
```

## `String::with_capacity` Pre-Allocation

### Why Pre-Allocate

`String` starts with zero capacity and doubles its buffer on each reallocation. If you know the approximate final size, pre-allocating avoids repeated reallocations and copies.

```rust
// Without pre-allocation: multiple reallocations as the string grows
let mut result = String::new();
for pago in pagos {
    result.push_str(&format!("{}: {}\n", pago.id, pago.monto));
}

// With pre-allocation: single allocation up front
let estimated_len = pagos.len() * 30; // ~30 chars per line
let mut result = String::with_capacity(estimated_len);
for pago in pagos {
    result.push_str(&format!("{}: {}\n", pago.id, pago.monto));
}
```

### Estimating Capacity

Use these heuristics for common patterns:

| Pattern | Estimate |
|---------|----------|
| CSV line from N fields | `fields.iter().map(\|f\| f.len() + 1).sum()` |
| Joining strings with separator | `items.iter().map(\|s\| s.len()).sum::<usize>() + items.len()` |
| Formatted output per record | `record_count * avg_chars_per_record` |
| SQL IN clause | `ids.len() * 12` (up to 10-digit IDs + comma + space) |

```rust
// Building a comma-separated ID list for logging
fn format_id_list(ids: &[i32]) -> String {
    let mut result = String::with_capacity(ids.len() * 12);
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        result.push_str(&id.to_string());
    }
    result
}
```

### `Vec::with_capacity` for Known Sizes

The same principle applies to `Vec`. When the output size matches the input size, pre-allocate.

```rust
// Preferred: pre-allocate when transforming a collection 1:1
fn to_responses(models: Vec<propiedad::Model>) -> Vec<PropiedadResponse> {
    let mut responses = Vec::with_capacity(models.len());
    for model in models {
        responses.push(PropiedadResponse::from(model));
    }
    responses
}

// Even better: collect from ExactSizeIterator (auto pre-allocates)
fn to_responses(models: Vec<propiedad::Model>) -> Vec<PropiedadResponse> {
    models.into_iter().map(PropiedadResponse::from).collect()
}
```

### `HashMap::with_capacity` for Bulk Inserts

When building a lookup table from a known-size collection, pre-allocate to avoid rehashing.

```rust
// Building a tenant lookup by ID
fn build_tenant_lookup(
    tenants: Vec<inquilino::Model>,
) -> HashMap<i32, inquilino::Model> {
    let mut map = HashMap::with_capacity(tenants.len());
    for tenant in tenants {
        map.insert(tenant.id, tenant);
    }
    map
}

// Or use collect with into_iter (also pre-allocates from ExactSizeIterator)
fn build_tenant_lookup(
    tenants: Vec<inquilino::Model>,
) -> HashMap<i32, inquilino::Model> {
    tenants.into_iter().map(|t| (t.id, t)).collect()
}
```
