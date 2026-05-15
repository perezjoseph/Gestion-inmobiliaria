use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub monto_mensual: f64,
    pub moneda: String,
    pub estado: String,
}

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: f64,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
}

#[derive(Debug, Clone)]
pub struct Gasto {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub categoria: String,
    pub monto: f64,
    pub moneda: String,
    pub fecha_gasto: chrono::NaiveDate,
    pub estado: String,
}

/// Allocates a partial payment across multiple pending debts for a tenant.
/// Business rule: apply payment to oldest debts first (by fecha_vencimiento).
/// If payment amount exceeds all debts, return the remainder.
///
/// Paradigm: Greedy (locally optimal choice at each step yields global optimum)
///
/// Analysis: The original O(n log n) sort + O(n) greedy scan is ALREADY OPTIMAL
/// for this problem. A min-heap would give O(n) construction but O(n log n) total
/// extraction — same asymptotic cost, worse constants, and less cache-friendly.
/// With n = 1-12 elements, the sort is essentially free. The greedy approach is
/// provably correct because paying oldest debts first minimizes late penalties
/// (exchange argument: swapping any allocation to a newer debt while an older one
/// remains unpaid increases total penalty days).
///
/// Verdict: No change needed. The code correctly applies the Greedy paradigm.
/// The min-heap suggestion is a false optimization — it doesn't improve complexity
/// and hurts readability for tiny n.
pub fn aplicar_pago_parcial(pagos_pendientes: &mut [Pago], monto_disponible: f64) -> f64 {
    // Sort by fecha_vencimiento (oldest first) — O(n log n), n ≤ 12
    pagos_pendientes.sort_by_key(|p| p.fecha_vencimiento);

    let mut restante = monto_disponible;

    for pago in pagos_pendientes.iter_mut() {
        if restante <= 0.0 {
            break;
        }

        let deuda = pago.monto;
        if restante >= deuda {
            pago.estado = "pagado".to_string();
            pago.fecha_pago = Some(chrono::Local::now().date_naive());
            restante -= deuda;
        } else {
            // Partial payment — reduce the amount owed
            pago.monto -= restante;
            restante = 0.0;
        }
    }

    restante
}

/// Calculates net income (ingresos - gastos) per propiedad per month.
/// Used for profitability analysis. Needs chronological ordering of months.
///
/// Paradigm: Space-Time Tradeoff (precompute lookup tables to eliminate nested scans)
///
/// Original: O(propiedades × months × (pagos + gastos)) — cubic behavior.
/// Optimized: O(pagos + gastos + contratos) preprocessing, then O(propiedades × months)
/// assembly — effectively linear in the input size.
///
/// Strategy: Build two HashMaps keyed by (propiedad_id, month_string):
///   1. ingresos_map: sum of paid pagos per propiedad per month
///   2. egresos_map: sum of paid gastos per propiedad per month
/// Also build a contrato_id → propiedad_id lookup to resolve pago → propiedad.
/// Then iterate propiedades × months with O(1) lookups instead of O(n) scans.
pub fn rentabilidad_por_propiedad_mes(
    contratos: &[Contrato],
    pagos: &[Pago],
    gastos: &[Gasto],
) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Step 1: Build contrato_id → propiedad_id lookup — O(contratos)
    let contrato_to_propiedad: HashMap<Uuid, Uuid> =
        contratos.iter().map(|c| (c.id, c.propiedad_id)).collect();

    // Step 2: Collect unique propiedades — O(contratos)
    let propiedad_ids: Vec<Uuid> = contratos
        .iter()
        .map(|c| c.propiedad_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Step 3: Build ingresos index keyed by (propiedad_id, month) — O(pagos)
    let mut ingresos_map: HashMap<(Uuid, String), f64> = HashMap::new();
    let mut meses_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let mes = pago.fecha_pago.unwrap().format("%Y-%m").to_string();
        if let Some(&propiedad_id) = contrato_to_propiedad.get(&pago.contrato_id) {
            *ingresos_map
                .entry((propiedad_id, mes.clone()))
                .or_insert(0.0) += pago.monto;
            meses_set.insert(mes);
        }
    }

    // Step 4: Build egresos index keyed by (propiedad_id, month) — O(gastos)
    let mut egresos_map: HashMap<(Uuid, String), f64> = HashMap::new();

    for gasto in gastos.iter().filter(|g| g.estado == "pagado") {
        let mes = gasto.fecha_gasto.format("%Y-%m").to_string();
        *egresos_map
            .entry((gasto.propiedad_id, mes.clone()))
            .or_insert(0.0) += gasto.monto;
        meses_set.insert(mes);
    }

    // Step 5: Assemble results — O(propiedades × months) with O(1) lookups
    let meses: Vec<String> = meses_set.into_iter().collect();
    let mut resultado: HashMap<Uuid, Vec<(String, f64)>> =
        HashMap::with_capacity(propiedad_ids.len());

    for &propiedad_id in &propiedad_ids {
        let serie: Vec<(String, f64)> = meses
            .iter()
            .map(|mes| {
                let ingresos = ingresos_map
                    .get(&(propiedad_id, mes.clone()))
                    .copied()
                    .unwrap_or(0.0);
                let egresos = egresos_map
                    .get(&(propiedad_id, mes.clone()))
                    .copied()
                    .unwrap_or(0.0);
                (mes.clone(), ingresos - egresos)
            })
            .collect();
        resultado.insert(propiedad_id, serie);
    }

    resultado
}

