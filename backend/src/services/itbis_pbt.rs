#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::models::fiscal::TipoFiscal;
use crate::services::itbis::{calcular_itbis, calcular_retencion};

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a TipoFiscal that is "registered" (persona_juridica or persona_fisica).
fn registered_tipo_fiscal() -> impl Strategy<Value = TipoFiscal> {
    prop_oneof![
        Just(TipoFiscal::PersonaJuridica),
        Just(TipoFiscal::PersonaFisica),
    ]
}

/// Generate any TipoFiscal variant.
fn any_tipo_fiscal() -> impl Strategy<Value = TipoFiscal> {
    prop_oneof![
        Just(TipoFiscal::PersonaJuridica),
        Just(TipoFiscal::PersonaFisica),
        Just(TipoFiscal::Informal),
    ]
}

/// Generate a property type that is gravable (commercial or industrial).
fn gravable_tipo_propiedad() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("comercial"), Just("industrial"),]
}

/// Generate any property type (both gravable and non-gravable).
fn any_tipo_propiedad() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("comercial"),
        Just("industrial"),
        Just("residencial"),
        Just("terreno"),
        Just("mixto"),
    ]
}

/// Generate a positive monto_base as Decimal from i64.
/// Uses values from 1 to 10_000_000_00 (centavos) to represent 0.01 to 100,000,000.00
fn positive_monto_base() -> impl Strategy<Value = Decimal> {
    (1i64..10_000_000_00i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate a non-negative monto_base (includes zero).
fn non_negative_monto_base() -> impl Strategy<Value = Decimal> {
    (0i64..10_000_000_00i64).prop_map(|cents| Decimal::new(cents, 2))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 8: ITBIS Applicability
    // For any (tipo_fiscal, tipo_propiedad, monto_base), ITBIS = monto_base * 0.18
    // iff registered AND commercial/industrial; zero otherwise.
    /// **Validates: Requirements 6.1, 6.2, 6.3, 6.8**
    #[test]
    fn itbis_applies_only_when_registered_and_commercial(
        tipo_fiscal in any_tipo_fiscal(),
        tipo_propiedad in any_tipo_propiedad(),
        monto_base in non_negative_monto_base(),
    ) {
        let result = calcular_itbis(monto_base, tipo_propiedad, &tipo_fiscal, None);

        let is_registered = matches!(
            tipo_fiscal,
            TipoFiscal::PersonaJuridica | TipoFiscal::PersonaFisica
        );
        let is_gravable = matches!(tipo_propiedad, "comercial" | "industrial");

        let tasa_18 = Decimal::new(18, 2);

        if is_registered && is_gravable {
            // ITBIS must equal monto_base * 0.18
            let expected_itbis = monto_base * tasa_18;
            prop_assert_eq!(
                result.monto_itbis, expected_itbis,
                "ITBIS should be monto_base * 0.18 for registered + commercial/industrial. \
                 tipo_fiscal={:?}, tipo_propiedad={}, monto_base={}",
                tipo_fiscal, tipo_propiedad, monto_base
            );
            prop_assert_eq!(
                result.tasa, tasa_18,
                "Tasa should be 0.18 when ITBIS applies"
            );
        } else {
            // ITBIS must be zero
            prop_assert_eq!(
                result.monto_itbis, Decimal::ZERO,
                "ITBIS should be zero when not (registered AND commercial/industrial). \
                 tipo_fiscal={:?}, tipo_propiedad={}, monto_base={}",
                tipo_fiscal, tipo_propiedad, monto_base
            );
            prop_assert_eq!(
                result.tasa, Decimal::ZERO,
                "Tasa should be zero when ITBIS does not apply"
            );
        }
    }

    // Feature: dr-landlord-compliance, Property 8: ITBIS Applicability (positive case)
    // Specifically verifies that registered entities with commercial/industrial properties
    // always get ITBIS applied.
    /// **Validates: Requirements 6.1, 6.2, 6.3**
    #[test]
    fn itbis_always_applied_for_registered_commercial(
        tipo_fiscal in registered_tipo_fiscal(),
        tipo_propiedad in gravable_tipo_propiedad(),
        monto_base in positive_monto_base(),
    ) {
        let result = calcular_itbis(monto_base, tipo_propiedad, &tipo_fiscal, None);

        let tasa_18 = Decimal::new(18, 2);
        let expected_itbis = monto_base * tasa_18;

        prop_assert_eq!(
            result.monto_itbis, expected_itbis,
            "ITBIS must be monto_base * 0.18 for registered + gravable property"
        );
        prop_assert!(
            result.monto_itbis > Decimal::ZERO,
            "ITBIS must be positive for positive monto_base with registered + gravable"
        );
    }

    // Feature: dr-landlord-compliance, Property 8: ITBIS Applicability (negative case — informal)
    // Informal organizations never have ITBIS regardless of property type.
    /// **Validates: Requirements 6.8**
    #[test]
    fn itbis_zero_for_informal_any_property(
        tipo_propiedad in any_tipo_propiedad(),
        monto_base in positive_monto_base(),
    ) {
        let result = calcular_itbis(monto_base, tipo_propiedad, &TipoFiscal::Informal, None);

        prop_assert_eq!(
            result.monto_itbis, Decimal::ZERO,
            "ITBIS must be zero for informal orgs regardless of property type. \
             tipo_propiedad={}, monto_base={}",
            tipo_propiedad, monto_base
        );
    }

    // Feature: dr-landlord-compliance, Property 8: ITBIS Applicability (negative case — residential)
    // Residential properties never have ITBIS regardless of tipo_fiscal.
    /// **Validates: Requirements 6.2**
    #[test]
    fn itbis_zero_for_residential_any_fiscal_type(
        tipo_fiscal in registered_tipo_fiscal(),
        monto_base in positive_monto_base(),
    ) {
        let result = calcular_itbis(monto_base, "residencial", &tipo_fiscal, None);

        prop_assert_eq!(
            result.monto_itbis, Decimal::ZERO,
            "ITBIS must be zero for residential properties even with registered org. \
             tipo_fiscal={:?}, monto_base={}",
            tipo_fiscal, monto_base
        );
    }

    // Feature: dr-landlord-compliance, Property 9: Payment Amount Invariant
    // For any payment with ITBIS: monto_total == monto_base + monto_itbis exactly.
    /// **Validates: Requirements 6.4**
    #[test]
    fn payment_amount_invariant(
        tipo_fiscal in any_tipo_fiscal(),
        tipo_propiedad in any_tipo_propiedad(),
        monto_base in non_negative_monto_base(),
    ) {
        let result = calcular_itbis(monto_base, tipo_propiedad, &tipo_fiscal, None);

        // The invariant: monto_total == monto_base + monto_itbis (exact, no rounding error)
        prop_assert_eq!(
            result.monto_total,
            result.monto_base + result.monto_itbis,
            "monto_total must equal monto_base + monto_itbis exactly. \
             Got: total={}, base={}, itbis={}",
            result.monto_total, result.monto_base, result.monto_itbis
        );

        // Also verify monto_base is preserved
        prop_assert_eq!(
            result.monto_base, monto_base,
            "monto_base in result must match input"
        );
    }

    // Feature: dr-landlord-compliance, Property 9: Payment Amount Invariant (with custom rate)
    // The invariant holds even with a custom ITBIS rate.
    /// **Validates: Requirements 6.4**
    #[test]
    fn payment_amount_invariant_custom_rate(
        tipo_fiscal in registered_tipo_fiscal(),
        tipo_propiedad in gravable_tipo_propiedad(),
        monto_base in positive_monto_base(),
        rate_cents in 1i64..50i64,
    ) {
        let tasa = Decimal::new(rate_cents, 2);
        let result = calcular_itbis(monto_base, tipo_propiedad, &tipo_fiscal, Some(tasa));

        prop_assert_eq!(
            result.monto_total,
            result.monto_base + result.monto_itbis,
            "monto_total must equal monto_base + monto_itbis with custom rate. \
             rate={}, total={}, base={}, itbis={}",
            tasa, result.monto_total, result.monto_base, result.monto_itbis
        );

        // ITBIS should equal monto_base * tasa
        let expected_itbis = monto_base * tasa;
        prop_assert_eq!(
            result.monto_itbis, expected_itbis,
            "ITBIS should equal monto_base * custom_rate"
        );
    }

    // Feature: dr-landlord-compliance, Property 10: ITBIS Retention Split
    // For any ITBIS where tenant is persona_juridica:
    // monto_retenido = monto_itbis * 0.30 and neto = monto_itbis * 0.70
    /// **Validates: Requirements 6.7**
    #[test]
    fn itbis_retention_split_persona_juridica(
        monto_base in positive_monto_base(),
    ) {
        // First compute ITBIS for a commercial property with a registered org
        let itbis_result = calcular_itbis(
            monto_base,
            "comercial",
            &TipoFiscal::PersonaJuridica,
            None,
        );

        // Now compute retention (tenant is persona_juridica)
        let retencion = calcular_retencion(itbis_result.monto_itbis, &TipoFiscal::PersonaJuridica);

        let tasa_30 = Decimal::new(30, 2);
        let tasa_70 = Decimal::new(70, 2);

        let expected_retenido = itbis_result.monto_itbis * tasa_30;
        let expected_neto = itbis_result.monto_itbis * tasa_70;

        prop_assert_eq!(
            retencion.monto_retenido, expected_retenido,
            "monto_retenido must equal monto_itbis * 0.30. \
             monto_itbis={}, got retenido={}",
            itbis_result.monto_itbis, retencion.monto_retenido
        );

        prop_assert_eq!(
            retencion.monto_neto, expected_neto,
            "monto_neto must equal monto_itbis * 0.70. \
             monto_itbis={}, got neto={}",
            itbis_result.monto_itbis, retencion.monto_neto
        );

        // Additional invariant: retenido + neto == monto_itbis
        prop_assert_eq!(
            retencion.monto_retenido + retencion.monto_neto,
            itbis_result.monto_itbis,
            "retenido + neto must equal original monto_itbis"
        );
    }

    // Feature: dr-landlord-compliance, Property 10: ITBIS Retention Split
    // For non-persona_juridica tenants, no retention applies.
    /// **Validates: Requirements 6.7**
    #[test]
    fn itbis_no_retention_for_non_juridica_tenants(
        monto_itbis_cents in 1i64..10_000_000_00i64,
        tenant_tipo in prop_oneof![
            Just(TipoFiscal::PersonaFisica),
            Just(TipoFiscal::Informal),
        ],
    ) {
        let monto_itbis = Decimal::new(monto_itbis_cents, 2);
        let retencion = calcular_retencion(monto_itbis, &tenant_tipo);

        prop_assert_eq!(
            retencion.monto_retenido, Decimal::ZERO,
            "No retention for non-persona_juridica tenant. tenant_tipo={:?}",
            tenant_tipo
        );
        prop_assert_eq!(
            retencion.monto_neto, monto_itbis,
            "monto_neto must equal full monto_itbis when no retention. tenant_tipo={:?}",
            tenant_tipo
        );
    }
}
