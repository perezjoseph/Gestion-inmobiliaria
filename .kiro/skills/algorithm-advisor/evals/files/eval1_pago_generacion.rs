use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub monto_mensual: f64,
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
pub struct PagoNuevo {
    pub contrato_id: Uuid,
    pub monto: f64,
    pub fecha_vencimiento: chrono::NaiveDate,
}

/// Filtra pagos nuevos que ya existen en el sistema.
/// BUG: O(n*m) — for each pago_nuevo, scans all pagos_existentes with .any().
pub fn filtrar_existentes(pagos_existentes: &[Pago], pagos_nuevos: &[PagoNuevo]) -> Vec<PagoNuevo> {
    pagos_nuevos
        .iter()
        .filter(|nuevo| {
            !pagos_existentes.iter().any(|existente| {
                existente.contrato_id == nuevo.contrato_id
                    && existente.fecha_vencimiento == nuevo.fecha_vencimiento
            })
        })
        .cloned()
        .collect()
}

/// Encuentra el último pago realizado para un contrato.
/// BUG: Sorts the entire Vec just to get the last element — O(n log n) when O(n) suffices.
pub fn ultimo_pago(pagos: &[Pago], contrato_id: Uuid) -> Option<Pago> {
    let mut pagos_contrato: Vec<&Pago> = pagos
        .iter()
        .filter(|p| p.contrato_id == contrato_id && p.fecha_pago.is_some())
        .collect();

    pagos_contrato.sort_by_key(|p| p.fecha_pago);
    pagos_contrato.last().cloned().cloned()
}

/// Calcula totales de pagos agrupados por mes.
/// BUG: For each unique month, rescans the entire pagos list — O(months * n).
pub fn totales_por_mes(pagos: &[Pago]) -> Vec<(String, f64)> {
    // Collect unique months
    let mut meses: Vec<String> = pagos
        .iter()
        .map(|p| p.fecha_vencimiento.format("%Y-%m").to_string())
        .collect();
    meses.sort();
    meses.dedup();

    // For each month, rescan all pagos
    meses
        .iter()
        .map(|mes| {
            let total: f64 = pagos
                .iter()
                .filter(|p| p.fecha_vencimiento.format("%Y-%m").to_string() == *mes)
                .filter(|p| p.estado == "pagado")
                .map(|p| p.monto)
                .sum();
            (mes.clone(), total)
        })
        .collect()
}

/// Calcula el monto total pendiente por contrato.
/// BUG: Collects into intermediate Vec before summing — unnecessary allocation.
pub fn pendiente_por_contrato(pagos: &[Pago], contrato_id: Uuid) -> f64 {
    let montos: Vec<f64> = pagos
        .iter()
        .filter(|p| p.contrato_id == contrato_id && p.estado == "pendiente")
        .map(|p| p.monto)
        .collect();
    montos.iter().sum()
}
