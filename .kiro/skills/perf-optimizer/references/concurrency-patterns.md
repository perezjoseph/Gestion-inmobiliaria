# Concurrency Patterns Reference Guide

## `tokio::spawn` vs `spawn_blocking`

### When to Use Each

| Function | Use When | Thread Pool |
|----------|----------|-------------|
| `tokio::spawn` | Running async code concurrently | tokio async worker threads |
| `tokio::task::spawn_blocking` | Running CPU-heavy or blocking sync code | Dedicated blocking thread pool |

### `tokio::spawn` for Async Work

Use `tokio::spawn` to run async tasks concurrently on the tokio runtime. The spawned future must be `Send + 'static`.

```rust
use sea_orm::DatabaseConnection;

/// Fire off multiple independent DB queries concurrently
pub async fn dashboard_stats(db: &DatabaseConnection) -> Result<DashboardResponse, AppError> {
    let db1 = db.clone();
    let db2 = db.clone();
    let db3 = db.clone();

    let (propiedades, contratos, pagos) = tokio::try_join!(
        async { propiedad::Entity::find().count(&db1).await.map_err(AppError::from) },
        async { contrato::Entity::find().count(&db2).await.map_err(AppError::from) },
        async { pago::Entity::find().count(&db3).await.map_err(AppError::from) },
    )?;

    Ok(DashboardResponse { propiedades, contratos, pagos })
}
```

Prefer `tokio::try_join!` over spawning separate tasks when you need all results before continuing — it avoids the overhead of task scheduling and handles errors cleanly.

### `spawn_blocking` for CPU-Bound Work

Use `spawn_blocking` for operations that would block the async runtime: password hashing, PDF generation, heavy computation.

```rust
use tokio::task::spawn_blocking;

pub async fn hash_password(password: String) -> Result<String, AppError> {
    // bcrypt hashing is CPU-intensive — run on the blocking pool
    spawn_blocking(move || {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}
```

### Common Mistake: Blocking the Async Runtime

```rust
// Anti-pattern: blocking call inside async handler starves other tasks
pub async fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    Ok(bcrypt::verify(password, hash)?) // blocks the tokio worker thread!
}

// Fix: offload to the blocking pool
pub async fn verify_password(password: String, hash: String) -> Result<bool, AppError> {
    spawn_blocking(move || {
        bcrypt::verify(&password, &hash)
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}
```

### `std::fs` vs `tokio::fs`

Standard library file I/O blocks the current thread. In async contexts, use `tokio::fs` which runs operations on the blocking pool internally.

```rust
// Anti-pattern: std::fs blocks the async runtime
async fn read_config() -> Result<String, std::io::Error> {
    std::fs::read_to_string("config.toml") // blocks!
}

// Fix: use tokio::fs
async fn read_config() -> Result<String, std::io::Error> {
    tokio::fs::read_to_string("config.toml").await
}
```


## `tokio::sync::Mutex` vs `std::sync::Mutex` in Async

### Decision Guide

| Mutex | Use When | Holds Across `.await`? |
|-------|----------|----------------------|
| `std::sync::Mutex` | Lock is held briefly, no `.await` inside critical section | No |
| `tokio::sync::Mutex` | Lock must be held across `.await` points | Yes |

### Why `std::sync::Mutex` Is Often Fine

`std::sync::Mutex` is lighter weight than `tokio::sync::Mutex`. If the critical section is short and contains no `.await`, prefer the standard library version — it avoids the overhead of async-aware locking.

```rust
use std::sync::Mutex;
use actix_web::web;

/// Short critical section with no .await — std::sync::Mutex is correct
pub async fn increment_counter(
    counter: web::Data<Mutex<u64>>,
) -> Result<HttpResponse, AppError> {
    let mut count = counter.lock().unwrap();
    *count += 1;
    let current = *count;
    drop(count); // explicit drop before any async work

    Ok(HttpResponse::Ok().json(serde_json::json!({ "count": current })))
}
```

