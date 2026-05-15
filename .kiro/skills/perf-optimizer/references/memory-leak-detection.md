# Memory Leak Detection Reference Guide

## Overview

Rust's ownership system prevents most memory leaks at compile time, but certain patterns can still cause logical memory leaks — memory that is technically reachable but will never be freed during the application's lifetime. These are especially common in long-running server applications like Actix-web services.

## Categories of Memory Leaks in Rust

### 1. Unbounded Collections in Long-Lived State

The most common leak in web applications: a `HashMap`, `Vec`, or similar collection stored in shared state (`web::Data`, `Arc`) that grows with each request but never shrinks.

#### Detection Heuristics

- Look for `HashMap` or `Vec` fields inside structs wrapped in `Arc`, `web::Data`, or `lazy_static`
- Check if `.insert()`, `.push()`, or `.entry().or_insert()` is called without corresponding `.remove()`, `.retain()`, or capacity checks
- Flag collections that lack TTL, LRU eviction, or max-size enforcement

#### Common Patterns in This Project

```rust
// LEAK: Rate limiter or session store that never evicts
struct RateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

// Each login attempt adds entries, but old entries are never cleaned up
async fn check_rate_limit(limiter: &RateLimiter, ip: String) {
    let mut attempts = limiter.attempts.lock().unwrap();
    attempts.entry(ip).or_default().push(Instant::now());
    // Old timestamps accumulate forever!
}
```

#### Fixes

**Option A: LRU Cache with bounded size**
```rust
use lru::LruCache;
use std::num::NonZeroUsize;

struct RateLimiter {
    attempts: Mutex<LruCache<String, Vec<Instant>>>,
}

impl RateLimiter {
    fn new(max_entries: usize) -> Self {
        Self {
            attempts: Mutex::new(LruCache::new(
                NonZeroUsize::new(max_entries).unwrap()
            )),
        }
    }
}
```

**Option B: Periodic cleanup with TTL**
```rust
fn cleanup_old_entries(map: &mut HashMap<String, Vec<Instant>>, max_age: Duration) {
    let cutoff = Instant::now() - max_age;
    map.retain(|_, timestamps| {
        timestamps.retain(|t| *t > cutoff);
        !timestamps.is_empty()
    });
}
```

**Option C: Bounded Vec with ring-buffer behavior**
```rust
use std::collections::VecDeque;

// Use VecDeque with a max length — drop oldest on overflow
fn add_bounded(deque: &mut VecDeque<Instant>, value: Instant, max: usize) {
    if deque.len() >= max {
        deque.pop_front();
    }
    deque.push_back(value);
}
```

### 2. Circular Arc References

Two or more structs holding `Arc` references to each other form a cycle. The reference count never reaches zero, so neither value is dropped.

#### Detection Heuristics

- Look for struct A containing `Arc<B>` and struct B containing `Arc<A>`
- Check for `Arc` captured in closures stored inside the same `Arc`'d struct
- Flag `Arc<Mutex<Vec<Arc<Self>>>>` patterns (parent holding children that reference parent)

#### Common Patterns

```rust
// LEAK: Parent and child reference each other via Arc
struct Service {
    workers: Vec<Arc<Worker>>,
}

struct Worker {
    service: Arc<Service>, // Cycle! Neither Service nor Worker is ever freed
}
```

#### Fix: Use `Weak` for Back-References

```rust
use std::sync::{Arc, Weak};

struct Service {
    workers: Vec<Arc<Worker>>,
}

struct Worker {
    service: Weak<Service>, // Weak doesn't prevent drop
}

impl Worker {
    fn get_service(&self) -> Option<Arc<Service>> {
        self.service.upgrade() // Returns None if Service was dropped
    }
}
```

#### When Arc Cycles Are Acceptable

- Intentional long-lived singletons (e.g., `web::Data` that lives for the entire server lifetime)
- These aren't true leaks since they're freed on process exit, but document the intent

### 3. Orphaned Tokio Tasks

`tokio::spawn` returns a `JoinHandle`. If the handle is dropped without being awaited or aborted, the task runs independently. For one-shot tasks this is fine, but for looping tasks it's a leak — the task runs forever consuming memory and CPU.

#### Detection Heuristics

- Look for `tokio::spawn(async { loop { ... } })` where the `JoinHandle` is not stored
- Flag `let _ = tokio::spawn(...)` with looping tasks
- Check for spawned tasks that hold `Arc` references — these keep the referenced data alive

#### Common Patterns in Actix-web

```rust
// LEAK: Background task runs forever, holds Arc<DatabaseConnection>
fn start_app(db: Arc<DatabaseConnection>) {
    // JoinHandle is dropped — task is orphaned
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            mark_overdue_payments(&db).await;
        }
    });
}
```

#### Fix: Managed Task Lifecycle

```rust
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

struct BackgroundTasks {
    handles: Vec<JoinHandle<()>>,
    cancel: CancellationToken,
}

impl BackgroundTasks {
    fn new() -> Self {
        Self {
            handles: Vec::new(),
            cancel: CancellationToken::new(),
        }
    }

    fn spawn_periodic<F, Fut>(&mut self, interval_secs: u64, task: F)
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let token = self.cancel.child_token();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(interval_secs)
            );
            loop {
                tokio::select! {
                    _ = interval.tick() => task().await,
                    _ = token.cancelled() => break,
                }
            }
        });
        self.handles.push(handle);
    }

    async fn shutdown(self) {
        self.cancel.cancel();
        for handle in self.handles {
            let _ = handle.await;
        }
    }
}
```


### 4. Unbounded Channels

`tokio::sync::mpsc::unbounded_channel` has no backpressure. If the producer sends faster than the consumer processes, the channel buffer grows without limit.

#### Detection Heuristics

