//! Overlap detection — performance analysis conclusion:
//!
//! KEEP THE CURRENT O(n²) IMPLEMENTATION.
//!
//! Rationale:
//! - At n=200 (bulk import worst case), we do ~19,900 comparisons of simple
//!   date/string checks. This completes in ~10-50 µs on modern hardware.
//! - A sort-based approach (O(n log n)) requires a sweep-line algorithm to
//!   correctly find ALL overlapping pairs (not just adjacent ones after sorting).
//!   This adds complexity for no measurable benefit at this scale.
//! - The real bottleneck during bulk import is database I/O, not this function.
//!
//! Revisit only if n regularly exceeds 10,000+.

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
///
/// Complexity: O(n²) pairwise comparison.
/// At n=200 (bulk import ceiling), this completes in microseconds.
/// No optimization needed for current and projected workloads.
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
