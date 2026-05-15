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
/// Dataset: a tenant typically has 1-12 pending payments.
/// Current approach: sorts debts, then greedily allocates.
///
/// The sort is O(n log n) — someone suggested using a min-heap for O(n) construction.
/// But is that actually better here? Think about what happens after allocation.
pub fn aplicar_pago_parcial(pagos_pendientes: &mut [Pago], monto_disponible: f64) -> f64 {
    // Sort by fecha_vencimiento (oldest first)
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
/// Dataset: ~50 propiedades, ~2000 pagos, ~500 gastos, spanning 24 months.
///
/// Current approach: for each propiedad, for each month, sums pagos and gastos separately.
/// This is O(propiedades × months × (pagos + gastos)) — genuinely cubic.
///
/// The challenge: you need to join THREE dimensions (propiedad, month, type).
/// What's the most efficient approach that keeps the code readable?
pub fn rentabilidad_por_propiedad_mes(
    contratos: &[Contrato],
    pagos: &[Pago],
    gastos: &[Gasto],
) -> HashMap<Uuid, Vec<(String, f64)>> {
    // Collect unique propiedades from contratos
    let propiedad_ids: Vec<Uuid> = contratos
        .iter()
        .map(|c| c.propiedad_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Collect unique months from pagos
    let mut meses: Vec<String> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .map(|p| p.fecha_pago.unwrap().format("%Y-%m").to_string())
        .collect();
    meses.sort();
    meses.dedup();

    let mut resultado: HashMap<Uuid, Vec<(String, f64)>> = HashMap::new();

    for &propiedad_id in &propiedad_ids {
        let mut serie = Vec::new();

        for mes in &meses {
            // Sum pagos for this propiedad in this month
            let ingresos: f64 = pagos
                .iter()
                .filter(|p| {
                    p.estado == "pagado"
                        && p.fecha_pago.is_some()
                        && p.fecha_pago.unwrap().format("%Y-%m").to_string() == *mes
                        && contratos
                            .iter()
                            .any(|c| c.id == p.contrato_id && c.propiedad_id == propiedad_id)
                })
                .map(|p| p.monto)
                .sum();

            // Sum gastos for this propiedad in this month
            let egresos: f64 = gastos
                .iter()
                .filter(|g| {
                    g.propiedad_id == propiedad_id
                        && g.estado == "pagado"
                        && g.fecha_gasto.format("%Y-%m").to_string() == *mes
                })
                .map(|g| g.monto)
                .sum();

            serie.push((mes.clone(), ingresos - egresos));
        }

        resultado.insert(propiedad_id, serie);
    }

    resultado
}

/// Finds the optimal month to schedule maintenance based on lowest occupancy.
/// For each month in the next 12 months, counts how many contracts are active.
/// Recommends the month with the fewest active contracts.
///
/// Dataset: ~200 contracts with various date ranges.
///
/// Current approach: for each of 12 future months, scans all contracts to check overlap.
/// This is O(12 × n) = O(n). Is this actually a problem?
///
/// Someone suggested an "interval tree" or "sweep line" algorithm.
/// Is that warranted here?
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
/// Dataset: ~2000 payments spanning 24 months.
///
/// Current approach: for each month M, counts payments with fecha_vencimiento < M
/// that are still "pendiente" or "atrasado". This rescans all payments per month.
///
/// The tricky part: this is a CUMULATIVE count that changes meaning as time progresses.
/// A simple HashMap grouping doesn't capture the "still unpaid from before" semantics.
pub fn cascada_morosidad(pagos: &[Pago]) -> Vec<(String, usize)> {
    let mut meses: Vec<String> = pagos
        .iter()
        .map(|p| p.fecha_vencimiento.format("%Y-%m").to_string())
        .collect();
    meses.sort();
    meses.dedup();

    meses
        .iter()
        .map(|mes_actual| {
            let acumulado_impago = pagos
                .iter()
                .filter(|p| {
                    p.fecha_vencimiento.format("%Y-%m").to_string() < *mes_actual
                        && (p.estado == "pendiente" || p.estado == "atrasado")
                })
                .count();
            (mes_actual.clone(), acumulado_impago)
        })
        .collect()
}
