#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::services::ipi::{calcular_ipi_monto, calcular_ipi_proporcional};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a positive valor_catastral as Decimal.
/// Range: 1 to 50,000,000 (covers realistic property values in DOP).
fn positive_valor_catastral() -> impl Strategy<Value = Decimal> {
    (1i64..5_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a positive umbral value.
/// Realistic range around the 2026 threshold (RD$10,695,494).
fn positive_umbral() -> impl Strategy<Value = Decimal> {
    (1_000_000i64..20_000_000i64).prop_map(|val| Decimal::new(val, 0))
}

/// Generate co-owner percentages that sum to exactly 100%.
/// Returns a Vec of 2-5 percentages.
fn coowner_percentages() -> impl Strategy<Value = Vec<Decimal>> {
    // Generate 2-5 integer parts that sum to 10000 (representing 100.00%)
    (2usize..=5usize).prop_flat_map(|count| {
        // Generate `count` random weights, then normalize to sum to 10000
        proptest::collection::vec(1u32..1000u32, count).prop_map(|weights| {
            let total: u32 = weights.iter().sum();
            let mut percentages: Vec<Decimal> = weights
                .iter()
                .map(|&w| {
                    // Scale to hundredths of a percent, floor
                    let scaled = (w as u64 * 10000) / total as u64;
                    Decimal::new(scaled as i64, 2)
                })
                .collect();

            // Adjust last element to make sum exactly 100.00
            let current_sum: Decimal = percentages.iter().copied().sum();
            let hundred = Decimal::new(100, 0);
            if let Some(last) = percentages.last_mut() {
                *last += hundred - current_sum;
            }

            percentages
        })
    })
}

/// Generate a positive IPI total amount.
fn positive_ipi_total() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000_00i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a valid ownership percentage (0.01 to 100.00).
fn valid_percentage() -> impl Strategy<Value = Decimal> {
    (1i64..10000i64).prop_map(|hundredths| Decimal::new(hundredths, 2))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 27: IPI Calculation
    // For any set of property values (excluding exento), IPI = max(0, sum - umbral) * 0.01.
    // IPI applies regardless of tipo_fiscal.
    /// **Validates: Requirements 9.1, 9.2, 9.7, 9.8**
    #[test]
    fn ipi_calculation_formula(
        valor_total in positive_valor_catastral(),
        umbral in positive_umbral(),
    ) {
        let result = calcular_ipi_monto(valor_total, umbral);

        let exceso = valor_total - umbral;
        if exceso > Decimal::ZERO {
            let expected = exceso * Decimal::new(1, 2); // * 0.01
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

    // Feature: dr-landlord-compliance, Property 27: IPI Calculation
    // IPI is always non-negative regardless of inputs.
    /// **Validates: Requirements 9.1, 9.2**
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

    // Feature: dr-landlord-compliance, Property 27: IPI Calculation
    // When valor_total equals the umbral exactly, IPI is zero.
    /// **Validates: Requirements 9.1, 9.2**
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

    // Feature: dr-landlord-compliance, Property 27: IPI Calculation
    // IPI rate is exactly 1% of the excess.
    /// **Validates: Requirements 9.2, 9.7**
    #[test]
    fn ipi_rate_is_one_percent(
        excess_cents in 1i64..5_000_000_000i64,
        umbral in positive_umbral(),
    ) {
        let excess = Decimal::new(excess_cents, 2);
        let valor_total = umbral + excess;
        let result = calcular_ipi_monto(valor_total, umbral);

        // IPI should be exactly 1% of the excess
        let expected = excess * Decimal::new(1, 2);
        prop_assert_eq!(
            result, expected,
            "IPI must be exactly 1% of excess. excess={}, expected={}, got={}",
            excess, expected, result
        );
    }

    // Feature: dr-landlord-compliance, Property 28: IPI Co-Owner Proportional Split
    // For any property with co-owners where sum(porcentaje) = 100%,
    // each co-owner's IPI = ipi_total * porcentaje / 100,
    // and sum of all equals the property's total IPI contribution.
    /// **Validates: Requirements 9.10**
    #[test]
    fn ipi_coowner_proportional_sum_equals_total(
        ipi_total in positive_ipi_total(),
        percentages in coowner_percentages(),
    ) {
        // Verify percentages sum to 100
        let pct_sum: Decimal = percentages.iter().copied().sum();
        prop_assert_eq!(
            pct_sum,
            Decimal::new(100, 0),
            "Test setup: percentages must sum to 100, got {}",
            pct_sum
        );

        // Calculate each co-owner's share
        let shares: Vec<Decimal> = percentages
            .iter()
            .map(|&pct| calcular_ipi_proporcional(ipi_total, pct))
            .collect();

        // Sum of all shares must equal ipi_total
        let shares_sum: Decimal = shares.iter().copied().sum();
        prop_assert_eq!(
            shares_sum, ipi_total,
            "Sum of co-owner IPI shares must equal total IPI. \
             ipi_total={}, shares_sum={}, percentages={:?}",
            ipi_total, shares_sum, percentages
        );
    }

    // Feature: dr-landlord-compliance, Property 28: IPI Co-Owner Proportional Split
    // Each co-owner's share equals ipi_total * porcentaje / 100.
    /// **Validates: Requirements 9.10**
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

    // Feature: dr-landlord-compliance, Property 28: IPI Co-Owner Proportional Split
    // A co-owner with 100% ownership gets the full IPI amount.
    /// **Validates: Requirements 9.10**
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

    // Feature: dr-landlord-compliance, Property 28: IPI Co-Owner Proportional Split
    // A co-owner with 0% ownership pays nothing.
    /// **Validates: Requirements 9.10**
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
