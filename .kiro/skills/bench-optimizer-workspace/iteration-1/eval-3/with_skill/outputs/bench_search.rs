//! Criterion benchmark: linear scan vs pre-indexed HashMap for payment search.
//!
//! Production context: ~5000 pagos, ~200 contratos (25 pagos per contract avg).
//! 80% of queries filter by contrato_id. This benchmark measures the common case.
//!
//! Run with: cargo bench --bench bench_search --release
//!
//! Cargo.toml additions:
//! ```toml
//! [dev-dependencies]
//! criterion = { version = "0.5", features = ["html_reports"] }
//! chrono = "0.4"
//! uuid = { version = "1", features = ["v4"] }
//! rand = "0.8"
//!
//! [[bench]]
//! name = "bench_search"
//! harness = false
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Data structures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: f64,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
    pub referencia: Option<String>,
}

// ─── Approach 1: Linear scan (current) ───────────────────────────────────────

fn buscar_pagos_linear<'a>(
    pagos: &'a [Pago],
    contrato_id: Option<Uuid>,
    estado: Option<&str>,
    fecha_desde: Option<chrono::NaiveDate>,
    fecha_hasta: Option<chrono::NaiveDate>,
    referencia: Option<&str>,
) -> Vec<&'a Pago> {
    pagos
        .iter()
        .filter(|p| {
            contrato_id.map_or(true, |id| p.contrato_id == id)
                && estado.map_or(true, |e| p.estado == e)
                && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                && referencia.map_or(true, |r| {
                    p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                })
        })
        .collect()
}

// ─── Approach 2: Pre-indexed HashMap by contrato_id ──────────────────────────

struct PagoIndex<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    all: &'a [Pago],
}

impl<'a> PagoIndex<'a> {
    fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 25);

        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_insert_with(|| Vec::with_capacity(25))
                .push(pago);
        }

        Self {
            by_contrato,
            all: pagos,
        }
    }

    fn buscar(
        &self,
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        match contrato_id {
            Some(id) => match self.by_contrato.get(&id) {
                Some(bucket) => bucket
                    .iter()
                    .filter(|p| {
                        estado.map_or(true, |e| p.estado == e)
                            && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                            && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                            && referencia.map_or(true, |r| {
                                p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                            })
                    })
                    .copied()
                    .collect(),
                None => Vec::new(),
            },
            None => self
                .all
                .iter()
                .filter(|p| {
                    estado.map_or(true, |e| p.estado == e)
                        && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                        && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                        && referencia.map_or(true, |r| {
                            p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                        })
                })
                .collect(),
        }
    }
}

// ─── Approach 3: Pre-indexed + sorted by date (binary search on date range) ─

struct PagoIndexSorted<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    all_sorted: Vec<&'a Pago>,
}

impl<'a> PagoIndexSorted<'a> {
    fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 25);

        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_insert_with(|| Vec::with_capacity(25))
                .push(pago);
        }

        for bucket in by_contrato.values_mut() {
            bucket.sort_unstable_by_key(|p| p.fecha_vencimiento);
        }

        let mut all_sorted: Vec<&Pago> = pagos.iter().collect();
        all_sorted.sort_unstable_by_key(|p| p.fecha_vencimiento);

        Self {
            by_contrato,
            all_sorted,
        }
    }

    fn buscar(
        &self,
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        let slice: &[&Pago] = match contrato_id {
            Some(id) => match self.by_contrato.get(&id) {
                Some(bucket) => bucket.as_slice(),
                None => return Vec::new(),
            },
            None => self.all_sorted.as_slice(),
        };

        let start = match fecha_desde {
            Some(d) => slice.partition_point(|p| p.fecha_vencimiento < d),
            None => 0,
        };
        let end = match fecha_hasta {
            Some(d) => slice.partition_point(|p| p.fecha_vencimiento <= d),
            None => slice.len(),
        };

        slice[start..end]
            .iter()
            .filter(|p| {
                estado.map_or(true, |e| p.estado == e)
                    && referencia.map_or(true, |r| {
                        p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                    })
            })
            .copied()
            .collect()
    }
}

// ─── Data generation ─────────────────────────────────────────────────────────

