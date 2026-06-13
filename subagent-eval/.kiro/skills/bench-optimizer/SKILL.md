---
name: bench-optimizer
description: >
  Benchmark-driven performance optimizer for Rust code. Instead of guessing which
  approach is faster, this skill writes competing implementations, benchmarks them
  with criterion, and keeps the winner. Use when optimizing hot paths, choosing between
  data structures, comparing algorithm approaches, or when you need empirical proof
  that one approach beats another. Triggers on: benchmark, criterion, flamegraph,
  perf, hot path, which is faster, compare approaches, measure performance, throughput,
  latency, optimize with data, profile.
license: MIT
allowed-tools: Read Write Grep Glob Shell
metadata:
  author: Joseph Perez
  version: "1.0.0"
  domain: performance
  triggers: benchmark, criterion, flamegraph, perf, hot path, which is faster, compare approaches, measure performance, throughput, latency, profile
  role: specialist
  scope: implementation
  output-format: code
  related-skills: lint-fixer, maintainability-reviewer
---

# Bench Optimizer

Performance optimization through measurement, not guessing. The model already knows
algorithm theory — this skill's value is in the *workflow*: write competing implementations,
benchmark them under realistic conditions, **actually run the benchmarks**, and keep
the empirically fastest one.

## Why This Exists

LLMs (including you) have a bias: when asked "which is faster?", you reason from
asymptotic complexity and general knowledge. But real performance depends on:
- Cache line behavior at actual dataset sizes
- Branch prediction patterns in real data distributions
- Allocation pressure under the specific workload
- Compiler optimizations that eliminate theoretical differences

The only way to know is to measure. This skill forces that discipline.

## Critical Rule: No Predictions

**NEVER produce "expected results" or "predicted speedup" sections.** Your theoretical
predictions are frequently wrong by 5-20x (e.g., predicting 200x speedup when reality
is 30x, or predicting 50µs when reality is 1ms). If you cannot run the benchmark,
say so explicitly and leave the recommendation blank until real data exists.

The whole point of this skill is to replace theory with measurement. Writing
"Expected improvement: 20-40%" is exactly the behavior this skill exists to prevent.

## Valid Outcome: Don't Optimize

"Don't optimize" is a perfectly valid benchmark conclusion. If the current code runs
in 218µs and the surrounding DB queries take 50ms, the optimization is pointless
regardless of how much faster the alternative is. Always consider:
- Absolute time vs system-level latency budget
- How often the code runs (once daily vs every request)
- Complexity cost of the optimization vs the gain

## Workflow

### Step 1: Identify the Target

Before optimizing, confirm the target is worth optimizing:

- Is this a hot path? (called frequently, or on large datasets)
- Is there evidence of slowness? (user report, profiling data, slow tests)
- What's the realistic dataset size in production?

If the answer is "n < 100 and called once per request" — stop. Don't optimize.
Document why and move on.

### Step 2: Write a Criterion Benchmark for the Current Code

Create a benchmark that exercises the function under realistic conditions.
Use production-representative data sizes and distributions.

```rust
// benches/target_bench.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_current(c: &mut Criterion) {
    let data = generate_realistic_data(PRODUCTION_SIZE);

    c.bench_function("current_implementation", |b| {
        b.iter(|| current_implementation(&data))
    });
}

criterion_group!(benches, bench_current);
criterion_main!(benches);
```

Key rules for benchmarks:
- Use `criterion::black_box` to prevent dead code elimination
- Generate data OUTSIDE the benchmark loop (measure the function, not setup)
- Use realistic sizes: check the actual production dataset sizes from the domain docs
- Include warm-up (criterion does this automatically)
- Run with `--release` (always)

### Step 3: Write 2-3 Competing Implementations

For each optimization idea, write a complete alternative implementation.
Don't just theorize — write the code.

Common competition axes:
- **Data structure**: HashMap vs BTreeMap vs Vec+sort vs IndexMap
- **Algorithm**: sort-then-scan vs hash-lookup vs brute-force
- **Memory strategy**: pre-allocate vs grow vs arena
- **Parallelism**: sequential vs rayon::par_iter vs manual chunking
- **Layout**: struct-of-arrays vs array-of-structs
- **Allocation**: owned vs borrowed vs Cow vs SmallVec/SmallString

Name them clearly:

```rust
fn approach_hashmap(data: &[Input]) -> Output { ... }
fn approach_sort_scan(data: &[Input]) -> Output { ... }
fn approach_parallel(data: &[Input]) -> Output { ... }
```

