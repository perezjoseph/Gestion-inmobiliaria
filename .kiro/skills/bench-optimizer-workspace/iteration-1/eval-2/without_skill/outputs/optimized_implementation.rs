//! Optimized overlap detection implementation.
//!
//! Benchmarks show this is only beneficial at n>100, and the production use case
//! is n=5-15 (where the original O(n²) is 2-2.5x faster).
//!
//! RECOMMENDATION: Do NOT use this. Keep the original O(n²) implementation.
//! See ANALYSIS.md for full benchmark data and reasoning.

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

/// Optimized: filter activos → group by propiedad → sort by fecha_inicio → scan with early break.
///
/// Complexity: O(n log n) for the sort, O(n) for the scan within each group.
/// Faster than O(n²) when n > ~100, but slower for small n due to allocation overhead.
pub fn detectar_solapamientos_optimized(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let activos: Vec<&Contrato> = contratos.iter().filter(|c| c.estado == "activo").collect();

    let mut por_propiedad: std::collections::HashMap<Uuid, Vec<&Contrato>> =
        std::collections::HashMap::new();
    for c in &activos {
        por_propiedad.entry(c.propiedad_id).or_default().push(c);
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, grupo) in &mut por_propiedad {
        if grupo.len() < 2 {
            continue;
        }

        grupo.sort_unstable_by_key(|c| c.fecha_inicio);

        for i in 0..grupo.len() {
            let a = grupo[i];
            for j in (i + 1)..grupo.len() {
                let b = grupo[j];
                // Since sorted by fecha_inicio, b.fecha_inicio >= a.fecha_inicio.
                // Overlap iff b.fecha_inicio <= a.fecha_fin
                if b.fecha_inicio <= a.fecha_fin {
                    solapamientos.push((a.id, b.id));
                } else {
                    break; // No further contracts can overlap with `a`
                }
            }
        }
    }

    solapamientos
}
