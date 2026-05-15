# Async Programming in Rust

## Basic Async/Await

```rust
use tokio;

async fn fetch_data(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = fetch_data("https://api.example.com").await?;
    println!("Data: {}", data);
    Ok(())
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        println!("Hello from async context");
    });
}
```

## Concurrent Execution

```rust
use tokio;

async fn sequential() {
    let result1 = async_operation1().await;
    let result2 = async_operation2().await;
}

async fn concurrent() {
    let (result1, result2) = tokio::join!(
        async_operation1(),
        async_operation2()
    );
}

async fn concurrent_with_errors() -> Result<(), Box<dyn std::error::Error>> {
    let (result1, result2) = tokio::try_join!(
        fallible_operation1(),
        fallible_operation2()
    )?;
    Ok(())
}

async fn spawn_tasks() {
    let handle1 = tokio::spawn(async {
        expensive_computation().await
    });

    let handle2 = tokio::spawn(async {
        another_computation().await
    });

    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();
}
```

## Select and Race Conditions

```rust
use tokio::time::{sleep, Duration};

async fn first_to_complete() {
    tokio::select! {
        result = async_operation1() => {
            println!("Operation 1 completed first: {:?}", result);
        }
        result = async_operation2() => {
            println!("Operation 2 completed first: {:?}", result);
        }
    }
}

async fn with_timeout() -> Result<String, &'static str> {
    tokio::select! {
        result = fetch_data("https://api.example.com") => {
            result.map_err(|_| "Fetch failed")
        }
        _ = sleep(Duration::from_secs(5)) => {
            Err("Timeout")
        }
    }
}

async fn cancellable_operation(mut cancel_rx: tokio::sync::watch::Receiver<bool>) {
    tokio::select! {
        result = long_running_task() => {
            println!("Task completed: {:?}", result);
        }
        _ = cancel_rx.changed() => {
            println!("Task cancelled");
        }
    }
}
```

## Streams

```rust
use tokio_stream::{self as stream, StreamExt};

async fn stream_example() {
    let mut stream = stream::iter(vec![1, 2, 3, 4, 5]);

    while let Some(value) = stream.next().await {
        println!("Value: {}", value);
    }
}

async fn stream_combinators() {
    let stream = stream::iter(vec![1, 2, 3, 4, 5])
        .filter(|x| *x % 2 == 0)
        .map(|x| x * 2);

    let results: Vec<_> = stream.collect().await;
}
```

## Channels for Communication

```rust
use tokio::sync::{mpsc, oneshot, broadcast, watch};

async fn mpsc_example() {
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move {
        tx.send("Hello").await.unwrap();
        tx.send("World").await.unwrap();
    });

    while let Some(msg) = rx.recv().await {
        println!("Received: {}", msg);
    }
}

async fn oneshot_example() {
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        tx.send("Result").unwrap();
    });

    let result = rx.await.unwrap();
}

async fn broadcast_example() {
    let (tx, mut rx1) = broadcast::channel(16);
    let mut rx2 = tx.subscribe();

    tokio::spawn(async move {
        tx.send("Message").unwrap();
    });

    println!("rx1: {}", rx1.recv().await.unwrap());
    println!("rx2: {}", rx2.recv().await.unwrap());
}

async fn watch_example() {
    let (tx, mut rx) = watch::channel("initial");

    tokio::spawn(async move {
        loop {
            rx.changed().await.unwrap();
            println!("Value changed to: {}", *rx.borrow());
        }
    });

    tx.send("updated").unwrap();
}
```

## Shared State

```rust
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

async fn mutex_example() {
    let data = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    for _ in 0..10 {
        let data = Arc::clone(&data);
        let handle = tokio::spawn(async move {
            let mut lock = data.lock().await;
            *lock += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

async fn rwlock_example() {
    let data = Arc::new(RwLock::new(vec![1, 2, 3]));

    let data1 = Arc::clone(&data);
    tokio::spawn(async move {
        let read = data1.read().await;
        println!("Read: {:?}", *read);
    });

    let mut write = data.write().await;
    write.push(4);
}
```

## Async Traits (with async-trait)

```rust
use async_trait::async_trait;

#[async_trait]
trait AsyncRepository {
    async fn find_by_id(&self, id: u64) -> Result<User, Error>;
    async fn save(&self, user: User) -> Result<(), Error>;
}

struct DatabaseRepository {
    pool: sqlx::PgPool,
}

#[async_trait]
impl AsyncRepository for DatabaseRepository {
    async fn find_by_id(&self, id: u64) -> Result<User, Error> {
        sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn save(&self, user: User) -> Result<(), Error> {
        sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
            .bind(&user.name)
            .bind(&user.email)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

## Background Tasks and Graceful Shutdown

```rust
use tokio::signal;

