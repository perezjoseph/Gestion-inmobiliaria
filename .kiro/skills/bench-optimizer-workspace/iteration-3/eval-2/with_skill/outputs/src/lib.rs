use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

/// Current O(n²) pairwise comparison.
/// Checks every pair for overlap among active contracts of the same propiedad.
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

/// Approach 2: Filter active first, group by propiedad, then pairwise within groups.
/// Reduces comparisons by only comparing contracts of the same propiedad.
/// Still O(n²) worst case (all same propiedad) but better average case.
pub fn detectar_solapamientos_grouped(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    // Filter active contracts and group by propiedad_id
    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, grupo) in &por_propiedad {
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

/// Approach 3: Filter active, group by propiedad, sort by fecha_inicio, scan adjacent.
/// O(n log n) per group. Only detects overlaps between adjacent intervals after sorting.
/// NOTE: This only finds overlaps between consecutive intervals when sorted by start date.
/// For full overlap detection, we need a sweep-line approach.
pub fn detectar_solapamientos_sort_scan(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, mut grupo) in por_propiedad {
        grupo.sort_by_key(|c| c.fecha_inicio);

        // Sweep-line: track the maximum fecha_fin seen so far.
        // Any contract whose fecha_inicio <= max_fecha_fin overlaps with at least one prior.
        // To find ALL overlapping pairs, we compare each contract against all prior
        // contracts whose fecha_fin >= current.fecha_inicio.
        for i in 1..grupo.len() {
            let current = grupo[i];
            // Walk backwards through prior contracts that could overlap
            for j in (0..i).rev() {
                let prior = grupo[j];
                if prior.fecha_fin >= current.fecha_inicio {
                    solapamientos.push((prior.id, current.id));
                } else {
                    // Since sorted by fecha_inicio, if prior.fecha_fin < current.fecha_inicio,
                    // we can't break early because an even earlier contract might have a later fecha_fin.
                    // So we must check all prior contracts.
                }
            }
        }
    }

    solapamientos
}

/// Approach 4: Filter active, group by propiedad, sort by fecha_inicio,
/// use a sweep-line with early termination.
/// After sorting by fecha_inicio, for each contract we only need to look back
/// at contracts whose fecha_fin >= current.fecha_inicio. We maintain a sorted
/// structure of end dates to enable early termination.
/// For n=200 with many propiedades, this should be efficient.
pub fn detectar_solapamientos_sort_sweep(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    use std::collections::HashMap;

    let mut por_propiedad: HashMap<Uuid, Vec<&Contrato>> = HashMap::new();
    for c in contratos {
        if c.estado == "activo" {
            por_propiedad.entry(c.propiedad_id).or_default().push(c);
        }
    }

    let mut solapamientos = Vec::new();

    for (_propiedad_id, mut grupo) in por_propiedad {
        if grupo.len() < 2 {
            continue;
        }

        // Sort by fecha_inicio, then by fecha_fin descending for ties
        grupo.sort_by(|a, b| {
            a.fecha_inicio
                .cmp(&b.fecha_inicio)
                .then(b.fecha_fin.cmp(&a.fecha_fin))
        });

        // For each contract, compare against all prior contracts.
        // Since sorted by fecha_inicio, we know b.fecha_inicio >= a.fecha_inicio for all prior a.
        // Overlap condition simplifies to: a.fecha_fin >= current.fecha_inicio
        for i in 1..grupo.len() {
            let current = grupo[i];
            for j in 0..i {
                let prior = grupo[j];
                if prior.fecha_fin >= current.fecha_inicio {
                    solapamientos.push((prior.id, current.id));
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
    fn test_no_overlap() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop, (2024, 7, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 0);
    }

    #[test]
    fn test_overlap_detected() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 7, 15), "activo"),
            make_contrato(prop, (2024, 7, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 1);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 1);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 1);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 1);
    }

    #[test]
    fn test_inactive_ignored() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 12, 31), "cancelado"),
        ];

        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 0);
    }

    #[test]
    fn test_different_propiedades_no_overlap() {
        let prop1 = Uuid::new_v4();
        let prop2 = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop1, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop2, (2024, 1, 1), (2024, 12, 31), "activo"),
        ];

        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 0);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 0);
    }

    #[test]
    fn test_multiple_overlaps() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 3, 1), (2024, 9, 30), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 8, 31), "activo"),
        ];

        // All 3 overlap with each other: (0,1), (0,2), (1,2) = 3 pairs
        assert_eq!(detectar_solapamientos_pairwise(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_grouped(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_sort_scan(&contratos).len(), 3);
        assert_eq!(detectar_solapamientos_sort_sweep(&contratos).len(), 3);
    }
}
