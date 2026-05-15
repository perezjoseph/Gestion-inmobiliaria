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
/// Current approach: iterates payments once, computes days_late, buckets into a BTreeMap.
/// The BTreeMap is used because the report needs buckets in sorted order (ascending by days).
///
/// Someone flagged this as "BTreeMap should be HashMap" — but is that correct here?
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
/// Current approach: for each inquilino, find their contratos, then check if any pago
/// on those contratos was ever late. Triple nested structure.
///
/// This looks like the classic O(n³) anti-pattern, but think carefully about what
/// data structures would actually help here and whether the approach is correct.
pub fn inquilinos_sin_atrasos(
    inquilinos: &[Inquilino],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<&Inquilino> {
    let mut buenos = Vec::new();

    for inquilino in inquilinos {
        let mut tiene_atraso = false;

        for contrato in contratos {
            if contrato.inquilino_id == inquilino.id {
                for pago in pagos {
                    if pago.contrato_id == contrato.id && pago.estado == "atrasado" {
                        tiene_atraso = true;
                        break;
                    }
                }
            }
            if tiene_atraso {
                break;
            }
        }

        if !tiene_atraso {
            buenos.push(inquilino);
        }
    }

    buenos
}

/// Calculates running total of payments received per month for a trend chart.
/// Needs to return months in chronological order with cumulative sums.
/// Dataset: ~2000 payments spanning 24 months.
///
/// Current approach: collects unique months, sorts them, then for each month
/// sums all payments up to and including that month.
/// This is O(months × n) — the repeated prefix sum is the real problem.
///
/// What's the right fix? HashMap grouping alone doesn't solve the cumulative aspect.
pub fn tendencia_pagos_acumulados(pagos: &[Pago]) -> Vec<(String, f64)> {
    let mut meses: Vec<String> = pagos
        .iter()
        .filter(|p| p.estado == "pagado")
        .map(|p| p.fecha_pago.unwrap().format("%Y-%m").to_string())
        .collect();
    meses.sort();
    meses.dedup();

    meses
        .iter()
        .map(|mes_actual| {
            let acumulado: f64 = pagos
                .iter()
                .filter(|p| {
                    p.estado == "pagado"
                        && p.fecha_pago.unwrap().format("%Y-%m").to_string() <= *mes_actual
                })
                .map(|p| p.monto)
                .sum();
            (mes_actual.clone(), acumulado)
        })
        .collect()
}

/// Detects duplicate payments: same contrato, same fecha_vencimiento, same monto,
/// paid more than once. Returns pairs of duplicate payment IDs.
/// Dataset: ~2000 payments. Duplicates are rare (< 1%).
///
/// Current approach: O(n²) pairwise comparison.
/// This one genuinely needs optimization — but what's the right key for the lookup?
pub fn detectar_pagos_duplicados(pagos: &[Pago]) -> Vec<(Uuid, Uuid)> {
    let mut duplicados = Vec::new();

    for i in 0..pagos.len() {
        for j in (i + 1)..pagos.len() {
            if pagos[i].contrato_id == pagos[j].contrato_id
                && pagos[i].fecha_vencimiento == pagos[j].fecha_vencimiento
                && pagos[i].monto == pagos[j].monto
                && pagos[i].estado == "pagado"
                && pagos[j].estado == "pagado"
            {
                duplicados.push((pagos[i].id, pagos[j].id));
            }
        }
    }

    duplicados
}

/// Summarizes payment methods used per month for a stacked bar chart.
/// Needs: month → { method → total_amount }.
/// Dataset: ~2000 payments, 4 payment methods, 24 months.
///
/// Current approach: builds nested HashMap in a single pass.
/// Is there a problem here?
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
