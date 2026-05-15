use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub fecha_inicio: chrono::NaiveDate,
    pub fecha_fin: chrono::NaiveDate,
    pub estado: String,
}

// ─── Original O(n²) pairwise comparison ───────────────────────────────────────

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

// ─── Optimized: filter activos first, group by propiedad, sort + scan ─────────

pub fn detectar_solapamientos_optimized(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    // Step 1: Filter only active contracts (avoids checking inactive pairs)
    let activos: Vec<&Contrato> = contratos.iter().filter(|c| c.estado == "activo").collect();

    // Step 2: Group by propiedad_id using a HashMap
    let mut por_propiedad: std::collections::HashMap<Uuid, Vec<&Contrato>> =
        std::collections::HashMap::new();
    for c in &activos {
        por_propiedad.entry(c.propiedad_id).or_default().push(c);
    }

    let mut solapamientos = Vec::new();

    // Step 3: For each propiedad group, sort by fecha_inicio and scan for overlaps
    for (_propiedad_id, grupo) in &mut por_propiedad {
        if grupo.len() < 2 {
            continue;
        }

        // Sort by start date
        grupo.sort_unstable_by_key(|c| c.fecha_inicio);

        // Scan: compare each contract with subsequent ones while they could overlap
        for i in 0..grupo.len() {
            let a = grupo[i];
            for j in (i + 1)..grupo.len() {
                let b = grupo[j];
                // Since sorted by fecha_inicio, b.fecha_inicio >= a.fecha_inicio.
                // Overlap condition: b.fecha_inicio <= a.fecha_fin
                if b.fecha_inicio <= a.fecha_fin {
                    solapamientos.push((a.id, b.id));
                } else {
                    // No further contracts can overlap with `a` (sorted order)
                    break;
                }
            }
        }
    }

    solapamientos
}

// ─── Optimized v2: sort-only approach (no HashMap), for single-propiedad case ─

/// When all contracts belong to the same propiedad (common validation path),
/// skip the HashMap overhead entirely.
pub fn detectar_solapamientos_sort_only(contratos: &[Contrato]) -> Vec<(Uuid, Uuid)> {
    // Filter active and sort by start date
    let mut activos: Vec<&Contrato> = contratos.iter().filter(|c| c.estado == "activo").collect();

    activos.sort_unstable_by_key(|c| c.fecha_inicio);

    let mut solapamientos = Vec::new();

    for i in 0..activos.len() {
        let a = activos[i];
        for j in (i + 1)..activos.len() {
            let b = activos[j];
            // Only need to check same propiedad
            if a.propiedad_id != b.propiedad_id {
                continue;
            }
            if b.fecha_inicio <= a.fecha_fin {
                solapamientos.push((a.id, b.id));
            }
            // Can't break early here since different propiedades are interleaved
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
        start: (i32, u32, u32),
        end: (i32, u32, u32),
        estado: &str,
    ) -> Contrato {
        Contrato {
            id: Uuid::new_v4(),
            propiedad_id,
            fecha_inicio: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            fecha_fin: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            estado: estado.to_string(),
        }
    }

    #[test]
    fn test_correctness_no_overlaps() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop, (2024, 7, 1), (2024, 12, 31), "activo"),
        ];

        let orig = detectar_solapamientos_original(&contratos);
        let opt = detectar_solapamientos_optimized(&contratos);

        assert!(orig.is_empty());
        assert!(opt.is_empty());
    }

    #[test]
    fn test_correctness_with_overlaps() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 6, 30), "activo"),
            make_contrato(prop, (2024, 3, 1), (2024, 9, 30), "activo"),
            make_contrato(prop, (2024, 8, 1), (2024, 12, 31), "activo"),
        ];

        let mut orig = detectar_solapamientos_original(&contratos);
        let mut opt = detectar_solapamientos_optimized(&contratos);

        // Sort results for comparison (order may differ)
        orig.sort();
        opt.sort();

        assert_eq!(orig.len(), opt.len());
        // Contract 0 overlaps with 1, contract 1 overlaps with 2
        assert_eq!(orig.len(), 2);
    }

    #[test]
    fn test_inactive_contracts_ignored() {
        let prop = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop, (2024, 6, 1), (2024, 12, 31), "cancelado"),
        ];

        let orig = detectar_solapamientos_original(&contratos);
        let opt = detectar_solapamientos_optimized(&contratos);

        assert!(orig.is_empty());
        assert!(opt.is_empty());
    }

    #[test]
    fn test_different_propiedades_no_overlap() {
        let prop1 = Uuid::new_v4();
        let prop2 = Uuid::new_v4();
        let contratos = vec![
            make_contrato(prop1, (2024, 1, 1), (2024, 12, 31), "activo"),
            make_contrato(prop2, (2024, 1, 1), (2024, 12, 31), "activo"),
        ];

        let orig = detectar_solapamientos_original(&contratos);
        let opt = detectar_solapamientos_optimized(&contratos);

        assert!(orig.is_empty());
        assert!(opt.is_empty());
    }
}