### When You Need `tokio::sync::Mutex`

If the critical section contains `.await` calls, you must use `tokio::sync::Mutex`. Holding a `std::sync::Mutex` across an `.await` point blocks the entire tokio worker thread.

```rust
use tokio::sync::Mutex;
use sea_orm::DatabaseConnection;

/// Lock held across .await — must use tokio::sync::Mutex
pub async fn cached_lookup(
    cache: &Mutex<HashMap<i32, PropiedadResponse>>,
    db: &DatabaseConnection,
    id: i32,
) -> Result<PropiedadResponse, AppError> {
    let mut guard = cache.lock().await;
    if let Some(cached) = guard.get(&id) {
        return Ok(cached.clone());
    }

    // .await while holding the lock — requires tokio::sync::Mutex
    let record = propiedad::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound("Propiedad no encontrada".into()))?;

    let response = PropiedadResponse::from(record);
    guard.insert(id, response.clone());
    Ok(response)
}
```

### Anti-Pattern: `std::sync::Mutex` Across `.await`

```rust
// Anti-pattern: std::sync::Mutex held across .await blocks the runtime
use std::sync::Mutex;

pub async fn bad_cached_lookup(
    cache: &Mutex<HashMap<i32, PropiedadResponse>>,
    db: &DatabaseConnection,
    id: i32,
) -> Result<PropiedadResponse, AppError> {
    let mut guard = cache.lock().unwrap();
    // This .await blocks the tokio worker thread while the Mutex is held!
    let record = propiedad::Entity::find_by_id(id).one(db).await?;
    guard.insert(id, PropiedadResponse::from(record.unwrap()));
    Ok(guard.get(&id).unwrap().clone())
}
```

### Prefer Restructuring Over `tokio::sync::Mutex`

Often you can restructure code to avoid holding a lock across `.await` entirely:

```rust
use std::sync::Mutex;

/// Restructured: release lock before .await, re-acquire after
pub async fn better_cached_lookup(
    cache: &Mutex<HashMap<i32, PropiedadResponse>>,
    db: &DatabaseConnection,
    id: i32,
) -> Result<PropiedadResponse, AppError> {
    // Check cache — short lock, no .await
    {
        let guard = cache.lock().unwrap();
        if let Some(cached) = guard.get(&id) {
            return Ok(cached.clone());
        }
    } // lock released here

    // Fetch from DB without holding the lock
    let record = propiedad::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::NotFound("Propiedad no encontrada".into()))?;

    let response = PropiedadResponse::from(record);

    // Re-acquire lock briefly to insert
    {
        let mut guard = cache.lock().unwrap();
        guard.insert(id, response.clone());
    }

    Ok(response)
}
```

### `RwLock` for Read-Heavy Workloads

When reads vastly outnumber writes, `tokio::sync::RwLock` allows concurrent readers.

```rust
use tokio::sync::RwLock;

pub async fn get_cached_config(
    config_cache: &RwLock<AppConfig>,
) -> AppConfig {
    // Multiple handlers can read concurrently
    config_cache.read().await.clone()
}

pub async fn update_config(
    config_cache: &RwLock<AppConfig>,
    new_config: AppConfig,
) {
    // Write lock is exclusive
    *config_cache.write().await = new_config;
}
```

## Channel Selection: `mpsc`, `broadcast`, `watch`

### Decision Matrix

| Channel | Pattern | Receivers | Buffering | Use When |
|---------|---------|-----------|-----------|----------|
| `tokio::sync::mpsc` | Many-to-one | Single consumer | Bounded or unbounded | Task queues, work distribution |
| `tokio::sync::broadcast` | One-to-many | Multiple consumers | Bounded, lossy | Event fan-out, notifications |
| `tokio::sync::watch` | One-to-many | Multiple consumers | Single latest value | Config updates, state changes |
| `tokio::sync::oneshot` | One-to-one | Single consumer | Single value | Request-response, task completion |

