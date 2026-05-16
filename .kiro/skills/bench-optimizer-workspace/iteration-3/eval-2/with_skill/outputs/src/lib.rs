use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

/// Current implementation: O(n²) pairwise comparison.
/// Checks all pairs for overlapping date ranges among active contracts
/// for the same propiedad.
pub fn detectar_solapamientos_pairwise(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
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

/// Approach 2: Filter active first, group by propiedad, then sort by fecha_inicio
/// within each group and scan adjacent pairs.
///
/// Complexity: O(n log n) due to sorting, but with lower constant factor for
/// the overlap detection phase (only adjacent comparisons needed for non-overlapping
/// detection). However, for detecting ALL overlaps (not just adjacent), we still
/// need a sweep-line approach.
///
/// Note: sorting + adjacent scan only finds adjacent overlaps. For ALL overlapping
/// pairs, we use a sweep-line: sort by start, then for each contract check against
/// all previous contracts whose end >= current start.
pub fn detectar_solapamientos_sort_sweep(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    let mut solapamientos = Vec::new();

    // Group active contracts by propiedad_id
    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    // For each propiedad, sort by fecha_inicio and sweep
    for (_propiedad_id, mut grupo) in por_propiedad {
        grupo.sort_unstable_by_key(|c| c.fecha_inicio);

        // Sweep: for each contract, check against all previous contracts
        // that haven't ended yet (fecha_fin >= current.fecha_inicio)
        for i in 1..grupo.len() {
            for j in 0..i {
                // Since sorted by fecha_inicio, we know grupo[j].fecha_inicio <= grupo[i].fecha_inicio
                // Overlap condition: grupo[j].fecha_fin >= grupo[i].fecha_inicio
                if grupo[j].fecha_fin >= grupo[i].fecha_inicio {
                    solapamientos.push((grupo[j].id, grupo[i].id));
                }
            }
        }
    }

    solapamientos
}

/// Approach 3: Pre-filter active contracts, then do pairwise only within
/// same propiedad groups. No sorting overhead, but avoids cross-propiedad comparisons.
///
/// This is the "minimal optimization" — same algorithm but skips the estado and
/// propiedad_id checks inside the inner loop by pre-grouping.
pub fn detectar_solapamientos_grouped(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    let mut solapamientos = Vec::new();

    // Group active contracts by propiedad_id
    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    // Pairwise within each group (no need to check propiedad_id or estado)
    for grupo in por_propiedad.values() {
        for i in 0..grupo.len() {
            for j in (i + 1)..grupo.len() {
                let a = grupo[i];
                let b = grupo[j];
                if a.fecha_inicio <= b.fecha_fin && b.fecha_inicio <= a.fecha_fin {
                    solapamientos.push((a.id, b.id));
                }
            }
        }
    }

    solapamientos
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_contrato(
        propiedad_id: Uuid,
        inicio: (i32, u32, u32),
        fin: (i32, u32, u32),
        estado: &str,
    ) -> Contrato {
        Contrato {
            id: Uuid::new_v4(),
            propiedad_id,
            fecha_inicio: NaiveDate::from_ymd_opt(inicio.0, inicio.1, inicio.2).unwrap(),
            fecha_fin: NaiveDate::from_ymd_opt(fin.0, fin.1, fin.2).unwrap(),
            estado: estado.to_string(),
        }
    }

    #[test]
    fn test_all_approaches_agree() {
        let prop_a = Uuid::new_v4();
        let prop_b = Uuid::new_v4();

        let contratos = vec![
            make_contrato(prop_a, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop_a, (2024, 5, 1), (2024, 12, 31), "activo"), // overlaps with [0]
            make_contrato(prop_a, (2025, 1, 1), (2025, 6, 30), "activo"),  // no overlap
            make_contrato(prop_a, (2024, 3, 1), (2024, 4, 30), "cancelado"), // inactive
            make_contrato(prop_b, (2024, 1, 1), (2024, 12, 31), "activo"), // different propiedad
        ];

        let mut r1 = detectar_solapamientos_pairwise(&contratos);
        let mut r2 = detectar_solapamientos_sort_sweep(&contratos);
        let mut r3 = detectar_solapamientos_grouped(&contratos);

        // Normalize: sort each pair and then sort the vec
        let normalize = |v: &mut Vec<(Uuid, Uuid)>| {
            for pair in v.iter_mut() {
                if pair.0 > pair.1 {
                    std::mem::swap(&mut pair.0, &mut pair.1);
                }
            }
            v.sort();
        };

        normalize(&mut r1);
        normalize(&mut r2);
        normalize(&mut r3);

        assert_eq!(r1.len(), 1, "Should find exactly 1 overlap");
        assert_eq!(r1, r2, "sort_sweep should match pairwise");
        assert_eq!(r1, r3, "grouped should match pairwise");
    }

    #[test]
    fn test_no_overlaps() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 3, 31), "activo"),
            make_contrato(prop, (2024, 4, 1), (2024, 6, 30), "activo"),
            make_contrato(prop, (2024, 7, 1), (2024, 9, 30), "activo"),
        ];

        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 0);
    }

    #[test]
    fn test_all_overlap() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
        ];

        // 3 contracts all overlapping = 3 pairs
        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 3);
    }
}
