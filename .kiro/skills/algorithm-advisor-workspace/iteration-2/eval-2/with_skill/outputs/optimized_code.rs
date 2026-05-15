use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
    pub metodo_pago: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub monto_mensual: f64,
    pub moneda: String,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
}

#[derive(Debug, Clone)]
pub struct Inquilino {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub cedula: String,
}

/// Generates a payment aging report: groups overdue payments by how many days late.
/// Buckets: 1-30 days, 31-60 days, 61-90 days, 90+ days.
/// Called monthly for ~500 overdue payments.
///
/// BTreeMap is intentional: the report needs buckets in sorted order (ascending by days).
/// With only 4 fixed buckets and a single O(n) pass, this is already optimal.
/// No optimization needed.
pub fn reporte_envejecimiento(pagos: &[Pago]) -> BTreeMap<String, Vec<&Pago>> {
    let hoy = chrono::Local::now().date_naive();
    let mut buckets: BTreeMap<String, Vec<&Pago>> = BTreeMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "atrasado") {
        let dias_atraso = (hoy - pago.fecha_vencimiento).num_days();
        let bucket = match dias_atraso {
            1..=30 => "01-30 días",
            31..=60 => "31-60 días",
            61..=90 => "61-90 días",
            _ => "90+ días",
        };
        buckets.entry(bucket.to_string()).or_default().push(pago);
    }

    buckets
}

/// Finds inquilinos who have NEVER paid late across all their contracts.
/// Used for "good tenant" reports. Dataset: ~100 inquilinos, ~200 contratos, ~2000 pagos.
///
/// Optimized from O(inquilinos × contratos × pagos) to O(inquilinos + contratos + pagos)
/// using Space-Time Tradeoff: precompute lookup structures to avoid nested scans.
///
/// Paradigm: Space-Time Tradeoff (precomputed HashSets and HashMap)
/// Before: O(n × m × p) where n=100, m=200, p=2000 → ~40,000,000 comparisons worst case
/// After: O(n + m + p) → ~2,300 operations
pub fn inquilinos_sin_atrasos<'a>(
    inquilinos: &'a [Inquilino],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<&'a Inquilino> {
    // Step 1: Build a set of contrato IDs that have at least one late payment — O(p)
    let contratos_con_atraso: HashSet<Uuid> = pagos
        .iter()
        .filter(|p| p.estado == "atrasado")
        .map(|p| p.contrato_id)
        .collect();

    // Step 2: Build a map from inquilino_id → their contrato IDs — O(m)
    let contratos_por_inquilino: HashMap<Uuid, Vec<Uuid>> =
        contratos.iter().fold(HashMap::new(), |mut map, c| {
            map.entry(c.inquilino_id).or_default().push(c.id);
            map
        });

    // Step 3: For each inquilino, check if any of their contratos had a late payment — O(n × avg_contratos)
    inquilinos
        .iter()
        .filter(|inquilino| {
            contratos_por_inquilino
                .get(&inquilino.id)
                .map(|contrato_ids| {
                    !contrato_ids
                        .iter()
                        .any(|cid| contratos_con_atraso.contains(cid))
                })
                .unwrap_or(true) // No contracts means no late payments
        })
        .collect()
}

/// Calculates running total of payments received per month for a trend chart.
/// Needs to return months in chronological order with cumulative sums.
/// Dataset: ~2000 payments spanning 24 months.
///
/// Optimized from O(months × n) repeated prefix sum to O(n + months log months).
/// Uses Transform and Conquer: group payments by month first (single pass),
/// then sort months and compute a running cumulative sum.
///
/// Paradigm: Transform and Conquer (instance simplification) + Space-Time Tradeoff (grouping)
/// Before: O(months × n) = O(24 × 2000) = ~48,000 comparisons with repeated string formatting
/// After: O(n + m log m) where m = unique months ≈ O(2000 + 24·5) ≈ ~2,120 operations
pub fn tendencia_pagos_acumulados(pagos: &[Pago]) -> Vec<(String, f64)> {
    // Step 1: Group payment amounts by month in a single pass — O(n)
    let mut totales_por_mes: BTreeMap<String, f64> = BTreeMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        if let Some(fecha) = pago.fecha_pago {
            let mes = fecha.format("%Y-%m").to_string();
            *totales_por_mes.entry(mes).or_default() += pago.monto;
        }
    }

    // Step 2: BTreeMap gives us sorted order. Compute running cumulative sum — O(m)
    let mut acumulado = 0.0;
    totales_por_mes
        .into_iter()
        .map(|(mes, total_mes)| {
            acumulado += total_mes;
            (mes, acumulado)
        })
        .collect()
}

/// Detects duplicate payments: same contrato, same fecha_vencimiento, same monto,
/// paid more than once. Returns pairs of duplicate payment IDs.
/// Dataset: ~2000 payments. Duplicates are rare (< 1%).
///
/// Optimized from O(n²) pairwise comparison to O(n) average case using
/// Space-Time Tradeoff: group by composite key, then only compare within groups.
///
/// Paradigm: Space-Time Tradeoff (HashMap grouping by composite key)
/// Before: O(n²) = ~2,000,000 comparisons
/// After: O(n) average — each payment is inserted into a bucket once;
///        pairwise comparison only within small buckets (duplicates are < 1%)
pub fn detectar_pagos_duplicados(pagos: &[Pago]) -> Vec<(Uuid, Uuid)> {
    // Group paid payments by (contrato_id, fecha_vencimiento, monto as bits) — O(n)
    // Using f64::to_bits() for exact equality comparison in the key
    let mut groups: HashMap<(Uuid, chrono::NaiveDate, u64), Vec<Uuid>> = HashMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        let key = (
            pago.contrato_id,
            pago.fecha_vencimiento,
            pago.monto.to_bits(),
        );
        groups.entry(key).or_default().push(pago.id);
    }

    // Only generate pairs within groups that have more than one payment — O(d²) where d ≪ n
    let mut duplicados = Vec::new();
    for ids in groups.values().filter(|ids| ids.len() > 1) {
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                duplicados.push((ids[i], ids[j]));
            }
        }
    }

    duplicados
}

/// Summarizes payment methods used per month for a stacked bar chart.
/// Needs: month → { method → total_amount }.
/// Dataset: ~2000 payments, 4 payment methods, 24 months.
///
/// BTreeMap is intentional: months must be in chronological order for the chart.
/// Already O(n) single pass with appropriate data structures. No optimization needed.
pub fn resumen_metodos_pago(pagos: &[Pago]) -> BTreeMap<String, HashMap<String, f64>> {
    let mut resultado: BTreeMap<String, HashMap<String, f64>> = BTreeMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        if let Some(fecha) = pago.fecha_pago {
            let mes = fecha.format("%Y-%m").to_string();
            let metodo = pago.metodo_pago.as_deref().unwrap_or("sin_especificar");
            *resultado
                .entry(mes)
                .or_default()
                .entry(metodo.to_string())
                .or_default() += pago.monto;
        }
    }

    resultado
}