### `mpsc` — Background Task Queue

Use `mpsc` when multiple producers send work to a single consumer. Common for background job processing.

```rust
use tokio::sync::mpsc;

#[derive(Debug)]
enum BackgroundJob {
    SendNotification { inquilino_id: i32, message: String },
    GenerateReport { contrato_id: i32 },
}

/// Set up a background worker with bounded channel
pub fn start_background_worker(
    db: DatabaseConnection,
) -> mpsc::Sender<BackgroundJob> {
    // Bounded channel — backpressure when 100 jobs are queued
    let (tx, mut rx) = mpsc::channel::<BackgroundJob>(100);

    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            match job {
                BackgroundJob::SendNotification { inquilino_id, message } => {
                    tracing::info!("Notificación para inquilino {}: {}", inquilino_id, message);
                    // process notification...
                }
                BackgroundJob::GenerateReport { contrato_id } => {
                    tracing::info!("Generando reporte para contrato {}", contrato_id);
                    // generate report...
                }
            }
        }
    });

    tx
}

/// Handler enqueues work without blocking the response
pub async fn create_pago(
    db: web::Data<DatabaseConnection>,
    jobs: web::Data<mpsc::Sender<BackgroundJob>>,
    body: web::Json<CreatePagoRequest>,
) -> Result<HttpResponse, AppError> {
    let pago = pagos::create(db.get_ref(), body.into_inner()).await?;

    // Fire-and-forget notification — doesn't block the HTTP response
    let _ = jobs.send(BackgroundJob::SendNotification {
        inquilino_id: pago.inquilino_id,
        message: format!("Pago registrado: {}", pago.monto),
    }).await;

    Ok(HttpResponse::Created().json(pago))
}
```

Always use bounded channels in production to prevent unbounded memory growth.

### `broadcast` — Event Fan-Out

Use `broadcast` when multiple consumers need to receive every message. Useful for real-time notifications.

```rust
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
enum SystemEvent {
    PagoCreated { contrato_id: i32, monto: Decimal },
    ContratoExpiring { contrato_id: i32, days_left: i32 },
}

/// Multiple listeners each receive every event
pub fn setup_event_bus() -> broadcast::Sender<SystemEvent> {
    let (tx, _) = broadcast::channel::<SystemEvent>(256);
    tx
}

/// Subscriber — each call to .subscribe() creates an independent receiver
pub async fn listen_for_payments(tx: &broadcast::Sender<SystemEvent>) {
    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let SystemEvent::PagoCreated { contrato_id, monto } = event {
                tracing::info!("Pago de {} en contrato {}", monto, contrato_id);
            }
        }
    });
}
```

Note: slow receivers that fall behind lose messages (`RecvError::Lagged`). Size the buffer for your throughput.

### `watch` — Latest State

Use `watch` when receivers only care about the most recent value. Ideal for configuration or status that changes infrequently.

```rust
use tokio::sync::watch;

/// Receivers always see the latest config — no message queue
pub fn setup_config_watch(
    initial: AppConfig,
) -> (watch::Sender<AppConfig>, watch::Receiver<AppConfig>) {
    watch::channel(initial)
}

/// Handler reads the latest config without blocking
pub async fn get_status(
    config_rx: web::Data<watch::Receiver<AppConfig>>,
) -> Result<HttpResponse, AppError> {
    let current = config_rx.borrow().clone();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "port": current.server_port,
        "status": "running"
    })))
}
```

### `oneshot` — Single Response

Use `oneshot` for request-response patterns where a spawned task returns a single result.

```rust
use tokio::sync::oneshot;

pub async fn expensive_calculation(
    db: web::Data<DatabaseConnection>,
) -> Result<HttpResponse, AppError> {
    let (tx, rx) = oneshot::channel();
    let db = db.get_ref().clone();

    tokio::spawn(async move {
        let result = compute_annual_report(&db).await;
        let _ = tx.send(result);
    });

    let report = rx.await
        .map_err(|_| AppError::Internal("Tarea cancelada".into()))?
        .map_err(AppError::from)?;

    Ok(HttpResponse::Ok().json(report))
}
```


