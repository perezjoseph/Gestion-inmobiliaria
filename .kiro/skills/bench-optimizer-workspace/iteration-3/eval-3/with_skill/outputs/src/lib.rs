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
// Approach 1: Current implementation — linear scan with chained filters
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
// Approach 2: Pre-indexed HashMap by contrato_id
// =============================================================================

/// Pre-built index that groups payments by contrato_id for O(1) lookup on the
/// most common filter (80% of queries). Falls back to full scan when no
/// contrato_id filter is provided.
pub struct PagoIndex<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    all: &'a [Pago],
}

impl<'a> PagoIndex<'a> {
    pub fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 10);
        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_with_capacity(4)
                .push(pago);
        }
        Self {
            by_contrato,
            all: pagos,
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
        let candidates: Box<dyn Iterator<Item = &&Pago>> = match contrato_id {
            Some(id) => match self.by_contrato.get(&id) {
                Some(vec) => Box::new(vec.iter()),
                None => return Vec::new(),
            },
            None => Box::new(self.all.iter().map(|p| p).collect::<Vec<_>>().leak().iter()),
        };

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
// Approach 3: Pre-indexed HashMap — simpler version without trait objects
// =============================================================================

/// Simpler indexed search that avoids dynamic dispatch. When contrato_id is
/// provided, searches only the relevant bucket. Otherwise falls back to linear.
pub struct PagoIndexSimple<'a> {
    by_contrato: HashMap<Uuid, Vec<&'a Pago>>,
    all: &'a [Pago],
}

impl<'a> PagoIndexSimple<'a> {
    pub fn new(pagos: &'a [Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<&'a Pago>> =
            HashMap::with_capacity(pagos.len() / 10);
        for pago in pagos {
            by_contrato
                .entry(pago.contrato_id)
                .or_with_capacity(4)
                .push(pago);
        }
        Self {
            by_contrato,
            all: pagos,
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
        if let Some(id) = contrato_id {
            // Fast path: use the index
            let Some(bucket) = self.by_contrato.get(&id) else {
                return Vec::new();
            };
            bucket
                .iter()
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
        } else {
            // Fallback: linear scan (no contrato_id filter)
            buscar_pagos_linear(self.all, None, estado, fecha_desde, fecha_hasta, referencia)
        }
    }
}

// =============================================================================
// Data generation for benchmarks
// =============================================================================

pub fn generate_realistic_pagos(n: usize, num_contratos: usize) -> Vec<Pago> {
    use chrono::NaiveDate;
    use rand::Rng;

    let mut rng = rand::thread_rng();

    // Pre-generate a fixed set of contrato IDs to create realistic distribution
    let contrato_ids: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();

    let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

    (0..n)
        .map(|_| {
            let days_offset = rng.gen_range(0..730); // 2 years of dates
            let fecha_vencimiento = base_date + chrono::Duration::days(days_offset);
            let is_paid = rng.gen_range(0..10) < 7; // 70% paid

            Pago {
                id: Uuid::new_v4(),
                contrato_id: contrato_ids[rng.gen_range(0..num_contratos)],
                monto: rng.gen_range(5000.0..50000.0),
                fecha_vencimiento,
                fecha_pago: if is_paid {
                    Some(fecha_vencimiento - chrono::Duration::days(rng.gen_range(0..5)))
                } else {
                    None
                },
                estado: match rng.gen_range(0..10) {
                    0 => "atrasado".to_string(),
                    1..=2 => "pendiente".to_string(),
                    _ => "pagado".to_string(),
                },
                referencia: if rng.gen_range(0..3) == 0 {
                    Some(format!("REF-{:04}", rng.gen_range(1000..9999)))
                } else {
                    None
                },
            }
        })
        .collect()
}
