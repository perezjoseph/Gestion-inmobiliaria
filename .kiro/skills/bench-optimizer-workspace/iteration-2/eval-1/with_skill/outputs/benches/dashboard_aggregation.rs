use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dashboard_bench::*;

use chrono::NaiveDate;
use rand::Rng;
use uuid::Uuid;

/// Generate realistic pagos matching production distribution:
/// - ~50 propiedades
/// - 70% "pagado", 20% "pendiente", 10% "atrasado"
/// - Dates span 24 months (2023-01 to 2024-12)
/// - Montos between 5000 and 50000
fn generate_realistic_pagos(n: usize, num_propiedades: usize) -> Vec<Pago> {
    let mut rng = rand::thread_rng();

    // Pre-generate property IDs
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let estado = match rng.gen_range(0..10) {
                0 => "atrasado".to_string(),
                1..=2 => "pendiente".to_string(),
                _ => "pagado".to_string(),
            };

            let year = rng.gen_range(2023..=2024);
            let month = rng.gen_range(1..=12);
            let day = rng.gen_range(1..=28);
            let fecha_vencimiento = NaiveDate::from_ymd_opt(year, month, day).unwrap();

            let fecha_pago = if estado == "pagado" {
                // Paid within a few days of due date
                let pay_day = (day + rng.gen_range(0..5)).min(28);
                Some(NaiveDate::from_ymd_opt(year, month, pay_day).unwrap())
            } else if estado == "atrasado" && rng.gen_bool(0.5) {
                // Some atrasado have a late payment date
                let pay_month = if month == 12 { 1 } else { month + 1 };
                let pay_year = if month == 12 { year + 1 } else { year };
                Some(NaiveDate::from_ymd_opt(pay_year, pay_month, day.min(28)).unwrap())
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: propiedades[rng.gen_range(0..num_propiedades)],
                monto: rng.gen_range(5000.0..50000.0),
                moneda: if rng.gen_bool(0.8) {
                    "DOP".to_string()
                } else {
                    "USD".to_string()
                },
                fecha_vencimiento,
                fecha_pago,
                estado,
            }
        })
        .collect()
}

fn bench_production_size(c: &mut Criterion) {
    // Production dataset: ~2000 pagos, ~50 propiedades
    let pagos = generate_realistic_pagos(2000, 50);

    let mut group = c.benchmark_group("dashboard_aggregation");

    group.bench_function("current", |b| {
        b.iter(|| ingresos_current(criterion::black_box(&pagos)))
    });

    group.bench_function("numeric_key", |b| {
        b.iter(|| ingresos_numeric_key(criterion::black_box(&pagos)))
    });

    group.bench_function("sort_scan", |b| {
        b.iter(|| ingresos_sort_scan(criterion::black_box(&pagos)))
    });

    group.bench_function("btreemap", |b| {
        b.iter(|| ingresos_btreemap(criterion::black_box(&pagos)))
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [500, 1000, 2000, 5000] {
        let pagos = generate_realistic_pagos(size, 50);

        group.bench_with_input(BenchmarkId::new("current", size), &pagos, |b, data| {
            b.iter(|| ingresos_current(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("numeric_key", size), &pagos, |b, data| {
            b.iter(|| ingresos_numeric_key(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("sort_scan", size), &pagos, |b, data| {
            b.iter(|| ingresos_sort_scan(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("btreemap", size), &pagos, |b, data| {
            b.iter(|| ingresos_btreemap(criterion::black_box(data)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_production_size, bench_scaling);
criterion_main!(benches);
