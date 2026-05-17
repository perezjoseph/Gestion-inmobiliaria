use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use uuid::Uuid;

use payment_search_bench::{buscar_pagos_linear, Pago, PagoIndex, PagoIndexDirect};

/// Generate realistic payment data matching production characteristics:
/// - ~200 unique contratos (5000 pagos / ~25 payments per contract)
/// - 70% pagado, 20% pendiente, 10% atrasado
/// - Dates spanning 24 months
/// - ~30% have a referencia string
fn generate_realistic_pagos(n: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();
    let num_contratos = (n / 25).max(1);
    let contrato_ids: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|i| {
            let contrato_id = contrato_ids[i % num_contratos];
            let month = (i % 24) as u32 + 1;
            let year = 2023 + (month - 1) / 12;
            let month_in_year = ((month - 1) % 12) + 1;

            let estado = match rng.gen_range(0..10) {
                0 => "atrasado".to_string(),
                1..=2 => "pendiente".to_string(),
                _ => "pagado".to_string(),
            };

            let fecha_vencimiento = NaiveDate::from_ymd_opt(year as i32, month_in_year, 1).unwrap();

            let fecha_pago = if estado == "pagado" {
                Some(
                    NaiveDate::from_ymd_opt(year as i32, month_in_year, rng.gen_range(1..=28))
                        .unwrap(),
                )
            } else {
                None
            };

            let referencia = if rng.gen_range(0..10) < 3 {
                Some(format!("REF-{:04}", rng.gen_range(1..9999)))
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
fn bench_contrato_id_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("contrato_id_filter");

    for size in [1000, 5000] {
        let pagos = generate_realistic_pagos(size);
        // Pick a contrato_id that exists in the data
        let target_contrato = pagos[size / 2].contrato_id;

        group.bench_with_input(
            BenchmarkId::new("linear_scan", size),
            &(&pagos, target_contrato),
            |b, (pagos, cid)| {
                b.iter(|| buscar_pagos_linear(black_box(pagos), Some(*cid), None, None, None, None))
            },
        );

        // For indexed approaches, build the index outside the loop (amortized cost)
        let index = PagoIndex::new(&pagos);
        group.bench_with_input(
            BenchmarkId::new("indexed_dyn", size),
            &(&index, target_contrato),
            |b, (idx, cid)| b.iter(|| idx.buscar(Some(black_box(*cid)), None, None, None, None)),
        );

        let index_direct = PagoIndexDirect::new(&pagos);
        group.bench_with_input(
            BenchmarkId::new("indexed_direct", size),
            &(&index_direct, target_contrato),
            |b, (idx, cid)| b.iter(|| idx.buscar(Some(black_box(*cid)), None, None, None, None)),
        );
    }

    group.finish();
}

/// Benchmark combined filter: contrato_id + estado (common in "show pending payments for contract")
fn bench_contrato_and_estado(c: &mut Criterion) {
    let mut group = c.benchmark_group("contrato_id_and_estado");

    let pagos = generate_realistic_pagos(5000);
    let target_contrato = pagos[2500].contrato_id;

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                Some(target_contrato),
                Some("pendiente"),
                None,
                None,
                None,
            )
        })
    });

    let index_direct = PagoIndexDirect::new(&pagos);
    group.bench_function("indexed_direct", |b| {
        b.iter(|| {
            index_direct.buscar(
                Some(black_box(target_contrato)),
                Some("pendiente"),
                None,
                None,
                None,
            )
        })
    });

    group.finish();
}

/// Benchmark the fallback case: no contrato_id filter (should be similar to linear).
fn bench_no_contrato_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("no_contrato_filter");

    let pagos = generate_realistic_pagos(5000);
    let fecha_desde = NaiveDate::from_ymd_opt(2024, 1, 1);
    let fecha_hasta = NaiveDate::from_ymd_opt(2024, 6, 30);

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                None,
                None,
                fecha_desde,
                fecha_hasta,
                None,
            )
        })
    });

    let index_direct = PagoIndexDirect::new(&pagos);
    group.bench_function("indexed_direct", |b| {
        b.iter(|| index_direct.buscar(black_box(None), None, fecha_desde, fecha_hasta, None))
    });

    group.finish();
}

/// Benchmark index build cost to understand amortization.
fn bench_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_build_cost");

    for size in [1000, 5000] {
        let pagos = generate_realistic_pagos(size);

        group.bench_with_input(BenchmarkId::new("build_index", size), &pagos, |b, pagos| {
            b.iter(|| PagoIndexDirect::new(black_box(pagos)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_contrato_id_filter,
    bench_contrato_and_estado,
    bench_no_contrato_filter,
    bench_index_build,
);
criterion_main!(benches);
