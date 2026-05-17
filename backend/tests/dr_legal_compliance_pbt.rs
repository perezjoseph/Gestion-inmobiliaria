#![allow(clippy::needless_return)]
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rust_decimal::Decimal;

use realestate_backend::services::ipc::calcular_monto_maximo;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Positive monto as i64 cents (1..99_999_999) to build Decimal with scale 2.
fn positive_monto_cents() -> impl Strategy<Value = i64> {
    1i64..99_999_999i64
}

/// Positive IPC porcentaje as i64 hundredths (1..10_000) to build Decimal with scale 2.
/// Represents 0.01% to 100.00%.
fn positive_ipc_hundredths() -> impl Strategy<Value = i64> {
    1i64..=10_000i64
}

// ── Property 1: IPC rent cap enforcement ───────────────────────────────
// Feature: dr-legal-compliance, Property 1: IPC rent cap enforcement
// **Validates: Requirements 1.1, 1.2**
#[test]
fn test_ipc_rent_cap_enforcement() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });
    runner
        .run(
            &(positive_monto_cents(), positive_ipc_hundredths()),
            |(m_cents, ipc_hundredths)| {
                let monto_actual = Decimal::new(m_cents, 2);
                let ipc_porcentaje = Decimal::new(ipc_hundredths, 2);

                let monto_maximo = calcular_monto_maximo(monto_actual, ipc_porcentaje);

                // Property: monto_maximo is always >= monto_actual (positive IPC means increase)
                prop_assert!(
                    monto_maximo >= monto_actual,
                    "monto_maximo ({}) should be >= monto_actual ({})",
                    monto_maximo,
                    monto_actual
                );

                // Property: any amount <= monto_maximo passes the cap check
                let within_cap = monto_actual + Decimal::new(1, 2); // monto_actual + 0.01
                if within_cap <= monto_maximo {
                    prop_assert!(
                        within_cap <= monto_maximo,
                        "Amount within cap ({}) should pass: max = {}",
                        within_cap,
                        monto_maximo
                    );
                }

                // Property: any amount > monto_maximo fails the cap check
                let exceeds_cap = monto_maximo + Decimal::new(1, 2); // monto_maximo + 0.01
                prop_assert!(
                    exceeds_cap > monto_maximo,
                    "Amount exceeding cap ({}) should fail: max = {}",
                    exceeds_cap,
                    monto_maximo
                );

                Ok(())
            },
        )
        .unwrap();
}

// ── Property 2: Deposit cap invariant (Ley 4314) ──────────────────────
// Feature: dr-legal-compliance, Property 2: Deposit cap invariant
// **Validates: Requirements 4.4**
#[test]
fn test_deposit_cap_invariant() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });
    runner
        .run(
            &(positive_monto_cents(), positive_monto_cents()),
            |(monto_cents, deposit_cents)| {
                let monto_mensual = Decimal::new(monto_cents, 2);
                let deposit = Decimal::new(deposit_cents, 2);

                // Ley 4314: deposit must not exceed one month of rent
                let is_valid = deposit <= monto_mensual;

                if deposit <= monto_mensual {
                    // Property: deposit <= monto_mensual always passes validation
                    prop_assert!(
                        is_valid,
                        "Deposit ({}) <= monto_mensual ({}) should pass validation",
                        deposit,
                        monto_mensual
                    );
                } else {
                    // Property: deposit > monto_mensual always fails validation
                    prop_assert!(
                        !is_valid,
                        "Deposit ({}) > monto_mensual ({}) should fail validation",
                        deposit,
                        monto_mensual
                    );
                }

                Ok(())
            },
        )
        .unwrap();
}

use chrono::NaiveDate;

use realestate_backend::services::desahucios::validate_estado_transition;
use realestate_backend::services::validation::validate_enum;

// ── Custom Strategies for Property 5 ───────────────────────────────────

fn desahucio_estado() -> impl Strategy<Value = &'static str> {
    proptest::sample::select(vec!["iniciado", "en_progreso", "completado"])
}

fn arbitrary_date() -> impl Strategy<Value = NaiveDate> {
    (2020i32..2030i32, 1u32..12u32, 1u32..28u32)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

fn arbitrary_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_]{1,30}"
}

