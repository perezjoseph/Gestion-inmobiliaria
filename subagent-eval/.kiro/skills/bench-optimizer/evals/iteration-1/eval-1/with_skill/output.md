# Grouping 2000 Payments by Property: HashMap vs Sort+Scan

## Step 1: Is This Worth Optimizing?

Looking at `dashboard.rs`, this grouping runs on every dashboard load. With ~2000 pagos and ~50 propiedades, and it's called per-request, so it qualifies as a hot path worth measuring.

That said, if the surrounding DB queries dominate (they often do), the absolute gain might be irrelevant. We'll know after measuring.

## Step 2: Setup

Add criterion to `backend/Cargo.toml`:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
rand = "0.10"

[[bench]]
name = "group_payments"
harness = false
```

## Step 3: The Benchmark

Create `backend/benches/group_payments.rs`:

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Pago {
    id: Uuid,
    contrato_id: Uuid,
    propiedad_id: Uuid,
    monto: f64,
    estado: String,
}

/// Generate realistic data: ~2000 pagos spread across ~50 propiedades.
/// 70% pagado, 20% pendiente, 10% atrasado.
fn generate_pagos(n: usize, num_propiedades: usize) -> Vec<Pago> {
    let mut rng = rand::rng();
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let prop_idx = rng.random_range(0..num_propiedades);
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: propiedades[prop_idx],
                monto: rng.random_range(5000.0..50000.0),
                estado: match rng.random_range(0..10u8) {
                    0 => "atrasado".to_string(),
                    1..=2 => "pendiente".to_string(),
                    _ => "pagado".to_string(),
                },
            }
        })
        .collect()
}

// ─── Approach A: HashMap grouping (current implementation) ───────────────────

fn group_hashmap<'a>(pagos: &'a [Pago]) -> HashMap<Uuid, Vec<&'a Pago>> {
    let mut map: HashMap<Uuid, Vec<&Pago>> = HashMap::new();
    for p in pagos {
        map.entry(p.propiedad_id).or_default().push(p);
    }
    map
}

// ─── Approach B: HashMap with pre-allocated capacity ─────────────────────────

fn group_hashmap_precap<'a>(pagos: &'a [Pago], num_propiedades: usize) -> HashMap<Uuid, Vec<&'a Pago>> {
    let mut map: HashMap<Uuid, Vec<&Pago>> = HashMap::with_capacity(num_propiedades);
    for p in pagos {
        map.entry(p.propiedad_id)
            .or_insert_with(|| Vec::with_capacity(pagos.len() / num_propiedades))
            .push(p);
    }
    map
}

// ─── Approach C: Sort then scan (the user's hypothesis) ──────────────────────

fn group_sort_scan<'a>(pagos: &'a mut [Pago]) -> Vec<(Uuid, Vec<&'a Pago>)> {
    pagos.sort_unstable_by_key(|p| p.propiedad_id);

    let mut result: Vec<(Uuid, Vec<&Pago>)> = Vec::new();
    let mut i = 0;
    while i < pagos.len() {
        let current_prop = pagos[i].propiedad_id;
        let start = i;
        while i < pagos.len() && pagos[i].propiedad_id == current_prop {
            i += 1;
        }
        let group: Vec<&Pago> = pagos[start..i].iter().collect();
        result.push((current_prop, group));
    }
    result
}

// ─── Benchmark ───────────────────────────────────────────────────────────────

fn bench_group_payments(c: &mut Criterion) {
    let pagos = generate_pagos(2000, 50);

    let mut group = c.benchmark_group("group_payments_by_propiedad");

    group.bench_function("hashmap_current", |b| {
        b.iter(|| group_hashmap(criterion::black_box(&pagos)))
    });

    group.bench_function("hashmap_precap", |b| {
        b.iter(|| group_hashmap_precap(criterion::black_box(&pagos), 50))
    });

    group.bench_function("sort_then_scan", |b| {
        b.iter_batched(
            || pagos.clone(),
            |mut data| group_sort_scan(criterion::black_box(&mut data)),
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// Also benchmark at different scales to see if behavior changes
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_payments_scaling");

    for &(n, props) in &[(500, 20), (2000, 50), (5000, 100)] {
        let pagos = generate_pagos(n, props);

        group.bench_with_input(
            BenchmarkId::new("hashmap", n),
            &pagos,
            |b, data| b.iter(|| group_hashmap(criterion::black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("sort_scan", n),
            &pagos,
            |b, data| {
                b.iter_batched(
                    || data.clone(),
                    |mut d| group_sort_scan(criterion::black_box(&mut d)),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_group_payments, bench_scaling);
criterion_main!(benches);
```

## Step 4: Run the Benchmark

```bash
cargo bench --bench group_payments --release
```

You **must** run this command to get real numbers. I'm not going to predict which approach wins — that's exactly the trap this workflow exists to prevent. The HashMap has O(n) amortized inserts but involves hashing Uuids and random memory access patterns. The sort+scan has O(n log n) sort cost but better cache locality during the scan phase. At n=2000 with 50 keys, either could win depending on your hardware's cache behavior and the cost of hashing 128-bit Uuids.

## Step 5: Interpreting Results

After running, criterion will report mean times with confidence intervals for each approach. Use these decision rules:

- **Winner < 10% faster**: keep the HashMap (it's simpler, more idiomatic Rust, and the current code already works)
- **Winner 10-50% faster**: consider adopting if readability is comparable
- **Winner > 50% faster**: adopt it and document why

Also consider: if the absolute time is under 100µs for 2000 items and your DB queries take 20-50ms, the grouping is <0.5% of request latency. In that case, **don't optimize** — keep the clearest code regardless of which is faster in isolation.

## Step 6: After You Have Results

If a clear winner emerges and the absolute improvement matters:

1. Replace the implementation in `dashboard.rs`
2. Run `cargo test` to verify correctness
3. Run `cargo clippy` and `cargo fmt`
4. Keep the benchmark file at `benches/group_payments.rs` for regression detection
5. Add a comment documenting what was measured and what won

## Notes on the Sort+Scan Approach

One thing to be aware of: the sort+scan approach requires a `&mut [Pago]` (or a clone), since it reorders the input. Your current code works with `&[pago::Model]` borrowed from the DB result. If sort+scan wins, you'd need to either clone the vec or sort indices instead, which adds allocation cost. The benchmark above accounts for this by using `iter_batched` with a clone in the setup.
