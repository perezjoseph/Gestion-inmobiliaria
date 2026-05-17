use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use uuid::Uuid;

use payment_search_bench::{buscar_pagos_linear, Pago, PagoIndex, PagoRefIndex};

/// Generate realistic payment data matching production characteristics:
/// - ~5000 pagos total (as documented in the function comment)
/// - ~200 distinct contratos (~25 pagos per contrato)
/// - 70% pagado, 20% pendiente, 10% atrasado
/// - Dates spanning 24 months
/// - ~30% have a referencia string
fn generate_realistic_pagos(n: usize, num_contratos: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();

    // Pre-generate contrato IDs so multiple pagos share the same contrato
    let contrato_ids: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let contrato_id = contrato_ids[rng.gen_range(0..num_contratos)];
            let estado = match rng.gen_range(0u8..10) {
                0 => "atrasado".to_string(),
                1..=2 => "pendiente".to_string(),
                _ => "pagado".to_string(),
            };

            // Dates spanning 2022-01-01 to 2023-12-31
            let days_offset = rng.gen_range(0..730);
            let fecha_vencimiento =
                NaiveDate::from_ymd_opt(2022, 1, 1).unwrap() + chrono::Duration::days(days_offset);

            let fecha_pago = if estado == "pagado" {
                let pay_offset = rng.gen_range(0..10); // paid within 10 days
                Some(fecha_vencimiento + chrono::Duration::days(pay_offset))
            } else {
                None
            };

            let referencia = if rng.gen_bool(0.3) {
                Some(format!("REF-{:06}", rng.gen_range(100000u32..999999)))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id,
                monto: rng.gen_range(5000.0..50000.0),
                fecha_vencimiento,
                fecha_pago,
                estado,
                referencia,
            }
        })
        .collect()
}

/// Benchmark the most common query pattern: filter by contrato_id (80% of queries).
/// This is the case where pre-indexing should shine.
fn bench_filter_by_contrato(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_contrato_id");

    let pagos = generate_realistic_pagos(5000, 200);

    // Pick a contrato_id that exists in the dataset
    let target_contrato = pagos[pagos.len() / 2].contrato_id;

    // Build indices outside the benchmark loop (amortized cost)
    let index = PagoIndex::build(&pagos);
    let ref_index = PagoRefIndex::build(&pagos);

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
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.bench_function("ref_index", |b| {
        b.iter(|| {
            ref_index.buscar(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark combined filter: contrato_id + estado (common in "show pending payments for contract")
fn bench_contrato_plus_estado(c: &mut Criterion) {
    let mut group = c.benchmark_group("contrato_id_plus_estado");

    let pagos = generate_realistic_pagos(5000, 200);
    let target_contrato = pagos[pagos.len() / 2].contrato_id;
    let index = PagoIndex::build(&pagos);
    let ref_index = PagoRefIndex::build(&pagos);

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
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.bench_function("ref_index", |b| {
        b.iter(|| {
            ref_index.buscar(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark the no-contrato case (date range filter only).
/// Index provides no benefit here — both should fall back to linear scan.
fn bench_date_range_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("date_range_only");

    let pagos = generate_realistic_pagos(5000, 200);
    let index = PagoIndex::build(&pagos);

    let desde = NaiveDate::from_ymd_opt(2022, 6, 1).unwrap();
    let hasta = NaiveDate::from_ymd_opt(2022, 8, 31).unwrap();

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(None),
                black_box(None),
                black_box(Some(desde)),
                black_box(Some(hasta)),
                black_box(None),
            )
        })
    });

    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(&pagos),
                black_box(None),
                black_box(None),
                black_box(Some(desde)),
                black_box(Some(hasta)),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark index build cost to understand amortization.
fn bench_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_build_cost");

    let pagos = generate_realistic_pagos(5000, 200);

    group.bench_function("build_hashmap_index", |b| {
        b.iter(|| PagoIndex::build(black_box(&pagos)))
    });

    group.bench_function("build_ref_index", |b| {
        b.iter(|| PagoRefIndex::build(black_box(&pagos)))
    });

    group.finish();
}

/// Benchmark scaling behavior across different dataset sizes.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_by_size");

    for &(size, num_contratos) in &[(500, 20), (1000, 50), (2500, 100), (5000, 200)] {
        let pagos = generate_realistic_pagos(size, num_contratos);
        let target_contrato = pagos[pagos.len() / 2].contrato_id;
        let index = PagoIndex::build(&pagos);

        group.bench_with_input(
            BenchmarkId::new("linear_scan", size),
            &(&pagos, target_contrato),
            |b, (pagos, target)| {
                b.iter(|| {
                    buscar_pagos_linear(
                        black_box(pagos),
                        black_box(Some(*target)),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                    )
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("hashmap_index", size),
            &(&pagos, target_contrato, &index),
            |b, (pagos, target, idx)| {
                b.iter(|| {
                    idx.buscar(
                        black_box(pagos),
                        black_box(Some(*target)),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                    )
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_filter_by_contrato,
    bench_contrato_plus_estado,
    bench_date_range_only,
    bench_index_build,
    bench_scaling,
);
criterion_main!(benches);
