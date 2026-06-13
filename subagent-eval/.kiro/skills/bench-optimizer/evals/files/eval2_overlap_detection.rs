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
/// Production dataset: typically 5-15 active contracts per propiedad,
/// but during bulk import can be up to 200 contracts being validated at once.
///
/// Current implementation: O(n²) pairwise comparison.
/// Question: would sorting by fecha_inicio and scanning adjacent pairs be faster?
/// Or is the pairwise approach fine for this dataset size?
pub fn detectar_solapamientos(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let mut solapamientos = Vec::new();

    for i in 0..contratos.len() {
        for j in (i + 1)..contratos.len() {
            let a = &contratos[i];
            let b = &contratos[j];

            if a.propiedad_id == b.propiedad_id
                && a.estado == "activo"
                && b.estado == "activo"
                && a.fecha_inicio <= b.fecha_fin
                && b.fecha_inicio <= a.fecha_fin
            {
                solapamientos.push((a.id, b.id));
            }
        }
    }

    solapamientos
}
