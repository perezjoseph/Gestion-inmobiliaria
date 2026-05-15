use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::{HashMap, HashSet, VecDeque};
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

/// Busca inquilinos por cédula usando un HashMap preconstruido.
/// Paradigm: Space-Time Tradeoff — precompute lookup table for O(1) access.
/// Before: O(n) linear scan per lookup.
/// After: O(1) amortized per lookup (O(n) to build the index once).
pub fn construir_indice_cedulas(inquilinos: &[Inquilino]) -> HashMap<&str, &Inquilino> {
    inquilinos.iter().map(|i| (i.cedula.as_str(), i)).collect()
}

pub fn buscar_inquilino_por_cedula<'a>(
    indice: &'a HashMap<&str, &'a Inquilino>,
    cedula: &str,
) -> Option<&'a Inquilino> {
    indice.get(cedula).copied()
}

/// Verifica si un conjunto de cédulas ya existen en el sistema.
/// Paradigm: Space-Time Tradeoff — HashSet for O(1) membership testing.
/// Before: O(n*m) — for each new cédula, scans the entire inquilinos list.
/// After: O(n + m) — build HashSet in O(n), check m cédulas in O(m).
pub fn cedulas_duplicadas(inquilinos: &[Inquilino], cedulas_nuevas: &[String]) -> Vec<String> {
    let existentes: HashSet<&str> = inquilinos.iter().map(|i| i.cedula.as_str()).collect();

    cedulas_nuevas
        .iter()
        .filter(|cedula| existentes.contains(cedula.as_str()))
        .cloned()
        .collect()
}

/// Agrupa contratos por inquilino usando HashMap.
/// Paradigm: Transform and Conquer — Representation Change.
/// Before: BTreeMap with O(log n) insert/lookup; ordering never used.
/// After: HashMap with O(1) amortized insert/lookup.
pub fn contratos_por_inquilino(
    contratos: &[ContratoActivo],
) -> HashMap<Uuid, Vec<&ContratoActivo>> {
    let mut mapa: HashMap<Uuid, Vec<&ContratoActivo>> = HashMap::with_capacity(contratos.len());
    for contrato in contratos {
        mapa.entry(contrato.inquilino_id)
            .or_default()
            .push(contrato);
    }
    mapa
}

/// Calcula el ingreso total por propiedad.
/// Paradigm: Space-Time Tradeoff — single-pass accumulation into HashMap.
/// Before: O(n²) — for each contrato, re-scans all contratos filtering by propiedad_id.
///         Also used BTreeMap (unnecessary ordering) and intermediate Vec allocation.
/// After: O(n) — single pass accumulating into HashMap, no intermediate allocation.
pub fn ingreso_por_propiedad(contratos: &[ContratoActivo]) -> HashMap<Uuid, f64> {
    let mut totales: HashMap<Uuid, f64> = HashMap::with_capacity(contratos.len());

    for contrato in contratos {
        *totales.entry(contrato.propiedad_id).or_default() += contrato.monto_mensual;
    }

    totales
}

/// Encuentra los top-N inquilinos por monto total de contratos.
/// Paradigm: Decrease and Conquer — partial sort using BinaryHeap (min-heap of size k).
/// Before: O(n log n) full sort of all entries, then truncate to N.
///         Also used BTreeMap (unnecessary ordering).
/// After: O(n + k log k) — single pass to accumulate totals in HashMap,
///        then use a min-heap of size k to extract top-N without full sort.
pub fn top_inquilinos(contratos: &[ContratoActivo], n: usize) -> Vec<(Uuid, f64)> {
    let mut totales: HashMap<Uuid, f64> = HashMap::with_capacity(contratos.len());
    for contrato in contratos {
        *totales.entry(contrato.inquilino_id).or_default() += contrato.monto_mensual;
    }

    if n == 0 {
        return Vec::new();
    }

    // Min-heap of size n: keeps the top-n largest values
    // Reverse wraps f64 ordering so BinaryHeap acts as a min-heap
    let mut heap: BinaryHeap<Reverse<(OrderedFloat, Uuid)>> = BinaryHeap::with_capacity(n + 1);

    for (id, total) in totales {
        heap.push(Reverse((OrderedFloat(total), id)));
        if heap.len() > n {
            heap.pop(); // remove the smallest, keeping top-n
        }
    }

    // Extract results sorted descending by total
    let mut result: Vec<(Uuid, f64)> = heap
        .into_iter()
        .map(|Reverse((OrderedFloat(total), id))| (id, total))
        .collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Wrapper for f64 to implement Ord for use in BinaryHeap.
/// Treats NaN as less than all other values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Procesa una cola de solicitudes de mantenimiento.
/// Paradigm: Transform and Conquer — Representation Change.
/// Before: Vec with insert(0, ...) — O(n) per enqueue due to element shifting.
/// After: VecDeque — O(1) amortized for both push_back and pop_front.
pub struct ColaSolicitudes {
    items: VecDeque<String>,
}

impl ColaSolicitudes {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    pub fn encolar(&mut self, solicitud: String) {
        self.items.push_back(solicitud);
    }

    pub fn desencolar(&mut self) -> Option<String> {
        self.items.pop_front()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
