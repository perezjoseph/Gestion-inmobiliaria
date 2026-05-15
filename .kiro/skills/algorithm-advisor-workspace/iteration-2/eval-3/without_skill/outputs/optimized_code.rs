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
/// Paradigm: GREEDY with in-place sort (keep as-is).
/// Rationale: With n=1-12 items, O(n log n) sort + O(n) greedy scan is optimal.
/// A min-heap would be O(n) to build but O(n log n) to extract all elements,
/// and we need to process ALL items in order (not just the minimum), so a heap
/// provides no advantage. The sort also leaves the slice in a useful sorted state
/// for the caller.
pub fn aplicar_pago_parcial(pagos_pendientes: &mut [Pago], monto_disponible: f64) -> f64 {
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
            pago.monto -= restante;
            restante = 0.0;
        }
    }

    restante
}

/// Calculates net income (ingresos - gastos) per propiedad per month.
///
/// Paradigm: TRANSFORM AND CONQUER (pre-index with HashMaps).
/// Rationale: The original is O(propiedades × months × (pagos + gastos)) — cubic.
/// We transform the data into indexed structures ONCE, then do O(1) lookups.
/// - Build a contrato_id → propiedad_id lookup map: O(contratos)
/// - Group pagos by (propiedad_id, month): O(pagos)
/// - Group gastos by (propiedad_id, month): O(gastos)
/// - Final assembly is O(propiedades × months) with O(1) per cell.
///
/// Total: O(contratos + pagos + gastos + propiedades × months) — linear in input size.
/// A simple HashMap per dimension won't work because we need a COMPOSITE key
/// (propiedad_id, month). The transform step builds this composite index.
pub fn rentabilidad_por_propiedad_mes(
    contratos: &[Contrato],
    pagos: &[Pago],
    gastos: &[Gasto],
) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Step 1: Build contrato_id → propiedad_id index
    let contrato_to_propiedad: HashMap<Uuid, Uuid> = contratos
        .iter()
        .map(|c| (c.id, c.propiedad_id))
        .collect();

    // Step 2: Collect unique propiedades
    let propiedad_ids: Vec<Uuid> = contratos
        .iter()
        .map(|c| c.propiedad_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Step 3: Group paid pagos by (propiedad_id, month) — single pass over pagos
    let mut ingresos_index: HashMap<(Uuid, String), f64> = HashMap::new();
    let mut meses_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado" && p.fecha_pago.is_some()) {
        let mes = pago.fecha_pago.unwrap().format("%Y-%m").to_string();
        if let Some(&propiedad_id) = contrato_to_propiedad.get(&pago.contrato_id) {
            *ingresos_index.entry((propiedad_id, mes.clone())).or_insert(0.0) += pago.monto;
            meses_set.insert(mes);
        }
    }

    // Step 4: Group paid gastos by (propiedad_id, month) — single pass over gastos
    let mut egresos_index: HashMap<(Uuid, String), f64> = HashMap::new();

    for gasto in gastos.iter().filter(|g| g.estado == "pagado") {
        let mes = gasto.fecha_gasto.format("%Y-%m").to_string();
        *egresos_index.entry((gasto.propiedad_id, mes.clone())).or_insert(0.0) += gasto.monto;
        meses_set.insert(mes);
    }

    // Step 5: Assemble results — O(propiedades × months) with O(1) lookups
    let meses: Vec<String> = meses_set.into_iter().collect();
    let mut resultado: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(propiedad_ids.len());

    for &propiedad_id in &propiedad_ids {
        let serie: Vec<(String, f64)> = meses
            .iter()
            .map(|mes| {
                let ingresos = ingresos_index
                    .get(&(propiedad_id, mes.clone()))
                    .copied()
                    .unwrap_or(0.0);
                let egresos = egresos_index
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
///
/// Paradigm: KEEP AS-IS (brute force is optimal).
/// Rationale: The outer loop is fixed at 12 iterations. The inner scan is O(n)
/// over ~200 contracts. Total work: 12 × 200 = 2400 comparisons.
/// An interval tree or sweep line would add O(n log n) construction overhead
/// for a problem that's already O(n) with a tiny constant factor.
/// The code is clear, correct, and fast enough. No optimization needed.
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
/// Paradigm: DECREASE AND CONQUER (prefix sum / cumulative accumulation).
/// Rationale: The original rescans ALL payments for each month — O(months × payments).
/// The insight is that the cumulative count for month M equals the cumulative count
/// for month M-1 PLUS any newly-delinquent payments from month M-1.
/// We sort payments into buckets by month, count delinquent ones per bucket,
/// then compute a prefix sum. Each month's cascade value is the sum of all
/// delinquent counts from prior months.
///
/// This reduces O(months × payments) to O(payments + months) — linear.
pub fn cascada_morosidad(pagos: &[Pago]) -> Vec<(String, usize)> {
    // Step 1: Count delinquent payments per month — single pass O(pagos)
    let mut impagos_por_mes: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut all_months: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for pago in pagos {
        let mes = pago.fecha_vencimiento.format("%Y-%m").to_string();
        all_months.insert(mes.clone());
        if pago.estado == "pendiente" || pago.estado == "atrasado" {
            *impagos_por_mes.entry(mes).or_insert(0) += 1;
        }
    }

    // Step 2: Compute prefix sum — for each month, the cascade is the sum of
    // delinquent payments from ALL prior months (not including current month).
    let meses: Vec<String> = all_months.into_iter().collect();
    let mut resultado: Vec<(String, usize)> = Vec::with_capacity(meses.len());
    let mut acumulado: usize = 0;

    for mes in &meses {
        // The cascade for this month = total delinquent from all PREVIOUS months
        resultado.push((mes.clone(), acumulado));
        // Add this month's delinquent count to the running total for future months
        acumulado += impagos_por_mes.get(mes).copied().unwrap_or(0);
    }

    resultado
}