/// Finds the optimal month to schedule maintenance based on lowest occupancy.
/// For each month in the next 12 months, counts how many contracts are active.
/// Recommends the month with the fewest active contracts.
///
/// Paradigm: None needed — the original is already optimal.
///
/// Analysis: The current approach is O(12 × n) = O(n) where n = ~200 contracts.
/// This is a fixed 12-iteration outer loop (constant), making it effectively O(n).
/// Total work: ~2400 comparisons — trivial.
///
/// Why NOT an interval tree or sweep line:
/// - Interval tree: O(n log n) construction + O(n + k) query. For 12 queries on 200
///   contracts, the construction overhead exceeds the brute-force cost.
/// - Sweep line: O(n log n) sort + O(n) sweep. More complex code for the same or
///   worse performance on this tiny dataset.
/// - Both add code complexity with zero practical benefit.
///
/// Verdict: No change needed. The code is already O(n) and the constant factor
/// is negligible. This is a case where "optimize" means "leave it alone."
pub fn mejor_mes_mantenimiento(contratos: &[Contrato]) -> Option<chrono::NaiveDate> {
    let hoy = chrono::Local::now().date_naive();
    let mut mejor_mes: Option<chrono::NaiveDate> = None;
    let mut menor_ocupacion = usize::MAX;

    for offset in 0..12 {
        let mes_inicio = hoy + chrono::Months::new(offset);
        let mes_fin = mes_inicio + chrono::Months::new(1) - chrono::Duration::days(1);

        let ocupacion = contratos
            .iter()
            .filter(|c| {
                c.estado == "activo" && c.fecha_inicio <= mes_fin && c.fecha_fin >= mes_inicio
            })
            .count();

        if ocupacion < menor_ocupacion {
            menor_ocupacion = ocupacion;
            mejor_mes = Some(mes_inicio);
        }
    }

    mejor_mes
}

/// Generates a "delinquency cascade" report: for each month, shows how many
/// payments from PREVIOUS months are still unpaid (snowball effect).
///
/// Paradigm: Dynamic Programming (prefix sum / cumulative computation)
///
/// Original: O(months × pagos) — for each month, rescans ALL payments to count
/// those with fecha_vencimiento < current month that are still unpaid.
/// With ~24 months and ~2000 payments, that's ~48,000 iterations.
///
/// Optimized: O(pagos + months) using a prefix sum approach.
/// Strategy:
///   1. Group unpaid payments by their vencimiento month — O(pagos)
///   2. Compute prefix sums over months — O(months)
///   3. For each month M, the cascade count = prefix_sum[M-1] (all unpaid from before M)
///
/// This is DP in its simplest form: the cumulative count for month M depends on
/// the cumulative count for month M-1 plus new delinquencies in month M-1.
/// Optimal substructure: cascade(M) = cascade(M-1) + new_unpaid(M-1).
pub fn cascada_morosidad(pagos: &[Pago]) -> Vec<(String, usize)> {
    // Step 1: Collect and sort unique months — O(pagos) + O(m log m)
    let mut meses_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for pago in pagos {
        meses_set.insert(pago.fecha_vencimiento.format("%Y-%m").to_string());
    }
    let meses: Vec<String> = meses_set.into_iter().collect();

    // Step 2: Count unpaid payments per vencimiento month — O(pagos)
    let mut impagos_por_mes: HashMap<String, usize> = HashMap::new();
    for pago in pagos {
        if pago.estado == "pendiente" || pago.estado == "atrasado" {
            let mes = pago.fecha_vencimiento.format("%Y-%m").to_string();
            *impagos_por_mes.entry(mes).or_insert(0) += 1;
        }
    }

    // Step 3: Compute prefix sums — O(months)
    // For month M, cascade = sum of all unpaid payments from months BEFORE M
    let mut acumulado: usize = 0;
    meses
        .iter()
        .map(|mes| {
            let resultado = (mes.clone(), acumulado);
            // Add this month's unpaid count to the running total for future months
            acumulado += impagos_por_mes.get(mes).copied().unwrap_or(0);
            resultado
        })
        .collect()
}
