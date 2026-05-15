//! Benchmark: Overlap detection approaches
//!
//! Compares three implementations:
//! 1. Original O(n²) pairwise comparison
//! 2. Sort+scan with HashMap grouping by propiedad
//! 3. Sort+partition (single sort, no HashMap)
//!
//! Tests at production-representative sizes:
//! - n=10 (typical single-property validation)
//! - n=50 (moderate load)
//! - n=200 (bulk import worst case)
//!
//! Run with: cargo bench --bench bench_overlap --release

use chrono::NaiveDate;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub estado: String,
}

// =============================================================================
// Implementations under test
// =============================================================================

fn detectar_solapamientos_original(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut solapamientos = Vec::new();

    for i in 0..contratos.len() {
        for j in (i + 1)..contratos.len() {
            let a = &contratos[i];
            let b = &contratos[j];

            if a.propiedad_id == b.propiedad_id
                && a.estado == "activo"
                && b.estado == "activo"
                && a.fecha_inicio <= b.fecha_fin
                && b.fecha_inicio <= a.fecha_fin
            {
                solapamientos.push((a.id, b.id));
            }
        }
    }

    solapamientos
}

fn detectar_solapamientos_sort_scan(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, grupo) in &mut por_propiedad {
        if grupo.len() < 2 {
            continue;
        }

        grupo.sort_unstable_by_key(|c| c.fecha_inicio);

        for i in 0..grupo.len() {
            for j in (i + 1)..grupo.len() {
                if grupo[j].fecha_inicio > grupo[i].fecha_fin {
                    break;
                }
                solapamientos.push((grupo[i].id, grupo[j].id));
            }
        }
    }

    solapamientos
}

fn detectar_solapamientos_sort_partition(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut activos: Vec<&Contrato> = contratos
        .iter()
        .filter(|c| c.estado == "activo")
        .collect();

    if activos.len() < 2 {
        return Vec::new();
    }

    activos.sort_unstable_by(|a, b| {
        a.propiedad_id
            .cmp(&b.propiedad_id)
            .then(a.fecha_inicio.cmp(&b.fecha_inicio))
    });

    let mut solapamientos = Vec::new();

    let mut i = 0;
    while i < activos.len() {
        let propiedad_id = activos[i].propiedad_id;
        let mut j = i + 1;
        while j < activos.len() && activos[j].propiedad_id == propiedad_id {
            j += 1;
        }

        for a in i..j {
            for b in (a + 1)..j {
                if activos[b].fecha_inicio > activos[a].fecha_fin {
                    break;
                }
                solapamientos.push((activos[a].id, activos[b].id));
            }
        }

        i = j;
    }

    solapamientos
}

// =============================================================================
// Data generation
// =============================================================================

/// Generate realistic contract data for benchmarking.
///
/// Distribution matches production:
/// - Multiple propiedades (n/5 properties, so ~3-5 contracts per property)
/// - 70% of contracts are "activo", 20% "vencido", 10% "cancelado"
/// - Date ranges span 12 months, with ~30% overlap probability among active ones
/// - Contract durations: 3-12 months
fn generate_contratos(n: usize) -> Vec<Contrato> {
    let mut rng = rand::thread_rng();
    let num_propiedades = (n / 5).max(1);
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let propiedad_id = propiedades[rng.gen_range(0..propiedades.len())];

            // Random start date within 2024
            let start_day = rng.gen_range(1..=330);
            let fecha_inicio = NaiveDate::from_yo_opt(2024, start_day).unwrap();

            // Duration: 90-365 days
            let duration_days = rng.gen_range(90..=365);
            let fecha_fin = fecha_inicio + chrono::Duration::days(duration_days);

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

/// Generate worst-case data: all contracts are active, same propiedad, all overlapping.
/// This stresses the O(n²) behavior maximally.
fn generate_worst_case(n: usize) -> Vec<Contrato> {
    let propiedad_id = Uuid::new_v4();
    let mut rng = rand::thread_rng();

    (0..n)
        .map(|_| {
            let start_day = rng.gen_range(1..=100);
            let fecha_inicio = NaiveDate::from_yo_opt(2024, start_day).unwrap();
            // All contracts span the entire year — guaranteed overlap
            let fecha_fin = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

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

// =============================================================================
// Benchmarks
// =============================================================================

fn bench_realistic_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_realistic");

    for size in [10, 15, 50, 100, 200] {
        let data = generate_contratos(size);

        group.bench_with_input(
            BenchmarkId::new("original_n2", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_original(d)),
        );
        group.bench_with_input(
            BenchmarkId::new("sort_scan", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_sort_scan(d)),
        );
        group.bench_with_input(
            BenchmarkId::new("sort_partition", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_sort_partition(d)),
        );
    }

    group.finish();
}

fn bench_worst_case_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_worst_case");

    for size in [10, 15, 50, 100, 200] {
        let data = generate_worst_case(size);

        group.bench_with_input(
            BenchmarkId::new("original_n2", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_original(d)),
        );
        group.bench_with_input(
            BenchmarkId::new("sort_scan", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_sort_scan(d)),
        );
        group.bench_with_input(
            BenchmarkId::new("sort_partition", size),
            &data,
            |b, d| b.iter(|| detectar_solapamientos_sort_partition(d)),
        );
    }

    group.finish();
}

fn bench_typical_usage(c: &mut Criterion) {
    // Benchmark at the typical production size (5-15 contracts)
    let data = generate_contratos(12);

    let mut group = c.benchmark_group("overlap_typical_n12");
    group.bench_function("original_n2", |b| {
        b.iter(|| detectar_solapamientos_original(&data))
    });
    group.bench_function("sort_scan", |b| {
        b.iter(|| detectar_solapamientos_sort_scan(&data))
    });
    group.bench_function("sort_partition", |b| {
        b.iter(|| detectar_solapamientos_sort_partition(&data))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_typical_usage,
    bench_realistic_scaling,
    bench_worst_case_scaling
);
criterion_main!(benches);
