use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use payment_search_bench::{buscar_pagos_linear, generate_test_data, PagoIndex};

/// Most common query pattern (~80%): filter by contrato_id only.
fn bench_search_by_contrato_id(c: &mut Criterion) {
    let pagos = generate_test_data(5000, 100);
    let target_contrato = pagos[0].contrato_id;
    let index = PagoIndex::new(pagos.clone());

    let mut group = c.benchmark_group("search_by_contrato_id");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.bench_function("indexed_hashmap", |b| {
        b.iter(|| {
            index.buscar_pagos(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.finish();
}

/// Combined filter: contrato_id + estado.
fn bench_search_contrato_and_estado(c: &mut Criterion) {
    let pagos = generate_test_data(5000, 100);
    let target_contrato = pagos[0].contrato_id;
    let index = PagoIndex::new(pagos.clone());

    let mut group = c.benchmark_group("search_contrato_and_estado");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.bench_function("indexed_hashmap", |b| {
        b.iter(|| {
            index.buscar_pagos(
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.finish();
}

/// Fallback case: no contrato_id filter (both approaches do linear scan).
fn bench_search_no_contrato_filter(c: &mut Criterion) {
    let pagos = generate_test_data(5000, 100);
    let index = PagoIndex::new(pagos.clone());

    let mut group = c.benchmark_group("search_no_contrato_filter");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(None),
                black_box(Some("pagado")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.bench_function("indexed_fallback", |b| {
        b.iter(|| {
            index.buscar_pagos(
                black_box(None),
                black_box(Some("pagado")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        });
    });

    group.finish();
}

/// Combined filter: contrato_id + date range.
fn bench_search_contrato_and_date_range(c: &mut Criterion) {
    let pagos = generate_test_data(5000, 100);
    let target_contrato = pagos[0].contrato_id;
    let fecha_desde = chrono::NaiveDate::from_ymd_opt(2023, 6, 1);
    let fecha_hasta = chrono::NaiveDate::from_ymd_opt(2023, 12, 31);
    let index = PagoIndex::new(pagos.clone());

    let mut group = c.benchmark_group("search_contrato_and_date_range");

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(fecha_desde),
                black_box(fecha_hasta),
                black_box(None),
            )
        });
    });

    group.bench_function("indexed_hashmap", |b| {
        b.iter(|| {
            index.buscar_pagos(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(fecha_desde),
                black_box(fecha_hasta),
                black_box(None),
            )
        });
    });

    group.finish();
}

/// Measure the one-time index construction cost.
fn bench_index_construction(c: &mut Criterion) {
    let pagos = generate_test_data(5000, 100);

    let mut group = c.benchmark_group("index_construction");

    group.bench_function("build_index_5000_pagos", |b| {
        b.iter(|| PagoIndex::new(black_box(pagos.clone())));
    });

    group.finish();
}

/// Scaling behavior: how both approaches perform as dataset grows.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_by_dataset_size");

    for size in [1000, 2500, 5000, 10000] {
        let pagos = generate_test_data(size, size / 50);
        let target_contrato = pagos[0].contrato_id;
        let index = PagoIndex::new(pagos.clone());

        group.bench_with_input(BenchmarkId::new("linear_scan", size), &size, |b, _| {
            b.iter(|| {
                buscar_pagos_linear(
                    black_box(&pagos),
                    black_box(Some(target_contrato)),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("indexed_hashmap", size), &size, |b, _| {
            b.iter(|| {
                index.buscar_pagos(
                    black_box(Some(target_contrato)),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_by_contrato_id,
    bench_search_contrato_and_estado,
    bench_search_no_contrato_filter,
    bench_search_contrato_and_date_range,
    bench_index_construction,
    bench_scaling,
);
criterion_main!(benches);
