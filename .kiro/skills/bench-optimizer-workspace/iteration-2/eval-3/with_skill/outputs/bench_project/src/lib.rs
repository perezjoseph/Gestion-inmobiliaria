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

// =============================================================================
// Approach 1: Current linear scan
// =============================================================================

pub fn buscar_pagos_linear<'a>(
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

// =============================================================================
// Approach 2: Pre-indexed by contrato_id (HashMap<Uuid, Vec<usize>>)
// =============================================================================

/// Index that maps contrato_id -> list of indices into the pagos slice.
/// Built once at startup or on data change, queried on every search request.
pub struct PagoIndex {
    by_contrato: HashMap<Uuid, Vec<usize>>,
}

impl PagoIndex {
    pub fn build(pagos: &[Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<usize>> = HashMap::with_capacity(pagos.len() / 10);
        for (i, p) in pagos.iter().enumerate() {
            by_contrato.entry(p.contrato_id).or_default().push(i);
        }
        Self { by_contrato }
    }

    pub fn buscar<'a>(
        &self,
        pagos: &'a [Pago],
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        match contrato_id {
            Some(id) => {
                // Use the index: O(1) lookup + scan only matching pagos
                let indices = match self.by_contrato.get(&id) {
                    Some(indices) => indices,
                    None => return Vec::new(),
                };
                indices
                    .iter()
                    .map(|&i| &pagos[i])
                    .filter(|p| {
                        estado.map_or(true, |e| p.estado == e)
                            && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                            && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                            && referencia.map_or(true, |r| {
                                p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                            })
                    })
                    .collect()
            }
            None => {
                // No contrato_id filter: fall back to linear scan
                buscar_pagos_linear(pagos, None, estado, fecha_desde, fecha_hasta, referencia)
            }
        }
    }
}

// =============================================================================
// Approach 3: Pre-indexed storing references directly
// =============================================================================

/// Alternative index storing references directly to avoid index indirection.
/// Trades memory (stores pointers) for potentially better cache behavior on lookup.
pub struct PagoRefIndex<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
}

impl<'a> PagoRefIndex<'a> {
    pub fn build(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 10);
        for p in pagos {
            by_contrato.entry(p.contrato_id).or_default().push(p);
        }
        Self { by_contrato }
    }

    pub fn buscar(
        &self,
        pagos: &'a [Pago],
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        match contrato_id {
            Some(id) => {
                let refs = match self.by_contrato.get(&id) {
                    Some(refs) => refs,
                    None => return Vec::new(),
                };
                refs.iter()
                    .filter(|p| {
                        estado.map_or(true, |e| p.estado == e)
                            && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
                            && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
                            && referencia.map_or(true, |r| {
                                p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                            })
                    })
                    .copied()
                    .collect()
            }
            None => {
                // Fall back to linear scan
                buscar_pagos_linear(pagos, None, estado, fecha_desde, fecha_hasta, referencia)
            }
        }
    }
}
