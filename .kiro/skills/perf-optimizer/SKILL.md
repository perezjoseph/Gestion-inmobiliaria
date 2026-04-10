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

## Reference Guide

Load detailed guidance based on context:

| Topic | Reference | Load When |
|-------|-----------|-----------|
| Rust Performance | `references/rust-performance.md` | Iterator patterns, zero-copy, Cow |
| Memory Optimization | `references/memory-optimization.md` | Allocations, Box/Rc/Arc, clone detection |
| Concurrency Patterns | `references/concurrency-patterns.md` | tokio, Send/Sync, locks, channels |
| Build Configuration | `references/build-config.md` | Cargo profiles, LTO, codegen-units |

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

## Constraints

### MUST DO
- Directly edit source files to apply optimizations
- Run `cargo fmt` after changes to maintain formatting
- Run `cargo clippy --all-targets` to validate changes introduce no warnings
- Log each change with file path, what was changed, and why
- Tailor fixes to Actix-web handlers, SeaORM queries, and tokio patterns

### MUST NOT DO
- Suggest fixes without applying them — always edit the code directly
- Introduce unsafe code without documenting safety invariants
- Remove intentional clones (e.g., cloning into a `tokio::spawn` closure for `Send` bounds)
- Touch `Arc::clone()` which is idiomatic for shared ownership