## `Send` and `Sync` Bound Troubleshooting

### What They Mean

| Trait | Meaning | Required When |
|-------|---------|---------------|
| `Send` | Safe to transfer ownership to another thread | `tokio::spawn`, passing data across `.await` |
| `Sync` | Safe to share references between threads | `&T` accessed from multiple tasks |

Most standard types are `Send + Sync`. Common exceptions: `Rc<T>` (not `Send`), `Cell<T>` / `RefCell<T>` (not `Sync`), raw pointers.

### The `tokio::spawn` Requires `Send + 'static`

`tokio::spawn` requires the future to be `Send + 'static` because the task may run on any worker thread.

```rust
// This compiles: all captured values are Send + 'static
let db = db.clone(); // DatabaseConnection is Send + Sync
tokio::spawn(async move {
    let count = propiedad::Entity::find().count(&db).await.unwrap();
    tracing::info!("Total propiedades: {}", count);
});
```

### Common Error: Non-Send Type Across `.await`

```rust
// Error: Rc<T> is not Send — can't be held across .await in a spawned task
use std::rc::Rc;

let shared = Rc::new(some_data);
tokio::spawn(async move {
    do_something(&shared).await; // compile error!
});

// Fix: use Arc instead of Rc
use std::sync::Arc;

let shared = Arc::new(some_data);
tokio::spawn(async move {
    do_something(&shared).await; // Arc<T> is Send when T: Send + Sync
});
```

### Non-Send Types Held Across `.await` (Without Spawn)

Even without `tokio::spawn`, holding a non-`Send` type across `.await` in an Actix-web handler can cause issues because Actix runs handlers on a multi-threaded runtime.

```rust
// Anti-pattern: MutexGuard held across .await
pub async fn bad_handler(
    db: web::Data<DatabaseConnection>,
    state: web::Data<std::sync::Mutex<AppState>>,
) -> Result<HttpResponse, AppError> {
    let guard = state.lock().unwrap();
    // MutexGuard is not Send — holding it across .await is problematic
    let data = propiedad::Entity::find().all(db.get_ref()).await?;
    drop(guard);
    Ok(HttpResponse::Ok().json(data))
}

// Fix: drop the guard before .await
pub async fn good_handler(
    db: web::Data<DatabaseConnection>,
    state: web::Data<std::sync::Mutex<AppState>>,
) -> Result<HttpResponse, AppError> {
    let snapshot = {
        let guard = state.lock().unwrap();
        guard.clone() // clone what you need, release the lock
    };
    let data = propiedad::Entity::find().all(db.get_ref()).await?;
    Ok(HttpResponse::Ok().json(data))
}
```

### Debugging `Send` Bound Errors

When the compiler says a future is not `Send`, the error message points to the type that breaks the bound. Common culprits:

| Type | Problem | Fix |
|------|---------|-----|
| `Rc<T>` | Not `Send` | Use `Arc<T>` |
| `RefCell<T>` | Not `Sync` | Use `Mutex<T>` or `RwLock<T>` |
| `MutexGuard<T>` | Not `Send` | Drop before `.await` |
| `&T` where `T: !Sync` | Not `Send` | Clone or use `Arc` |
| Raw pointers | Not `Send` | Wrap in a `Send` newtype with `unsafe impl Send` (document safety) |

### Actix-web `web::Data` and Send/Sync

`web::Data<T>` requires `T: Send + Sync` because it's shared across worker threads. SeaORM's `DatabaseConnection` satisfies both bounds.

```rust
// This works: DatabaseConnection is Send + Sync
actix_web::App::new()
    .app_data(web::Data::new(db))  // DatabaseConnection
    .app_data(web::Data::new(config))  // AppConfig (must be Send + Sync)
```

