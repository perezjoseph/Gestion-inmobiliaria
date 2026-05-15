use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use chrono::NaiveDate;
use rand::Rng;
use uuid::Uuid;

use dashboard_aggregation_bench::{
    current, numeric_month_keys, preallocated, sort_based, Pago,
};

/// Generate realistic pagos matching production distribution:
/// - ~50 propiedades spread across pagos
/// - 70% "pagado", 20% "pendiente", 10% "atrasado"
/// - Dates span 24 months (2023-01 to 2024-12)
/// - Montos between 5000 and 50000 DOP
fn generate_realistic_pagos(n: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();
    let num_propiedades = 50;
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let contratos: Vec<Uuid> = (0..200).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let estado = match rng.gen_range(0..10) {
                0 => "atrasado",
                1..=2 => "pendiente",
                _ => "pagado",
            };

            let year = if rng.gen_bool(0.5) { 2023 } else { 2024 };
            let month = rng.gen_range(1..=12);
            let day = rng.gen_range(1..=28);
            let fecha = NaiveDate::from_ymd_opt(year, month, day).unwrap();

            let fecha_pago = if estado == "pagado" {
                // Paid pagos have a fecha_pago (sometimes a few days after vencimiento)
                let offset = rng.gen_range(0..=5);
                Some(fecha + chrono::Duration::days(offset))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id: contratos[rng.gen_range(0..contratos.len())],
                propiedad_id: propiedades[rng.gen_range(0..propiedades.len())],
                monto: rng.gen_range(5000.0..50000.0),
                moneda: if rng.gen_bool(0.9) {
                    "DOP".to_string()
                } else {
                    "USD".to_string()
                },
                fecha_vencimiento: fecha,
                fecha_pago,
                estado: estado.to_string(),
            }
        })
        .collect()
}

fn bench_comparison(c: &mut Criterion) {
    // Production size: ~2000 pagos
    let pagos = generate_realistic_pagos(2000);

    let mut group = c.benchmark_group("dashboard_aggregation");

    group.bench_function("current", |b| {
        b.iter(|| current(criterion::black_box(&pagos)))
    });

    group.bench_function("numeric_month_keys", |b| {
        b.iter(|| numeric_month_keys(criterion::black_box(&pagos)))
    });

    group.bench_function("preallocated", |b| {
        b.iter(|| preallocated(criterion::black_box(&pagos)))
    });

    group.bench_function("sort_based", |b| {
        b.iter(|| sort_based(criterion::black_box(&pagos)))
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_scaling");

    for size in [500, 1000, 2000, 5000] {
        let pagos = generate_realistic_pagos(size);

        group.bench_with_input(
            BenchmarkId::new("current", size),
            &pagos,
            |b, data| b.iter(|| current(criterion::black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("numeric_month_keys", size),
            &pagos,
            |b, data| b.iter(|| numeric_month_keys(criterion::black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("preallocated", size),
            &pagos,
            |b, data| b.iter(|| preallocated(criterion::black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("sort_based", size),
            &pagos,
            |b, data| b.iter(|| sort_based(criterion::black_box(data))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_comparison, bench_scaling);
criterion_main!(benches);
