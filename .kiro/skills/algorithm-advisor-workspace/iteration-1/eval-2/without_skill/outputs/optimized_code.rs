use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Propiedad {
    pub id: Uuid,
    pub titulo: String,
    pub ciudad: String,
    pub estado: String,
}

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
    pub estado: String,
}

#[derive(Debug, Clone)]
pub struct ResumenPropiedad {
    pub propiedad_titulo: String,
    pub total_contratos: usize,
    pub total_pagado: f64,
    pub total_pendiente: f64,
}

/// Genera un resumen por propiedad con totales de pagos.
/// Optimized from O(p * c * g) to O(p + c + g) using HashMap lookups.
pub fn generar_resumen(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<ResumenPropiedad> {
    // Pre-aggregate pagos by contrato_id: O(g)
    let mut pagos_por_contrato: HashMap<Uuid, (f64, f64)> = HashMap::with_capacity(contratos.len());
    for pago in pagos {
        let entry = pagos_por_contrato
            .entry(pago.contrato_id)
            .or_insert((0.0, 0.0));
        match pago.estado.as_str() {
            "pagado" => entry.0 += pago.monto,
            "pendiente" | "atrasado" => entry.1 += pago.monto,
            _ => {}
        }
    }

    // Group contratos by propiedad_id: O(c)
    let mut contratos_por_propiedad: HashMap<Uuid, Vec<&Contrato>> =
        HashMap::with_capacity(propiedades.len());
    for contrato in contratos {
        contratos_por_propiedad
            .entry(contrato.propiedad_id)
            .or_default()
            .push(contrato);
    }

    // Build results with O(1) lookups per contrato: O(p + c)
    propiedades
        .iter()
        .map(|propiedad| {
            let (total_contratos, total_pagado, total_pendiente) = contratos_por_propiedad
                .get(&propiedad.id)
                .map(|cs| {
                    let count = cs.len();
                    let (pagado, pendiente) = cs.iter().fold((0.0, 0.0), |(p, pend), c| {
                        let (cp, cpend) =
                            pagos_por_contrato.get(&c.id).copied().unwrap_or((0.0, 0.0));
                        (p + cp, pend + cpend)
                    });
                    (count, pagado, pendiente)
                })
                .unwrap_or((0, 0.0, 0.0));

            ResumenPropiedad {
                propiedad_titulo: propiedad.titulo.clone(),
                total_contratos,
                total_pagado,
                total_pendiente,
            }
        })
        .collect()
}

/// Encuentra propiedades que tienen contratos activos con pagos atrasados.
/// Optimized from O(p * c * g) to O(p + c + g) using HashSet lookups.
pub fn propiedades_con_atrasos(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<Uuid> {
    // Collect contrato_ids that have at least one pago atrasado: O(g)
    let contratos_con_atraso: HashSet<Uuid> = pagos
        .iter()
        .filter(|p| p.estado == "atrasado")
        .map(|p| p.contrato_id)
        .collect();

    // Group active contratos by propiedad_id: O(c)
    let mut contratos_activos_por_propiedad: HashMap<Uuid, Vec<&Contrato>> =
        HashMap::with_capacity(propiedades.len());
    for contrato in contratos {
        if contrato.estado == "activo" {
            contratos_activos_por_propiedad
                .entry(contrato.propiedad_id)
                .or_default()
                .push(contrato);
        }
    }

    // For each propiedad, check if any active contrato has atraso: O(p + c) amortized
    propiedades
        .iter()
        .filter(|propiedad| {
            contratos_activos_por_propiedad
                .get(&propiedad.id)
                .is_some_and(|cs| cs.iter().any(|c| contratos_con_atraso.contains(&c.id)))
        })
        .map(|p| p.id)
        .collect()
}

/// Calcula la tasa de ocupación por ciudad.
/// Optimized from O(cities * n) to O(n) using single-pass grouping.
pub fn ocupacion_por_ciudad(propiedades: &[Propiedad]) -> Vec<(String, f64)> {
    // Single pass: group counts by ciudad: O(n)
    let mut por_ciudad: HashMap<&str, (usize, usize)> = HashMap::new();
    for propiedad in propiedades {
        let entry = por_ciudad
            .entry(propiedad.ciudad.as_str())
            .or_insert((0, 0));
        entry.0 += 1;
        if propiedad.estado == "ocupada" {
            entry.1 += 1;
        }
    }

    // Convert to result vec: O(cities)
    por_ciudad
        .into_iter()
        .map(|(ciudad, (total, ocupadas))| {
            let tasa = if total > 0 {
                ocupadas as f64 / total as f64
            } else {
                0.0
            };
            (ciudad.to_owned(), tasa)
        })
        .collect()
}
