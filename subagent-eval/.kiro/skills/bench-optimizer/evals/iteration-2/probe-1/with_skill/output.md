# HashMap vs BTreeMap Lookup Benchmark (10,000 entries, high query rate)

## Step 1: Target Identification

This is a valid optimization target:
- **Hot path**: 50,000 queries/sec means ~20µs budget per query at the system level.
- **Dataset size**: 10,000 entries — large enough for data structure choice to matter.
- **CPU-bound**: Pure lookup operations, no I/O involved.

The choice between HashMap and BTreeMap at this scale is non-obvious. HashMap has O(1) average lookup but worse cache behavior due to hashing + pointer chasing. BTreeMap has O(log n) lookup but better cache locality due to sorted node layout. At n=10,000, log₂(n) ≈ 13 comparisons — the constant factors and cache effects dominate. This warrants measurement.

## Step 2–4: Benchmark Code

I've written a complete criterion benchmark that tests both data structures under your exact conditions.

### Cargo.toml additions

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
rand = "0.8"

[[bench]]
name = "map_lookup"
harness = false
```

### `benches/map_lookup.rs`

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;

const NUM_ENTRIES: usize = 10_000;
const NUM_LOOKUPS: usize = 1_000; // per iteration (criterion runs many iterations)

/// Generate a HashMap with 10,000 entries (u64 keys, u64 values).
fn build_hashmap(n: usize) -> HashMap<u64, u64> {
    let mut map = HashMap::with_capacity(n);
    for i in 0..n as u64 {
        map.insert(i, i * 7 + 13);
    }
    map
}

/// Generate a BTreeMap with 10,000 entries (u64 keys, u64 values).
fn build_btreemap(n: usize) -> BTreeMap<u64, u64> {
    let mut map = BTreeMap::new();
    for i in 0..n as u64 {
        map.insert(i, i * 7 + 13);
    }
    map
}

/// Pre-generate random lookup keys (mix of hits and misses).
/// 80% existing keys (hits), 20% non-existing keys (misses).
fn generate_lookup_keys(n: usize, num_entries: usize) -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(42); // deterministic for reproducibility
    (0..n)
        .map(|_| {
            if rng.gen_bool(0.8) {
                // Hit: key exists in the map
                rng.gen_range(0..num_entries as u64)
            } else {
                // Miss: key does not exist
                rng.gen_range(num_entries as u64..(num_entries as u64 * 2))
            }
        })
        .collect()
}

fn bench_map_lookup(c: &mut Criterion) {
    let hashmap = build_hashmap(NUM_ENTRIES);
    let btreemap = build_btreemap(NUM_ENTRIES);
    let keys = generate_lookup_keys(NUM_LOOKUPS, NUM_ENTRIES);

    let mut group = c.benchmark_group("map_lookup_10k");

    // Benchmark HashMap lookups
    group.bench_function("hashmap_get", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for key in &keys {
                if let Some(val) = hashmap.get(black_box(key)) {
                    sum = sum.wrapping_add(*val);
                }
            }
            black_box(sum)
        })
    });

    // Benchmark BTreeMap lookups
    group.bench_function("btreemap_get", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for key in &keys {
                if let Some(val) = btreemap.get(black_box(key)) {
                    sum = sum.wrapping_add(*val);
                }
            }
            black_box(sum)
        })
    });

    group.finish();
}

/// Also benchmark with varying sizes to see scaling behavior.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("map_lookup_scaling");
    let keys_per_iter = 1_000;

    for size in [100, 1_000, 10_000, 50_000] {
        let hashmap = build_hashmap(size);
        let btreemap = build_btreemap(size);
        let keys = generate_lookup_keys(keys_per_iter, size);

        group.bench_with_input(BenchmarkId::new("hashmap", size), &(), |b, _| {
            b.iter(|| {
                let mut sum = 0u64;
                for key in &keys {
                    if let Some(val) = hashmap.get(black_box(key)) {
                        sum = sum.wrapping_add(*val);
                    }
                }
                black_box(sum)
            })
        });

        group.bench_with_input(BenchmarkId::new("btreemap", size), &(), |b, _| {
            b.iter(|| {
                let mut sum = 0u64;
                for key in &keys {
                    if let Some(val) = btreemap.get(black_box(key)) {
                        sum = sum.wrapping_add(*val);
                    }
                }
                black_box(sum)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_map_lookup, bench_scaling);
criterion_main!(benches);
```

## Step 5: Run the Benchmark

I cannot run `cargo bench` in this environment — the Rust toolchain is not available. The benchmark is complete and ready to execute. To get your answer, run:

```bash
cargo bench --bench map_lookup --release
```

This will produce criterion output with mean times, confidence intervals, and a direct comparison between the two approaches. The HTML report will be in `target/criterion/map_lookup_10k/report/index.html`.

## What to Look For in Results

- **Mean time per iteration** for each approach (each iteration does 1,000 lookups).
- Divide by 1,000 to get per-lookup time.
- At 50,000 queries/sec you need each lookup under 20µs — both approaches will be well under that (expect sub-microsecond per lookup), so correctness and readability are secondary factors.
- The scaling benchmark will show how each approach behaves as data grows beyond 10k.

## BENCH-VERDICT
```
status: READY_TO_RUN
benchmark_file: benches/map_lookup.rs
approaches: [hashmap_get, btreemap_get]
data_size: 10000 items
run_command: cargo bench --bench map_lookup --release
```

---

**Note**: I cannot predict the winner or provide estimated numbers. The skill rules prohibit producing "expected results" without actual measurement. Run the benchmark and the numbers will tell you definitively which is faster and by exactly how much. Once you share the output, I can interpret the results and give you a final MEASURED verdict with a concrete recommendation.
