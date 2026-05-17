use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

/// Detects overlapping active contracts for a given propiedad.
/// Called on every contract creation/update as a validation step.
///
/// Complexity: O(n²) pairwise comparison.
/// At n=200 (bulk import worst case) this performs ~19,900 lightweight comparisons,
/// completing in sub-millisecond time. No optimization needed for this workload.
///
/// A sort-based approach would NOT be correct here without a sweep-line algorithm,
/// since adjacent-only checks miss non-adjacent overlaps.
pub fn detectar_solapamientos(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut solapamientos = Vec::new();

    for (i, a) in contratos.iter().enumerate() {
        if a.estado != "activo" {
            continue;
        }

        for b in &contratos[i + 1..] {
            if b.estado != "activo" {
                continue;
            }

            let is_same_propiedad = a.propiedad_id == b.propiedad_id;
            let is_overlapping = a.fecha_inicio <= b.fecha_fin && b.fecha_inicio <= a.fecha_fin;

            if is_same_propiedad && is_overlapping {
                solapamientos.push((a.id, b.id));
            }
        }
    }

    solapamientos
}
