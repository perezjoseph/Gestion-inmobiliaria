use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub monto_mensual: f64,
    pub moneda: String,
    pub estado: String,
}

#[derive(Debug, Clone)]
pub struct Propiedad {
    pub id: Uuid,
    pub titulo: String,
    pub unidades: Vec<Unidad>,
}

#[derive(Debug, Clone)]
pub struct Unidad {
    pub id: Uuid,
    pub numero_unidad: String,
    pub estado: String,
}

/// Validates that no two active contracts for the same propiedad have overlapping date ranges.
/// Called on every contract creation/update — typically checking against 3-8 existing contracts
/// per propiedad (small n, called frequently).
///
/// Current approach: O(n²) pairwise comparison.
/// Question: Is this worth optimizing? What's the right approach given the constraints?
pub fn validar_solapamiento_contratos(contratos_activos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut solapamientos = Vec::new();

    for i in 0..contratos_activos.len() {
        for j in (i + 1)..contratos_activos.len() {
            let a = &contratos_activos[i];
            let b = &contratos_activos[j];

            if a.propiedad_id == b.propiedad_id
                && a.fecha_inicio <= b.fecha_fin
                && b.fecha_inicio <= a.fecha_fin
            {
                solapamientos.push((a.id, b.id));
            }
        }
    }

    solapamientos
}

/// Generates a rent roll report: for each propiedad, list all active contracts
/// with their monthly amounts, grouped by currency.
///
/// Called once per month for reporting. Dataset: ~50 propiedades, ~200 contracts.
/// The nested loop here looks like O(n²) but the grouping structure matters.
///
/// Current approach: builds a HashMap, then for each propiedad iterates its contracts.
/// Is there actually a problem here?
pub fn generar_rol_cobros(
    propiedades: &[Propiedad],
    contratos: &[Contrato],
) -> HashMap<Uuid, Vec<(String, f64)>> {
    let contratos_por_propiedad: HashMap<Uuid, Vec<&Contrato>> =
        contratos.iter().fold(HashMap::new(), |mut map, c| {
            map.entry(c.propiedad_id).or_default().push(c);
            map
        });

    let mut resultado: HashMap<Uuid, Vec<(String, f64)>> = HashMap::new();

    for propiedad in propiedades {
        if let Some(prop_contratos) = contratos_por_propiedad.get(&propiedad.id) {
            let montos: Vec<(String, f64)> = prop_contratos
                .iter()
                .filter(|c| c.estado == "activo")
                .map(|c| (c.moneda.clone(), c.monto_mensual))
                .collect();
            resultado.insert(propiedad.id, montos);
        }
    }

    resultado
}

/// Finds all propiedades where ALL units are occupied (fully occupied buildings).
/// Dataset: ~50 propiedades with 1-20 units each.
///
/// Current approach: for each propiedad, checks if all units have estado "ocupada".
/// Uses .all() which short-circuits on first non-match.
///
/// A "clever" approach might pre-index unit states, but is that actually better here?
pub fn propiedades_completamente_ocupadas(propiedades: &[Propiedad]) -> Vec<Uuid> {
    propiedades
        .iter()
        .filter(|p| !p.unidades.is_empty() && p.unidades.iter().all(|u| u.estado == "ocupada"))
        .map(|p| p.id)
        .collect()
}

/// Detects contracts that will expire in the next N days.
/// Called daily by a background job. Dataset: ~200 active contracts.
///
/// Current approach: linear scan with date comparison.
/// Someone suggested using a BTreeMap indexed by fecha_fin for "O(log n) range query".
/// Is that actually better for this use case?
pub fn contratos_por_vencer(contratos: &[Contrato], dias: i64) -> Vec<&Contrato> {
    let hoy = chrono::Local::now().date_naive();
    let limite = hoy + chrono::Duration::days(dias);

    contratos
        .iter()
        .filter(|c| c.estado == "activo" && c.fecha_fin >= hoy && c.fecha_fin <= limite)
        .collect()
}

/// Calculates the total monthly income per currency across all active contracts.
/// Called on dashboard load. Dataset: ~200 contracts.
///
/// Current approach: single pass with HashMap accumulation.
/// Is there anything wrong here?
pub fn ingreso_mensual_por_moneda(contratos: &[Contrato]) -> HashMap<String, f64> {
    let mut totales: HashMap<String, f64> = HashMap::new();

    for contrato in contratos.iter().filter(|c| c.estado == "activo") {
        *totales.entry(contrato.moneda.clone()).or_default() += contrato.monto_mensual;
    }

    totales
}
