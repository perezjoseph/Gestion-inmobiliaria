use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use payment_search_bench::{buscar_pagos_linear, generate_realistic_pagos, PagoIndexSimple};
use uuid::Uuid;

/// Production dataset: ~5000 pagos, ~200 contratos.
/// Typical query: filter by contrato_id (80% of queries), result set 20-100.
const PRODUCTION_SIZE: usize = 5000;
const NUM_CONTRATOS: usize = 200;

/// Benchmark the most common query pattern: filter by a single contrato_id.
/// This represents 80% of real queries.
fn bench_filter_by_contrato_id(c: &mut Criterion) {
    let pagos = generate_realistic_pagos(PRODUCTION_SIZE, NUM_CONTRATOS);

    // Pick a contrato_id that exists in the dataset
    let target_contrato_id = pagos[0].contrato_id;

    // Pre-build the index (this cost is amortized across many queries)
    let index = PagoIndexSimple::new(&pagos);

    let mut group = c.benchmark_group("filter_by_contrato_id");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato_id)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(Some(target_contrato_id)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark with combined filters: contrato_id + estado (common in UI).
fn bench_contrato_and_estado(c: &mut Criterion) {
    let pagos = generate_realistic_pagos(PRODUCTION_SIZE, NUM_CONTRATOS);
    let target_contrato_id = pagos[0].contrato_id;
    let index = PagoIndexSimple::new(&pagos);

    let mut group = c.benchmark_group("contrato_id_and_estado");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato_id)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(Some(target_contrato_id)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark the fallback case: no contrato_id filter (date range only).
/// Both approaches should perform similarly here since the index can't help.
fn bench_no_contrato_filter(c: &mut Criterion) {
    let pagos = generate_realistic_pagos(PRODUCTION_SIZE, NUM_CONTRATOS);
    let index = PagoIndexSimple::new(&pagos);

    let fecha_desde = chrono::NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
    let fecha_hasta = chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();

    let mut group = c.benchmark_group("no_contrato_filter");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(None),
                black_box(None),
                black_box(Some(fecha_desde)),
                black_box(Some(fecha_hasta)),
                black_box(None),
            )
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(None),
                black_box(None),
                black_box(Some(fecha_desde)),
                black_box(Some(fecha_hasta)),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark index construction cost to understand amortization.
fn bench_index_construction(c: &mut Criterion) {
    let pagos = generate_realistic_pagos(PRODUCTION_SIZE, NUM_CONTRATOS);

    c.bench_function("index_construction_5000", |b| {
        b.iter(|| PagoIndexSimple::new(black_box(&pagos)))
    });
}

/// Scaling benchmark: how does the advantage change with dataset size?
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_contrato_filter");

    for size in [100, 500, 1000, 2500, 5000] {
        let num_contratos = (size / 25).max(10); // ~25 pagos per contrato
        let pagos = generate_realistic_pagos(size, num_contratos);
        let target_id = pagos[0].contrato_id;
        let index = PagoIndexSimple::new(&pagos);

        group.bench_with_input(BenchmarkId::new("linear_scan", size), &size, |b, _| {
            b.iter(|| {
                buscar_pagos_linear(
                    black_box(&pagos),
                    black_box(Some(target_id)),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
            })
        });

        group.bench_with_input(BenchmarkId::new("hashmap_index", size), &size, |b, _| {
            b.iter(|| {
                index.buscar(
                    black_box(Some(target_id)),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_filter_by_contrato_id,
    bench_contrato_and_estado,
    bench_no_contrato_filter,
    bench_index_construction,
    bench_scaling,
);
criterion_main!(benches);