### Step 4: Benchmark All Approaches

Add all approaches to the same benchmark group for direct comparison:

```rust
fn bench_comparison(c: &mut Criterion) {
    let data = generate_realistic_data(PRODUCTION_SIZE);

    let mut group = c.benchmark_group("optimization_target");

    group.bench_function("current", |b| b.iter(|| current(&data)));
    group.bench_function("hashmap", |b| b.iter(|| approach_hashmap(&data)));
    group.bench_function("sort_scan", |b| b.iter(|| approach_sort_scan(&data)));
    group.bench_function("parallel", |b| b.iter(|| approach_parallel(&data)));

    group.finish();
}
```

For size-dependent behavior, benchmark across multiple sizes:

```rust
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [10, 100, 1000, 5000] {
        let data = generate_realistic_data(size);
        group.bench_with_input(
            BenchmarkId::new("current", size),
            &data,
            |b, d| b.iter(|| current(d)),
        );
        group.bench_with_input(
            BenchmarkId::new("optimized", size),
            &data,
            |b, d| b.iter(|| approach_hashmap(d)),
        );
    }

    group.finish();
}
```

### Step 5: Run and Interpret Results

```bash
cargo bench --bench target_bench
```

**You MUST actually run this command.** Do not skip this step. Do not write
"expected results" based on theory. If the benchmark fails to compile or run,
fix it until it works. The entire value of this workflow is in the real numbers.

Read the output. Criterion reports:
- Mean execution time with confidence interval
- Throughput (if configured)
- Comparison to baseline (if previous run exists)

**Decision rules:**
- If the winner is < 10% faster: keep the simpler/more readable version
- If the winner is 10-50% faster: adopt it if readability is comparable
- If the winner is > 50% faster: adopt it, document the tradeoff
- If results are noisy (wide confidence intervals): increase sample size or reduce system load
- If the absolute time is negligible vs system latency (e.g., 200µs function in a 50ms request): **don't optimize** — document why and move on

### Step 6: Adopt the Winner

Replace the original implementation with the winning approach:
1. Swap the implementation
2. Run `cargo test` to verify correctness
3. Run `cargo clippy` and `cargo fmt`
4. Keep the benchmark file for regression detection
5. Document the result in a comment: what was tried, what won, by how much

### Step 7: Verify No Regression

After adopting, run the benchmark one more time to confirm the improvement
holds in the final integrated code (not just the isolated benchmark).

## Benchmark Setup

### Cargo.toml Configuration

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "target_bench"
harness = false
```

### Realistic Data Generation

Always generate data that matches production characteristics:

```rust
use rand::Rng;
use uuid::Uuid;

/// Generate data matching production distribution:
/// - ~50 propiedades, ~200 contratos, ~2000 pagos
/// - 70% of pagos are "pagado", 20% "pendiente", 10% "atrasado"
/// - Dates span 24 months
fn generate_realistic_pagos(n: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();
    (0..n).map(|_| Pago {
        id: Uuid::new_v4(),
        contrato_id: Uuid::new_v4(),
        monto: rng.gen_range(5000.0..50000.0),
        estado: match rng.gen_range(0..10) {
            0 => "atrasado".to_string(),
            1..=2 => "pendiente".to_string(),
            _ => "pagado".to_string(),
        },
        // ... realistic date distribution
    }).collect()
}
```

## When NOT to Benchmark

- **n < 100 and called infrequently**: The overhead of benchmarking exceeds any possible gain.
  Just pick the clearest implementation.
- **IO-bound code**: If the function spends 99% of time waiting on DB/network,
  optimizing the CPU portion is pointless. Profile first to confirm CPU-bound.
- **One-time operations**: Migrations, startup initialization, CLI tools that run once.
  Developer time > machine time here.
- **Already fast enough**: If the endpoint responds in < 5ms and the SLA is 200ms,
  there's no user-visible benefit to optimizing further.

## Common Pitfalls

1. **Benchmarking with unrealistic data**: Using 10 items when production has 10,000.
   Results won't transfer.
2. **Forgetting `--release`**: Debug builds have no optimizations. Results are meaningless.
3. **Measuring setup cost**: Generate data outside `b.iter()`. Only measure the target function.
4. **Ignoring allocation**: A function might be "fast" but allocate 10MB per call.
   Use `#[global_allocator]` with a counting allocator for allocation-sensitive paths.
