#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{Datelike, NaiveDate};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use rust_decimal::Decimal;
use std::cmp::min;

const TOPE_LEGAL_PORCENTAJE: Decimal = Decimal::TEN;

fn calcular_monto_maximo(monto_actual: Decimal, ipc_porcentaje: Decimal) -> Decimal {
    let porcentaje_aplicable = if ipc_porcentaje > TOPE_LEGAL_PORCENTAJE {
        TOPE_LEGAL_PORCENTAJE
    } else {
        ipc_porcentaje
    };
    monto_actual * (Decimal::ONE + porcentaje_aplicable / Decimal::from(100))
}

fn should_trigger_renewal(fecha_fin: NaiveDate, today: NaiveDate) -> bool {
    let dias_restantes = (fecha_fin - today).num_days();
    dias_restantes <= 60 && dias_restantes > 0
}

fn next_anniversary(fecha_inicio: NaiveDate, reference_date: NaiveDate) -> Option<NaiveDate> {
    if reference_date <= fecha_inicio {
        return Some(fecha_inicio);
    }
    let years_elapsed = reference_date.year() - fecha_inicio.year();
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

fn positive_monto() -> impl Strategy<Value = Decimal> {
    (1i64..100_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn ipc_percentage() -> impl Strategy<Value = Decimal> {
    (1i64..5000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

fn ipc_above_cap() -> impl Strategy<Value = Decimal> {
    (1001i64..5000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

fn ipc_at_or_below_cap() -> impl Strategy<Value = Decimal> {
    (1i64..=1000i64).prop_map(|basis_points| Decimal::new(basis_points, 2))
}

fn fecha_fin_strategy() -> impl Strategy<Value = NaiveDate> {
    (2025i32..=2030, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

fn today_within_60_days(fecha_fin: NaiveDate) -> impl Strategy<Value = NaiveDate> {
    (1i64..=60).prop_map(move |dias| fecha_fin - chrono::Duration::days(dias))
}

fn today_outside_60_days(fecha_fin: NaiveDate) -> impl Strategy<Value = NaiveDate> {
    (61i64..=365).prop_map(move |dias| fecha_fin - chrono::Duration::days(dias))
}

fn fecha_inicio_strategy() -> impl Strategy<Value = NaiveDate> {
    (2020i32..=2025, 1u32..=12, 1u32..=28)
        .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn rent_indexation_formula_with_legal_cap(
        monto_actual in positive_monto(),
        ipc in ipc_percentage(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

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

        let absolute_max = monto_actual * Decimal::new(110, 2);
        prop_assert!(
            result <= absolute_max,
            "Result {} exceeds absolute max {} (monto_actual * 1.10). monto_actual={}, ipc={}",
            result, absolute_max, monto_actual, ipc
        );
    }

    #[test]
    fn rent_indexation_cap_applied_when_ipc_above_10(
        monto_actual in positive_monto(),
        ipc in ipc_above_cap(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

        let capped_result = monto_actual * Decimal::new(110, 2);
        prop_assert_eq!(
            result, capped_result,
            "When IPC ({}) > 10%, result must be monto_actual * 1.10. \
             monto_actual={}, expected={}, got={}",
            ipc, monto_actual, capped_result, result
        );
    }

    #[test]
    fn rent_indexation_uses_actual_ipc_when_below_cap(
        monto_actual in positive_monto(),
        ipc in ipc_at_or_below_cap(),
    ) {
        let result = calcular_monto_maximo(monto_actual, ipc);

        let expected = monto_actual * (Decimal::ONE + ipc / Decimal::from(100));
        prop_assert_eq!(
            result, expected,
            "When IPC ({}) <= 10%, result must use actual IPC. \
             monto_actual={}, expected={}, got={}",
            ipc, monto_actual, expected, result
        );
    }

    #[test]
    fn indexation_60_day_trigger_within_window(
        fecha_fin in fecha_fin_strategy(),
    ) {
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

    #[test]
    fn indexation_anniversary_alignment(
        fecha_inicio in fecha_inicio_strategy(),
        years_forward in 1u32..=5,
    ) {
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

        prop_assert!(
            expected_anniversary.is_some(),
            "Anniversary date should be valid for fecha_inicio={}, years_forward={}",
            fecha_inicio, years_forward
        );

        let anniversary = expected_anniversary.unwrap();

        prop_assert_eq!(
            anniversary.month(), fecha_inicio.month(),
            "Anniversary month must match fecha_inicio month. \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

        prop_assert!(
            anniversary.day() == fecha_inicio.day()
                || anniversary.day() == days_in_month(anniversary_year, fecha_inicio.month()),
            "Anniversary day must match fecha_inicio day (or month's last day). \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

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

        prop_assert!(
            anniversary > reference_date,
            "Anniversary ({}) must be after reference date ({}). fecha_inicio={}",
            anniversary, reference_date, fecha_inicio
        );

        prop_assert_eq!(
            anniversary.month(), fecha_inicio.month(),
            "Anniversary month must match contract start month. \
             fecha_inicio={}, anniversary={}",
            fecha_inicio, anniversary
        );

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
