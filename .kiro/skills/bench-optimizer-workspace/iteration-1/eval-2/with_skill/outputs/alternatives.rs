use chrono::NaiveDate;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub estado: String,
}

// =============================================================================
// Approach 1: Original O(n²) pairwise comparison
// =============================================================================

/// Current implementation: brute-force pairwise comparison.
/// For each pair (i, j), checks if both are active, same propiedad, and dates overlap.
/// Time: O(n²), Space: O(1) extra (besides output).
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

// =============================================================================
// Approach 2: Filter-first, then sort by fecha_inicio within each propiedad group
// =============================================================================

/// Optimization: filter active contracts first, group by propiedad_id using a HashMap,
/// then sort each group by fecha_inicio and scan adjacent pairs.
///
/// Key insight: after sorting by start date, an overlap can only occur between
/// consecutive contracts (contract[i].fecha_fin >= contract[i+1].fecha_inicio).
/// However, this only detects *adjacent* overlaps. For full overlap detection
/// (where a long contract overlaps multiple subsequent ones), we need a sweep.
///
/// Time: O(n log n) for sort, O(n) for scan = O(n log n) total.
/// Space: O(n) for the HashMap groups.
pub fn detectar_solapamientos_sort_scan(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    // Step 1: Filter to only active contracts and group by propiedad_id
    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    // Step 2: For each propiedad group, sort by fecha_inicio and sweep
    for (_propiedad_id, grupo) in &mut por_propiedad {
        if grupo.len() < 2 {
            continue;
        }

        grupo.sort_unstable_by_key(|c| c.fecha_inicio);

        // Sweep: for each contract, check all subsequent contracts that start
        // before the current one ends. Since sorted by start, once we find one
        // that starts after our end, we can stop checking further.
        for i in 0..grupo.len() {
            for j in (i + 1)..grupo.len() {
                if grupo[j].fecha_inicio > grupo[i].fecha_fin {
                    break; // sorted, so no further overlaps with contract i
                }
                solapamientos.push((grupo[i].id, grupo[j].id));
            }
        }
    }

    solapamientos
}

// =============================================================================
// Approach 3: Pre-filter active + sort globally, partition by propiedad
// =============================================================================

/// Optimization variant: collect active contracts into a Vec, sort by
/// (propiedad_id, fecha_inicio), then sweep within each propiedad partition.
/// Avoids HashMap allocation overhead — single sort + linear scan.
///
/// Time: O(n log n) for sort, O(n) amortized for sweep.
/// Space: O(n) for the filtered+sorted vec.
pub fn detectar_solapamientos_sort_partition(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    // Step 1: Filter active contracts
    let mut activos: Vec<&Contrato> = contratos
        .iter()
        .filter(|c| c.estado == "activo")
        .collect();

    if activos.len() < 2 {
        return Vec::new();
    }

    // Step 2: Sort by (propiedad_id, fecha_inicio)
    activos.sort_unstable_by(|a, b| {
        a.propiedad_id
            .cmp(&b.propiedad_id)
            .then(a.fecha_inicio.cmp(&b.fecha_inicio))
    });

    let mut solapamientos = Vec::new();

    // Step 3: Sweep within each propiedad partition
    let mut i = 0;
    while i < activos.len() {
        // Find the end of this propiedad's partition
        let propiedad_id = activos[i].propiedad_id;
        let mut j = i + 1;
        while j < activos.len() && activos[j].propiedad_id == propiedad_id {
            j += 1;
        }

        // Sweep within partition [i..j)
        for a in i..j {
            for b in (a + 1)..j {
                if activos[b].fecha_inicio > activos[a].fecha_fin {
                    break; // sorted by start, no more overlaps with contract a
                }
                solapamientos.push((activos[a].id, activos[b].id));
            }
        }

        i = j;
    }

    solapamientos
}

// =============================================================================
// Test: all approaches produce the same results
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::HashSet;
    use uuid::Uuid;

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

    fn normalize(pairs: Vec<(Uuid, Uuid)>) -> HashSet<(Uuid, Uuid)> {
        pairs
            .into_iter()
            .map(|(a, b)| if a < b { (a, b) } else { (b, a) })
            .collect()
    }

    #[test]
    fn all_approaches_agree() {
        let prop1 = Uuid::new_v4();
        let prop2 = Uuid::new_v4();

        let contratos = vec![
            // prop1: two overlapping active contracts
            make_contrato(prop1, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop1, (2024, 3, 1), (2024, 9, 30), "activo"),
            // prop1: one inactive (should not overlap)
            make_contrato(prop1, (2024, 2, 1), (2024, 5, 31), "vencido"),
            // prop1: active but non-overlapping
            make_contrato(prop1, (2024, 10, 1), (2025, 3, 31), "activo"),
            // prop2: two active, non-overlapping
            make_contrato(prop2, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop2, (2024, 7, 1), (2024, 12, 31), "activo"),
        ];

        let r1 = normalize(detectar_solapamientos_original(&contratos));
        let r2 = normalize(detectar_solapamientos_sort_scan(&contratos));
        let r3 = normalize(detectar_solapamientos_sort_partition(&contratos));

        assert_eq!(r1.len(), 1, "Should find exactly 1 overlap");
        assert_eq!(r1, r2, "sort_scan should match original");
        assert_eq!(r1, r3, "sort_partition should match original");
    }

    #[test]
    fn no_overlaps_when_all_inactive() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "vencido"),
            make_contrato(prop, (2024, 6, 1), (2024, 12, 31), "cancelado"),
        ];

        assert!(detectar_solapamientos_original(&contratos).is_empty());
        assert!(detectar_solapamientos_sort_scan(&contratos).is_empty());
        assert!(detectar_solapamientos_sort_partition(&contratos).is_empty());
    }

    #[test]
    fn multiple_overlaps_detected() {
        let prop = Uuid::new_v4();
        // Three contracts all overlapping each other
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 3, 1), (2024, 9, 30), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 8, 31), "activo"),
        ];

        let r1 = normalize(detectar_solapamientos_original(&contratos));
        let r2 = normalize(detectar_solapamientos_sort_scan(&contratos));
        let r3 = normalize(detectar_solapamientos_sort_partition(&contratos));

        // 3 contracts all overlapping = 3 pairs: (0,1), (0,2), (1,2)
        assert_eq!(r1.len(), 3);
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
    }
}
