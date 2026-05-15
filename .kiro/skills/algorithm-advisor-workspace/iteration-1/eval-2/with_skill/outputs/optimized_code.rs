use std::collections::HashMap;
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
/// Optimized: O(p + c + g) using HashMap lookups instead of O(p * c * g) triple nested loop.
/// Paradigm: Space-Time Tradeoff — precompute lookup tables for contratos and pagos.
pub fn generar_resumen(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<ResumenPropiedad> {
    // Step 1: Build pagos lookup by contrato_id — O(g)
    let pagos_by_contrato: HashMap<Uuid, Vec<&Pago>> =
        pagos.iter().fold(HashMap::new(), |mut map, pago| {
            map.entry(pago.contrato_id).or_default().push(pago);
            map
        });

    // Step 2: Build contratos lookup by propiedad_id — O(c)
    let contratos_by_propiedad: HashMap<Uuid, Vec<&Contrato>> =
        contratos.iter().fold(HashMap::new(), |mut map, contrato| {
            map.entry(contrato.propiedad_id).or_default().push(contrato);
            map
        });

    // Step 3: Single pass over propiedades with O(1) lookups — O(p + c + g) total
    let mut resultados = Vec::with_capacity(propiedades.len());

    for propiedad in propiedades {
        let mut total_contratos = 0;
        let mut total_pagado = 0.0;
        let mut total_pendiente = 0.0;

        if let Some(prop_contratos) = contratos_by_propiedad.get(&propiedad.id) {
            total_contratos = prop_contratos.len();

            for contrato in prop_contratos {
                if let Some(contrato_pagos) = pagos_by_contrato.get(&contrato.id) {
                    for pago in contrato_pagos {
                        match pago.estado.as_str() {
                            "pagado" => total_pagado += pago.monto,
                            "pendiente" | "atrasado" => total_pendiente += pago.monto,
                            _ => {}
                        }
                    }
                }
            }
        }

        resultados.push(ResumenPropiedad {
            propiedad_titulo: propiedad.titulo.clone(),
            total_contratos,
            total_pagado,
            total_pendiente,
        });
    }

    resultados
}

/// Encuentra propiedades que tienen contratos con pagos atrasados.
/// Optimized: O(p + c + g) using HashMap lookups instead of O(p * c * g) triple nested loop.
/// Paradigm: Space-Time Tradeoff — precompute a set of contrato IDs with atrasos,
/// then a set of propiedad IDs with active contratos that have atrasos.
pub fn propiedades_con_atrasos(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<Uuid> {
    // Step 1: Collect contrato IDs that have at least one pago atrasado — O(g)
    let contratos_con_atraso: std::collections::HashSet<Uuid> = pagos
        .iter()
        .filter(|p| p.estado == "atrasado")
        .map(|p| p.contrato_id)
        .collect();

    // Step 2: Collect propiedad IDs that have an active contrato with atraso — O(c)
    let propiedades_con_atraso: std::collections::HashSet<Uuid> = contratos
        .iter()
        .filter(|c| c.estado == "activo" && contratos_con_atraso.contains(&c.id))
        .map(|c| c.propiedad_id)
        .collect();

    // Step 3: Filter propiedades preserving input order — O(p)
    propiedades
        .iter()
        .filter(|p| propiedades_con_atraso.contains(&p.id))
        .map(|p| p.id)
        .collect()
}

/// Calcula la tasa de ocupación por ciudad.
/// Optimized: O(n) single pass with HashMap aggregation instead of O(cities * n).
/// Paradigm: Space-Time Tradeoff — aggregate counts in a single pass using HashMap.
pub fn ocupacion_por_ciudad(propiedades: &[Propiedad]) -> Vec<(String, f64)> {
    // Single pass: accumulate (total, ocupadas) per ciudad — O(n)
    let mut stats: HashMap<&str, (usize, usize)> = HashMap::new();

    for propiedad in propiedades {
        let entry = stats.entry(propiedad.ciudad.as_str()).or_insert((0, 0));
        entry.0 += 1;
        if propiedad.estado == "ocupada" {
            entry.1 += 1;
        }
    }

    // Convert to sorted output for deterministic results — O(cities * log(cities))
    let mut resultado: Vec<(String, f64)> = stats
        .into_iter()
        .map(|(ciudad, (total, ocupadas))| {
            let tasa = if total > 0 {
                ocupadas as f64 / total as f64
            } else {
                0.0
            };
            (ciudad.to_owned(), tasa)
        })
        .collect();

    resultado.sort_by(|a, b| a.0.cmp(&b.0));
    resultado
}
