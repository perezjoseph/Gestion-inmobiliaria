#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::inconsistent_digit_grouping
)]

use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::services::ipi::{calcular_ipi_monto, calcular_ipi_proporcional};

fn positive_valor_catastral() -> impl Strategy<Value = Decimal> {
    (1i64..5_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn positive_umbral() -> impl Strategy<Value = Decimal> {
    (1_000_000i64..20_000_000i64).prop_map(|val| Decimal::new(val, 0))
}

fn coowner_percentages() -> impl Strategy<Value = Vec<Decimal>> {
    (2usize..=5usize).prop_flat_map(|count| {
        proptest::collection::vec(1u32..1000u32, count).prop_map(|weights| {
            let total: u32 = weights.iter().sum();
            let mut percentages: Vec<Decimal> = weights
                .iter()
                .map(|&w| {
                    let scaled = (w as u64 * 10000) / total as u64;
                    Decimal::new(scaled as i64, 2)
                })
                .collect();

            let current_sum: Decimal = percentages.iter().copied().sum();
            let hundred = Decimal::new(100, 0);
            if let Some(last) = percentages.last_mut() {
                *last += hundred - current_sum;
            }

            percentages
        })
    })
}

fn positive_ipi_total() -> impl Strategy<Value = Decimal> {
    (1i64..100_000_000_i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn valid_percentage() -> impl Strategy<Value = Decimal> {
    (1i64..10000i64).prop_map(|hundredths| Decimal::new(hundredths, 2))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn ipi_calculation_formula(
        valor_total in positive_valor_catastral(),
        umbral in positive_umbral(),
    ) {
        let result = calcular_ipi_monto(valor_total, umbral);

        let exceso = valor_total - umbral;
        if exceso > Decimal::ZERO {
            let expected = exceso * Decimal::new(1, 2);
            prop_assert_eq!(
                result, expected,
                "IPI should equal (valor_total - umbral) * 0.01 when valor > umbral. \
                 valor_total={}, umbral={}, exceso={}",
                valor_total, umbral, exceso
            );
        } else {
            prop_assert_eq!(
                result, Decimal::ZERO,
                "IPI should be zero when valor_total <= umbral. \
                 valor_total={}, umbral={}",
                valor_total, umbral
            );
        }
    }

    #[test]
    fn ipi_never_negative(
        valor_total in positive_valor_catastral(),
        umbral in positive_umbral(),
    ) {
        let result = calcular_ipi_monto(valor_total, umbral);
        prop_assert!(
            result >= Decimal::ZERO,
            "IPI must never be negative. Got {} for valor_total={}, umbral={}",
            result, valor_total, umbral
        );
    }

    #[test]
    fn ipi_zero_at_threshold(
        umbral in positive_umbral(),
    ) {
        let result = calcular_ipi_monto(umbral, umbral);
        prop_assert_eq!(
            result, Decimal::ZERO,
            "IPI should be zero when valor_total equals umbral exactly. umbral={}",
            umbral
        );
    }

    #[test]
    fn ipi_rate_is_one_percent(
        excess_cents in 1i64..5_000_000_000i64,
        umbral in positive_umbral(),
    ) {
        let excess = Decimal::new(excess_cents, 2);
        let valor_total = umbral + excess;
        let result = calcular_ipi_monto(valor_total, umbral);

        let expected = excess * Decimal::new(1, 2);
        prop_assert_eq!(
            result, expected,
            "IPI must be exactly 1% of excess. excess={}, expected={}, got={}",
            excess, expected, result
        );
    }

    #[test]
    fn ipi_coowner_proportional_sum_equals_total(
        ipi_total in positive_ipi_total(),
        percentages in coowner_percentages(),
    ) {
        let pct_sum: Decimal = percentages.iter().copied().sum();
        prop_assert_eq!(
            pct_sum,
            Decimal::new(100, 0),
            "Test setup: percentages must sum to 100, got {}",
            pct_sum
        );

        let shares: Vec<Decimal> = percentages
            .iter()
            .map(|&pct| calcular_ipi_proporcional(ipi_total, pct))
            .collect();

        let shares_sum: Decimal = shares.iter().copied().sum();
        prop_assert_eq!(
            shares_sum, ipi_total,
            "Sum of co-owner IPI shares must equal total IPI. \
             ipi_total={}, shares_sum={}, percentages={:?}",
            ipi_total, shares_sum, percentages
        );
    }

    #[test]
    fn ipi_coowner_individual_share_formula(
        ipi_total in positive_ipi_total(),
        porcentaje in valid_percentage(),
    ) {
        let result = calcular_ipi_proporcional(ipi_total, porcentaje);
        let expected = ipi_total * porcentaje / Decimal::new(100, 0);

        prop_assert_eq!(
            result, expected,
            "Co-owner share must equal ipi_total * porcentaje / 100. \
             ipi_total={}, porcentaje={}, expected={}, got={}",
            ipi_total, porcentaje, expected, result
        );
    }

    #[test]
    fn ipi_coowner_full_ownership_gets_full_amount(
        ipi_total in positive_ipi_total(),
    ) {
        let result = calcular_ipi_proporcional(ipi_total, Decimal::new(100, 0));
        prop_assert_eq!(
            result, ipi_total,
            "100% owner must pay full IPI. ipi_total={}, got={}",
            ipi_total, result
        );
    }

    #[test]
    fn ipi_coowner_zero_ownership_pays_nothing(
        ipi_total in positive_ipi_total(),
    ) {
        let result = calcular_ipi_proporcional(ipi_total, Decimal::ZERO);
        prop_assert_eq!(
            result, Decimal::ZERO,
            "0% owner must pay zero IPI. ipi_total={}, got={}",
            ipi_total, result
        );
    }
}
