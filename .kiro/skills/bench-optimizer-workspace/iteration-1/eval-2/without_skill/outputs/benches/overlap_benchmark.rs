use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use overlap_bench::{
    detectar_solapamientos_optimized, detectar_solapamientos_original,
    detectar_solapamientos_sort_only, Contrato,
};
use rand::Rng;
use uuid::Uuid;

/// Generate a realistic dataset of contracts for benchmarking.
/// - `n`: total number of contracts
/// - `num_propiedades`: how many distinct propiedades to spread across
/// - `active_ratio`: fraction of contracts that are "activo"
fn generate_contratos(n: usize, num_propiedades: usize, active_ratio: f64) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let propiedad_id = propiedades[rng.gen_range(0..num_propiedades)];
            // Random start date in 2024
            let start_day = rng.gen_range(1..=300);
            let duration = rng.gen_range(30..=180); // 1-6 month contracts
            let fecha_inicio = NaiveDate::from_yo_opt(2024, start_day).unwrap();
            let fecha_fin = fecha_inicio + chrono::Duration::days(duration);
            let estado = if rng.gen_bool(active_ratio) {
                "activo"
            } else {
                "vencido"
            };

            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio,
                fecha_fin,
                estado: estado.to_string(),
            }
        })
        .collect()
}

/// Generate contracts all for the same propiedad (worst case for overlap detection)
fn generate_single_propiedad(n: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let propiedad_id = Uuid::new_v4();

    (0..n)
        .map(|_| {
            let start_day = rng.gen_range(1..=300);
            let duration = rng.gen_range(30..=180);
            let fecha_inicio = NaiveDate::from_yo_opt(2024, start_day).unwrap();
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

fn bench_normal_use(c: &mut Criterion) {
    let mut group = c.benchmark_group("normal_use_n5_to_15");

    for n in [5, 10, 15] {
        let contratos = generate_single_propiedad(n);

        group.bench_with_input(
            BenchmarkId::new("original_O(n²)", n),
            &contratos,
            |b, data| {
                b.iter(|| detectar_solapamientos_original(black_box(data)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_sort+scan", n),
            &contratos,
            |b, data| {
                b.iter(|| detectar_solapamientos_optimized(black_box(data)));
            },
        );

        group.bench_with_input(BenchmarkId::new("sort_only", n), &contratos, |b, data| {
            b.iter(|| detectar_solapamientos_sort_only(black_box(data)));
        });
    }

    group.finish();
}

fn bench_bulk_import(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_import_n200");

    // Scenario 1: 200 contracts, single propiedad (worst case)
    let contratos_single = generate_single_propiedad(200);

    group.bench_function("original_O(n²)_single_prop", |b| {
        b.iter(|| detectar_solapamientos_original(black_box(&contratos_single)));
    });

    group.bench_function("optimized_sort+scan_single_prop", |b| {
        b.iter(|| detectar_solapamientos_optimized(black_box(&contratos_single)));
    });

    group.bench_function("sort_only_single_prop", |b| {
        b.iter(|| detectar_solapamientos_sort_only(black_box(&contratos_single)));
    });

    // Scenario 2: 200 contracts spread across 20 propiedades (realistic bulk import)
    let contratos_multi = generate_contratos(200, 20, 0.8);

    group.bench_function("original_O(n²)_multi_prop", |b| {
        b.iter(|| detectar_solapamientos_original(black_box(&contratos_multi)));
    });

    group.bench_function("optimized_sort+scan_multi_prop", |b| {
        b.iter(|| detectar_solapamientos_optimized(black_box(&contratos_multi)));
    });

    group.bench_function("sort_only_multi_prop", |b| {
        b.iter(|| detectar_solapamientos_sort_only(black_box(&contratos_multi)));
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for n in [10, 50, 100, 200, 500] {
        let contratos = generate_single_propiedad(n);

        group.bench_with_input(
            BenchmarkId::new("original_O(n²)", n),
            &contratos,
            |b, data| {
                b.iter(|| detectar_solapamientos_original(black_box(data)));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("optimized_sort+scan", n),
            &contratos,
            |b, data| {
                b.iter(|| detectar_solapamientos_optimized(black_box(data)));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_normal_use, bench_bulk_import, bench_scaling);
criterion_main!(benches);