- Look for `mpsc::unbounded_channel()` usage
- Check if the producer can burst faster than the consumer
- Flag unbounded channels in request handlers (each request could send messages)

#### Fix: Use Bounded Channels

```rust
// Anti-pattern: unbounded channel in request path
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

// Fix: bounded channel with backpressure
let (tx, mut rx) = tokio::sync::mpsc::channel(100); // max 100 buffered

// The sender now returns an error or blocks when the buffer is full
tx.send(message).await?; // .await applies backpressure
```

### 5. Forgotten Event Listeners (Frontend — Leptos/Yew)

In WASM frontends, event listeners or intervals created in components that aren't cleaned up when the component unmounts.

#### Detection Heuristics

- Look for `window().set_interval()` or `window().add_event_listener()` without corresponding cleanup
- Check `use_effect` hooks for missing cleanup/destructor returns
- Flag `gloo_timers::callback::Interval` or `Timeout` stored in component state without drop handling

#### Common Patterns in This Project's Frontend

```rust
// LEAK: interval created but never cleared on component unmount
use_effect(move || {
    let handle = gloo_timers::callback::Interval::new(5000, move || {
        refresh_data();
    });
    std::mem::forget(handle); // Explicitly leaked!
    || () // Empty cleanup
});

// Fix: return the handle so it's dropped on unmount
use_effect_with((), move |_| {
    let handle = gloo_timers::callback::Interval::new(5000, move || {
        refresh_data();
    });
    move || drop(handle) // Interval cancelled on unmount
});
```

### 6. `std::mem::forget` and `ManuallyDrop`

Explicit leak mechanisms. Sometimes used intentionally (e.g., FFI), but flag for review.

#### Detection Heuristics

- Grep for `std::mem::forget` and `ManuallyDrop::new` calls
- Verify each usage has a documented reason
- Flag any usage in application code (as opposed to FFI or unsafe abstractions)

### 7. Large Temporary Allocations in Request Handlers

Not a traditional leak, but allocations that spike per-request and aren't bounded can cause OOM under load.

#### Detection Heuristics

- Look for `Vec::new()` or `String::new()` in handlers that collect unbounded query results
- Flag handlers that load all records into memory (e.g., `find().all(&db).await?` without `.paginate()`)
- Check for response builders that accumulate large bodies in memory

#### Fix: Stream or Paginate

```rust
// Anti-pattern: load all records into memory
async fn export_all_pagos(db: &DatabaseConnection) -> Result<Vec<pago::Model>> {
    pago::Entity::find().all(db).await? // Could be millions of rows!
}

// Fix: paginate and process in chunks
async fn export_pagos_chunked(db: &DatabaseConnection) -> Result<()> {
    let mut paginator = pago::Entity::find().paginate(db, 100);
    while let Some(batch) = paginator.fetch_and_next().await? {
        process_batch(batch).await?;
    }
    Ok(())
}
```

## Memory Leak Detection Checklist

Use this checklist when reviewing Rust code for memory leaks:

| # | Check | Severity | Where to Look |
|---|-------|----------|---------------|
| 1 | Collections in `web::Data` or `Arc` structs have size bounds | P0 | Shared state structs |
| 2 | No circular `Arc` references between structs | P0 | Struct definitions with `Arc` fields |
| 3 | All looping `tokio::spawn` tasks have stored `JoinHandle` + abort | P1 | `main.rs`, `app.rs`, service init code |
| 4 | All `tokio::time::interval` loops have cancellation tokens | P1 | Background task spawning |
| 5 | Bounded channels used in request paths | P1 | Handler → background task communication |
| 6 | Frontend `use_effect` hooks return cleanup closures | P1 | Component files in `frontend/src/` |
| 7 | No `std::mem::forget` in application code | P2 | Grep across all `.rs` files |
| 8 | Query results are paginated, not loaded entirely into memory | P2 | Service layer database queries |
| 9 | Temporary `Vec`/`String` in handlers use `with_capacity` | P3 | Handler and service functions |

## Actix-web Specific Considerations

### `web::Data` Lifecycle

`web::Data<T>` wraps `T` in `Arc`. Data registered with `.app_data()` lives for the entire server lifetime. This is fine for configuration and DB pools, but dangerous for mutable caches:

```rust
// Safe: immutable config, lives for server lifetime
let config = web::Data::new(AppConfig::load()?);

// Safe: connection pool with built-in lifecycle management
let db = web::Data::new(Database::connect(&config.database_url).await?);

// DANGEROUS: mutable cache with no eviction
let cache = web::Data::new(Mutex::new(HashMap::<String, Value>::new()));
// Every request can insert — this grows forever
```

### Connection Pool Leaks

SeaORM's `DatabaseConnection` manages a connection pool. Leaks happen when:
- Transactions are started but never committed or rolled back
- Connections are held across `.await` points unnecessarily
- Custom connection wrappers bypass the pool's lifecycle

```rust
// Anti-pattern: transaction started but not committed on all paths
async fn update_contrato(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    let contrato = contrato::Entity::find_by_id(id).one(&txn).await?;
    if contrato.is_none() {
        return Err(AppError::NotFound); // Transaction not committed or rolled back!
    }
    // ... update logic ...
    txn.commit().await?;
    Ok(())
}

// Fix: transaction is rolled back on drop, but be explicit
async fn update_contrato(db: &DatabaseConnection, id: i32) -> Result<()> {
    let txn = db.begin().await?;
    let result = async {
        let contrato = contrato::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::NotFound)?;
        // ... update logic ...
        Ok(())
    }.await;

    match result {
        Ok(()) => txn.commit().await?,
        Err(e) => {
            txn.rollback().await?;
            return Err(e);
        }
    }
    Ok(())
}
```