async fn background_task(mut shutdown: tokio::sync::watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                println!("Background task running...");
            }
            _ = shutdown.changed() => {
                println!("Shutting down background task");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let task = tokio::spawn(background_task(shutdown_rx));

    signal::ctrl_c().await.unwrap();
    shutdown_tx.send(true).unwrap();
    task.await.unwrap();
}
```

## Error Handling in Async

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum AsyncError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Task failed")]
    TaskFailed(#[from] tokio::task::JoinError),
}

async fn robust_operation() -> Result<String, AsyncError> {
    let timeout = Duration::from_secs(5);

    let result = tokio::time::timeout(timeout, async {
        reqwest::get("https://api.example.com")
            .await?
            .text()
            .await
    })
    .await
    .map_err(|_| AsyncError::Timeout)??;

    Ok(result)
}
```

## Runtime Configuration

```rust
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("my-worker")
        .thread_stack_size(3 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        println!("Running on custom runtime");
    });
}
```

## Deadlock Prevention

Async deadlocks are especially dangerous because they produce no crash, no error, and zero CPU usage — the application just silently stops responding. The Rust compiler catches data races but not deadlocks.

### Common Async Deadlock Patterns

**1. `std::sync::Mutex` guard held across `.await`**

On a single-threaded runtime, Task A acquires the lock, yields at `.await`, Task B gets scheduled, tries to lock the same mutex, blocks the only thread forever. Clippy warns about this (`clippy::await_holding_lock`).

```rust
// DEADLOCK on current_thread runtime
let guard = data.lock().unwrap();
some_async_call().await; // yields while holding guard
drop(guard);

// FIX: scope the guard so it drops before the .await
{
    let mut guard = data.lock().unwrap();
    *guard += 1;
} // guard dropped here
some_async_call().await;
```

**2. `std::sync::Mutex` shared between async task and `spawn_blocking`**

A `spawn_blocking` task holds a `std::sync::Mutex` and calls `block_on(future)`. That future needs a tokio worker thread to make progress, but the worker is blocked waiting for the mutex. Single mutex, two tasks, deadlock.

```rust
// DEADLOCK: blocking task starves the async runtime
let mutex = Arc::new(std::sync::Mutex::new(()));
tokio::task::spawn_blocking({
    let mutex = mutex.clone();
    move || {
        let _guard = mutex.lock().unwrap();
        tokio::runtime::Handle::current().block_on(async_work()); // needs a worker thread
    }
});

// FIX: use tokio::sync::Mutex with .blocking_lock() in spawn_blocking
let mutex = Arc::new(tokio::sync::Mutex::new(()));
tokio::task::spawn_blocking({
    let mutex = mutex.clone();
    move || {
        let _guard = mutex.blocking_lock();
        tokio::runtime::Handle::current().block_on(async_work());
    }
});
```

**3. Lock ordering inversion with multiple mutexes**

Two tasks acquire the same two mutexes in different orders. Classic circular wait.

```rust
// DEADLOCK: inconsistent lock ordering
// Task A: locks sessions then users
// Task B: locks users then sessions

// FIX: always acquire in the same order via a single function
async fn acquire_locks(
    sessions: &Mutex<Sessions>,
    users: &Mutex<Users>,
) -> (MutexGuard<'_, Sessions>, MutexGuard<'_, Users>) {
    let s = sessions.lock().await; // always first
    let u = users.lock().await;    // always second
    (s, u)
}
```

**4. `tokio::sync::Mutex` with dropped/paused futures**

If a future waiting in the mutex's FIFO queue stops being polled (dropped without cancellation or paused), it blocks all subsequent waiters even though no one holds the lock. Ensure futures contending on a mutex are always polled to completion or properly cancelled.

**5. Re-entrant locking**

A function locks a mutex, then calls another function that tries to lock the same mutex. `std::sync::Mutex` is not re-entrant — this deadlocks on the same thread.

### Prevention Rules

- Default to `tokio::sync::Mutex` in async code; only use `std::sync::Mutex` when the critical section is short and never crosses an `.await`
- Scope lock guards to the smallest possible block — drop before any `.await`
- When multiple locks are needed, define a single `acquire_locks()` function that always acquires in the same order
- Use channels (`mpsc`, `oneshot`, `watch`) instead of shared state when possible — they eliminate lock ordering problems entirely
- Run `cargo clippy` — it catches `await_holding_lock` and `await_holding_refcell_ref`
- Use `#[tokio::test(flavor = "multi_thread")]` for tests with lock contention so tasks can actually make progress concurrently

## Best Practices

- Use tokio::spawn for CPU-bound tasks on multi-threaded runtime
- Use spawn_blocking for blocking operations (file I/O, sync code)
- Prefer tokio::sync primitives over std::sync in async code
- Use channels for task communication instead of shared state when possible
- Always handle JoinHandle results (tasks can panic)
- Use select! for cancellation patterns
- Avoid holding locks across .await points (see Deadlock Prevention above)
- Use timeout for all external I/O operations
- Implement graceful shutdown with channels
- Use async-trait for trait-based async code
- Prefer try_join! over manual error handling
- Use Arc<Mutex<T>> sparingly (channels often better)
- Test async code with tokio::test macro
- Monitor task spawning to prevent unbounded growth
