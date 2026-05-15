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
/// BTreeMap is intentional here: only 4 buckets, and sorted output is required
/// for the report. No optimization needed.
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
/// Used for "good tenant" reports.
///
/// Optimized from O(inquilinos × contratos × pagos) to O(inquilinos + contratos + pagos)
/// by pre-building lookup indexes:
/// 1. Collect all contrato_ids that have at least one late payment (single pass over pagos).
/// 2. Map inquilino_id → set of contrato_ids (single pass over contratos).
/// 3. For each inquilino, check if any of their contratos appear in the late set.
pub fn inquilinos_sin_atrasos<'a>(
    inquilinos: &'a [Inquilino],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<&'a Inquilino> {
    // Step 1: Set of contrato_ids that have at least one late payment — O(pagos)
    let contratos_con_atraso: HashSet<Uuid> = pagos
        .iter()
        .filter(|p| p.estado == "atrasado")
        .map(|p| p.contrato_id)
        .collect();

    // Step 2: Map inquilino_id → their contrato_ids — O(contratos)
    let mut contratos_por_inquilino: HashMap<Uuid, Vec<Uuid>> =
        HashMap::with_capacity(inquilinos.len());
    for contrato in contratos {
        contratos_por_inquilino
            .entry(contrato.inquilino_id)
            .or_default()
            .push(contrato.id);
    }

    // Step 3: Filter inquilinos whose contratos have no late payments — O(inquilinos × avg_contratos)
    inquilinos
        .iter()
        .filter(|inquilino| {
            contratos_por_inquilino
                .get(&inquilino.id)
                .map(|ids| !ids.iter().any(|id| contratos_con_atraso.contains(id)))
                .unwrap_or(true) // No contratos means no late payments
        })
        .collect()
}

/// Calculates running total of payments received per month for a trend chart.
/// Returns months in chronological order with cumulative sums.
///
/// Optimized from O(months × n) to O(n + months × log(months)):
/// 1. Single pass to group payment amounts by month into a HashMap.
/// 2. Sort the month keys (chronological order via string comparison on YYYY-MM).
/// 3. Single pass to compute running cumulative sum.
pub fn tendencia_pagos_acumulados(pagos: &[Pago]) -> Vec<(String, f64)> {
    // Step 1: Group totals by month — O(n)
    let mut totales_por_mes: HashMap<String, f64> = HashMap::new();
    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        if let Some(fecha) = pago.fecha_pago {
            let mes = fecha.format("%Y-%m").to_string();
            *totales_por_mes.entry(mes).or_default() += pago.monto;
        }
    }

    // Step 2: Sort months chronologically — O(m log m)
    let mut meses: Vec<String> = totales_por_mes.keys().cloned().collect();
    meses.sort();

    // Step 3: Compute running cumulative sum — O(m)
    let mut acumulado = 0.0;
    meses
        .into_iter()
        .map(|mes| {
            acumulado += totales_por_mes[&mes];
            (mes, acumulado)
        })
        .collect()
}

/// Detects duplicate payments: same contrato, same fecha_vencimiento, same monto,
/// paid more than once. Returns pairs of duplicate payment IDs.
///
/// Optimized from O(n²) to O(n) by grouping payments into buckets keyed by
/// (contrato_id, fecha_vencimiento, monto_bits). Within each bucket, all pairs
/// are duplicates. Bucket sizes are tiny (duplicates are < 1%), so the inner
/// pairwise loop is effectively constant.
pub fn detectar_pagos_duplicados(pagos: &[Pago]) -> Vec<(Uuid, Uuid)> {
    // Group paid payments by their deduplication key
    let mut grupos: HashMap<(Uuid, chrono::NaiveDate, u64), Vec<Uuid>> = HashMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        let key = (
            pago.contrato_id,
            pago.fecha_vencimiento,
            pago.monto.to_bits(), // exact f64 comparison via bit representation
        );
        grupos.entry(key).or_default().push(pago.id);
    }

    // Emit all pairs within each group
    let mut duplicados = Vec::new();
    for ids in grupos.values() {
        if ids.len() > 1 {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    duplicados.push((ids[i], ids[j]));
                }
            }
        }
    }

    duplicados
}

/// Summarizes payment methods used per month for a stacked bar chart.
/// Needs: month → { method → total_amount }, months in chronological order.
///
/// BTreeMap is intentional here: months must be in sorted order for the chart.
/// Single-pass aggregation is already optimal — no changes needed.
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