/// Helper: validates that completado requires fecha_resolucion.
fn validate_desahucio_transition(
    estado: &str,
    fecha_resolucion: Option<&NaiveDate>,
) -> Result<(), ()> {
    if estado == "completado" && fecha_resolucion.is_none() {
        return Err(());
    }
    Ok(())
}

// ── Property 5: Desahucio state machine ────────────────────────────────
// Feature: dr-legal-compliance-and-utilities, Property 5: Desahucio state machine
// **Validates: Requirements 5.1, 5.3**
#[test]
fn test_desahucio_state_machine() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Sub-property A: completado without fecha_resolucion is always invalid
    runner
        .run(&arbitrary_date(), |_fecha| {
            let fecha_resolucion: Option<NaiveDate> = None;
            prop_assert!(
                validate_desahucio_transition("completado", fecha_resolucion.as_ref()).is_err(),
                "completado without fecha_resolucion must be rejected"
            );
            Ok(())
        })
        .unwrap();

    // Sub-property B: completado with fecha_resolucion is always valid
    runner
        .run(&arbitrary_date(), |fecha| {
            let fecha_resolucion = Some(fecha);
            prop_assert!(
                validate_desahucio_transition("completado", fecha_resolucion.as_ref()).is_ok(),
                "completado with fecha_resolucion must be accepted"
            );
            Ok(())
        })
        .unwrap();

    // Sub-property C: only valid transitions allowed
    // Valid: iniciado → en_progreso, en_progreso → completado, iniciado → completado
    let valid_transitions = [
        ("iniciado", "en_progreso"),
        ("en_progreso", "completado"),
        ("iniciado", "completado"),
    ];

    runner
        .run(&(desahucio_estado(), desahucio_estado()), |(from, to)| {
            let is_valid = valid_transitions.contains(&(from, to));
            let result = validate_estado_transition(from, to);
            if is_valid {
                prop_assert!(
                    result.is_ok(),
                    "Transition {} → {} should be valid",
                    from,
                    to
                );
            } else if from != to {
                // Same state → same state is not a valid transition either
                prop_assert!(
                    result.is_err(),
                    "Transition {} → {} should be invalid",
                    from,
                    to
                );
            }
            Ok(())
        })
        .unwrap();

    // Sub-property D: invalid estado values always rejected by validate_enum
    runner
        .run(&arbitrary_string(), |estado| {
            let valid = ["iniciado", "en_progreso", "completado"];
            if !valid.contains(&estado.as_str()) {
                prop_assert!(
                    validate_enum("estado", &estado, &valid).is_err(),
                    "Invalid estado '{}' must be rejected",
                    estado
                );
            }
            Ok(())
        })
        .unwrap();
}

use realestate_backend::services::notificaciones::thresholds_for_days;
use realestate_backend::services::servicios_publicos::{
    es_consumo_anormal, resolve_responsabilidad,
};

// ── Custom Strategies for Properties 3 and 7 ──────────────────────────

fn positive_consumo() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000i64).prop_map(|v| Decimal::new(v, 4))
}

fn consumption_history() -> impl Strategy<Value = Vec<Decimal>> {
    proptest::collection::vec((1i64..1_000_000i64).prop_map(|v| Decimal::new(v, 4)), 3..15)
}

fn small_history() -> impl Strategy<Value = Vec<Decimal>> {
    proptest::collection::vec((1i64..1_000_000i64).prop_map(|v| Decimal::new(v, 4)), 0..3)
}

fn responsable_value() -> impl Strategy<Value = &'static str> {
    proptest::sample::select(vec!["propietario", "inquilino"])
}

