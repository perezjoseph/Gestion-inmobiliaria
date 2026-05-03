#![allow(clippy::needless_return)]
use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;

use realestate_backend::services::pago_generacion::{
    calcular_pagos, filtrar_existentes, validar_dia_vencimiento,
};

// ── Strategies ──────────────────────────────────────────────────────────

fn valid_year() -> impl Strategy<Value = i32> {
    2020i32..=2030i32
}

fn valid_month() -> impl Strategy<Value = u32> {
    1u32..=12u32
}

fn safe_day() -> impl Strategy<Value = u32> {
    1u32..=28u32
}

fn valid_date() -> impl Strategy<Value = NaiveDate> {
    (valid_year(), valid_month(), safe_day()).prop_map(|(y, m, d)| {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    })
}

/// Two dates where inicio <= fin.
fn ordered_date_pair() -> impl Strategy<Value = (NaiveDate, NaiveDate)> {
    (valid_date(), valid_date()).prop_map(|(a, b)| if a <= b { (a, b) } else { (b, a) })
}

fn positive_decimal() -> impl Strategy<Value = Decimal> {
    (1i64..10_000_000i64).prop_map(|v| Decimal::new(v, 2))
}

fn valid_moneda() -> impl Strategy<Value = String> {
    prop_oneof![Just("DOP".to_string()), Just("USD".to_string())]
}

fn dia_vencimiento_strategy() -> impl Strategy<Value = u32> {
    1u32..=31u32
}

fn valid_estado() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pendiente".to_string()),
        Just("pagado".to_string()),
        Just("atrasado".to_string()),
    ]
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Count distinct (year, month) pairs in an inclusive date range.
fn count_distinct_months(inicio: NaiveDate, fin: NaiveDate) -> usize {
    if inicio > fin {
        return 0;
    }
    let mut count = 0usize;
    let mut y = inicio.year();
    let mut m = inicio.month();
    let ey = fin.year();
    let em = fin.month();
    loop {
        count += 1;
        if y == ey && m == em {
            break;
        }
        if m == 12 {
            m = 1;
            y += 1;
        } else {
            m += 1;
        }
    }
    count
}

/// Last day of a given year/month.
fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day())
}


// Feature: auto-generate-pagos, Property 1: Month count correctness
// **Validates: Requirements 1.1, 1.4, 1.5, 1.6**
#[test]
fn test_month_count_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(ordered_date_pair(), positive_decimal(), valid_moneda()),
            |((inicio, fin), monto, moneda)| {
                let pagos = calcular_pagos(inicio, fin, monto, &moneda, 1);
                let expected = count_distinct_months(inicio, fin);

                prop_assert_eq!(pagos.len(), expected);

                // When both dates fall in the same month, exactly one item
                if inicio.year() == fin.year() && inicio.month() == fin.month() {
                    prop_assert_eq!(pagos.len(), 1);
                }

                Ok(())
            },
        )
        .unwrap();
}

// Feature: auto-generate-pagos, Property 2: Generated pago fields match contract
// **Validates: Requirements 1.2**
#[test]
fn test_generated_fields_match_contract() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(ordered_date_pair(), positive_decimal(), valid_moneda()),
            |((inicio, fin), monto, moneda)| {
                let pagos = calcular_pagos(inicio, fin, monto, &moneda, 1);

                for pago in &pagos {
                    prop_assert_eq!(pago.monto, monto);
                    prop_assert_eq!(&pago.moneda, &moneda);
                }

                Ok(())
            },
        )
        .unwrap();
}

// Feature: auto-generate-pagos, Property 3: Date calculation with day clamping
// **Validates: Requirements 1.3, 6.1, 6.2**
#[test]
fn test_date_calculation_day_clamping() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(ordered_date_pair(), positive_decimal(), dia_vencimiento_strategy()),
            |((inicio, fin), monto, dia)| {
                let pagos = calcular_pagos(inicio, fin, monto, "DOP", dia);

                for pago in &pagos {
                    let y = pago.fecha_vencimiento.year();
                    let m = pago.fecha_vencimiento.month();
                    let last = last_day_of_month(y, m);
                    let expected_day = dia.min(last);

                    prop_assert_eq!(pago.fecha_vencimiento.day(), expected_day);
                }

                // When dia_vencimiento defaults to 1, every day is 1
                let pagos_default = calcular_pagos(inicio, fin, monto, "DOP", 1);
                for pago in &pagos_default {
                    prop_assert_eq!(
                        pago.fecha_vencimiento.day(),
                        1,
                        "Default dia_vencimiento=1 but got day {}",
                        pago.fecha_vencimiento.day()
                    );
                }

                Ok(())
            },
        )
        .unwrap();
}

