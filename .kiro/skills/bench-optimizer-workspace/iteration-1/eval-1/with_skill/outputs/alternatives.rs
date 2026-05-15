//! Alternative implementations for `ingresos_por_propiedad_mes`.
//!
//! Each approach targets a different bottleneck hypothesis:
//! - `ingresos_numeric_key`: eliminates per-pago String allocation from `format!("%Y-%m")`
//! - `ingresos_sort_scan`: eliminates the inner HashMap entirely; uses sort + linear scan
//! - `ingresos_preallocated`: same as numeric_key but with capacity hints to reduce rehashing
//!
//! All produce identical output to the original function.

use chrono::Datelike;
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

// ─── Original (baseline) ────────────────────────────────────────────────────

/// Current implementation. Single-pass HashMap accumulation with String key per month.
/// Bottleneck hypothesis: `format!("%Y-%m")` allocates a new String for every single pago,
/// even though there are only ~24 distinct months. This is O(n) allocations where O(1)
/// would suffice if we used a numeric key.
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

// ─── Alternative 1: Numeric key (avoid per-pago String allocation) ──────────

/// Replaces the `format!("%Y-%m")` String key with a `(i32, u32)` tuple.
/// String formatting happens only once per unique (propiedad, month) pair at the end,
/// reducing allocations from ~1400 (70% of 2000 pagos) to ~1200 (50 props × 24 months).
///
/// Expected improvement: moderate (20-40%) — eliminates the main allocation hotspot
/// in the inner loop while keeping the same algorithmic structure.
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

// ─── Alternative 2: Sort-then-scan (no inner HashMap) ───────────────────────

/// Eliminates the inner HashMap entirely. Instead:
/// 1. Filter to paid pagos, extract (propiedad_id, year, month, monto)
/// 2. Sort by (propiedad_id, year, month)
/// 3. Linear scan to accumulate totals for each group
///
/// Trade-off: O(n log n) sort vs O(n) hash insertions. For n=1400 (paid pagos),
/// the sort approach may win due to better cache locality and zero hash overhead.
/// The sort is on a compact tuple (Uuid, i32, u32, f64) = 40 bytes, which fits
/// nicely in cache lines.
///
/// Expected improvement: competitive with numeric_key. May win at larger sizes
/// due to sequential memory access pattern.
pub fn ingresos_sort_scan(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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

    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50);

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

// ─── Alternative 3: Pre-allocated + numeric key ─────────────────────────────

/// Same algorithm as `ingresos_numeric_key` but with capacity hints on both
/// the outer HashMap (~50 propiedades) and inner HashMaps (~24 months).
/// This eliminates rehashing during growth.
///
/// Expected improvement: small incremental gain over numeric_key (~5-10%)
/// from avoiding reallocation. The capacity hints are based on known production
/// data characteristics (50 propiedades, 24 months).
pub fn ingresos_preallocated(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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

// ─── Correctness verification ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_pago(propiedad_id: Uuid, monto: f64, year: i32, month: u32, estado: &str) -> Pago {
        let fecha = NaiveDate::from_ymd_opt(year, month, 15).unwrap();
        Pago {
            id: Uuid::new_v4(),
            contrato_id: Uuid::new_v4(),
            propiedad_id,
            monto,
            moneda: "DOP".to_string(),
            fecha_vencimiento: fecha,
            fecha_pago: if estado == "pagado" {
                Some(fecha)
            } else {
                None
            },
            estado: estado.to_string(),
        }
    }

    #[test]
    fn all_implementations_produce_same_output() {
        let prop_a = Uuid::new_v4();
        let prop_b = Uuid::new_v4();

        let pagos = vec![
            make_pago(prop_a, 1000.0, 2024, 1, "pagado"),
            make_pago(prop_a, 2000.0, 2024, 1, "pagado"),
            make_pago(prop_a, 500.0, 2024, 2, "pagado"),
            make_pago(prop_b, 3000.0, 2024, 3, "pagado"),
            make_pago(prop_a, 999.0, 2024, 1, "pendiente"), // filtered out
            make_pago(prop_b, 888.0, 2024, 3, "atrasado"),  // filtered out
        ];

        let baseline = ingresos_current(&pagos);
        let alt1 = ingresos_numeric_key(&pagos);
        let alt2 = ingresos_sort_scan(&pagos);
        let alt3 = ingresos_preallocated(&pagos);

        // Verify baseline correctness
        assert_eq!(baseline[&prop_a].len(), 2);
        assert_eq!(baseline[&prop_a][0], ("2024-01".to_string(), 3000.0));
        assert_eq!(baseline[&prop_a][1], ("2024-02".to_string(), 500.0));
        assert_eq!(baseline[&prop_b].len(), 1);
        assert_eq!(baseline[&prop_b][0], ("2024-03".to_string(), 3000.0));

        // All alternatives must match baseline
        assert_eq!(baseline, alt1, "numeric_key differs from baseline");
        assert_eq!(baseline, alt2, "sort_scan differs from baseline");
        assert_eq!(baseline, alt3, "preallocated differs from baseline");
    }

    #[test]
    fn empty_input_returns_empty() {
        let pagos: Vec<Pago> = vec![];
        assert!(ingresos_current(&pagos).is_empty());
        assert!(ingresos_numeric_key(&pagos).is_empty());
        assert!(ingresos_sort_scan(&pagos).is_empty());
        assert!(ingresos_preallocated(&pagos).is_empty());
    }

    #[test]
    fn all_filtered_out_returns_empty() {
        let prop = Uuid::new_v4();
        let pagos = vec![
            make_pago(prop, 1000.0, 2024, 1, "pendiente"),
            make_pago(prop, 2000.0, 2024, 2, "atrasado"),
        ];
        assert!(ingresos_current(&pagos).is_empty());
        assert!(ingresos_numeric_key(&pagos).is_empty());
        assert!(ingresos_sort_scan(&pagos).is_empty());
        assert!(ingresos_preallocated(&pagos).is_empty());
    }
}