If you need to share a non-`Send` type, wrap it in a `Mutex`:

```rust
// Wrapping a non-Sync type for use with web::Data
let state = web::Data::new(std::sync::Mutex::new(non_sync_value));
```

## SeaORM Connection Pool Sizing

### How SeaORM Pools Work

`sea_orm::Database::connect()` creates a connection pool via `sqlx`. The pool manages a set of reusable database connections, avoiding the overhead of establishing a new connection per query.

### Configuring Pool Size

Use `ConnectOptions` to control pool parameters:

```rust
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(10)       // max concurrent connections
        .min_connections(2)        // keep at least 2 idle connections
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(true);

    Database::connect(opts).await
}
```

### Pool Sizing Guidelines

| Parameter | Guideline | Reasoning |
|-----------|-----------|-----------|
| `max_connections` | 2× CPU cores for web servers (e.g., 8 cores → 16) | Each async task yields while waiting for I/O, so more connections than cores is fine |
| `min_connections` | 2–5 for low-traffic apps | Avoids cold-start latency on first requests |
| `connect_timeout` | 5s | Fail fast if the database is unreachable |
| `acquire_timeout` | 5s | Fail fast if the pool is exhausted |
| `idle_timeout` | 600s (10 min) | Release idle connections to free DB resources |
| `max_lifetime` | 1800s (30 min) | Prevent stale connections from accumulating |

### Anti-Pattern: Too Many Connections

```rust
// Anti-pattern: setting max_connections too high
opts.max_connections(100); // PostgreSQL default max_connections is 100!
```

Each PostgreSQL connection consumes ~10MB of memory. Setting the pool size close to the server's `max_connections` leaves no room for admin connections, migrations, or other services. A good rule:

```
pool_max = (db_max_connections - reserved) / number_of_app_instances
```

For a single-instance app with PostgreSQL defaults:
```
pool_max = (100 - 10) / 1 = 90  (theoretical max)
pool_max = 10-20                  (practical recommendation)
```

### Anti-Pattern: No Acquire Timeout

```rust
// Anti-pattern: no acquire timeout — handler hangs forever if pool is exhausted
let db = Database::connect(&database_url).await?;
// Default acquire timeout may be very long or infinite
```

Always set `acquire_timeout` so handlers return an error instead of hanging when the pool is exhausted:

```rust
let mut opts = ConnectOptions::new(database_url);
opts.max_connections(10)
    .acquire_timeout(Duration::from_secs(5)); // fail after 5s if no connection available

let db = Database::connect(opts).await?;
```

### Monitoring Pool Health

Log pool metrics to detect exhaustion early:

```rust
use sea_orm::DatabaseConnection;

/// Call periodically or expose as a health endpoint
pub async fn check_pool_health(db: &DatabaseConnection) {
    // SeaORM exposes the underlying sqlx pool via get_postgres_connection_pool()
    // For monitoring, a simple query serves as a health check
    match db.execute_unprepared("SELECT 1").await {
        Ok(_) => tracing::debug!("Database pool healthy"),
        Err(e) => tracing::warn!("Database pool issue: {}", e),
    }
}
```

### Matching Pool Size to Actix-web Workers

Actix-web defaults to one worker per CPU core. Each worker handles many concurrent requests via async. Size the pool so all workers can make concurrent queries without exhausting connections:

```rust
// Example: 4 CPU cores, 2-3 concurrent DB queries per request
// Workers: 4 (default)
// Concurrent requests per worker: ~25 (Actix default)
// Peak concurrent DB queries: 4 × 25 × 0.5 (not all requests hit DB) = 50
// Pool size: 10-20 is usually sufficient due to query latency overlap

let mut opts = ConnectOptions::new(database_url);
opts.max_connections(15)
    .min_connections(3);
```

The async runtime multiplexes many requests over fewer connections because each query spends most of its time waiting for the database response, not holding the connection actively.
