use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Pago {
    pub id: Uuid,
    pub contrato_id: Uuid,
    pub propiedad_id: Uuid,
    pub monto: f64,
    pub moneda: String,
    pub fecha_vencimiento: chrono::NaiveDate,
    pub fecha_pago: Option<chrono::NaiveDate>,
    pub estado: String,
}

// =============================================================================
// APPROACH 1: Current implementation (baseline)
// Single-pass HashMap accumulation with String keys for months.
// =============================================================================
pub fn ingresos_current(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<String, f64>> = HashMap::new();

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let mes = pago.fecha_pago.unwrap().format("%Y-%m").to_string();
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(mes)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(String, f64)> = meses.into_iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            (prop_id, sorted)
        })
        .collect()
}

// =============================================================================
// APPROACH 2: Numeric month key + pre-allocation
// Avoids chrono::format! per pago by extracting (year, month) as a u32 key.
// Uses a compact numeric key (year*100 + month) instead of String allocation.
// Converts to String only at the end.
// =============================================================================
pub fn ingresos_numeric_key(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;

    // Pre-allocate with estimated capacity
    let mut por_propiedad: HashMap<Uuid, HashMap<u32, f64>> = HashMap::with_capacity(50);

    for pago in pagos.iter() {
        if pago.estado != "pagado" {
            continue;
        }
        let fecha = match pago.fecha_pago {
            Some(f) => f,
            None => continue,
        };
        // Compact key: 202301 for Jan 2023
        let mes_key = (fecha.year() as u32) * 100 + fecha.month();
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(mes_key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<(String, f64)> = meses
                .into_iter()
                .map(|(k, v)| {
                    let year = k / 100;
                    let month = k % 100;
                    (format!("{year:04}-{month:02}"), v)
                })
                .collect();
            sorted.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            (prop_id, sorted)
        })
        .collect()
}

// =============================================================================
// APPROACH 3: Sort-first, then linear scan grouping
// Sort pagos by (propiedad_id, month) first, then accumulate in a single pass
// without any HashMap lookups. Avoids hashing overhead entirely.
// =============================================================================
pub fn ingresos_sort_scan(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;

    // Filter and collect with numeric month key
    let mut filtered: Vec<(Uuid, u32, f64)> = Vec::with_capacity(pagos.len());
    for pago in pagos.iter() {
        if pago.estado != "pagado" {
            continue;
        }
        let fecha = match pago.fecha_pago {
            Some(f) => f,
            None => continue,
        };
        let mes_key = (fecha.year() as u32) * 100 + fecha.month();
        filtered.push((pago.propiedad_id, mes_key, pago.monto));
    }

    // Sort by propiedad_id then month
    filtered.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // Linear scan to group
    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50);

    if filtered.is_empty() {
        return result;
    }

    let mut current_prop = filtered[0].0;
    let mut current_month = filtered[0].1;
    let mut current_sum = 0.0_f64;
    let mut current_months: Vec<(String, f64)> = Vec::with_capacity(24);

    for (prop_id, mes_key, monto) in &filtered {
        if *prop_id != current_prop {
            // Flush current month
            let year = current_month / 100;
            let month = current_month % 100;
            current_months.push((format!("{year:04}-{month:02}"), current_sum));
            // Flush current property
            result.insert(current_prop, std::mem::take(&mut current_months));
            current_months = Vec::with_capacity(24);
            current_prop = *prop_id;
            current_month = *mes_key;
            current_sum = *monto;
        } else if *mes_key != current_month {
            // Flush current month
            let year = current_month / 100;
            let month = current_month % 100;
            current_months.push((format!("{year:04}-{month:02}"), current_sum));
            current_month = *mes_key;
            current_sum = *monto;
        } else {
            current_sum += *monto;
        }
    }

    // Flush last group
    let year = current_month / 100;
    let month = current_month % 100;
    current_months.push((format!("{year:04}-{month:02}"), current_sum));
    result.insert(current_prop, current_months);

    result
}

// =============================================================================
// APPROACH 4: Numeric key + BTreeMap (auto-sorted, no final sort step)
// Uses BTreeMap for the inner map so entries are already sorted by month key.
// =============================================================================
pub fn ingresos_btreemap(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    use chrono::Datelike;
    use std::collections::BTreeMap;

    let mut por_propiedad: HashMap<Uuid, BTreeMap<u32, f64>> = HashMap::with_capacity(50);

    for pago in pagos.iter() {
        if pago.estado != "pagado" {
            continue;
        }
        let fecha = match pago.fecha_pago {
            Some(f) => f,
            None => continue,
        };
        let mes_key = (fecha.year() as u32) * 100 + fecha.month();
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(mes_key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let sorted: Vec<(String, f64)> = meses
                .into_iter()
                .map(|(k, v)| {
                    let year = k / 100;
                    let month = k % 100;
                    (format!("{year:04}-{month:02}"), v)
                })
                .collect();
            (prop_id, sorted)
        })
        .collect()
}

// =============================================================================
// Test: All approaches produce the same result
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_test_pagos() -> Vec<Pago> {
        let prop1 = Uuid::new_v4();
        let prop2 = Uuid::new_v4();
        vec![
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: prop1,
                monto: 1000.0,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                fecha_pago: Some(NaiveDate::from_ymd_opt(2024, 1, 5).unwrap()),
                estado: "pagado".to_string(),
            },
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: prop1,
                monto: 2000.0,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                fecha_pago: Some(NaiveDate::from_ymd_opt(2024, 1, 10).unwrap()),
                estado: "pagado".to_string(),
            },
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: prop1,
                monto: 1500.0,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                fecha_pago: Some(NaiveDate::from_ymd_opt(2024, 2, 3).unwrap()),
                estado: "pagado".to_string(),
            },
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: prop2,
                monto: 5000.0,
                moneda: "USD".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                fecha_pago: Some(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()),
                estado: "pagado".to_string(),
            },
            // Should be excluded: pendiente
            Pago {
                id: Uuid::new_v4(),
                contrato_id: Uuid::new_v4(),
                propiedad_id: prop1,
                monto: 9999.0,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                fecha_pago: None,
                estado: "pendiente".to_string(),
            },
        ]
    }

    #[test]
    fn all_approaches_produce_same_result() {
        let pagos = make_test_pagos();

        let r1 = ingresos_current(&pagos);
        let r2 = ingresos_numeric_key(&pagos);
        let r3 = ingresos_sort_scan(&pagos);
        let r4 = ingresos_btreemap(&pagos);

        // All should have same keys
        assert_eq!(r1.len(), r2.len());
        assert_eq!(r1.len(), r3.len());
        assert_eq!(r1.len(), r4.len());

        // Compare values for each property
        for (prop_id, months1) in &r1 {
            let months2 = r2.get(prop_id).unwrap();
            let months3 = r3.get(prop_id).unwrap();
            let months4 = r4.get(prop_id).unwrap();

            assert_eq!(months1.len(), months2.len());
            assert_eq!(months1.len(), months3.len());
            assert_eq!(months1.len(), months4.len());

            for i in 0..months1.len() {
                assert_eq!(months1[i].0, months2[i].0);
                assert_eq!(months1[i].0, months3[i].0);
                assert_eq!(months1[i].0, months4[i].0);
                assert!((months1[i].1 - months2[i].1).abs() < 0.01);
                assert!((months1[i].1 - months3[i].1).abs() < 0.01);
                assert!((months1[i].1 - months4[i].1).abs() < 0.01);
            }
        }
    }
}