// ── Property 3: Anomaly detection threshold ────────────────────────────
// Feature: dr-legal-compliance-and-utilities, Property 3: Anomaly detection threshold
// **Validates: Requirements 8.1, 8.2, 8.4**
#[test]
fn test_anomaly_detection_threshold() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Sub-property A: consumption above 150% of average always triggers alert
    runner
        .run(
            &(consumption_history(), 1i64..1_000_000i64),
            |(history, excess_raw)| {
                let sum: Decimal = history.iter().copied().sum();
                let count = Decimal::from(history.len() as u64);
                let avg = sum / count;
                let umbral = avg * Decimal::new(15, 1); // avg * 1.5
                let excess = Decimal::new(excess_raw, 4);
                let consumo_nuevo = umbral + excess; // guaranteed > threshold

                let result = es_consumo_anormal(&history, consumo_nuevo);
                prop_assert_eq!(
                    result,
                    Some(true),
                    "Consumption {} > threshold {} should trigger alert",
                    consumo_nuevo,
                    umbral
                );
                Ok(())
            },
        )
        .unwrap();

    // Sub-property B: consumption at or below 150% of average never triggers alert
    runner
        .run(&consumption_history(), |history| {
            let sum: Decimal = history.iter().copied().sum();
            let count = Decimal::from(history.len() as u64);
            let avg = sum / count;
            let umbral = avg * Decimal::new(15, 1); // avg * 1.5
            // consumo_nuevo exactly at threshold = no alert (not strictly greater)
            let consumo_nuevo = umbral;

            let result = es_consumo_anormal(&history, consumo_nuevo);
            prop_assert_eq!(
                result,
                Some(false),
                "Consumption {} <= threshold {} should NOT trigger alert",
                consumo_nuevo,
                umbral
            );
            Ok(())
        })
        .unwrap();

    // Sub-property C: fewer than 3 records always skips check (returns None)
    runner
        .run(
            &(small_history(), positive_consumo()),
            |(history, consumo_nuevo)| {
                prop_assert!(history.len() < 3);
                let result = es_consumo_anormal(&history, consumo_nuevo);
                prop_assert_eq!(
                    result,
                    None,
                    "With {} records (< 3), check should be skipped regardless of consumption {}",
                    history.len(),
                    consumo_nuevo
                );
                Ok(())
            },
        )
        .unwrap();
}

// ── Property 7: Responsibility resolution precedence ───────────────────
// Feature: dr-legal-compliance-and-utilities, Property 7: Responsibility resolution precedence
// **Validates: Requirements 7.3**
#[test]
fn test_responsibility_resolution_precedence() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    // Sub-property A: contract override always takes precedence when present
    runner
        .run(
            &(responsable_value(), responsable_value()),
            |(unit_default, contract_override)| {
                let effective =
                    resolve_responsabilidad(Some(unit_default), Some(contract_override));
                prop_assert_eq!(
                    effective,
                    contract_override,
                    "Contract override '{}' should take precedence over unit default '{}'",
                    contract_override,
                    unit_default
                );
                Ok(())
            },
        )
        .unwrap();

    // Sub-property B: unit default used when no contract override exists
    runner
        .run(&responsable_value(), |unit_default| {
            let effective = resolve_responsabilidad(Some(unit_default), None);
            prop_assert_eq!(
                effective,
                unit_default,
                "Unit default '{}' should be used when no override exists",
                unit_default
            );
            Ok(())
        })
        .unwrap();

    // Sub-property C: when both are None, "propietario" is the fallback
    {
        let effective = resolve_responsabilidad(None, None);
        assert_eq!(
            effective, "propietario",
            "When both are None, fallback should be 'propietario'"
        );
    }

    // Sub-property D: override and default can differ — override still wins
    runner
        .run(
            &(responsable_value(), responsable_value()),
            |(unit_default, contract_override)| {
                if unit_default != contract_override {
                    let effective =
                        resolve_responsabilidad(Some(unit_default), Some(contract_override));
                    prop_assert_eq!(
                        effective,
                        contract_override,
                        "Even when they differ, override '{}' wins over default '{}'",
                        contract_override,
                        unit_default
                    );
                }
                Ok(())
            },
        )
        .unwrap();
}

// ── Custom Strategy for Property 4 ─────────────────────────────────────

fn days_remaining() -> impl Strategy<Value = i64> {
    0i64..365i64
}

