use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

/// Original O(n²) pairwise comparison.
/// Checks every pair of contracts for overlap.
pub fn detectar_solapamientos_original(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
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

/// Approach 1: Filter first, then sort by fecha_inicio, then scan adjacent pairs.
///
/// Key insight: after filtering to only active contracts for the same propiedad,
/// sorting by start date means overlaps can only occur between adjacent entries
/// (if A doesn't overlap B, then A can't overlap anything after B either).
///
/// Complexity: O(n log n) after filtering.
/// But note: this only finds *adjacent* overlaps. If contract A overlaps B and C,
/// but B and C don't overlap each other, this still finds A-B and A-C only if
/// we scan forward until no overlap. We need a sweep-line approach.
pub fn detectar_solapamientos_sort_scan(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    // Group active contracts by propiedad_id
    let mut by_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            by_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, group) in &mut by_propiedad {
        if group.len() < 2 {
            continue;
        }

        // Sort by fecha_inicio
        group.sort_unstable_by_key(|c| c.fecha_inicio);

        // Sweep-line: for each contract, check forward until no overlap possible
        for i in 0..group.len() {
            let a = group[i];
            for j in (i + 1)..group.len() {
                let b = group[j];
                // Since sorted by fecha_inicio, if b.fecha_inicio > a.fecha_fin,
                // no further contracts can overlap with a
                if b.fecha_inicio > a.fecha_fin {
                    break;
                }
                // b.fecha_inicio <= a.fecha_fin is guaranteed, check reverse
                // a.fecha_inicio <= b.fecha_fin is guaranteed since a starts before b
                solapamientos.push((a.id, b.id));
            }
        }
    }

    solapamientos
}

/// Approach 2: Pre-filter active only, then brute-force on the filtered set.
///
/// The original checks estado == "activo" inside the inner loop. If most contracts
/// are NOT active, filtering first reduces the effective n significantly.
/// Still O(k²) where k = number of active contracts, but k << n in practice.
pub fn detectar_solapamientos_prefilter(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    let activos: Vec<&Contrato> = contratos.iter().filter(|c| c.estado == "activo").collect();

    let mut solapamientos = Vec::new();

    for i in 0..activos.len() {
        for j in (i + 1)..activos.len() {
            let a = activos[i];
            let b = activos[j];

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

/// Approach 3: HashMap grouping + sort-scan (combines grouping with sweep-line).
/// Uses SmallVec-like optimization with pre-allocated capacity.
///
/// Same algorithm as sort_scan but with capacity hints and fewer allocations.
pub fn detectar_solapamientos_grouped_optimized(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    // Pre-filter and group in one pass
    let mut by_propiedad: HashMap<Uuid, Vec<usize>> = HashMap::with_capacity(contratos.len() / 5);
    for (idx, c) in contratos.iter().enumerate() {
        if c.estado == "activo" {
            by_propiedad.entry(c.propiedad_id).or_default().push(idx);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, indices) in &by_propiedad {
        if indices.len() < 2 {
            continue;
        }

        // Collect and sort by fecha_inicio
        let mut sorted: Vec<&Contrato> = indices.iter().map(|&i| &contratos[i]).collect();
        sorted.sort_unstable_by_key(|c| c.fecha_inicio);

        // Sweep-line
        for i in 0..sorted.len() {
            let a = sorted[i];
            for j in (i + 1)..sorted.len() {
                let b = sorted[j];
                if b.fecha_inicio > a.fecha_fin {
                    break;
                }
                solapamientos.push((a.id, b.id));
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
    fn test_no_overlap() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop, (2024, 7, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_original(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_prefilter(&contratos).len(), 0);
        assert_eq!(
            detectar_solapamientos_grouped_optimized(&contratos).len(),
            0
        );
    }

    #[test]
    fn test_overlap_detected() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 7, 31), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_original(&contratos).len(), 1);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 1);
        assert_eq!(detectar_solapamientos_prefilter(&contratos).len(), 1);
        assert_eq!(
            detectar_solapamientos_grouped_optimized(&contratos).len(),
            1
        );
    }

    #[test]
    fn test_different_propiedad_no_overlap() {
        let prop1 = Uuid::new_v4();
        let prop2 = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop1, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop2, (2024, 1, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_original(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_prefilter(&contratos).len(), 0);
        assert_eq!(
            detectar_solapamientos_grouped_optimized(&contratos).len(),
            0
        );
    }

    #[test]
    fn test_inactive_not_counted() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 12, 31), "cancelado"),
        ];

        assert_eq!(detectar_solapamientos_original(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_prefilter(&contratos).len(), 0);
        assert_eq!(
            detectar_solapamientos_grouped_optimized(&contratos).len(),
            0
        );
    }

    #[test]
    fn test_multiple_overlaps() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 3, 1), (2024, 9, 30), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 8, 31), "activo"),
        ];

        // All three overlap each other: (0,1), (0,2), (1,2) = 3 pairs
        assert_eq!(detectar_solapamientos_original(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_prefilter(&contratos).len(), 3);
        assert_eq!(
            detectar_solapamientos_grouped_optimized(&contratos).len(),
            3
        );
    }
}
