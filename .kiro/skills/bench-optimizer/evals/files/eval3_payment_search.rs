use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub monto: f64,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
    pub referencia: Option<String>,
}

/// Searches payments by multiple optional filters: contrato_id, estado, date range,
/// and partial referencia match. Called from the payments list page with pagination.
/// Production dataset: ~5000 pagos total, typical result set 20-100 after filtering.
///
/// Current implementation: linear scan with chained filters.
/// The team wants to know if pre-indexing (HashMap by contrato_id, BTreeMap by date)
/// would be faster for the common query patterns:
///   - Filter by contrato_id (most common, ~80% of queries)
///   - Filter by date range (second most common)
///   - Filter by estado
///   - Text search on referencia (rare, ~5% of queries)
pub fn buscar_pagos<'a>(
    pagos: &'a [Pago],
    contrato_id: Option<Uuid>,
    estado: Option<&str>,
    fecha_desde: Option<chrono::NaiveDate>,
    fecha_hasta: Option<chrono::NaiveDate>,
    referencia: Option<&str>,
) -> Vec<&'a Pago> {
    pagos
        .iter()
        .filter(|p| {
            contrato_id.map_or(true, |id| p.contrato_id == id)
                && estado.map_or(true, |e| p.estado == e)
                && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                && referencia.map_or(true, |r| {
                    p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                })
        })
        .collect()
}
