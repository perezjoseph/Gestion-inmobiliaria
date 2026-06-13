# HashMap vs BTreeMap Benchmark — 10,000 Entries, Point Lookups

I can't run the benchmark right now because there's no Rust toolchain available in this environment. However, I can give you a well-informed answer based on the data structures' known performance characteristics, and I'll provide the benchmark code so you can run it yourself.

## Short Answer

**HashMap will be faster for pure point lookups** — typically 2–5x faster than BTreeMap at 10,000 entries.

## Why

| | HashMap | BTreeMap |
|---|---|---|
| Lookup complexity | O(1) amortized | O(log n) — ~14 comparisons for 10k entries |
| Cache behavior | Single hash + 1–2 cache lines | Tree traversal across multiple cache lines |
| At 10k entries | ~20–40 ns/lookup | ~60–150 ns/lookup |

HashMap uses a flat hash table (Rust's std uses hashbrown/SwissTable internally), so a lookup is: hash the key → probe a few slots in a contiguous array. BTreeMap walks a B-tree with a branching factor of ~11, requiring ~3–4 node hops, each potentially a cache miss.

At 50,000 queries/second (~20µs between queries), both are extremely fast and neither will be a bottleneck. But if latency or throughput matters at higher volumes, HashMap wins for unordered point lookups.

## When BTreeMap Wins Instead

- You need range queries (`range(start..end)`)
- You need sorted iteration
- You need deterministic ordering for reproducibility
- Keys don't implement `Hash` but do implement `Ord`

## Benchmark Code (run it yourself)

Add to `Cargo.toml`:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "map_lookup"
harness = false
```

Create `benches/map_lookup.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::{BTreeMap, HashMap};

const N: usize = 10_000;

fn bench_hashmap_lookup(c: &mut Criterion) {
    let map: HashMap<u64, u64> = (0..N as u64).map(|i| (i, i * 2)).collect();
    c.bench_function("hashmap_lookup_10k", |b| {
        b.iter(|| {
            for i in 0..N as u64 {
                black_box(map.get(&i));
            }
        })
    });
}

fn bench_btreemap_lookup(c: &mut Criterion) {
    let map: BTreeMap<u64, u64> = (0..N as u64).map(|i| (i, i * 2)).collect();
    c.bench_function("btreemap_lookup_10k", |b| {
        b.iter(|| {
            for i in 0..N as u64 {
                black_box(map.get(&i));
            }
        })
    });
}

criterion_group!(benches, bench_hashmap_lookup, bench_btreemap_lookup);
criterion_main!(benches);
```

Run with:

```bash
cargo bench --bench map_lookup
```

## Expected Results

On typical x86-64 hardware you'll see something like:

```
hashmap_lookup_10k      time: [180 µs  185 µs  190 µs]
btreemap_lookup_10k     time: [520 µs  535 µs  550 µs]
```

That's roughly **2.8–3x faster for HashMap** on sequential u64 keys. With string keys or random access patterns the gap often widens to 4–5x.

## Recommendation

**Use `HashMap`** for your lookup table. At 10k entries with 50k queries/sec, it's the right default for unordered point lookups. Switch to `BTreeMap` only if you later need ordered iteration or range queries.
