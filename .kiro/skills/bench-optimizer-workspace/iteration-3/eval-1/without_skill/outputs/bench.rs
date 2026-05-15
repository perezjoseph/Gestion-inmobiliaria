use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use uuid::Uuid;

// ─── Original implementation ───────────────────────────────────────────────────

mod original {
    use super::*;

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

    pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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
}

// ─── Optimized implementation ──────────────────────────────────────────────────

mod optimized {
    use super::*;
    use chrono::Datelike;

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

    pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
        let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> = HashMap::with_capacity(50);

        for pago in pagos.iter() {
            if pago.estado != "pagado" {
                continue;
            }
            let fecha = match pago.fecha_pago {
                Some(f) => f,
                None => continue,
            };

            let key = (fecha.year(), fecha.month());
            *por_propiedad
                .entry(pago.propiedad_id)
                .or_insert_with(|| HashMap::with_capacity(24))
                .entry(key)
                .or_insert(0.0) += pago.monto;
        }

        por_propiedad
            .into_iter()
            .map(|(prop_id, meses)| {
                let mut sorted: Vec<(String, f64)> = meses
                    .into_iter()
                    .map(|((y, m), total)| (format!("{:04}-{:02}", y, m), total))
                    .collect();
                sorted.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                (prop_id, sorted)
            })
            .collect()
    }
}

// ─── Test data generator ───────────────────────────────────────────────────────

fn generate_pagos(num_pagos: usize, num_propiedades: usize) -> Vec<original::Pago> {
    let propiedades: Vec<Uuid> = (0..num_propiedades).map(|_| Uuid::new_v4()).collect();
    let estados = ["pagado", "pendiente", "atrasado"];

    (0..num_pagos)
        .map(|i| {
            let prop_idx = i % num_propiedades;
            let estado_idx = i % 3; // ~67% will be "pagado" (idx 0)
            let month = (i % 24) as u32 + 1;
            let year = 2023 + (i % 2) as i32;
            let fecha_pago = if estado_idx == 0 {
                Some(NaiveDate::from_ymd_opt(year, month.min(12), 15).unwrap())
            } else {
                None
            };

            original::Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: propiedades[prop_idx],
                monto: 1000.0 + (i as f64 * 0.5),
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(year, month.min(12), 1).unwrap(),
                fecha_pago,
                estado: estados[estado_idx].to_string(),
            }
        })
        .collect()
}

fn convert_to_optimized(pagos: &[original::Pago]) -> Vec<optimized::Pago> {
    pagos
        .iter()
        .map(|p| optimized::Pago {
            id: p.id,
            contrato_id: p.contrato_id,
            propiedad_id: p.propiedad_id,
            monto: p.monto,
            moneda: p.moneda.clone(),
            fecha_vencimiento: p.fecha_vencimiento,
            fecha_pago: p.fecha_pago,
            estado: p.estado.clone(),
        })
        .collect()
}

// ─── Benchmarks ────────────────────────────────────────────────────────────────

fn bench_dashboard_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_aggregation");

    // Production-like dataset: ~2000 pagos, ~50 propiedades
    let pagos_orig = generate_pagos(2000, 50);
    let pagos_opt = convert_to_optimized(&pagos_orig);

    group.bench_function("original", |b| {
        b.iter(|| original::ingresos_por_propiedad_mes(black_box(&pagos_orig)))
    });

    group.bench_function("optimized", |b| {
        b.iter(|| optimized::ingresos_por_propiedad_mes(black_box(&pagos_opt)))
    });

    group.finish();
}

criterion_group!(benches, bench_dashboard_aggregation);
criterion_main!(benches);