5. **Optimizing cold paths**: Profile first. The hot path is often not where you think.
6. **Micro-benchmark ≠ system performance**: A function 2x faster in isolation might
   not improve end-to-end latency if it's 1% of the request.

## Constraints

### MUST DO
- Write a criterion benchmark BEFORE optimizing
- **Actually run `cargo bench --release`** — do not skip this step
- Test at production-representative data sizes
- Write at least 2 competing approaches
- Keep the benchmark file after adopting the winner (regression detection)
- Verify correctness with `cargo test` after swapping implementations
- Document the **measured** result in a code comment (actual timings, not predictions)
- Consider absolute time vs system latency — "don't optimize" is a valid conclusion
- Produce a self-contained, runnable benchmark (include Cargo.toml setup if needed)

### MUST NOT DO
- **Produce "expected results" or "predicted speedup" without running benchmarks** — this is the #1 anti-pattern this skill exists to prevent
- Recommend an approach based on theoretical analysis alone
- Guess which approach is faster without measuring
- Optimize code that isn't on a hot path
- Optimize when absolute time is negligible vs surrounding I/O latency
- Delete benchmark files after optimization (they prevent regressions)
- Use `#[bench]` (unstable) — always use criterion
- Benchmark in debug mode
- Optimize for n=10 when production n=10,000 (or vice versa)


---

## Memory Efficiency Profiling

Performance isn't only speed. A function that runs in 50µs but allocates 2MB per call
will destroy throughput under concurrency (GC pressure in other runtimes, allocator
contention in Rust, cache thrashing, OOM on constrained k8s pods).

This section covers measuring and optimizing memory usage with the same discipline
as latency: **measure first, decide after**.

### When to Profile Memory

- **High-concurrency endpoints**: Each concurrent request multiplies per-call allocation.
  50 concurrent requests × 2MB/call = 100MB transient heap pressure.
- **Streaming/batch operations**: Processing 10,000 records shouldn't require 10,000× object allocation.
- **Constrained environments**: K8s pods with memory limits (e.g., 384Mi for backend).
  If peak RSS approaches the limit, the OOM killer strikes.
- **Long-lived processes**: Small leaks compound. A 1KB leak per request = 86MB/day at 1 req/s.
- **Report generation**: Building PDF/XLSX for large datasets can spike memory.

### When NOT to Profile Memory

- **One-shot CLI tools**: They exit immediately. OS reclaims everything.
- **Allocations < 1KB on cold paths**: Not worth the complexity.
- **Already below 50% of pod memory limit with headroom**: Focus elsewhere.

### Strategy 1: Counting Allocator (Per-Function Measurement)

Use `dhat` in testing mode to assert allocation counts and bytes for specific code paths.
This catches regressions where a refactor accidentally adds allocations to a hot path.

#### Setup

```toml
# Cargo.toml
[dev-dependencies]
dhat = "0.3"

[features]
dhat-heap = []

[profile.release]
debug = 1  # needed for dhat backtraces
```

#### Writing a Memory Test

```rust
// tests/memory_budget.rs
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn dashboard_stats_allocation_budget() {
    let _profiler = dhat::Profiler::builder().testing().build();

    // Exercise the pure-computation portion (post-DB-fetch aggregation)
    let pagos = generate_realistic_pagos(2000);
    let _result = aggregate_dashboard_metrics(&pagos);

    let stats = dhat::HeapStats::get();

    // Assert: peak heap usage should stay under 512KB for 2000 pagos
    assert!(
        stats.max_bytes < 512 * 1024,
        "Peak heap {:.1}KB exceeds 512KB budget",
        stats.max_bytes as f64 / 1024.0
    );

    // Assert: total allocations should be reasonable (not N+1 patterns)
    assert!(
        stats.total_blocks < 100,
        "Too many allocations ({}): possible N+1 allocation pattern",
        stats.total_blocks
    );
}
```

**Key rules:**
- Put each memory test in its **own integration test file** (dhat uses global state).
- Always test in **release mode**: `cargo test --release -p realestate-backend --test memory_budget`
- Set budgets based on measured baselines, not guesses. Run once to get the current
  numbers, then set the assertion 20% above as a regression ceiling.

### Strategy 2: Comparing Allocation Pressure Between Approaches

When choosing between implementations, measure both time AND allocations:

