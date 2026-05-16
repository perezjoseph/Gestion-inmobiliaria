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

/// Optimized dashboard aggregation: total income per propiedad per month.
///
/// Key optimizations vs. the original:
/// 1. Numeric month key (u32 = year*12 + month) instead of formatted String — eliminates
///    one heap allocation per pago in the hot loop.
/// 2. Pre-sized outer HashMap (capacity 50 propiedades) to reduce rehashing.
/// 3. Converts numeric keys back to "YYYY-MM" strings only once during the final
///    collect+sort phase (50 propiedades × 24 months = 1200 conversions vs. 2000 in the loop).
/// 4. Sorts by numeric key (u32 cmp) which is cheaper than string comparison, then maps to string.
///
/// Semantics are identical to the original: filters to estado=="pagado" with fecha_pago present,
/// groups by propiedad_id, sums monto per month, returns sorted by month ascending.
pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Use a numeric key for months to avoid per-iteration String allocations.
    // Key encoding: year * 12 + (month - 1), which preserves chronological ordering.
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

// Need chrono's Datelike trait for .year() and .month0()
use chrono::Datelike;
