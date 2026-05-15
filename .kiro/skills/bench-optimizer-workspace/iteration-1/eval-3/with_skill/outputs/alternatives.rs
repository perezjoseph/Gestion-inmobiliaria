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
// Approach 1: Current — Linear scan with chained filters
// =============================================================================

/// Linear scan: iterates all pagos and applies each filter in sequence.
/// Simple, no setup cost, but O(n) for every query regardless of filters.
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
// Approach 2: Pre-indexed HashMap by contrato_id
// =============================================================================

/// Index structure: groups payment references by contrato_id.
/// Build cost is O(n), but subsequent lookups by contrato_id are O(1) amortized
/// to find the bucket, then O(k) where k = payments for that contract.
///
/// Since 80% of queries filter by contrato_id, this avoids scanning the full
/// 5000-element vec for the most common case.
pub struct PagoIndex<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    all: &'a [Pago],
}

impl<'a> PagoIndex<'a> {
    /// Build the index. Call once at startup or when the dataset changes.
    pub fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 25); // ~200 contracts for 5000 pagos

        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_insert_with(|| Vec::with_capacity(25))
                .push(pago);
        }

        Self {
            by_contrato,
            all: pagos,
        }
    }

    /// Query using the index when contrato_id is provided, fallback to linear
    /// scan when it's not.
    pub fn buscar(
        &self,
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        let candidates: Box<dyn Iterator<Item = &&Pago>> = match contrato_id {
            Some(id) => match self.by_contrato.get(&id) {
                Some(bucket) => Box::new(bucket.iter()),
                None => return Vec::new(),
            },
            None => Box::new(self.all.iter().map(|p| p).collect::<Vec<_>>().leak().iter()),
        };

        // This leak trick is ugly — use the non-dyn version below for real code.
        // Included here only to show the concept. The benchmark uses the clean version.
        candidates
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
}

// =============================================================================
// Approach 2b: Pre-indexed HashMap — clean version (no dyn dispatch)
// =============================================================================

/// Clean indexed search: two code paths, no trait objects, no allocations
/// beyond the result vec.
pub fn buscar_pagos_indexed<'a>(
    index: &'a PagoIndexClean<'a>,
    contrato_id: Option<Uuid>,
    estado: Option<&str>,
    fecha_desde: Option<chrono::NaiveDate>,
    fecha_hasta: Option<chrono::NaiveDate>,
    referencia: Option<&str>,
) -> Vec<&'a Pago> {
    let apply_filters = |p: &&Pago| -> bool {
        estado.map_or(true, |e| p.estado == e)
            && fecha_desde.map_or(true, |d| p.fecha_vencimiento >= d)
            && fecha_hasta.map_or(true, |d| p.fecha_vencimiento <= d)
            && referencia.map_or(true, |r| {
                p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
            })
    };

    match contrato_id {
        Some(id) => match index.by_contrato.get(&id) {
            Some(bucket) => bucket.iter().filter(apply_filters).copied().collect(),
            None => Vec::new(),
        },
        None => index.all.iter().filter(|p| apply_filters(p)).collect(),
    }
}

pub struct PagoIndexClean<'a> {
    pub by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    pub all: &'a [Pago],
}

impl<'a> PagoIndexClean<'a> {
    pub fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 25);

        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_insert_with(|| Vec::with_capacity(25))
                .push(pago);
        }

        Self {
            by_contrato,
            all: pagos,
        }
    }
}

// =============================================================================
// Approach 3: Pre-indexed + sorted by date within each bucket
// =============================================================================

/// Extends the HashMap index by sorting each bucket by fecha_vencimiento.
/// This enables binary search for date range queries within a contrato bucket,
/// reducing from O(k) to O(log k + result_size) for date-filtered queries.
pub struct PagoIndexSorted<'a> {
    pub by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    pub all_sorted: Vec<&'a Pago>,
}

impl<'a> PagoIndexSorted<'a> {
    pub fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 25);

        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_insert_with(|| Vec::with_capacity(25))
                .push(pago);
        }

        // Sort each bucket by date for binary search
        for bucket in by_contrato.values_mut() {
            bucket.sort_unstable_by_key(|p| p.fecha_vencimiento);
        }

        // Also keep a globally sorted view for no-contrato queries
        let mut all_sorted: Vec<&Pago> = pagos.iter().collect();
        all_sorted.sort_unstable_by_key(|p| p.fecha_vencimiento);

        Self {
            by_contrato,
            all_sorted,
        }
    }

    pub fn buscar(
        &self,
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        let slice: &[&Pago] = match contrato_id {
            Some(id) => match self.by_contrato.get(&id) {
                Some(bucket) => bucket.as_slice(),
                None => return Vec::new(),
            },
            None => self.all_sorted.as_slice(),
        };

        // If we have date bounds and the slice is sorted, use binary search to narrow
        let start = match fecha_desde {
            Some(d) => slice.partition_point(|p| p.fecha_vencimiento < d),
            None => 0,
        };
        let end = match fecha_hasta {
            Some(d) => slice.partition_point(|p| p.fecha_vencimiento <= d),
            None => slice.len(),
        };

        slice[start..end]
            .iter()
            .filter(|p| {
                estado.map_or(true, |e| p.estado == e)
                    && referencia.map_or(true, |r| {
                        p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                    })
            })
            .copied()
            .collect()
    }
}
