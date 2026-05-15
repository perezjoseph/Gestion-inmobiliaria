use chrono::NaiveDate;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use overlap_bench::*;
use rand::Rng;
use uuid::Uuid;

/// Generate realistic contract data for benchmarking.
/// Simulates a bulk import scenario where multiple contracts exist for the same propiedad.
/// - `n`: total number of contracts
/// - ~70% are "activo", ~20% "vencido", ~10% "cancelado"
/// - All contracts belong to a small number of propiedades (simulates bulk import for one property)
/// - Date ranges span 24 months with some overlaps
fn generate_contratos(n: usize, num_propiedades: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let propiedad_id = propiedades[rng.gen_range(0..num_propiedades)];
            let start_month = rng.gen_range(1..=22u32);
            let duration_months = rng.gen_range(1..=12u32);
            let end_month = (start_month + duration_months).min(24);

            let fecha_inicio =
                NaiveDate::from_ymd_opt(2024, ((start_month - 1) % 12) + 1, 1).unwrap();
            let fecha_fin = NaiveDate::from_ymd_opt(
                2024 + (end_month / 13) as i32,
                ((end_month - 1) % 12) + 1,
                28,
            )
            .unwrap();

            let estado = match rng.gen_range(0..10) {
                0 => "cancelado".to_string(),
                1..=2 => "vencido".to_string(),
                _ => "activo".to_string(),
            };

            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio,
                fecha_fin,
                estado,
            }
        })
        .collect()
}

/// Benchmark at production-representative sizes:
/// - n=10: normal use (5-15 contracts per propiedad)
/// - n=50: moderate load
/// - n=200: bulk import scenario (the case we're investigating)
fn bench_overlap_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_detection");

    // Single propiedad scenario (worst case for overlap detection — all contracts same property)
    for size in [10, 50, 200] {
        let contratos = generate_contratos(size, 1);

        group.bench_with_input(
            BenchmarkId::new("bruteforce", size),
            &contratos,
            |b, data| b.iter(|| detectar_solapamientos_bruteforce(data)),
        );

        group.bench_with_input(
            BenchmarkId::new("sort_scan", size),
            &contratos,
            |b, data| b.iter(|| detectar_solapamientos_sort_scan(data)),
        );

        group.bench_with_input(
            BenchmarkId::new("sort_early_exit", size),
            &contratos,
            |b, data| b.iter(|| detectar_solapamientos_sort_early_exit(data)),
        );

        group.bench_with_input(
            BenchmarkId::new("prefilter_only", size),
            &contratos,
            |b, data| b.iter(|| detectar_solapamientos_prefilter(data)),
        );
    }

    group.finish();
}

/// Benchmark the realistic multi-propiedad scenario during bulk import
/// (200 contracts spread across 5 propiedades)
fn bench_multi_propiedad(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_multi_propiedad");

    let contratos = generate_contratos(200, 5);

    group.bench_function("bruteforce", |b| {
        b.iter(|| detectar_solapamientos_bruteforce(&contratos))
    });

    group.bench_function("sort_scan", |b| {
        b.iter(|| detectar_solapamientos_sort_scan(&contratos))
    });

    group.bench_function("sort_early_exit", |b| {
        b.iter(|| detectar_solapamientos_sort_early_exit(&contratos))
    });

    group.bench_function("prefilter_only", |b| {
        b.iter(|| detectar_solapamientos_prefilter(&contratos))
    });

    group.finish();
}

criterion_group!(benches, bench_overlap_detection, bench_multi_propiedad);
criterion_main!(benches);
