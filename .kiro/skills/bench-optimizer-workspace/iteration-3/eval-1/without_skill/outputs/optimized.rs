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
/// 1. Uses a compact (i32, u32) tuple as the month key instead of formatting a
///    String on every iteration. This eliminates ~2000 heap allocations per call.
/// 2. Pre-allocates HashMap capacity based on known dataset characteristics
///    (~50 propiedades, ~24 months).
/// 3. Converts to the final String key only once per unique month at the end,
///    reducing total format calls from ~2000 to ~50*24 = 1200 worst case (typically
///    far fewer since not every property has data for every month).
/// 4. Uses `if let` instead of filter + unwrap for cleaner control flow.
pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Use a numeric month key to avoid per-pago string allocation.
    // Key: (year, month) — compact, hashable, no heap allocation.
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

// Re-export chrono traits needed for year()/month()
use chrono::Datelike;
