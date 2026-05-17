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

// ─── Linear scan (current implementation) ───────────────────────────────────

/// Current implementation: linear scan with chained filters.
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

// ─── Indexed implementation (pre-indexed by contrato_id) ────────────────────

/// Pre-built index that groups payments by contrato_id for O(1) lookup
/// of the most common query pattern (~80% of queries filter by contrato_id).
pub struct PagoIndex {
    /// All payments stored for fallback linear scan
    all_pagos: Vec<Pago>,
    /// HashMap index: contrato_id -> indices into all_pagos
    by_contrato: HashMap<Uuid, Vec<usize>>,
}

impl PagoIndex {
    /// Build the index from a slice of payments. This is a one-time cost
    /// amortized over many queries (index built once, queried on every page load).
    pub fn new(pagos: Vec<Pago>) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<usize>> = HashMap::with_capacity(pagos.len() / 10);
        for (i, pago) in pagos.iter().enumerate() {
            by_contrato.entry(pago.contrato_id).or_default().push(i);
        }
        Self {
            all_pagos: pagos,
            by_contrato,
        }
    }

    /// Indexed search: when contrato_id is provided, uses the HashMap for O(1)
    /// lookup then applies remaining filters only to the subset.
    /// Falls back to linear scan when no contrato_id filter is given.
    pub fn buscar_pagos(
        &self,
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&Pago> {
        let candidates: Box<dyn Iterator<Item = &Pago>> = match contrato_id {
            Some(id) => {
                if let Some(indices) = self.by_contrato.get(&id) {
                    Box::new(indices.iter().map(|&i| &self.all_pagos[i]))
                } else {
                    return Vec::new();
                }
            }
            None => Box::new(self.all_pagos.iter()),
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
            .collect()
    }
}

// ─── Test data generation ───────────────────────────────────────────────────

/// Generate realistic test data: ~5000 payments across ~100 contracts
pub fn generate_test_data(num_pagos: usize, num_contratos: usize) -> Vec<Pago> {
    use chrono::NaiveDate;
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let contratos: Vec<Uuid> = (0..num_contratos).map(|_| Uuid::new_v4()).collect();
    let estados = ["pendiente", "pagado", "atrasado"];
    let base_date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap_or_default();

    let mut pagos = Vec::with_capacity(num_pagos);
    for _ in 0..num_pagos {
        let contrato_id = contratos[rng.gen_range(0..num_contratos)];
        let days_offset = rng.gen_range(0..730); // 2 years of data
        let fecha_vencimiento = base_date + chrono::Duration::days(days_offset);
        let estado = estados[rng.gen_range(0..3)].to_string();

        let fecha_pago = if estado == "pagado" {
            Some(fecha_vencimiento - chrono::Duration::days(rng.gen_range(0..5)))
        } else {
            None
        };

        let referencia = if rng.gen_bool(0.3) {
            Some(format!("REF-{:04}", rng.gen_range(1000..9999)))
        } else {
            None
        };

        pagos.push(Pago {
            id: Uuid::new_v4(),
            contrato_id,
            monto: rng.gen_range(5000.0..50000.0),
            fecha_vencimiento,
            fecha_pago,
            estado,
            referencia,
        });
    }
    pagos
}