/// Generate realistic payment data matching production distribution:
/// - `n` total pagos
/// - ~200 unique contratos (so ~25 pagos per contract at n=5000)
/// - 70% pagado, 20% pendiente, 10% atrasado
/// - Dates spanning 24 months
/// - ~30% have a referencia string
fn generate_pagos(n: usize) -> Vec<Pago> {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let num_contratos = (n / 25).max(1);
    let contrato_ids: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();

    let base_date = chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

    (0..n)
        .map(|_| {
            let contrato_id = contrato_ids[rng.gen_range(0..num_contratos)];
            let days_offset = rng.gen_range(0..730); // 24 months
            let fecha_vencimiento = base_date + chrono::Duration::days(days_offset);

            let estado = match rng.gen_range(0u8..10) {
                0 => "atrasado".to_string(),
                1..=2 => "pendiente".to_string(),
                _ => "pagado".to_string(),
            };

            let fecha_pago = if estado == "pagado" {
                Some(fecha_vencimiento - chrono::Duration::days(rng.gen_range(0..5)))
            } else {
                None
            };

            let referencia = if rng.gen_bool(0.3) {
                Some(format!("REF-{:06}", rng.gen_range(0u32..999999)))
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id,
                monto: rng.gen_range(5000.0..50000.0),
                fecha_vencimiento,
                fecha_pago,
                estado,
                referencia,
            }
        })
        .collect()
}

// ─── Benchmarks ──────────────────────────────────────────────────────────────

/// Benchmark the most common query pattern: filter by contrato_id (80% of queries).
fn bench_by_contrato_id(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_by_contrato_id");

    let pagos = generate_pagos(5000);
    // Pick a contrato_id that exists in the dataset
    let target_contrato = pagos[0].contrato_id;

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    let index = PagoIndex::new(&pagos);
    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    let index_sorted = PagoIndexSorted::new(&pagos);
    group.bench_function("hashmap_sorted_index", |b| {
        b.iter(|| {
            index_sorted.buscar(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark combined filter: contrato_id + date range (common pagination pattern).
fn bench_contrato_and_date_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_contrato_and_date_range");

    let pagos = generate_pagos(5000);
    let target_contrato = pagos[0].contrato_id;
    let fecha_desde = chrono::NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
    let fecha_hasta = chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(Some(fecha_desde)),
                black_box(Some(fecha_hasta)),
                black_box(None),
            )
        })
    });

    let index = PagoIndex::new(&pagos);
    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(Some(fecha_desde)),
                black_box(Some(fecha_hasta)),
                black_box(None),
            )
        })
    });

    let index_sorted = PagoIndexSorted::new(&pagos);
    group.bench_function("hashmap_sorted_index", |b| {
        b.iter(|| {
            index_sorted.buscar(
                black_box(Some(target_contrato)),
                black_box(None),
                black_box(Some(fecha_desde)),
                black_box(Some(fecha_hasta)),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark no-filter query (worst case for index — must scan everything).
fn bench_no_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_no_filter");

    let pagos = generate_pagos(5000);

    group.bench_function("linear_scan", |b| {
        b.iter(|| {
            buscar_pagos_linear(
                black_box(&pagos),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    let index = PagoIndex::new(&pagos);
    group.bench_function("hashmap_index", |b| {
        b.iter(|| {
            index.buscar(
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    let index_sorted = PagoIndexSorted::new(&pagos);
    group.bench_function("hashmap_sorted_index", |b| {
        b.iter(|| {
            index_sorted.buscar(
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
                black_box(None),
            )
        })
    });

    group.finish();
}

/// Benchmark scaling behavior across dataset sizes.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_scaling");

    for size in [100, 500, 1000, 5000] {
        let pagos = generate_pagos(size);
        let target_contrato = pagos[0].contrato_id;

        group.bench_with_input(BenchmarkId::new("linear_scan", size), &pagos, |b, pagos| {
            b.iter(|| {
                buscar_pagos_linear(
                    black_box(pagos),
                    black_box(Some(target_contrato)),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                    black_box(None),
                )
            })
        });

        let index = PagoIndex::new(&pagos);
        group.bench_with_input(
            BenchmarkId::new("hashmap_index", size),
            &index,
            |b, index| {
                b.iter(|| {
                    index.buscar(
                        black_box(Some(target_contrato)),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                        black_box(None),
                    )
                })
            },
        );
    }

    group.finish();
}

/// Benchmark index build cost (amortized over many queries).
fn bench_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_build_cost");

    let pagos = generate_pagos(5000);

    group.bench_function("hashmap_index_build", |b| {
        b.iter(|| PagoIndex::new(black_box(&pagos)))
    });

    group.bench_function("hashmap_sorted_index_build", |b| {
        b.iter(|| PagoIndexSorted::new(black_box(&pagos)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_by_contrato_id,
    bench_contrato_and_date_range,
    bench_no_filter,
    bench_scaling,
    bench_index_build,
);
criterion_main!(benches);
