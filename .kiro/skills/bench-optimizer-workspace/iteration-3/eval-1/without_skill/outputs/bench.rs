//! Benchmark comparing original vs optimized dashboard aggregation.
//!
//! Run with: cargo bench
//! Requires criterion in [dev-dependencies].

use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use uuid::Uuid;

// ---------- Struct (shared) ----------

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub propiedad_id: Uuid,
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: NaiveDate,
    pub fecha_pago: Option<NaiveDate>,
    pub estado: String,
}

// ---------- Original implementation ----------

fn ingresos_original(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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

// ---------- Optimized implementation ----------

use chrono::Datelike;

fn ingresos_optimized(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<u32, f64>> = HashMap::with_capacity(50);

    for pago in pagos.iter() {
        if pago.estado != "pagado" {
            continue;
        }
        let fecha = match pago.fecha_pago {
            Some(f) => f,
            None => continue,
        };

        let month_key = fecha.year() as u32 * 12 + fecha.month0();

        *por_propiedad
            .entry(pago.propiedad_id)
            .or_insert_with(|| HashMap::with_capacity(24))
            .entry(month_key)
            .or_insert(0.0) += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(u32, f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|(k, v)| {
                    let year = k / 12;
                    let month = k % 12 + 1;
                    (format!("{:04}-{:02}", year, month), v)
                })
                .collect();
            (prop_id, result)
        })
        .collect()
}

// ---------- Test data generator ----------

fn generate_pagos(num_pagos: usize, num_propiedades: usize) -> Vec<Pago> {
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let estados = ["pagado", "pendiente", "atrasado"];

    (0..num_pagos)
        .map(|i| {
            let prop_idx = i % num_propiedades;
            let estado_idx = i % 3;
            let month = (i % 24) as u32 + 1;
            let year = 2023 + (month - 1) / 12;
            let month_in_year = ((month - 1) % 12) + 1;

            let fecha_pago = if estados[estado_idx] == "pagado" {
                Some(NaiveDate::from_ymd_opt(year as i32, month_in_year, 15).unwrap())
            } else {
                None
            };

            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: propiedades[prop_idx],
                monto: 1000.0 + (i as f64 * 0.5),
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(year as i32, month_in_year, 1).unwrap(),
                fecha_pago,
                estado: estados[estado_idx].to_string(),
            }
        })
        .collect()
}

// ---------- Benchmarks ----------

fn bench_dashboard_aggregation(c: &mut Criterion) {
    let pagos = generate_pagos(2000, 50);

    let mut group = c.benchmark_group("dashboard_aggregation");

    group.bench_function("original", |b| {
        b.iter(|| ingresos_original(black_box(&pagos)))
    });

    group.bench_function("optimized", |b| {
        b.iter(|| ingresos_optimized(black_box(&pagos)))
    });

    group.finish();
}

criterion_group!(benches, bench_dashboard_aggregation);
criterion_main!(benches);
