use chrono::NaiveDate;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use uuid::Uuid;

use overlap_detection_bench::{
    detectar_solapamientos_grouped, detectar_solapamientos_pairwise,
    detectar_solapamientos_sort_sweep, Contrato,
};

/// Generate realistic contract data for a single propiedad.
/// Simulates the production scenario: contracts for one propiedad being validated.
/// - ~30% of contracts overlap with at least one other (realistic for bulk import validation)
/// - All contracts are "activo" (worst case for the algorithm — no early filtering)
/// - Date ranges span ~2 years
fn generate_contratos_single_propiedad(n: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let propiedad_id = Uuid::new_v4();
    let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

    (0..n)
        .map(|_| {
            // Random start within 24 months
            let start_offset = rng.gen_range(0..730);
            // Duration between 30 and 365 days
            let duration = rng.gen_range(30..365);
            let fecha_inicio = base_date + chrono::Duration::days(start_offset);
            let fecha_fin = fecha_inicio + chrono::Duration::days(duration);

            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio,
                fecha_fin,
                estado: "activo".to_string(),
            }
        })
        .collect()
}

/// Generate contracts spread across multiple propiedades.
/// Simulates bulk import where contracts belong to different properties.
/// ~10 propiedades, contracts distributed among them.
fn generate_contratos_multi_propiedad(n: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let num_propiedades = 10;
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

    (0..n)
        .map(|_| {
            let propiedad_id = propiedades[rng.gen_range(0..num_propiedades)];
            let start_offset = rng.gen_range(0..730);
            let duration = rng.gen_range(30..365);
            let fecha_inicio = base_date + chrono::Duration::days(start_offset);
            let fecha_fin = fecha_inicio + chrono::Duration::days(duration);

            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio,
                fecha_fin,
                // Mix of active and inactive to test filtering
                estado: if rng.gen_range(0..10) < 8 {
                    "activo".to_string()
                } else {
                    "cancelado".to_string()
                },
            }
        })
        .collect()
}

/// Benchmark at production-representative sizes:
/// - n=10: typical single-property validation
/// - n=15: upper end of normal use
/// - n=50: moderate bulk import
/// - n=200: maximum bulk import size
fn bench_single_propiedad_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_propiedad");

    for size in [10, 15, 50, 200] {
        let data = generate_contratos_single_propiedad(size);

        group.bench_with_input(BenchmarkId::new("pairwise", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_pairwise(d))
        });
        group.bench_with_input(BenchmarkId::new("sort_sweep", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_sort_sweep(d))
        });
        group.bench_with_input(BenchmarkId::new("grouped", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_grouped(d))
        });
    }

    group.finish();
}

/// Benchmark the multi-propiedad scenario (bulk import with mixed properties).
/// This tests whether grouping by propiedad provides benefit when contracts
/// are spread across multiple properties.
fn bench_multi_propiedad_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_propiedad");

    for size in [10, 15, 50, 200] {
        let data = generate_contratos_multi_propiedad(size);

        group.bench_with_input(BenchmarkId::new("pairwise", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_pairwise(d))
        });
        group.bench_with_input(BenchmarkId::new("sort_sweep", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_sort_sweep(d))
        });
        group.bench_with_input(BenchmarkId::new("grouped", size), &data, |b, d| {
            b.iter(|| detectar_solapamientos_grouped(d))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_propiedad_scaling,
    bench_multi_propiedad_scaling
);
criterion_main!(benches);
