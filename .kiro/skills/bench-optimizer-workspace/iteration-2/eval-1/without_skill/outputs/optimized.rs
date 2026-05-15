use std::collections::HashMap;

use chrono::Datelike;
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

/// Compact month key: encodes year+month as a u32 (year * 12 + month).
/// Avoids heap-allocated String formatting on every iteration.
#[inline]
fn month_key(date: chrono::NaiveDate) -> u32 {
    let y = date.year() as u32;
    let m = date.month();
    y * 12 + m
}

/// Converts a compact month key back to "YYYY-MM" string for the output.
#[inline]
fn month_key_to_string(key: u32) -> String {
    let year = key / 12;
    let month = key % 12;
    format!("{:04}-{:02}", year, month)
}

/// Aggregates total income per propiedad per month for the dashboard.
/// Called on every dashboard load. Production dataset: ~2000 pagos, ~50 propiedades, 24 months.
///
/// Optimizations applied:
/// 1. Replaced per-pago String formatting with integer month key (avoids ~2000 allocations).
/// 2. Pre-allocated outer HashMap with expected capacity (~50 propiedades).
/// 3. Sort by integer key (cheaper than string comparison).
/// 4. Convert to String only once per unique month in the final output phase.
/// 5. Used `if let` instead of filter+unwrap to avoid double option check.
pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Pre-allocate for ~50 propiedades
    let mut por_propiedad: HashMap<Uuid, HashMap<u32, f64>> = HashMap::with_capacity(64);

    for pago in pagos {
        if pago.estado != "pagado" {
            continue;
        }
        let Some(fecha) = pago.fecha_pago else {
            continue;
        };

        let key = month_key(fecha);
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(u32, f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|(k, v)| (month_key_to_string(k), v))
                .collect();
            (prop_id, result)
        })
        .collect()
}
