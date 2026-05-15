---
name: perf-optimizer
description: >
  Detects and fixes Rust performance anti-patterns by directly editing source files.
  Eliminates unnecessary clones, redundant allocations, inefficient string operations,
  blocking calls in async contexts, and suboptimal iterator usage. Runs cargo fmt and
  cargo clippy after changes. Use when optimizing Rust code, fixing performance issues,
  or removing unnecessary clones and allocations.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: project
  version: "1.0.0"
  domain: performance
  triggers: performance, optimization, clone, allocation, async, iterator, memory
  role: specialist
  scope: analysis
  output-format: report
  related-skills: algorithm-advisor, maintainability-reviewer
---

# Performance Optimizer

Specialist skill for detecting and fixing Rust performance anti-patterns in Actix-web + SeaORM + tokio codebases. Actively refactors code to eliminate inefficiencies, then validates changes with cargo fmt and cargo clippy.

## Core Workflow

1. **Scan for unnecessary `.clone()` calls** — Identify `.clone()` on `String`, `Vec`, `HashMap`, and other heap types where borrowing with `&T` or passing by reference suffices
2. **Detect redundant heap allocations** — Flag `String` where `&str` works, `Vec` where a slice `&[T]` works, and missing `with_capacity` pre-allocation for known-size collections
3. **Find inefficient string concatenation** — Detect repeated `format!` or `+` operator usage inside loops where `String::push_str` or a single `format!` with joins would be more efficient
4. **Identify blocking operations in async contexts** — Flag `std::fs` calls (use `tokio::fs`), `std::thread::sleep` (use `tokio::time::sleep`), and `std::sync::Mutex` in async functions (use `tokio::sync::Mutex`)
5. **Check iterator patterns** — Detect collect-then-iterate anti-patterns, unnecessary intermediate collections, and cases where `iter()` should be `into_iter()` to avoid cloning
6. **Evaluate `Cow<T>` opportunities** — Identify functions that sometimes borrow and sometimes own data, where `Cow<'_, str>` or `Cow<'_, [T]>` would avoid unnecessary allocation
7. **Detect memory leaks** — Identify unbounded caches/collections that grow without eviction, circular `Arc` references missing `Weak`, spawned tasks with dropped `JoinHandle`s, intervals/timers without cancellation on drop, and long-lived state behind `Arc`/`web::Data` that accumulates entries across requests

## Reference Guide

Load detailed guidance based on context:

| Topic | Reference | Load When |
|-------|-----------|-----------|
| Rust Performance | `references/rust-performance.md` | Iterator patterns, zero-copy, Cow |
| Memory Optimization | `references/memory-optimization.md` | Allocations, Box/Rc/Arc, clone detection |
| Concurrency Patterns | `references/concurrency-patterns.md` | tokio, Send/Sync, locks, channels |
| Build Configuration | `references/build-config.md` | Cargo profiles, LTO, codegen-units |
| Memory Leak Detection | `references/memory-leak-detection.md` | Unbounded caches, Arc cycles, task/timer cleanup, Drop impls |

## Detection Rules

### Unnecessary Clones

Look for `.clone()` calls where the cloned value is:
- Immediately passed to a function that accepts `&T`
- Used only for reading (no mutation after clone)
- A `String` or `Vec` passed to a function that could accept `&str` or `&[T]`

```rust
// Anti-pattern: cloning to pass to a function that borrows
let name = user.name.clone();
log::info!("Processing {}", name);

// Fix: borrow directly
log::info!("Processing {}", &user.name);
```

### Redundant Heap Allocations

Flag patterns where owned types are created but only borrowed:

```rust
// Anti-pattern: owned String for comparison
let status = String::from("active");
if record.status == status { ... }

// Fix: use &str
if record.status == "active" { ... }
```

### Blocking in Async

Flag standard library blocking calls inside `async fn` or `async` blocks:

```rust
// Anti-pattern: blocking the tokio runtime
async fn read_config() -> Result<String, std::io::Error> {
    std::fs::read_to_string("config.toml") // blocks!
}

// Fix: use tokio::fs
async fn read_config() -> Result<String, std::io::Error> {
    tokio::fs::read_to_string("config.toml").await
}
```

### Iterator Anti-Patterns

Flag collect-then-iterate and unnecessary intermediate collections:

```rust
// Anti-pattern: collecting then iterating
let items: Vec<_> = source.iter().filter(|x| x.active).collect();
for item in items.iter() { ... }

// Fix: chain directly
for item in source.iter().filter(|x| x.active) { ... }
```

### Memory Leak Patterns

