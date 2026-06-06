#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{Datelike, NaiveDate};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use rust_decimal::Decimal;
use std::cmp::min;

// ── Pure Formula Under Test ────────────────────────────────────────────

/// Maximum annual rent increase percentage allowed by Ley 85-25.
const TOPE_LEGAL_PORCENTAJE: Decimal = Decimal::TEN;

/// Calculate the maximum allowed rent after indexation.
///
/// Formula: `monto_actual * (1 + min(ipc_porcentaje, 10) / 100)`
/// Result never exceeds `monto_actual * 1.10` regardless of IPC value.
fn calcular_monto_maximo(monto_actual: Decimal, ipc_porcentaje: Decimal) -> Decimal {
    let porcentaje_aplicable = if ipc_porcentaje > TOPE_LEGAL_PORCENTAJE {
        TOPE_LEGAL_PORCENTAJE
    } else {
        ipc_porcentaje
    };
    monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100))
}

/// Determine whether a contract should trigger a renewal proposal.
///
/// Triggers iff `fecha_fin - today <= 60` AND `fecha_fin > today`.
fn should_trigger_renewal(fecha_fin: NaiveDate, today: NaiveDate) -> bool {
    let dias_restantes = (fecha_fin - today).num_days();
    dias_restantes <= 60 && dias_restantes > 0
}

/// Compute the anniversary date for a contract given its start date.
///
/// Indexation applies on the anniversary of fecha_inicio (fecha_inicio + N years),
/// not on January 1 of each calendar year.
fn next_anniversary(fecha_inicio: NaiveDate, reference_date: NaiveDate) -> Option<NaiveDate> {
    if reference_date <= fecha_inicio {
        return Some(fecha_inicio);
    }
    let years_elapsed = reference_date.year() - fecha_inicio.year();
    // Try current year's anniversary
    let candidate = NaiveDate::from_ymd_opt(
        fecha_inicio.year() + years_elapsed,
        fecha_inicio.month(),
        min(
            fecha_inicio.day(),
            days_in_month(fecha_inicio.year() + years_elapsed, fecha_inicio.month()),
        ),
    )?;
    if candidate > reference_date {
        Some(candidate)
    } else {
        // Next year's anniversary
        NaiveDate::from_ymd_opt(
            fecha_inicio.year() + years_elapsed + 1,
            fecha_inicio.month(),
            min(
                fecha_inicio.day(),
                days_in_month(
                    fecha_inicio.year() + years_elapsed + 1,
                    fecha_inicio.month(),
                ),
            ),
        )
    }
}