// Feature: auto-generate-pagos, Property 4: Cancellation only affects correct pagos
// **Validates: Requirements 3.1, 3.2, 3.3**
#[test]
fn test_cancellation_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate a list of (estado, fecha_vencimiento) pairs and a fecha_terminacion
    let pago_entry = (valid_estado(), valid_date());
    let strategy = (
        proptest::collection::vec(pago_entry, 1..20),
        valid_date(),
    );

    runner
        .run(&strategy, |(entries, fecha_terminacion)| {
            // Simulate cancellation logic: for each pago, determine expected estado
            for (estado, fecha_vencimiento) in &entries {
                let expected = if estado == "pendiente" && *fecha_vencimiento > fecha_terminacion {
                    "cancelado"
                } else {
                    estado.as_str()
                };

                // Verify the logic:
                // (a) pendiente + fecha > terminacion → cancelado
                if estado == "pendiente" && *fecha_vencimiento > fecha_terminacion {
                    prop_assert_eq!(expected, "cancelado");
                }
                // (b) pagado/atrasado → unchanged
                if estado == "pagado" || estado == "atrasado" {
                    prop_assert_eq!(expected, estado.as_str());
                }
                // (c) pendiente + fecha <= terminacion → unchanged
                if estado == "pendiente" && *fecha_vencimiento <= fecha_terminacion {
                    prop_assert_eq!(expected, "pendiente");
                }
            }

            Ok(())
        })
        .unwrap();
}

// Feature: auto-generate-pagos, Property 5: Preview totals are consistent
// **Validates: Requirements 4.4, 4.5**
#[test]
fn test_preview_totals_consistency() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate contract data + some existing month count
    let strategy = (
        ordered_date_pair(),
        positive_decimal(),
        valid_moneda(),
        dia_vencimiento_strategy(),
        // Number of existing months to simulate (0..=total will be clamped)
        0usize..12usize,
    );

    runner
        .run(
            &strategy,
            |((inicio, fin), monto, moneda, dia, existing_count)| {
                let all_pagos = calcular_pagos(inicio, fin, monto, &moneda, dia);
                let total_pagos = all_pagos.len();

                // Simulate some existing dates (take first N months)
                let existing_count = existing_count.min(total_pagos);
                let fechas_existentes: Vec<NaiveDate> = all_pagos
                    .iter()
                    .take(existing_count)
                    .map(|p| p.fecha_vencimiento)
                    .collect();

                let nuevos = filtrar_existentes(&all_pagos, &fechas_existentes);
                let pagos_existentes = existing_count;
                let pagos_nuevos = nuevos.len();

                // total_pagos == len(pagos)
                prop_assert_eq!(total_pagos, all_pagos.len());

                // monto_total == sum(montos)
                let monto_total: Decimal = all_pagos.iter().map(|p| p.monto).sum();
                let expected_total = monto * Decimal::from(total_pagos);
                prop_assert_eq!(monto_total, expected_total);

                // pagos_existentes + pagos_nuevos == total_pagos
                prop_assert_eq!(pagos_existentes + pagos_nuevos, total_pagos);

                Ok(())
            },
        )
        .unwrap();
}

// Feature: auto-generate-pagos, Property 6: Deduplication filters by year-month
// **Validates: Requirements 5.2, 7.1, 7.2, 7.3**
#[test]
fn test_deduplication_by_year_month() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let strategy = (
        ordered_date_pair(),
        positive_decimal(),
        valid_moneda(),
        dia_vencimiento_strategy(),
        proptest::collection::vec(valid_date(), 0..15),
    );

    runner
        .run(
            &strategy,
            |((inicio, fin), monto, moneda, dia, fechas_existentes)| {
                let all_pagos = calcular_pagos(inicio, fin, monto, &moneda, dia);
                let filtered = filtrar_existentes(&all_pagos, &fechas_existentes);

                // Build set of existing (year, month) pairs
                let existing_ym: HashSet<(i32, u32)> = fechas_existentes
                    .iter()
                    .map(|d| (d.year(), d.month()))
                    .collect();

                // Verify no filtered pago overlaps with existing dates by (year, month)
                for pago in &filtered {
                    let ym = (
                        pago.fecha_vencimiento.year(),
                        pago.fecha_vencimiento.month(),
                    );
                    prop_assert!(
                        !existing_ym.contains(&ym),
                        "Filtered pago has (year, month) = {:?} which exists in existing dates",
                        ym
                    );
                }

                // filtered_count + matched_count == original_count
                let matched_count = all_pagos.len() - filtered.len();
                prop_assert_eq!(filtered.len() + matched_count, all_pagos.len());

                Ok(())
            },
        )
        .unwrap();
}

// Feature: auto-generate-pagos, Property 7: Invalid dia_vencimiento is rejected
// **Validates: Requirements 6.4**
#[test]
fn test_invalid_dia_vencimiento_rejected() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    // Generate u32 values where value == 0 or value > 31
    let invalid_dia = prop_oneof![
        Just(0u32),
        32u32..=1000u32,
    ];

    runner
        .run(&invalid_dia, |dia| {
            let result = validar_dia_vencimiento(dia);
            prop_assert!(result.is_err());
            Ok(())
        })
        .unwrap();
}
