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
/// BUG: O(p * c * g) triple nested loop — for each propiedad, scans all contratos,
/// and for each contrato scans all pagos.
pub fn generar_resumen(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<ResumenPropiedad> {
    let mut resultados = Vec::new();

    for propiedad in propiedades {
        let mut total_contratos = 0;
        let mut total_pagado = 0.0;
        let mut total_pendiente = 0.0;

        for contrato in contratos {
            if contrato.propiedad_id == propiedad.id {
                total_contratos += 1;

                for pago in pagos {
                    if pago.contrato_id == contrato.id {
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
/// BUG: O(p * c * g) — nested search instead of building lookup maps.
pub fn propiedades_con_atrasos(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
    pagos: &[Pago],
) -> Vec<Uuid> {
    let mut resultado = Vec::new();

    for propiedad in propiedades {
        let mut tiene_atraso = false;
        for contrato in contratos {
            if contrato.propiedad_id == propiedad.id && contrato.estado == "activo" {
                for pago in pagos {
                    if pago.contrato_id == contrato.id && pago.estado == "atrasado" {
                        tiene_atraso = true;
                        break;
                    }
                }
            }
            if tiene_atraso {
                break;
            }
        }
        if tiene_atraso {
            resultado.push(propiedad.id);
        }
    }

    resultado
}

/// Calcula la tasa de ocupación por ciudad.
/// BUG: For each unique city, re-scans the entire propiedades list.
pub fn ocupacion_por_ciudad(propiedades: &[Propiedad]) -> Vec<(String, f64)> {
    // First collect unique cities — O(n) scan
    let mut ciudades: Vec<String> = propiedades.iter().map(|p| p.ciudad.clone()).collect();
    ciudades.sort();
    ciudades.dedup();

    // For each city, scan all propiedades again — O(cities * n)
    ciudades
        .iter()
        .map(|ciudad| {
            let total = propiedades.iter().filter(|p| &p.ciudad == ciudad).count();
            let ocupadas = propiedades
                .iter()
                .filter(|p| &p.ciudad == ciudad && p.estado == "ocupada")
                .count();
            let tasa = if total > 0 {
                ocupadas as f64 / total as f64
            } else {
                0.0
            };
            (ciudad.clone(), tasa)
        })
        .collect()
}
