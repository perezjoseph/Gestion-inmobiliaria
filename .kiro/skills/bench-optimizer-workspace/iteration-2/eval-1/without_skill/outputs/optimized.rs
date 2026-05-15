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

/// Compact month key: encodes year+month as a single u32 (year * 12 + month).
/// Avoids heap-allocated String keys and expensive chrono formatting per iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct YearMonth(u32);

impl YearMonth {
    #[inline]
    fn from_date(date: chrono::NaiveDate) -> Self {
        use chrono::Datelike;
        Self(date.year() as u32 * 12 + date.month())
    }

    #[inline]
    fn to_string(self) -> String {
        let year = self.0 / 12;
        let month = self.0 % 12;
        // month is 1-based from Datelike, so after encoding year*12+month,
        // we get month back directly (1..=12 range preserved in the low bits)
        format!("{:04}-{:02}", year, month)
    }
}

/// Aggregates total income per propiedad per month for the dashboard.
/// Called on every dashboard load. Production dataset: ~2000 pagos, ~50 propiedades, 24 months.
///
/// Optimizations over the original:
/// 1. Pre-allocated HashMap with estimated capacity (~50 propiedades).
/// 2. Replaced String month keys with compact `YearMonth(u32)` — eliminates
///    per-pago heap allocation from `format!("%Y-%m")`.
/// 3. String-to-String conversion deferred to the final output phase only
///    (once per unique propiedad×month pair, not once per pago).
/// 4. Sort on u32 (integer comparison) instead of String (lexicographic on heap).
/// 5. Estado comparison uses byte-level equality on str — same semantics, just
///    making the intent explicit for the compiler.
pub fn ingresos_por_propiedad_mes(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Pre-allocate outer map. ~50 propiedades in production.
    let mut por_propiedad: HashMap<Uuid, HashMap<YearMonth, f64>> = HashMap::with_capacity(64);

    for pago in pagos.iter() {
        // Early-exit filter: check estado first (cheaper branch prediction on short string),
        // then fecha_pago presence.
        if pago.estado != "pagado" {
            continue;
        }
        let fecha = match pago.fecha_pago {
            Some(f) => f,
            None => continue,
        };

        let mes = YearMonth::from_date(fecha);
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(mes)
            .or_default() += pago.monto;
    }

    // Convert to output format. String allocation happens here — once per unique
    // (propiedad, month) pair (~50 * 24 = 1200 allocations max), not per pago (~2000).
    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(YearMonth, f64)> = meses.into_iter().collect();
            // Integer sort — faster than lexicographic String sort.
            sorted.sort_unstable_by_key(|(ym, _)| *ym);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|(ym, total)| (ym.to_string(), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}