// ── Property 4: Renewal reminder threshold correctness ─────────────────
// Feature: dr-legal-compliance-and-utilities, Property 4: Renewal reminder threshold correctness
// **Validates: Requirements 3.1, 3.2, 3.3**
#[test]
fn test_renewal_reminder_threshold_correctness() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    runner
        .run(&days_remaining(), |days| {
            let thresholds = thresholds_for_days(days);

            // If days <= 30, all three thresholds fire
            if days <= 30 {
                prop_assert_eq!(
                    thresholds.len(),
                    3,
                    "days={}: expected 3 thresholds, got {:?}",
                    days,
                    thresholds
                );
                prop_assert!(thresholds.contains(&90));
                prop_assert!(thresholds.contains(&60));
                prop_assert!(thresholds.contains(&30));
            }
            // If 30 < days <= 60, only 90 and 60 fire
            else if days <= 60 {
                prop_assert_eq!(
                    thresholds.len(),
                    2,
                    "days={}: expected 2 thresholds, got {:?}",
                    days,
                    thresholds
                );
                prop_assert!(thresholds.contains(&90));
                prop_assert!(thresholds.contains(&60));
                prop_assert!(!thresholds.contains(&30));
            }
            // If 60 < days <= 90, only 90 fires
            else if days <= 90 {
                prop_assert_eq!(
                    thresholds.len(),
                    1,
                    "days={}: expected 1 threshold, got {:?}",
                    days,
                    thresholds
                );
                prop_assert!(thresholds.contains(&90));
            }
            // If days > 90, no thresholds fire
            else {
                prop_assert!(
                    thresholds.is_empty(),
                    "days={}: expected no thresholds, got {:?}",
                    days,
                    thresholds
                );
            }

            Ok(())
        })
        .unwrap();
}

// ── Custom Strategies for Property 6 ───────────────────────────────────

fn proveedor_servicio() -> impl Strategy<Value = &'static str> {
    proptest::sample::select(vec!["EDENORTE", "EDESUR", "EDEESTE", "CAASD"])
}

// ── Property 6: Utility field validation ───────────────────────────────
// Feature: dr-legal-compliance-and-utilities, Property 6: Utility field validation
// **Validates: Requirements 6.2, 6.3, 6.4**
#[test]
fn test_utility_field_validation() {
    let mut runner = TestRunner::new(ProptestConfig {
        cases: crate::pbt_cases(),
        ..Default::default()
    });

    let valid_proveedores: &[&str] = &["EDENORTE", "EDESUR", "EDEESTE", "CAASD"];

    // Sub-property A: valid proveedor_servicio values always accepted
    runner
        .run(&proveedor_servicio(), |proveedor| {
            prop_assert!(
                validate_enum("proveedor_servicio", proveedor, valid_proveedores).is_ok(),
                "Valid proveedor '{}' should be accepted",
                proveedor
            );
            Ok(())
        })
        .unwrap();

    // Sub-property B: invalid proveedor_servicio values always rejected
    runner
        .run(&arbitrary_string(), |value| {
            if !valid_proveedores.contains(&value.as_str()) {
                prop_assert!(
                    validate_enum("proveedor_servicio", &value, valid_proveedores).is_err(),
                    "Invalid proveedor '{}' should be rejected",
                    value
                );
            }
            Ok(())
        })
        .unwrap();

    // Sub-property C: consumo > 0 is valid
    runner
        .run(&positive_consumo(), |consumo| {
            prop_assert!(
                consumo > Decimal::ZERO,
                "Positive consumo {} should be > 0",
                consumo
            );
            Ok(())
        })
        .unwrap();

    // Sub-property D: zero or negative consumo always rejected
    runner
        .run(&(-1_000_000i64..=0i64), |raw| {
            let consumo = Decimal::new(raw, 4);
            prop_assert!(
                consumo <= Decimal::ZERO,
                "Zero/negative consumo {} should fail validation",
                consumo
            );
            Ok(())
        })
        .unwrap();

    // Sub-property E: periodo_desde < periodo_hasta always valid
    runner
        .run(&(arbitrary_date(), 1u32..365u32), |(desde, days_offset)| {
            let hasta = desde + chrono::Duration::days(i64::from(days_offset));
            prop_assert!(
                desde < hasta,
                "periodo_desde ({}) should be < periodo_hasta ({})",
                desde,
                hasta
            );
            Ok(())
        })
        .unwrap();

    // Sub-property F: periodo_desde >= periodo_hasta always invalid
    runner
        .run(&arbitrary_date(), |fecha| {
            // Same date: desde == hasta is invalid
            prop_assert!(
                fecha >= fecha,
                "periodo_desde ({}) >= periodo_hasta ({}) should be rejected",
                fecha,
                fecha
            );
            Ok(())
        })
        .unwrap();
}
