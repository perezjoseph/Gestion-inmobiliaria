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
// Approach 1: Current implementation (baseline)
// HashMap<Uuid, HashMap<String, f64>> with String month keys via format!
// =============================================================================
pub fn current(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
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
// Approach 2: Numeric month keys
// Avoid String allocation and chrono formatting by using (i32, u32) as key,
// then format only at the end for the final output.
// =============================================================================
pub fn numeric_month_keys(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> = HashMap::new();

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let fecha = pago.fecha_pago.unwrap();
        let key = (fecha.year(), fecha.month());
        *por_propiedad
            .entry(pago.propiedad_id)
            .or_default()
            .entry(key)
            .or_default() += pago.monto;
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<((i32, u32), f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

use chrono::Datelike;

// =============================================================================
// Approach 3: Pre-allocated with capacity hints
// Same as numeric keys but with capacity pre-allocation based on known production sizes.
// ~50 propiedades, ~24 months per propiedad.
// =============================================================================
pub fn preallocated(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut por_propiedad: HashMap<Uuid, HashMap<(i32, u32), f64>> = HashMap::with_capacity(50);

    for pago in pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
    {
        let fecha = pago.fecha_pago.unwrap();
        let key = (fecha.year(), fecha.month());
        por_propiedad
            .entry(pago.propiedad_id)
            .or_insert_with(|| HashMap::with_capacity(24))
            .entry(key)
            .and_modify(|v| *v += pago.monto)
            .or_insert(pago.monto);
    }

    por_propiedad
        .into_iter()
        .map(|(prop_id, meses)| {
            let mut sorted: Vec<((i32, u32), f64)> = meses.into_iter().collect();
            sorted.sort_unstable_by_key(|&(k, _)| k);
            let result: Vec<(String, f64)> = sorted
                .into_iter()
                .map(|((y, m), total)| (format!("{y:04}-{m:02}"), total))
                .collect();
            (prop_id, result)
        })
        .collect()
}

// =============================================================================
// Approach 4: Sort-based approach
// Sort pagos by (propiedad_id, year, month), then linear scan to accumulate.
// Avoids HashMap overhead entirely for the inner grouping.
// =============================================================================
pub fn sort_based(pagos: &[Pago]) -> HashMap<Uuid, Vec<(String, f64)>> {
    let mut filtered: Vec<(Uuid, i32, u32, f64)> = pagos
        .iter()
        .filter(|p| p.estado == "pagado" && p.fecha_pago.is_some())
        .map(|p| {
            let fecha = p.fecha_pago.unwrap();
            (p.propiedad_id, fecha.year(), fecha.month() as i32, p.monto)
        })
        .collect();

    filtered.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    let mut result: HashMap<Uuid, Vec<(String, f64)>> = HashMap::with_capacity(50);

    let mut i = 0;
    while i < filtered.len() {
        let (prop_id, year, month, _) = filtered[i];
        let mut total = 0.0;

        let mut j = i;
        while j < filtered.len()
            && filtered[j].0 == prop_id
            && filtered[j].1 == year
            && filtered[j].2 == month
        {
            total += filtered[j].3;
            j += 1;
        }

        result
            .entry(prop_id)
            .or_insert_with(|| Vec::with_capacity(24))
            .push((format!("{year:04}-{month:02}"), total));

        i = j;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_pago(propiedad_id: Uuid, monto: f64, fecha_pago: NaiveDate, estado: &str) -> Pago {
        Pago {
            id: Uuid::new_v4(),
            contrato_id: Uuid::new_v4(),
            propiedad_id,
            monto,
            moneda: "DOP".to_string(),
            fecha_vencimiento: fecha_pago,
            fecha_pago: if estado == "pagado" {
                Some(fecha_pago)
            } else {
                None
            },
            estado: estado.to_string(),
        }
    }

    #[test]
    fn all_approaches_produce_same_results() {
        let prop_a = Uuid::new_v4();
        let prop_b = Uuid::new_v4();

        let pagos = vec![
            make_pago(
                prop_a,
                1000.0,
                NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                "pagado",
            ),
            make_pago(
                prop_a,
                2000.0,
                NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
                "pagado",
            ),
            make_pago(
                prop_a,
                1500.0,
                NaiveDate::from_ymd_opt(2024, 2, 10).unwrap(),
                "pagado",
            ),
            make_pago(
                prop_b,
                3000.0,
                NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),
                "pagado",
            ),
            make_pago(
                prop_a,
                500.0,
                NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                "pendiente",
            ),
        ];

        let r1 = current(&pagos);
        let r2 = numeric_month_keys(&pagos);
        let r3 = preallocated(&pagos);
        let r4 = sort_based(&pagos);

        // All should have same keys
        assert_eq!(r1.len(), r2.len());
        assert_eq!(r1.len(), r3.len());
        assert_eq!(r1.len(), r4.len());

        // Compare values for each propiedad
        for (prop_id, months) in &r1 {
            assert_eq!(months, r2.get(prop_id).unwrap());
            assert_eq!(months, r3.get(prop_id).unwrap());
            assert_eq!(months, r4.get(prop_id).unwrap());
        }

        // Verify specific values
        let prop_a_months = r1.get(&prop_a).unwrap();
        assert_eq!(prop_a_months.len(), 2); // Jan and Feb (pendiente excluded)
        assert_eq!(prop_a_months[0], ("2024-01".to_string(), 3000.0));
        assert_eq!(prop_a_months[1], ("2024-02".to_string(), 1500.0));

        let prop_b_months = r1.get(&prop_b).unwrap();
        assert_eq!(prop_b_months.len(), 1);
        assert_eq!(prop_b_months[0], ("2024-01".to_string(), 3000.0));
    }
}
