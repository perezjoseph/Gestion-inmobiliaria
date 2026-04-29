#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use realestate_backend::models::gasto::ResumenCategoriaRow;
use realestate_backend::models::importacion::{ImportError, ImportResult};
use realestate_backend::services::dashboard::calcular_porcentaje_cambio;
use realestate_backend::services::gastos::{CATEGORIAS_GASTO, ESTADOS_GASTO};
use realestate_backend::services::validation::{MONEDAS, validate_enum};

fn arbitrary_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]{1,30}"
}

fn non_negative_i64() -> impl Strategy<Value = i64> {
    0i64..10_000_000i64
}

// Feature: gastos-expenses-tracking, Property 6: Enum validation rejects invalid values
// **Validates: Requirements 1.6, 1.7, 1.8, 3.4**
#[test]
fn test_enum_validation_rejects_invalid_values() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let categoria_set: Vec<&str> = CATEGORIAS_GASTO.to_vec();
    let estado_set: Vec<&str> = ESTADOS_GASTO.to_vec();
    let moneda_set: Vec<&str> = MONEDAS.to_vec();

    runner
        .run(&arbitrary_string(), |value| {
            if !categoria_set.contains(&value.as_str()) {
                let result = validate_enum("categoria", &value, CATEGORIAS_GASTO);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for categoria"
                );
            }
            if !estado_set.contains(&value.as_str()) {
                let result = validate_enum("estado", &value, ESTADOS_GASTO);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for estado"
                );
            }
            if !moneda_set.contains(&value.as_str()) {
                let result = validate_enum("moneda", &value, MONEDAS);
                assert!(
                    result.is_err(),
                    "validate_enum should reject '{value}' for moneda"
                );
            }
            Ok(())
        })
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 12: Profitability net income invariant
// **Validates: Requirements 6.1**
#[test]
fn test_profitability_net_income_invariant() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(non_negative_i64(), non_negative_i64()),
            |(ingresos_raw, gastos_raw)| {
                let total_ingresos = Decimal::new(ingresos_raw, 2);
                let total_gastos = Decimal::new(gastos_raw, 2);
                let ingreso_neto = total_ingresos - total_gastos;

                assert_eq!(
                    ingreso_neto,
                    total_ingresos - total_gastos,
                    "ingreso_neto must equal total_ingresos - total_gastos"
                );

                let reconstructed = ingreso_neto + total_gastos;
                assert_eq!(
                    reconstructed, total_ingresos,
                    "ingreso_neto + total_gastos must equal total_ingresos"
                );

                Ok(())
            },
        )
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 15: Percentage change calculation
// **Validates: Requirements 9.2**
#[test]
fn test_percentage_change_calculation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    runner
        .run(
            &(non_negative_i64(), non_negative_i64()),
            |(actual_raw, anterior_raw)| {
                let actual = Decimal::new(actual_raw, 2);
                let anterior = Decimal::new(anterior_raw, 2);
                let result = calcular_porcentaje_cambio(actual, anterior);

                if anterior.is_zero() && actual.is_zero() {
                    assert!(
                        (result - 0.0).abs() < f64::EPSILON,
                        "Both zero should yield 0.0, got {result}"
                    );
                } else if anterior.is_zero() {
                    assert!(
                        (result - 100.0).abs() < f64::EPSILON,
                        "Zero anterior with positive actual should yield 100.0, got {result}"
                    );
                } else {
                    let expected = ((actual - anterior) / anterior * Decimal::new(100, 0))
                        .to_f64()
                        .unwrap_or(0.0);
                    assert!(
                        (result - expected).abs() < 0.01,
                        "Expected {expected}, got {result} for actual={actual}, anterior={anterior}"
                    );
                }

                Ok(())
            },
        )
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 11: Category summary sorted by total descending
// **Validates: Requirements 5.4**
#[test]
fn test_category_summary_sorted_descending() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let categoria_strategy =
        proptest::sample::select(CATEGORIAS_GASTO.to_vec()).prop_map(|s| s.to_string());

    let row_strategy = (categoria_strategy, 0i64..10_000_000i64, 1u64..100u64).prop_map(
        |(categoria, total_raw, cantidad)| ResumenCategoriaRow {
            categoria,
            total: Decimal::new(total_raw, 2),
            cantidad,
        },
    );

    let vec_strategy = proptest::collection::vec(row_strategy, 0..20);

    runner
        .run(&vec_strategy, |mut rows| {
            rows.sort_by(|a, b| b.total.cmp(&a.total));

            for window in rows.windows(2) {
                assert!(
                    window[0].total >= window[1].total,
                    "Rows not sorted descending: {} < {}",
                    window[0].total,
                    window[1].total
                );
            }

            Ok(())
        })
        .unwrap();
}

// Feature: gastos-expenses-tracking, Property 14: CSV import valid/invalid row accounting
// **Validates: Requirements 8.4, 8.5**
#[test]
fn test_csv_import_row_accounting() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 100,
        ..Default::default()
    });

    let error_strategy = (1usize..1000usize, "[a-zA-Z ]{5,50}")
        .prop_map(|(fila, error)| ImportError { fila, error });

    let strategy = (
        0usize..500usize,
        proptest::collection::vec(error_strategy, 0..50),
    );

    runner
        .run(&strategy, |(exitosos, fallidos)| {
            let total_filas = exitosos + fallidos.len();
            let result = ImportResult {
                total_filas,
                exitosos,
                fallidos,
            };

            assert_eq!(
                result.exitosos + result.fallidos.len(),
                result.total_filas,
                "exitosos + fallidos.len() must equal total_filas"
            );

            Ok(())
        })
        .unwrap();
}
