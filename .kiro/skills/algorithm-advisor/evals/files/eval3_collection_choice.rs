use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Inquilino {
    pub id: Uuid,
    pub cedula: String,
    pub nombre: String,
    pub apellido: String,
}

#[derive(Debug, Clone)]
pub struct ContratoActivo {
    pub id: Uuid,
    pub inquilino_id: Uuid,
    pub propiedad_id: Uuid,
    pub monto_mensual: f64,
}

/// Busca inquilinos por cédula en una lista.
/// BUG: Uses Vec with linear scan O(n) per lookup instead of HashMap O(1).
pub fn buscar_inquilino_por_cedula<'a>(
    inquilinos: &'a [Inquilino],
    cedula: &str,
) -> Option<&'a Inquilino> {
    inquilinos.iter().find(|i| i.cedula == cedula)
}

/// Verifica si un conjunto de cédulas ya existen en el sistema.
/// BUG: O(n*m) — for each cedula to check, scans the entire inquilinos list.
pub fn cedulas_duplicadas(inquilinos: &[Inquilino], cedulas_nuevas: &[String]) -> Vec<String> {
    cedulas_nuevas
        .iter()
        .filter(|cedula| inquilinos.iter().any(|i| &i.cedula == *cedula))
        .cloned()
        .collect()
}

/// Agrupa contratos por inquilino usando BTreeMap.
/// BUG: Uses BTreeMap (O(log n) insert/lookup) when ordering is never used —
/// the result is only accessed via .get() and .iter() without order requirements.
pub fn contratos_por_inquilino(
    contratos: &[ContratoActivo],
) -> BTreeMap<Uuid, Vec<&ContratoActivo>> {
    let mut mapa: BTreeMap<Uuid, Vec<&ContratoActivo>> = BTreeMap::new();
    for contrato in contratos {
        mapa.entry(contrato.inquilino_id)
            .or_default()
            .push(contrato);
    }
    mapa
}

/// Calcula el ingreso total por propiedad.
/// BUG: Uses BTreeMap when HashMap would suffice — no ordering needed.
/// Also collects into intermediate Vec before summing.
pub fn ingreso_por_propiedad(contratos: &[ContratoActivo]) -> BTreeMap<Uuid, f64> {
    let mut totales: BTreeMap<Uuid, f64> = BTreeMap::new();

    for contrato in contratos {
        let montos: Vec<f64> = contratos
            .iter()
            .filter(|c| c.propiedad_id == contrato.propiedad_id)
            .map(|c| c.monto_mensual)
            .collect();
        let total: f64 = montos.iter().sum();
        totales.insert(contrato.propiedad_id, total);
    }

    totales
}

/// Encuentra los top-N inquilinos por monto total de contratos.
/// BUG: Sorts the entire map just to take the first N elements.
/// A partial sort or selection algorithm would be O(n + k log k) instead of O(n log n).
pub fn top_inquilinos(contratos: &[ContratoActivo], n: usize) -> Vec<(Uuid, f64)> {
    let mut totales: BTreeMap<Uuid, f64> = BTreeMap::new();
    for contrato in contratos {
        *totales.entry(contrato.inquilino_id).or_default() += contrato.monto_mensual;
    }

    let mut sorted: Vec<(Uuid, f64)> = totales.into_iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(n);
    sorted
}

/// Procesa una cola de solicitudes de mantenimiento.
/// BUG: Uses Vec with insert(0, ...) for a FIFO queue — O(n) per insertion.
pub struct ColaSolicitudes {
    items: Vec<String>,
}

impl ColaSolicitudes {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn encolar(&mut self, solicitud: String) {
        self.items.insert(0, solicitud);
    }

    pub fn desencolar(&mut self) -> Option<String> {
        self.items.pop()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
