use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dashboard_aggregation_bench::{
    approach_numeric_key, approach_presized_numeric, approach_sort_scan, current, Pago,
};
use uuid::Uuid;

/// Generate realistic pagos matching production distribution:
/// - ~50 propiedades spread across pagos
/// - 70% "pagado", 20% "pendiente", 10% "atrasado"
/// - Dates span 24 months (2023-01 to 2024-12)
/// - fecha_pago is Some for "pagado", None for others
fn generate_realistic_pagos(n: usize) -> Vec<Pago> {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let propiedades: Vec<Uuid> = (0..50).map(|_| Uuid::new_v4()).collect();
    let contratos: Vec<Uuid> = (0..200).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let estado_roll: u32 = rng.gen_range(0..10);
            let estado = match estado_roll {
                0 => "atrasado",
                1..=2 => "pendiente",
                _ => "pagado",
            };

            // Random month in 2023-2024 range (24 months)
            let year = if rng.gen_bool(0.5) { 2023 } else { 2024 };
            let month = rng.gen_range(1..=12);
            let day = rng.gen_range(1..=28);
            let fecha_vencimiento = chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap();

            let fecha_pago = if estado == "pagado" {
                // Paid within a few days of due date
                let offset: i64 = rng.gen_range(-5..=10);
                Some(fecha_vencimiento + chrono::Duration::days(offset))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id: contratos[rng.gen_range(0..contratos.len())],
                propiedad_id: propiedades[rng.gen_range(0..propiedades.len())],
                monto: rng.gen_range(5000.0..50000.0),
                moneda: "DOP".to_string(),
                fecha_vencimiento,
                fecha_pago,
                estado: estado.to_string(),
            }
        })
        .collect()
}

fn bench_dashboard_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingresos_por_propiedad_mes");

    // Production size: ~2000 pagos
    let data_2000 = generate_realistic_pagos(2000);

    group.bench_function("current", |b| {
        b.iter(|| current(criterion::black_box(&data_2000)))
    });

    group.bench_function("numeric_key", |b| {
        b.iter(|| approach_numeric_key(criterion::black_box(&data_2000)))
    });

    group.bench_function("presized_numeric", |b| {
        b.iter(|| approach_presized_numeric(criterion::black_box(&data_2000)))
    });

    group.bench_function("sort_scan", |b| {
        b.iter(|| approach_sort_scan(criterion::black_box(&data_2000)))
    });

    group.finish();
}

/// Also test scaling behavior to see if any approach has different characteristics
/// at smaller or larger sizes.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [500, 2000, 5000] {
        let data = generate_realistic_pagos(size);

        group.bench_with_input(BenchmarkId::new("current", size), &data, |b, d| {
            b.iter(|| current(criterion::black_box(d)))
        });

        group.bench_with_input(BenchmarkId::new("presized_numeric", size), &data, |b, d| {
            b.iter(|| approach_presized_numeric(criterion::black_box(d)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_dashboard_aggregation, bench_scaling);
criterion_main!(benches);
