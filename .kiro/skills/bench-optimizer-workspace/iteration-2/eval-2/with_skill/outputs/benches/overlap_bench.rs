use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use overlap_bench::{
    detectar_solapamientos_grouped_optimized, detectar_solapamientos_original,
    detectar_solapamientos_prefilter, detectar_solapamientos_sort_scan, Contrato,
};
use rand::Rng;
use uuid::Uuid;

/// Generate realistic contract data matching production characteristics:
/// - Multiple propiedades (n/5 unique propiedades, so ~5 contracts per propiedad on average)
/// - 60% active, 20% vencido, 10% cancelado, 10% terminado
/// - Date ranges spanning 24 months with realistic overlap patterns
fn generate_contratos(n: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let num_propiedades = (n / 5).max(1);
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let propiedad_id = propiedades[rng.gen_range(0..propiedades.len())];
            let start_month = rng.gen_range(1..=24);
            let duration_months = rng.gen_range(3..=12);
            let start_day = rng.gen_range(1..=28);

            let year_start = 2023 + (start_month - 1) / 12;
            let month_start = ((start_month - 1) % 12) + 1;

            let end_month_total = start_month + duration_months;
            let year_end = 2023 + (end_month_total - 1) / 12;
            let month_end = ((end_month_total - 1) % 12) + 1;

            let estado = match rng.gen_range(0..10) {
                0..=5 => "activo",
                6..=7 => "vencido",
                8 => "cancelado",
                _ => "terminado",
            };

            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio: chrono::NaiveDate::from_ymd_opt(
                    year_start as i32,
                    month_start as u32,
                    start_day,
                )
                .unwrap(),
                fecha_fin: chrono::NaiveDate::from_ymd_opt(
                    year_end as i32,
                    month_end as u32,
                    start_day.min(28),
                )
                .unwrap(),
                estado: estado.to_string(),
            }
        })
        .collect()
}

/// Benchmark at production-representative sizes:
/// - n=10: typical normal use (5-15 contracts)
/// - n=50: moderate load
/// - n=200: bulk import scenario (the case that prompted this question)
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_detection");

    for size in [10, 50, 200] {
        let data = generate_contratos(size);

        group.bench_with_input(BenchmarkId::new("original_n2", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_original(d))
        });

        group.bench_with_input(BenchmarkId::new("sort_scan", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_sort_scan(d))
        });

        group.bench_with_input(BenchmarkId::new("prefilter_n2", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_prefilter(d))
        });

        group.bench_with_input(
            BenchmarkId::new("grouped_optimized", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_grouped_optimized(d)),
        );
    }

    group.finish();
}

/// Benchmark the bulk import scenario specifically (n=200)
/// with higher measurement time for more stable results.
fn bench_bulk_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_import_200");
    group.measurement_time(std::time::Duration::from_secs(5));

    let data = generate_contratos(200);

    group.bench_function("original_n2", |b| {
        b.iter(|| detectar_solapamientos_original(&data))
    });

    group.bench_function("sort_scan", |b| {
        b.iter(|| detectar_solapamientos_sort_scan(&data))
    });

    group.bench_function("prefilter_n2", |b| {
        b.iter(|| detectar_solapamientos_prefilter(&data))
    });

    group.bench_function("grouped_optimized", |b| {
        b.iter(|| detectar_solapamientos_grouped_optimized(&data))
    });

    group.finish();
}

criterion_group!(benches, bench_scaling, bench_bulk_import);
criterion_main!(benches);
