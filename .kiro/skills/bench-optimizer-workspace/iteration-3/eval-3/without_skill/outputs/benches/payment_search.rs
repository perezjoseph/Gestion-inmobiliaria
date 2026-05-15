use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use payment_search_bench::{buscar_pagos_linear, Pago, PagoIndex};
use rand::Rng;
use uuid::Uuid;

/// Generate a realistic dataset: ~5000 payments spread across ~200 contracts.
/// Each contract has ~25 payments on average.
fn generate_dataset(n: usize, num_contratos: usize) -> (Vec<Pago>, Vec<Uuid>) {
    let mut rng = rand::thread_rng();
    let contratos: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();
    let estados = ["pendiente", "pagado", "atrasado"];

    let pagos: Vec<Pago> = (0..n)
        .map(|i| {
            let contrato_id = contratos[rng.gen_range(0..num_contratos)];
            let day_offset = rng.gen_range(0..730);
            let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
            let fecha_vencimiento = base_date
                .checked_add_signed(chrono::Duration::days(day_offset))
                .unwrap();
            let estado = estados[rng.gen_range(0..3)].to_string();
            let referencia = if rng.gen_bool(0.7) {
                Some(format!("REF-{:06}", i))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id,
                monto: rng.gen_range(5000.0..50000.0),
                fecha_vencimiento,
                fecha_pago: if estado == "pagado" {
                    Some(
                        fecha_vencimiento
                            .checked_add_signed(chrono::Duration::days(rng.gen_range(-5..10)))
                            .unwrap(),
                    )
                } else {
                    None
                },
                estado,
                referencia,
            }
        })
        .collect();

    (pagos, contratos)
}

fn bench_contrato_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_contrato_id");

    for size in [1000, 5000, 10000] {
        let num_contratos = size / 25; // ~25 payments per contract
        let (pagos, contratos) = generate_dataset(size, num_contratos);
        let target_contrato = contratos[0];
        let index = PagoIndex::build(&pagos);

        group.bench_with_input(BenchmarkId::new("linear_scan", size), &size, |b, _| {
            b.iter(|| {
                buscar_pagos_linear(
                    black_box(&pagos),
                    black_box(Some(target_contrato)),
                    None,
                    None,
                    None,
                    None,
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("indexed_hashmap", size), &size, |b, _| {
            b.iter(|| {
                index.buscar(
                    black_box(&pagos),
                    black_box(Some(target_contrato)),
                    None,
                    None,
                    None,
                    None,
                )
            });
        });
    }

    group.finish();
}

fn bench_contrato_with_estado(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_contrato_and_estado");

    let (pagos, contratos) = generate_dataset(5000, 200);
    let target_contrato = contratos[0];
    let index = PagoIndex::build(&pagos);

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                None,
                None,
                None,
            )
        });
    });

    group.bench_function("indexed_hashmap", |b| {
        b.iter(|| {
            index.buscar(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(Some("pendiente")),
                None,
                None,
                None,
            )
        });
    });

    group.finish();
}

fn bench_no_contrato_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("no_contrato_filter");

    let (pagos, _contratos) = generate_dataset(5000, 200);
    let index = PagoIndex::build(&pagos);
    let fecha_desde = NaiveDate::from_ymd_opt(2023, 6, 1);
    let fecha_hasta = NaiveDate::from_ymd_opt(2023, 12, 31);

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                None,
                None,
                black_box(fecha_desde),
                black_box(fecha_hasta),
                None,
            )
        });
    });

    group.bench_function("indexed_hashmap", |b| {
        b.iter(|| {
            index.buscar(
                black_box(&pagos),
                None,
                None,
                black_box(fecha_desde),
                black_box(fecha_hasta),
                None,
            )
        });
    });

    group.finish();
}

fn bench_index_build_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_build_cost");

    for size in [1000, 5000, 10000] {
        let num_contratos = size / 25;
        let (pagos, _) = generate_dataset(size, num_contratos);

        group.bench_with_input(BenchmarkId::new("build_index", size), &size, |b, _| {
            b.iter(|| PagoIndex::build(black_box(&pagos)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_contrato_filter,
    bench_contrato_with_estado,
    bench_no_contrato_filter,
    bench_index_build_cost,
);
criterion_main!(benches);
