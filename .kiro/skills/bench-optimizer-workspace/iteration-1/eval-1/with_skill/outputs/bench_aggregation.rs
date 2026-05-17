use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;

// ─── Domain types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub propiedad_id: Uuid,
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
}

// ─── Current implementation (baseline) ──────────────────────────────────────

pub fn ingresos_current(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<String, f64>> = HashMap::new();

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let mes = pago.fecha_pago.unwrap().format("%Y-%m").to_string();
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(mes)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(String, f64)> = meses.into_iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            (prop_id, sorted)
        })
        .collect()
}

// ─── Alternative 1: Pre-allocated + avoid format! per iteration ─────────────
// Uses a (year, month) tuple key instead of formatting a String each iteration,
// then converts to String only at the end.

pub fn ingresos_numeric_key(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> = HashMap::new();

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let fecha = pago.fecha_pago.unwrap();
        let key = (fecha.year(), fecha.month());
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<((i32, u32), f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

// ─── Alternative 2: Sort-then-scan (avoid inner HashMap entirely) ───────────
// Pre-filter, sort by (propiedad_id, year, month), then linear scan to accumulate.

pub fn ingresos_sort_scan(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;

    // Collect only paid pagos with their numeric month key
    let mut filtered: Vec<(Uuid, i32, u32, f64)> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .map(|p| {
            let fecha = p.fecha_pago.unwrap();
            (p.propiedad_id, fecha.year(), fecha.month(), p.monto)
        })
        .collect();

    // Sort by propiedad, then year, then month
    filtered.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50); // ~50 propiedades

    let mut i = 0;
    while i < filtered.len() {
        let (prop_id, year, month, _) = filtered[i];
        let mut total = 0.0;
        let mut j = i;
        // Accumulate all entries with same (propiedad, year, month)
        while j < filtered.len()
            && filtered[j].0 == prop_id
            && filtered[j].1 == year
            && filtered[j].2 == month
        {
            total += filtered[j].3;
            j += 1;
        }
        result
            .entry(prop_id)
            .or_default()
            .push((format!("{year:04}-{month:02}"), total));
        i = j;
    }

    result
}

// ─── Alternative 3: Pre-allocated HashMap + with_capacity ───────────────────
// Same algorithm as current but with capacity hints and Datelike instead of format.

pub fn ingresos_preallocated(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;

    let estimated_props = 50;
    let estimated_months = 24;

    let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> =
        HashMap::with_capacity(estimated_props);

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let fecha = pago.fecha_pago.unwrap();
        let key = (fecha.year(), fecha.month());
        let inner = por_propiedad
            .entry(pago.propiedad_id)
            .or_insert_with(|| HashMap::with_capacity(estimated_months));
        *inner.entry(key).or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<((i32, u32), f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

// ─── Data generation ────────────────────────────────────────────────────────

/// Generate realistic pagos matching production distribution:
/// - ~50 propiedades
/// - 70% "pagado", 20% "pendiente", 10% "atrasado"
/// - Dates span 24 months (2023-01 to 2024-12)
fn generate_realistic_pagos(n: usize) -> Vec<Pago> {
    use chrono::NaiveDate;

    let mut rng = rand::thread_rng();
    let num_propiedades = 50;
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let contratos: Vec<Uuid> = (0..200).map(|_| Uuid::new_v4()).collect();

    (0..n)
        .map(|_| {
            let estado = match rng.gen_range(0u8..10) {
                0 => "atrasado",
                1..=2 => "pendiente",
                _ => "pagado",
            };

            let year = rng.gen_range(2023..=2024);
            let month = rng.gen_range(1..=12);
            let day = rng.gen_range(1..=28);
            let fecha_vencimiento = NaiveDate::from_ymd_opt(year, month, day).unwrap();

            let fecha_pago = if estado == "pagado" {
                // Paid within a few days of due date
                Some(fecha_vencimiento + chrono::Duration::days(rng.gen_range(0..=5)))
            } else if estado == "atrasado" {
                // Some atrasado have late payment, some have none
                if rng.gen_bool(0.5) {
                    Some(fecha_vencimiento + chrono::Duration::days(rng.gen_range(10..=60)))
                } else {
                    None
                }
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

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_comparison(c: &mut Criterion) {
    // Production size: ~2000 pagos, ~50 propiedades, 24 months
    let pagos = generate_realistic_pagos(2000);

    let mut group = c.benchmark_group("dashboard_aggregation");

    group.bench_function("current_hashmap_format", |b| {
        b.iter(|| ingresos_current(criterion::black_box(&pagos)))
    });

    group.bench_function("numeric_key_then_format", |b| {
        b.iter(|| ingresos_numeric_key(criterion::black_box(&pagos)))
    });

    group.bench_function("sort_scan", |b| {
        b.iter(|| ingresos_sort_scan(criterion::black_box(&pagos)))
    });

    group.bench_function("preallocated_numeric_key", |b| {
        b.iter(|| ingresos_preallocated(criterion::black_box(&pagos)))
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_scaling");

    for size in [500, 1000, 2000, 5000] {
        let pagos = generate_realistic_pagos(size);

        group.bench_with_input(BenchmarkId::new("current", size), &pagos, |b, data| {
            b.iter(|| ingresos_current(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("numeric_key", size), &pagos, |b, data| {
            b.iter(|| ingresos_numeric_key(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("sort_scan", size), &pagos, |b, data| {
            b.iter(|| ingresos_sort_scan(criterion::black_box(data)))
        });

        group.bench_with_input(BenchmarkId::new("preallocated", size), &pagos, |b, data| {
            b.iter(|| ingresos_preallocated(criterion::black_box(data)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_comparison, bench_scaling);
criterion_main!(benches);
