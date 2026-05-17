use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use uuid::Uuid;

use dashboard_bench::{
    ingresos_numeric_key, ingresos_original, ingresos_presized, ingresos_sort_scan, Pago,
};

/// Generate realistic test data matching production characteristics:
/// ~50 propiedades, ~24 months of data, ~2000 pagos total.
fn generate_pagos(count: usize, num_propiedades: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let contratos: Vec<Uuid> = (0..num_propiedades * 2).map(|_| Uuid::new_v4()).collect();

    let estados = ["pagado", "pendiente", "atrasado"];

    (0..count)
        .map(|_| {
            let prop_idx = rng.gen_range(0..num_propiedades);
            let contrato_idx = rng.gen_range(0..contratos.len());
            let estado = estados[rng.gen_range(0..estados.len())];

            // Random date within 2 years (2023-01 to 2024-12)
            let year = if rng.gen_bool(0.5) { 2023 } else { 2024 };
            let month = rng.gen_range(1..=12);
            let day = rng.gen_range(1..=28);
            let fecha_vencimiento = NaiveDate::from_ymd_opt(year, month, day).unwrap();

            let fecha_pago = if estado == "pagado" {
                // Paid within a few days of due date
                let offset = rng.gen_range(0..=5);
                Some(fecha_vencimiento + chrono::Duration::days(offset))
            } else if estado == "atrasado" && rng.gen_bool(0.3) {
                // Some late payments were eventually paid
                let offset = rng.gen_range(10..=30);
                Some(fecha_vencimiento + chrono::Duration::days(offset))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id: contratos[contrato_idx],
                propiedad_id: propiedades[prop_idx],
                monto: rng.gen_range(5000.0..=50000.0),
                moneda: if rng.gen_bool(0.8) {
                    "DOP".to_string()
                } else {
                    "USD".to_string()
                },
                fecha_vencimiento,
                fecha_pago,
                estado: estado.to_string(),
            }
        })
        .collect()
}

fn bench_dashboard_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_aggregation");

    // Production-like dataset: 2000 pagos, 50 propiedades
    let pagos_2000 = generate_pagos(2000, 50);

    group.bench_with_input(
        BenchmarkId::new("original", "2000_pagos"),
        &pagos_2000,
        |b, data| b.iter(|| ingresos_original(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("numeric_key", "2000_pagos"),
        &pagos_2000,
        |b, data| b.iter(|| ingresos_numeric_key(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("presized", "2000_pagos"),
        &pagos_2000,
        |b, data| b.iter(|| ingresos_presized(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("sort_scan", "2000_pagos"),
        &pagos_2000,
        |b, data| b.iter(|| ingresos_sort_scan(black_box(data))),
    );

    // Also test with smaller dataset (500 pagos) to see scaling
    let pagos_500 = generate_pagos(500, 20);

    group.bench_with_input(
        BenchmarkId::new("original", "500_pagos"),
        &pagos_500,
        |b, data| b.iter(|| ingresos_original(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("numeric_key", "500_pagos"),
        &pagos_500,
        |b, data| b.iter(|| ingresos_numeric_key(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("presized", "500_pagos"),
        &pagos_500,
        |b, data| b.iter(|| ingresos_presized(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("sort_scan", "500_pagos"),
        &pagos_500,
        |b, data| b.iter(|| ingresos_sort_scan(black_box(data))),
    );

    // Large dataset (10000 pagos) to stress test
    let pagos_10000 = generate_pagos(10000, 100);

    group.bench_with_input(
        BenchmarkId::new("original", "10000_pagos"),
        &pagos_10000,
        |b, data| b.iter(|| ingresos_original(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("numeric_key", "10000_pagos"),
        &pagos_10000,
        |b, data| b.iter(|| ingresos_numeric_key(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("presized", "10000_pagos"),
        &pagos_10000,
        |b, data| b.iter(|| ingresos_presized(black_box(data))),
    );

    group.bench_with_input(
        BenchmarkId::new("sort_scan", "10000_pagos"),
        &pagos_10000,
        |b, data| b.iter(|| ingresos_sort_scan(black_box(data))),
    );

    group.finish();
}

criterion_group!(benches, bench_dashboard_aggregation);
criterion_main!(benches);