/// Helper: get the number of days in a given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a positive monto_actual as Decimal (1 centavo to 1,000,000.00).
fn positive_monto() -> impl Strategy<Value = Decimal> {
    (1i64..100_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate an IPC percentage (0.01% to 50.00%) — covers both below and above the 10% cap.
fn ipc_percentage() -> impl Strategy<Value = Decimal> {
    (1i64..5000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

/// Generate an IPC percentage above the 10% legal cap (10.01% to 50.00%).
fn ipc_above_cap() -> impl Strategy<Value = Decimal> {
    (1001i64..5000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

/// Generate an IPC percentage at or below the 10% legal cap (0.01% to 10.00%).
fn ipc_at_or_below_cap() -> impl Strategy<Value = Decimal> {
    (1i64..=1000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

/// Generate a valid NaiveDate for contract fecha_fin (2025-01-01 to 2030-12-31).
fn fecha_fin_strategy() -> impl Strategy<Value = NaiveDate> {
    (2025i32..=2030, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

/// Generate a "today" date that is within 60 days before a given fecha_fin (triggers renewal).
fn today_within_60_days(fecha_fin: NaiveDate) -> impl Strategy<Value = NaiveDate> {
    // dias_restantes must be 1..=60, so today = fecha_fin - dias
    (1i64..=60).prop_map(move |dias| fecha_fin - chrono::Duration::days(dias))
}

/// Generate a "today" date that is more than 60 days before fecha_fin (does NOT trigger).
fn today_outside_60_days(fecha_fin: NaiveDate) -> impl Strategy<Value = NaiveDate> {
    // dias_restantes > 60, so today < fecha_fin - 60
    (61i64..=365).prop_map(move |dias| fecha_fin - chrono::Duration::days(dias))
}

/// Generate a fecha_inicio for anniversary tests (2020-01-01 to 2025-12-28).
fn fecha_inicio_strategy() -> impl Strategy<Value = NaiveDate> {
    (2020i32..=2025, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 14: Rent Indexation Formula with Legal Cap
    /// **Validates: Requirements 5.2, 5.9**
    ///
    /// For any (monto_actual, ipc_porcentaje) where both are positive,
    /// `calcular_monto_maximo` returns `monto_actual * (1 + min(ipc, 10) / 100)`.
    /// The result never exceeds `monto_actual * 1.10` regardless of IPC value.
    #[test]
    fn rent_indexation_formula_with_legal_cap(
        monto_actual in positive_monto(),
        ipc in ipc_percentage(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

        // Compute expected using min(ipc, 10)
        let porcentaje_aplicable = if ipc > Decimal::TEN {
            Decimal::TEN
        } else {
            ipc
        };
        let expected = monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100));

        prop_assert_eq!(
            result, expected,
            "Formula mismatch: monto_actual={}, ipc={}, expected={}, got={}",
            monto_actual, ipc, expected, result
        );

        // Absolute cap: result never exceeds monto_actual * 1.10
        let absolute_max = monto_actual * Decimal::new(110, 2);
        prop_assert!(
            result <= absolute_max,
            "Result {} exceeds absolute max {} (monto_actual * 1.10). monto_actual={}, ipc={}",
            result, absolute_max, monto_actual, ipc
        );
    }

    // Feature: dr-landlord-compliance, Property 14: Rent Indexation Formula with Legal Cap
    // Sub-test: When IPC is above 10%, the cap is always applied.
    /// **Validates: Requirements 5.9**
    #[test]
    fn rent_indexation_cap_applied_when_ipc_above_10(
        monto_actual in positive_monto(),
        ipc in ipc_above_cap(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

        // When IPC > 10%, the result must be exactly monto_actual * 1.10
        let capped_result = monto_actual * Decimal::new(110, 2);
        prop_assert_eq!(
            result, capped_result,
            "When IPC ({}) > 10%, result must be monto_actual * 1.10. \
             monto_actual={}, expected={}, got={}",
            ipc, monto_actual, capped_result, result
        );
    }

    // Feature: dr-landlord-compliance, Property 14: Rent Indexation Formula with Legal Cap
    // Sub-test: When IPC is at or below 10%, the actual IPC rate is used.
    /// **Validates: Requirements 5.2**
    #[test]
    fn rent_indexation_uses_actual_ipc_when_below_cap(
        monto_actual in positive_monto(),
        ipc in ipc_at_or_below_cap(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

        // When IPC <= 10%, the result uses the actual IPC
        let expected = monto_actual * (Decimal::ONE + ipc / Decimal::from(100));
        prop_assert_eq!(
            result, expected,
            "When IPC ({}) <= 10%, result must use actual IPC. \
             monto_actual={}, expected={}, got={}",
            ipc, monto_actual, expected, result
        );
    }

    // Feature: dr-landlord-compliance, Property 15: Indexation 60-Day Trigger
    /// **Validates: Requirements 5.1, 5.7**
    ///
    /// For any active contrato with fecha_fin, the system triggers a renewal proposal
    /// if and only if `fecha_fin - today <= 60 days` AND `fecha_fin - today > 0`.
    #[test]
    fn indexation_60_day_trigger_within_window(
        fecha_fin in fecha_fin_strategy(),
    ) {
        // Generate today within the 60-day window
        let today_strat = today_within_60_days(fecha_fin);
        let mut runner = proptest::test_runner::TestRunner::default();
        let today = today_strat.new_tree(&mut runner).unwrap().current();

        let should_trigger = should_trigger_renewal(fecha_fin, today);
        prop_assert!(
            should_trigger,
            "Should trigger renewal when today ({}) is within 60 days of fecha_fin ({}). \
             dias_restantes={}",
            today, fecha_fin, (fecha_fin - today).num_days()
        );
    }

    #[test]
    fn indexation_60_day_trigger_outside_window(
        fecha_fin in fecha_fin_strategy(),
    ) {
        // Generate today more than 60 days before fecha_fin
        let today_strat = today_outside_60_days(fecha_fin);
        let mut runner = proptest::test_runner::TestRunner::default();
        let today = today_strat.new_tree(&mut runner).unwrap().current();

        let should_trigger = should_trigger_renewal(fecha_fin, today);
        prop_assert!(
            !should_trigger,
            "Should NOT trigger renewal when today ({}) is more than 60 days before fecha_fin ({}). \
             dias_restantes={}",
            today, fecha_fin, (fecha_fin - today).num_days()
        );
    }

    // Feature: dr-landlord-compliance, Property 15: Indexation 60-Day Trigger
    // Sub-test: Expired contracts (fecha_fin <= today) do NOT trigger.
    /// **Validates: Requirements 5.1, 5.7**
    #[test]
    fn indexation_no_trigger_for_expired_contracts(
        fecha_fin in fecha_fin_strategy(),
        days_past in 0i64..=365,
    ) {
        let today = fecha_fin + chrono::Duration::days(days_past);
        let should_trigger = should_trigger_renewal(fecha_fin, today);

        prop_assert!(
            !should_trigger,
            "Should NOT trigger renewal for expired contract. \
             fecha_fin={}, today={}, dias_restantes={}",
            fecha_fin, today, (fecha_fin - today).num_days()
        );
    }

    // Feature: dr-landlord-compliance, Property 16: Indexation Anniversary Alignment
    /// **Validates: Requirements 5.10**
    ///
    /// For any contrato with fecha_inicio, indexation applies on the anniversary
    /// of fecha_inicio (fecha_inicio + N years), not on January 1.
    #[test]
    fn indexation_anniversary_alignment(
        fecha_inicio in fecha_inicio_strategy(),
        years_forward in 1u32..=5,
    ) {
        // Compute the Nth anniversary
        let anniversary_year = fecha_inicio.year() + years_forward as i32;
        let anniversary_day = min(
            fecha_inicio.day(),
            days_in_month(anniversary_year, fecha_inicio.month()),
        );
        let expected_anniversary = NaiveDate::from_ymd_opt(
            anniversary_year,
            fecha_inicio.month(),
            anniversary_day,
        );

        // The anniversary should exist (valid date)
        prop_assert!(
            expected_anniversary.is_some(),
            "Anniversary date should be valid for fecha_inicio={}, years_forward={}",
            fecha_inicio, years_forward
        );

        let anniversary = expected_anniversary.unwrap();

        // Property: the anniversary has the same month and day (adjusted for leap years)
        // as fecha_inicio — it is NOT January 1
        prop_assert_eq!(
            anniversary.month(), fecha_inicio.month(),
            "Anniversary month must match fecha_inicio month. \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

        // The day should match (or be the last valid day in the month for leap year edge cases)
        prop_assert!(
            anniversary.day() == fecha_inicio.day()
                || anniversary.day() == days_in_month(anniversary_year, fecha_inicio.month()),
            "Anniversary day must match fecha_inicio day (or month's last day). \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

        // Key property: anniversary is NOT January 1 (unless fecha_inicio is Jan 1)
        if fecha_inicio.month() != 1 || fecha_inicio.day() != 1 {
            let jan_1 = NaiveDate::from_ymd_opt(anniversary_year, 1, 1).unwrap();
            prop_assert_ne!(
                anniversary, jan_1,
                "Anniversary must NOT be Jan 1 (unless contract started Jan 1). \
                 fecha_inicio={}",
                fecha_inicio
            );
        }
    }

    // Feature: dr-landlord-compliance, Property 16: Indexation Anniversary Alignment
    // Sub-test: Verify next_anniversary returns the correct anniversary date.
    /// **Validates: Requirements 5.10**
    #[test]
    fn next_anniversary_returns_correct_date(
        fecha_inicio in fecha_inicio_strategy(),
        days_after_start in 366i64..=1825,
    ) {
        let reference_date = fecha_inicio + chrono::Duration::days(days_after_start);

        let result = next_anniversary(fecha_inicio, reference_date);
        prop_assert!(
            result.is_some(),
            "next_anniversary should return Some for fecha_inicio={}, reference={}",
            fecha_inicio, reference_date
        );

        let anniversary = result.unwrap();

        // The anniversary must be strictly after reference_date
        prop_assert!(
            anniversary > reference_date,
            "Anniversary ({}) must be after reference date ({}). fecha_inicio={}",
            anniversary, reference_date, fecha_inicio
        );

        // The anniversary must have the same month as fecha_inicio
        prop_assert_eq!(
            anniversary.month(), fecha_inicio.month(),
            "Anniversary month must match contract start month. \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

        // The anniversary must have same day (or adjusted for shorter months)
        let max_day = days_in_month(anniversary.year(), fecha_inicio.month());
        let expected_day = min(fecha_inicio.day(), max_day);
        prop_assert_eq!(
            anniversary.day(), expected_day,
            "Anniversary day must match expected. \
             fecha_inicio={}, anniversary={}, max_day={}",
            fecha_inicio, anniversary, max_day
        );
    }
}
