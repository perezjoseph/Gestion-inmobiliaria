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

// === Approach A: Linear scan (current implementation) ===

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

// === Approach B: Pre-indexed HashMap by contrato_id ===

/// Index that groups payment indices by contrato_id for O(1) lookup of the subset.
pub struct PagoIndex {
    by_contrato: HashMap<Uuid, Vec<usize>>,
}

impl PagoIndex {
    pub fn build(pagos: &[Pago]) -> Self {
        let mut by_contrato: HashMap<Uuid, Vec<usize>> = HashMap::with_capacity(pagos.len() / 10);
        for (i, pago) in pagos.iter().enumerate() {
            by_contrato.entry(pago.contrato_id).or_default().push(i);
        }
        Self { by_contrato }
    }

    pub fn buscar<'a>(
        &'a self,
        pagos: &'a [Pago],
        contrato_id: Option<Uuid>,
        estado: Option<&str>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        referencia: Option<&str>,
    ) -> Vec<&'a Pago> {
        let candidates: Box<dyn Iterator<Item = &'a Pago> + 'a> = match contrato_id {
            Some(id) => {
                if let Some(indices) = self.by_contrato.get(&id) {
                    Box::new(indices.iter().map(move |&i| &pagos[i]))
                } else {
                    return Vec::new();
                }
            }
            None => Box::new(pagos.iter()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_test_data() -> (Vec<Pago>, Uuid) {
        let target_contrato = Uuid::new_v4();
        let pagos: Vec<Pago> = (0..100)
            .map(|i| Pago {
                id: Uuid::new_v4(),
                contrato_id: if i % 10 == 0 {
                    target_contrato
                } else {
                    Uuid::new_v4()
                },
                monto: 1000.0 + i as f64,
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .checked_add_signed(chrono::Duration::days(i))
                    .unwrap(),
                fecha_pago: None,
                estado: if i % 3 == 0 {
                    "pendiente".to_string()
                } else {
                    "pagado".to_string()
                },
                referencia: Some(format!("REF-{:04}", i)),
            })
            .collect();
        (pagos, target_contrato)
    }

    #[test]
    fn both_approaches_return_same_results() {
        let (pagos, target) = make_test_data();
        let index = PagoIndex::build(&pagos);

        let linear = buscar_pagos_linear(&pagos, Some(target), None, None, None, None);
        let indexed = index.buscar(&pagos, Some(target), None, None, None, None);

        assert_eq!(linear.len(), indexed.len());
        for (a, b) in linear.iter().zip(indexed.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn both_approaches_handle_no_filter() {
        let (pagos, _) = make_test_data();
        let index = PagoIndex::build(&pagos);

        let linear = buscar_pagos_linear(&pagos, None, None, None, None, None);
        let indexed = index.buscar(&pagos, None, None, None, None, None);

        assert_eq!(linear.len(), indexed.len());
    }
}
