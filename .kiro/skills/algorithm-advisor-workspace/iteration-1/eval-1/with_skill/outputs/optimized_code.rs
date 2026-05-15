use std::collections::{HashMap, HashSet};
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
/// Optimized: O(n + m) using HashSet lookup instead of O(n * m) nested scan.
pub fn filtrar_existentes(pagos_existentes: &[Pago], pagos_nuevos: &[PagoNuevo]) -> Vec<PagoNuevo> {
    let existentes_set: HashSet<(Uuid, chrono::NaiveDate)> = pagos_existentes
        .iter()
        .map(|p| (p.contrato_id, p.fecha_vencimiento))
        .collect();

    pagos_nuevos
        .iter()
        .filter(|nuevo| !existentes_set.contains(&(nuevo.contrato_id, nuevo.fecha_vencimiento)))
        .cloned()
        .collect()
}

/// Encuentra el último pago realizado para un contrato.
/// Optimized: O(n) single-pass max instead of O(n log n) sort.
pub fn ultimo_pago(pagos: &[Pago], contrato_id: Uuid) -> Option<Pago> {
    pagos
        .iter()
        .filter(|p| p.contrato_id == contrato_id && p.fecha_pago.is_some())
        .max_by_key(|p| p.fecha_pago)
        .cloned()
}

/// Calcula totales de pagos agrupados por mes.
/// Optimized: O(n) single-pass aggregation with HashMap instead of O(months * n) repeated scans.
pub fn totales_por_mes(pagos: &[Pago]) -> Vec<(String, f64)> {
    let mut totales: HashMap<String, f64> = HashMap::new();

    for pago in pagos.iter().filter(|p| p.estado == "pagado") {
        let mes = pago.fecha_vencimiento.format("%Y-%m").to_string();
        *totales.entry(mes).or_insert(0.0) += pago.monto;
    }

    let mut resultado: Vec<(String, f64)> = totales.into_iter().collect();
    resultado.sort_by(|a, b| a.0.cmp(&b.0));
    resultado
}

/// Calcula el monto total pendiente por contrato.
/// Optimized: direct iterator sum without intermediate Vec allocation.
pub fn pendiente_por_contrato(pagos: &[Pago], contrato_id: Uuid) -> f64 {
    pagos
        .iter()
        .filter(|p| p.contrato_id == contrato_id && p.estado == "pendiente")
        .map(|p| p.monto)
        .sum()
}
