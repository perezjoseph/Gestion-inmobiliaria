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

// ─── Original Implementation ───────────────────────────────────────────────────
/// Original: HashMap accumulation with String formatting per item and sort at end.
pub fn ingresos_original(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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

// ─── Optimized V1: Numeric month key instead of String ─────────────────────────
/// Avoids String allocation for month keys by using (i32, u32) tuple as key.
/// Formats to String only at the end when building the result.
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

// ─── Optimized V2: Pre-sized + numeric key ─────────────────────────────────────
/// Same as V1 but pre-sizes the outer HashMap based on estimated propiedad count.
/// Also uses with_capacity on inner maps.
pub fn ingresos_presized(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Estimate: ~50 propiedades, ~24 months per propiedad
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
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_insert_with(|| HashMap::with_capacity(estimated_months))
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

// ─── Optimized V3: Sort-first then linear scan ─────────────────────────────────
/// Sorts pagos by (propiedad_id, fecha_pago) first, then does a single linear
/// scan to accumulate. Avoids HashMap overhead entirely for the inner grouping.
pub fn ingresos_sort_scan(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Filter first
    let mut filtered: Vec<&Pago> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .collect();

    if filtered.is_empty() {
        return HashMap::new();
    }

    // Sort by propiedad_id then by fecha_pago
    filtered.sort_unstable_by(|a, b| {
        a.propiedad_id
            .cmp(&b.propiedad_id)
            .then_with(|| a.fecha_pago.cmp(&b.fecha_pago))
    });

    let mut result: HashMap<Uuid, Vec<(String, f64)>> =
        HashMap::with_capacity(50);

    let mut current_prop = filtered[0].propiedad_id;
    let mut current_month = {
        let f = filtered[0].fecha_pago.unwrap();
        (f.year(), f.month())
    };
    let mut current_sum = 0.0_f64;
    let mut months_vec: Vec<(String, f64)> = Vec::with_capacity(24);

    for pago in &filtered {
        let fecha = pago.fecha_pago.unwrap();
        let month = (fecha.year(), fecha.month());

        if pago.propiedad_id != current_prop {
            // Flush current month
            months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
            // Flush current propiedad
            result.insert(current_prop, months_vec);
            months_vec = Vec::with_capacity(24);
            current_prop = pago.propiedad_id;
            current_month = month;
            current_sum = pago.monto;
        } else if month != current_month {
            // Flush current month
            months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
            current_month = month;
            current_sum = pago.monto;
        } else {
            current_sum += pago.monto;
        }
    }

    // Flush last group
    months_vec.push((format!("{:04}-{:02}", current_month.0, current_month.1), current_sum));
    result.insert(current_prop, months_vec);

    result
}

// ─── Helper: use Datelike trait ────────────────────────────────────────────────
use chrono::Datelike;
