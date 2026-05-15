use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
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

/// Busca inquilinos por cédula usando un HashMap indexado por cédula.
/// O(1) lookup en lugar de O(n) linear scan.
pub fn buscar_inquilino_por_cedula<'a>(
    inquilinos_por_cedula: &'a HashMap<String, Inquilino>,
    cedula: &str,
) -> Option<&'a Inquilino> {
    inquilinos_por_cedula.get(cedula)
}

/// Construye el índice de inquilinos por cédula para lookups O(1).
pub fn construir_indice_cedula(inquilinos: &[Inquilino]) -> HashMap<String, Inquilino> {
    inquilinos
        .iter()
        .map(|i| (i.cedula.clone(), i.clone()))
        .collect()
}

/// Verifica si un conjunto de cédulas ya existen en el sistema.
/// O(n + m) usando HashSet en lugar de O(n*m) con doble iteración.
pub fn cedulas_duplicadas(inquilinos: &[Inquilino], cedulas_nuevas: &[String]) -> Vec<String> {
    let existentes: HashSet<&str> = inquilinos.iter().map(|i| i.cedula.as_str()).collect();

    cedulas_nuevas
        .iter()
        .filter(|cedula| existentes.contains(cedula.as_str()))
        .cloned()
        .collect()
}

/// Agrupa contratos por inquilino usando HashMap.
/// O(1) amortizado por insert/lookup en lugar de O(log n) con BTreeMap.
/// No se requiere orden — solo acceso por clave e iteración.
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
/// Single-pass O(n) con HashMap en lugar de O(n²) con filtrado repetido + BTreeMap.
pub fn ingreso_por_propiedad(contratos: &[ContratoActivo]) -> HashMap<Uuid, f64> {
    let mut totales: HashMap<Uuid, f64> = HashMap::with_capacity(contratos.len());

    for contrato in contratos {
        *totales.entry(contrato.propiedad_id).or_default() += contrato.monto_mensual;
    }

    totales
}

/// Encuentra los top-N inquilinos por monto total de contratos.
/// Usa BinaryHeap con Reverse (min-heap de tamaño k) para O(n log k)
/// en lugar de sort completo O(n log n).
pub fn top_inquilinos(contratos: &[ContratoActivo], n: usize) -> Vec<(Uuid, f64)> {
    let mut totales: HashMap<Uuid, f64> = HashMap::with_capacity(contratos.len());
    for contrato in contratos {
        *totales.entry(contrato.inquilino_id).or_default() += contrato.monto_mensual;
    }

    // Min-heap de tamaño n para selección parcial O(n log k)
    let mut heap: BinaryHeap<Reverse<OrdF64Entry>> = BinaryHeap::with_capacity(n + 1);

    for (id, total) in totales {
        heap.push(Reverse(OrdF64Entry { monto: total, id }));
        if heap.len() > n {
            heap.pop();
        }
    }

    // Extraer y ordenar descendente
    let mut resultado: Vec<(Uuid, f64)> = heap
        .into_iter()
        .map(|Reverse(entry)| (entry.id, entry.monto))
        .collect();
    resultado.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    resultado
}

/// Wrapper para permitir Ord en f64 dentro del BinaryHeap.
#[derive(Debug, Clone, PartialEq)]
struct OrdF64Entry {
    monto: f64,
    id: Uuid,
}

impl Eq for OrdF64Entry {}

impl PartialOrd for OrdF64Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF64Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.monto
            .partial_cmp(&other.monto)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Procesa una cola de solicitudes de mantenimiento.
/// Usa VecDeque para FIFO O(1) en ambos extremos en lugar de Vec con insert(0) O(n).
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