```rust
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn compare_grouping_strategies_memory() {
    let pagos = generate_realistic_pagos(2000);

    // Approach A: HashMap grouping (current)
    {
        let _profiler = dhat::Profiler::builder().testing().build();
        let _result = group_by_hashmap(&pagos);
        let stats = dhat::HeapStats::get();
        eprintln!("HashMap: {} bytes in {} blocks (peak: {} bytes)",
            stats.total_bytes, stats.total_blocks, stats.max_bytes);
    }

    // Note: dhat panics if two Profilers exist simultaneously.
    // Each block must fully drop the Profiler before the next starts.
    // In practice, put each approach in a separate test function.
}
```

**Important**: `dhat` panics if multiple Profilers coexist. For comparing approaches,
use **separate test functions** (one per approach) and compare the printed stats manually,
or use `dhat::assert_eq!` with known baselines.

### Strategy 3: Allocation-Aware Criterion Benchmarks

For approaches where you want both speed AND allocation data in one run, use a
counting allocator alongside criterion:

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAlloc;
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

fn bench_with_alloc_tracking(c: &mut Criterion) {
    let data = generate_realistic_data(2000);
    let mut group = c.benchmark_group("memory_comparison");

    group.bench_function("current", |b| {
        b.iter(|| {
            ALLOC_COUNT.store(0, Ordering::Relaxed);
            let result = current_approach(&data);
            let allocs = ALLOC_COUNT.load(Ordering::Relaxed);
            // Print once per benchmark (criterion runs many iterations)
            std::hint::black_box((result, allocs))
        })
    });

    group.finish();
}
```

**Warning**: This adds overhead to every allocation. Only use in dedicated memory
benchmarks, never in production code.

### Strategy 4: Peak RSS Measurement (System-Level)

For end-to-end memory usage of the entire process under load, measure RSS externally:

```bash
# Linux: run the binary and check peak memory
/usr/bin/time -v ./target/release/realestate-backend 2>&1 | grep "Maximum resident"

# During load test: track RSS over time
while true; do ps -o rss= -p $(pgrep realestate-backend); sleep 1; done
```

For K8s: check pod metrics via `kubectl top pod` during realistic load.

### Common Memory Optimization Patterns

Once you've measured and confirmed a memory problem, these are the common fixes:

| Pattern | Problem | Fix |
|---------|---------|-----|
| `Vec` growing | Multiple reallocations | `Vec::with_capacity(known_size)` |
| String formatting | Temporary `String` per iteration | `write!` to a reusable buffer |
| Cloning owned data | Unnecessary heap copies | `&str` / `&[T]` / `Cow<'_, T>` |
| N+1 allocations | One `Vec` per item in a loop | Single pre-sized collection |
| Large return types | Stack copies on return | Return `Box<T>` or pass `&mut T` |
| Temporary collections | Build a Vec just to iterate it | Iterator chains (lazy) |
| String keys in HashMap | Each key is a heap allocation | Intern strings, use `&str`, or indices |
| Redundant serialization | Serialize to String then to bytes | Serialize directly to writer |

### Decision Framework

After measuring memory, use this to decide what to do:

| Situation | Action |
|-----------|--------|
| Peak heap < 10% of pod limit, few allocations | Don't optimize. Document baseline. |
| Peak heap > 50% of pod limit | Optimize. Risk of OOM under load. |
| Allocation count scales linearly with N | Check for N+1 patterns. May need batching. |
| Single function dominates allocation | Target that function specifically. |
| Memory grows over time (leak) | Use dhat profiling to find unfreed blocks. |
| Competing approaches differ by < 20% memory | Keep the faster/simpler one. |
| Competing approaches differ by > 50% memory | Adopt the leaner one if speed is comparable. |

### Constraints (Memory)

#### MUST DO
- Measure allocation count AND bytes before optimizing memory
- Use `dhat` in testing mode or a counting allocator — never guess
- Run memory tests in **release mode** (debug builds have different allocation behavior)
- Set regression budgets based on measured baselines (not theory)
- Consider peak vs total: a function might allocate 1MB total but only 10KB peak
- Factor in concurrency: per-request cost × expected concurrent requests = real pressure
- Put each `dhat` test in its own integration test file (global state conflicts)

#### MUST NOT DO
- Guess memory usage without measuring
- Optimize allocations on cold paths (once-per-startup code doesn't matter)
- Use `dhat` in production builds (it adds significant overhead)
- Mix dhat profiling with criterion in the same binary (use separate test targets)
- Assume "fewer allocations = faster" without benchmarking both (allocator fast paths exist)
- Optimize memory at the cost of > 2x slowdown without confirming memory is the actual constraint
