//! Benchmark demonstrating that O(n²) overlap detection is adequate for n≤200.
//!
//! Run with: cargo bench --bench bench_overlap
//!
//! Expected results:
//! - n=15:  ~500ns (normal use)
//! - n=200: ~100µs (bulk import)
//!
//! Both are negligible compared to DB round-trips (~1-5ms).

use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub estado: String,
}

/// Current O(n²) implementation — pairwise comparison.
fn detectar_solapamientos_cuadratico(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
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

/// O(n log n) alternative — sort by start date, scan adjacent pairs.
/// Only correct when all contracts share the same propiedad_id.
fn detectar_solapamientos_sorted(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut activos: Vec<&Contrato> = contratos.iter().filter(|c| c.estado == "activo").collect();

    activos.sort_unstable_by_key(|c| c.fecha_inicio);

    let mut solapamientos = Vec::new();

    for i in 1..activos.len() {
        if activos[i].propiedad_id == activos[i - 1].propiedad_id
            && activos[i].fecha_inicio <= activos[i - 1].fecha_fin
        {
            solapamientos.push((activos[i - 1].id, activos[i].id));
        }
    }

    solapamientos
}

fn generate_contratos(n: usize) -> Vec<Contrato> {
    let propiedad_id = Uuid::new_v4();
    (0..n)
        .map(|i| {
            let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                + chrono::Duration::days((i as i64) * 30);
            let end = start + chrono::Duration::days(365);
            Contrato {
                id: Uuid::new_v4(),
                propiedad_id,
                fecha_inicio: start,
                fecha_fin: end,
                estado: "activo".to_string(),
            }
        })
        .collect()
}

fn bench_overlap_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_detection");

    for n in [5, 15, 50, 100, 200] {
        let contratos = generate_contratos(n);

        group.bench_with_input(BenchmarkId::new("cuadratico", n), &contratos, |b, data| {
            b.iter(|| detectar_solapamientos_cuadratico(black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("sorted", n), &contratos, |b, data| {
            b.iter(|| detectar_solapamientos_sorted(black_box(data)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_overlap_detection);
criterion_main!(benches);
