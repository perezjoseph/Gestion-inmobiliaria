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

// ─── Approach 1: Current linear scan ───────────────────────────────────────────

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

// ─── Approach 2: Pre-indexed by contrato_id (HashMap) ──────────────────────────

/// Pre-built index that groups payments by contrato_id.
/// Build cost is amortized across many queries (the index lives as long as the dataset).
pub struct PagoIndex {
    by_contrato: HashMap<Uuid, Vec<usize>>,
    all_indices: Vec<usize>,
}

impl PagoIndex {
    pub fn build(pagos: &[Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<usize>> = HashMap::new();
        let mut all_indices = Vec::with_capacity(pagos.len());

        for (i, pago) in pagos.iter().enumerate() {
            by_contrato.entry(pago.contrato_id).or_default().push(i);
            all_indices.push(i);
        }

        Self {
            by_contrato,
            all_indices,
        }
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
        // If contrato_id is specified, use the index to narrow candidates.
        // Otherwise fall back to scanning all indices.
        let candidates: &[usize] = match contrato_id {
            Some(id) => self.by_contrato.get(&id).map_or(&[], |v| v.as_slice()),
            None => &self.all_indices,
        };

        candidates
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
}

// ─── Approach 3: Pre-indexed + pre-sorted by date within each contrato ─────────

/// Enhanced index: groups by contrato_id AND sorts each group by fecha_vencimiento.
/// Enables binary search for date range queries within a contrato.
pub struct PagoIndexSorted {
    by_contrato: HashMap<Uuid, Vec<usize>>,
    all_sorted_by_date: Vec<usize>,
}

impl PagoIndexSorted {
    pub fn build(pagos: &[Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<usize>> = HashMap::new();

        for (i, pago) in pagos.iter().enumerate() {
            by_contrato.entry(pago.contrato_id).or_default().push(i);
        }

        // Sort each group by fecha_vencimiento for binary search
        for indices in by_contrato.values_mut() {
            indices.sort_by_key(|&i| pagos[i].fecha_vencimiento);
        }

        // Also maintain a global sorted list for queries without contrato_id
        let mut all_sorted_by_date: Vec<usize> = (0..pagos.len()).collect();
        all_sorted_by_date.sort_by_key(|&i| pagos[i].fecha_vencimiento);

        Self {
            by_contrato,
            all_sorted_by_date,
        }
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
        let candidates: &[usize] = match contrato_id {
            Some(id) => self.by_contrato.get(&id).map_or(&[], |v| v.as_slice()),
            None => &self.all_sorted_by_date,
        };

        // If we have date bounds and the list is sorted by date, use binary search
        // to narrow the scan range.
        let slice = if fecha_desde.is_some() || fecha_hasta.is_some() {
            let start = match fecha_desde {
                Some(d) => candidates.partition_point(|&i| pagos[i].fecha_vencimiento < d),
                None => 0,
            };
            let end = match fecha_hasta {
                Some(d) => candidates.partition_point(|&i| pagos[i].fecha_vencimiento <= d),
                None => candidates.len(),
            };
            &candidates[start..end]
        } else {
            candidates
        };

        slice
            .iter()
            .map(|&i| &pagos[i])
            .filter(|p| {
                estado.map_or(true, |e| p.estado == e)
                    && referencia.map_or(true, |r| {
                        p.referencia.as_ref().map_or(false, |pr| pr.contains(r))
                    })
            })
            .collect()
    }
}

// ─── Test helpers ──────────────────────────────────────────────────────────────

pub mod testdata {
    use super::Pago;
    use chrono::NaiveDate;
    use rand::Rng;
    use uuid::Uuid;

    /// Generate realistic payment data matching production characteristics:
    /// - ~5000 pagos total
    /// - ~200 distinct contratos (so ~25 pagos per contrato on average)
    /// - 70% pagado, 20% pendiente, 10% atrasado
    /// - Dates span 24 months
    /// - ~30% have a referencia string
    pub fn generate_pagos(n: usize, num_contratos: usize) -> Vec<Pago> {
        let mut rng = rand::thread_rng();
        let contratos: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();
        let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();

        (0..n)
            .map(|_| {
                let contrato_id = contratos[rng.gen_range(0..num_contratos)];
                let days_offset = rng.gen_range(0..730); // 24 months
                let fecha_vencimiento = base_date + chrono::Days::new(days_offset);
                let estado = match rng.gen_range(0u8..10) {
                    0 => "atrasado".to_string(),
                    1..=2 => "pendiente".to_string(),
                    _ => "pagado".to_string(),
                };
                let fecha_pago = if estado == "pagado" {
                    Some(fecha_vencimiento - chrono::Days::new(rng.gen_range(0..5)))
                } else {
                    None
                };
                let referencia = if rng.gen_range(0u8..10) < 3 {
                    Some(format!("REF-{:06}", rng.gen_range(0u32..999_999)))
                } else {
                    None
                };

                Pago {
                    id: Uuid::new_v4(),
                    contrato_id,
                    monto: rng.gen_range(5000.0..50000.0),
                    fecha_vencimiento,
                    fecha_pago,
                    estado,
                    referencia,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// Verify all three approaches return the same results for the same query.
    #[test]
    fn all_approaches_agree() {
        let pagos = testdata::generate_pagos(500, 50);
        let index = PagoIndex::build(&pagos);
        let index_sorted = PagoIndexSorted::build(&pagos);

        // Pick a contrato_id that exists
        let target_contrato = pagos[0].contrato_id;
        let fecha_desde = Some(NaiveDate::from_ymd_opt(2023, 6, 1).unwrap());
        let fecha_hasta = Some(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());

        let mut r1 = buscar_pagos_linear(
            &pagos,
            Some(target_contrato),
            Some("pagado"),
            fecha_desde,
            fecha_hasta,
            None,
        );
        let mut r2 = index.buscar(
            &pagos,
            Some(target_contrato),
            Some("pagado"),
            fecha_desde,
            fecha_hasta,
            None,
        );
        let mut r3 = index_sorted.buscar(
            &pagos,
            Some(target_contrato),
            Some("pagado"),
            fecha_desde,
            fecha_hasta,
            None,
        );

        // Sort by id for comparison
        r1.sort_by_key(|p| p.id);
        r2.sort_by_key(|p| p.id);
        r3.sort_by_key(|p| p.id);

        assert_eq!(r1.len(), r2.len());
        assert_eq!(r1.len(), r3.len());
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.id, b.id);
        }
        for (a, b) in r1.iter().zip(r3.iter()) {
            assert_eq!(a.id, b.id);
        }
    }
}