#### Unbounded Caches / Collections

Flag `HashMap` or `Vec` fields in long-lived structs (especially behind `Arc` or `web::Data`) that grow via `.insert()` or `.push()` but never have entries removed or evicted.

```rust
// Anti-pattern: unbounded in-memory cache that grows forever
struct AppState {
    cache: Mutex<HashMap<String, CachedResult>>,
}

async fn handler(state: web::Data<AppState>, key: String) -> impl Responder {
    let mut cache = state.cache.lock().unwrap();
    cache.insert(key, compute_result()); // grows without bound!
}

// Fix: use an LRU cache with a max capacity
use lru::LruCache;
use std::num::NonZeroUsize;

struct AppState {
    cache: Mutex<LruCache<String, CachedResult>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
        }
    }
}
```

#### Circular Arc References

Flag two structs that hold `Arc` references to each other — this creates a reference cycle that is never freed.

```rust
// Anti-pattern: circular Arc reference — neither is ever dropped
struct Parent {
    children: Vec<Arc<Child>>,
}
struct Child {
    parent: Arc<Parent>, // cycle!
}

// Fix: use Weak for the back-reference
use std::sync::Weak;

struct Child {
    parent: Weak<Parent>, // no cycle — Weak doesn't prevent drop
}

// Access the parent via upgrade()
if let Some(parent) = child.parent.upgrade() {
    // use parent
}
```

#### Dropped JoinHandles for Looping Tasks

Flag `tokio::spawn` calls whose `JoinHandle` is discarded (not stored, awaited, or aborted) when the spawned task contains a loop.

```rust
// Anti-pattern: spawned task runs forever, no way to stop it
fn start_background_sync(db: Arc<DatabaseConnection>) {
    tokio::spawn(async move {
        loop {
            sync_data(&db).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }); // JoinHandle dropped — task is orphaned
}

// Fix: store the handle and abort on shutdown
struct BackgroundSync {
    handle: JoinHandle<()>,
}

impl BackgroundSync {
    fn start(db: Arc<DatabaseConnection>) -> Self {
        let handle = tokio::spawn(async move {
            loop {
                sync_data(&db).await;
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        Self { handle }
    }
}

impl Drop for BackgroundSync {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
```

#### Intervals Without Cancellation

Flag `tokio::time::interval` created inside spawned tasks without a cancellation mechanism.

```rust
// Anti-pattern: interval runs forever with no shutdown signal
tokio::spawn(async {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        cleanup_expired_sessions().await;
    }
});

// Fix: use CancellationToken for graceful shutdown
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();
let child_token = token.child_token();

tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                cleanup_expired_sessions().await;
            }
            _ = child_token.cancelled() => {
                break; // clean exit
            }
        }
    }
});

// On shutdown:
token.cancel();
```

### Memory Leak Detection Decision Tree

```
Is there a collection (HashMap, Vec) in a long-lived struct?
├── Does it grow via insert/push across requests?
│   ├── Is there eviction, TTL, or size cap?
│   │   └── NO → Flag: unbounded cache, add LRU or size limit
│   │   └── YES → OK
│   └── Is it request-scoped (created and dropped per request)?
│       └── YES → OK
├── Are there two structs holding Arc to each other?
│   └── YES → Flag: circular reference, use Weak for back-ref
├── Is tokio::spawn called with a looping task?
│   ├── Is the JoinHandle stored and aborted on drop?
│   │   └── NO → Flag: orphaned task, store handle + abort in Drop
│   │   └── YES → OK
│   └── Is there a CancellationToken or shutdown signal?
│       └── NO → Flag: no cancellation, add token or select! with shutdown
└── Is tokio::time::interval used inside a spawned task?
    ├── Is there a way to break the loop?
    │   └── NO → Flag: interval without cancellation
    │   └── YES → OK
```

## Constraints

### MUST DO
- Directly edit source files to apply optimizations
- Run `cargo fmt` after changes to maintain formatting
- Run `cargo clippy --all-targets` to validate changes introduce no warnings
- Log each change with file path, what was changed, and why
- Tailor fixes to Actix-web handlers, SeaORM queries, and tokio patterns
- Check for memory leaks: unbounded caches, circular Arc refs, orphaned tasks, missing Drop impls
- Use category "memory-leak" for leak-related findings in fingerprints

### MUST NOT DO
- Suggest fixes without applying them — always edit the code directly
- Introduce unsafe code without documenting safety invariants
- Remove intentional clones (e.g., cloning into a `tokio::spawn` closure for `Send` bounds)
- Touch `Arc::clone()` which is idiomatic for shared ownership
