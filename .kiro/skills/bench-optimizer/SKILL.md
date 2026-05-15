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
  author: project
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
algorithm theory — this skill's value is in the *workflow*: write multiple implementations,
benchmark them under realistic conditions, and keep the empirically fastest one.

## Why This Exists

LLMs (including you) have a bias: when asked "which is faster?", you reason from
asymptotic complexity and general knowledge. But real performance depends on:
- Cache line behavior at actual dataset sizes
- Branch prediction patterns in real data distributions
- Allocation pressure under the specific workload
- Compiler optimizations that eliminate theoretical differences

The only way to know is to measure. This skill forces that discipline.

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

Read the output. Criterion reports:
- Mean execution time with confidence interval
- Throughput (if configured)
- Comparison to baseline (if previous run exists)

**Decision rules:**
- If the winner is < 10% faster: keep the simpler/more readable version
- If the winner is 10-50% faster: adopt it if readability is comparable
- If the winner is > 50% faster: adopt it, document the tradeoff
- If results are noisy (wide confidence intervals): increase sample size or reduce system load

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
- Test at production-representative data sizes
- Write at least 2 competing approaches
- Run benchmarks with `--release`
- Keep the benchmark file after adopting the winner (regression detection)
- Verify correctness with `cargo test` after swapping implementations
- Document the benchmark result in a code comment

### MUST NOT DO
- Guess which approach is faster without measuring
- Optimize code that isn't on a hot path
- Delete benchmark files after optimization (they prevent regressions)
- Use `#[bench]` (unstable) — always use criterion
- Benchmark in debug mode
- Optimize for n=10 when production n=10,000 (or vice versa)
