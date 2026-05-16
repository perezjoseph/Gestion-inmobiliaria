use std::collections::HashMap;
use uuid::Uuid;

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

// ─── Current Implementation ───────────────────────────────────────────────────

/// Original: single-pass HashMap accumulation with String month keys.
pub fn current(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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

// ─── Approach A: Numeric month key ───────────────────────────────────────────
//
// Avoids repeated String allocation for month keys by using (i32, u32) tuple
// as the inner HashMap key. Only converts to String at the end.

pub fn approach_numeric_key(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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
            let result = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

use chrono::Datelike;

// ─── Approach B: Pre-sized + numeric key ─────────────────────────────────────
//
// Same as A but pre-allocates HashMaps with expected capacity based on
// production sizes (~50 propiedades, ~24 months).

pub fn approach_presized_numeric(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> = HashMap::with_capacity(50);

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let fecha = pago.fecha_pago.unwrap();
        let key = (fecha.year(), fecha.month());
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_insert_with(|| HashMap::with_capacity(24))
            .entry(key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<((i32, u32), f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

// ─── Approach C: Sort-then-scan ──────────────────────────────────────────────
//
// Instead of hashing, sort pagos by (propiedad_id, year, month) then do a
// linear scan to accumulate. Avoids inner HashMap entirely.
// Trade-off: requires a sorted copy of the filtered data.

pub fn approach_sort_scan(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Filter first, collect indices or references
    let mut filtered: Vec<&Pago> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .collect();

    if filtered.is_empty() {
        return HashMap::new();
    }

    // Sort by propiedad_id then by date
    filtered.sort_unstable_by(|a, b| {
        a.propiedad_id
            .cmp(&b.propiedad_id)
            .then_with(|| a.fecha_pago.cmp(&b.fecha_pago))
    });

    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50);
    let mut current_prop = filtered[0].propiedad_id;
    let first_fecha = filtered[0].fecha_pago.unwrap();
    let mut current_month = (first_fecha.year(), first_fecha.month());
    let mut current_sum = 0.0;
    let mut months_vec: Vec<(String, f64)> = Vec::with_capacity(24);

    for pago in &filtered {
        let fecha = pago.fecha_pago.unwrap();
        let month = (fecha.year(), fecha.month());

        if pago.propiedad_id != current_prop {
            // Flush current month
            months_vec.push((
                format!("{:04}-{:02}", current_month.0, current_month.1),
                current_sum,
            ));
            // Flush current propiedad
            result.insert(current_prop, months_vec);
            months_vec = Vec::with_capacity(24);
            current_prop = pago.propiedad_id;
            current_month = month;
            current_sum = pago.monto;
        } else if month != current_month {
            // Flush current month
            months_vec.push((
                format!("{:04}-{:02}", current_month.0, current_month.1),
                current_sum,
            ));
            current_month = month;
            current_sum = pago.monto;
        } else {
            current_sum += pago.monto;
        }
    }

    // Flush last group
    months_vec.push((
        format!("{:04}-{:02}", current_month.0, current_month.1),
        current_sum,
    ));
    result.insert(current_prop, months_vec);

    result
}
