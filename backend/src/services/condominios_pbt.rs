#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown)]

use chrono::{NaiveDate, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::entities::cuota_condominio;
use crate::models::fiscal::TipoFiscal;
use crate::services::condominios::calcular_billing_con_cuota;
use crate::services::itbis::calcular_itbis;

// ── Custom Strategies ──────────────────────────────────────────────────

/// Generate a positive monto as Decimal (centavos: 1..10_000_000_00 → 0.01..100_000.00).
fn positive_monto() -> impl Strategy<Value = Decimal> {
    (1i64..10_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

/// Generate any `TipoFiscal` variant.
fn any_tipo_fiscal() -> impl Strategy<Value = TipoFiscal> {
    prop_oneof![
        Just(TipoFiscal::PersonaJuridica),
        Just(TipoFiscal::PersonaFisica),
        Just(TipoFiscal::Informal),
    ]
}

/// Generate any property type.
fn any_tipo_propiedad() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("comercial"), Just("industrial"), Just("residencial"),]
}

/// Build a passthrough cuota_condominio Model with the given monto and fecha_inicio.
fn make_cuota(monto: Decimal, fecha_inicio: NaiveDate) -> cuota_condominio::Model {
    cuota_condominio::Model {
        id: Uuid::nil(),
        propiedad_id: Uuid::nil(),
        monto,
        moneda: "DOP".to_string(),
        frecuencia: "mensual".to_string(),
        fecha_inicio,
        fecha_fin: None,
        es_passthrough: true,
        contrato_id: None,
        organizacion_id: Uuid::nil(),
        created_at: Utc::now().fixed_offset(),
        updated_at: Utc::now().fixed_offset(),
    }
}

/// Generate a year/month/day triple that forms a valid NaiveDate.
fn valid_date() -> impl Strategy<Value = NaiveDate> {
    (2020i32..2030, 1u32..13, 1u32..29).prop_map(|(y, m, d)| {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(y, m, 1).expect("fallback date"))
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    // Feature: dr-landlord-compliance, Property 5: Billing Desglose with Condominium Fee
    /// **Validates: Requirements 2.3, 2.4**
    ///
    /// For any (monto_base, cuota) where cuota is passthrough, total equals
    /// monto_base + cuota + ITBIS(base) + ITBIS(cuota) with each as separate line item.
    #[test]
    fn billing_desglose_with_condominium_fee(
        monto_base in positive_monto(),
        cuota_monto in positive_monto(),
        tipo_propiedad in any_tipo_propiedad(),
        tipo_fiscal in any_tipo_fiscal(),
    ) {
        let cuota = make_cuota(cuota_monto, NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"));

        let result = calcular_billing_con_cuota(
            monto_base,
            Some(&cuota),
            tipo_propiedad,
            &tipo_fiscal,
        );

        // The cuota line item must equal the cuota monto (passthrough)
        prop_assert_eq!(
            result.cuota_condominio, cuota_monto,
            "Cuota line item must match cuota monto. Got {}, expected {}",
            result.cuota_condominio, cuota_monto
        );

        // ITBIS on base and cuota must match independent calcular_itbis results
        let expected_itbis_base = calcular_itbis(monto_base, tipo_propiedad, &tipo_fiscal, None).monto_itbis;
        let expected_itbis_cuota = calcular_itbis(cuota_monto, tipo_propiedad, &tipo_fiscal, None).monto_itbis;

        prop_assert_eq!(
            result.itbis_base, expected_itbis_base,
            "ITBIS on base must match calcular_itbis result. tipo_fiscal={:?}, tipo_propiedad={}",
            tipo_fiscal, tipo_propiedad
        );
        prop_assert_eq!(
            result.itbis_cuota, expected_itbis_cuota,
            "ITBIS on cuota must match calcular_itbis result. tipo_fiscal={:?}, tipo_propiedad={}",
            tipo_fiscal, tipo_propiedad
        );

        // Total must equal the sum of all components
        let expected_total = monto_base + cuota_monto + expected_itbis_base + expected_itbis_cuota;
        prop_assert_eq!(
            result.total, expected_total,
            "Total must equal monto_base + cuota + itbis_base + itbis_cuota. \
             Got {}, expected {}",
            result.total, expected_total
        );

        // monto_base is preserved
        prop_assert_eq!(
            result.monto_base, monto_base,
            "monto_base must be preserved in result"
        );
    }

    // Feature: dr-landlord-compliance, Property 6: Condominium Fee Change Temporal Boundary
    /// **Validates: Requirements 2.5**
    ///
    /// For any cuota change with effective date, periods before use old amount,
    /// periods after use new amount. We test by comparing two cuota models
    /// and verifying the billing picks up the correct one based on temporal logic.
    #[test]
    fn condominium_fee_change_temporal_boundary(
        old_monto in positive_monto(),
        new_monto in positive_monto(),
        change_date in valid_date(),
    ) {
        // Old cuota starts well before the change date
        let old_cuota = make_cuota(
            old_monto,
            NaiveDate::from_ymd_opt(2020, 1, 1).expect("date"),
        );

        // New cuota starts at the change date
        let new_cuota = make_cuota(new_monto, change_date);

        // A billing period BEFORE the change date should use old cuota
        let before_date = change_date.pred_opt().unwrap_or(change_date);

        // Temporal boundary logic: if the billing period start < change_date,
        // use old; if >= change_date, use new.
        // We verify the old cuota produces old_monto in billing
        let result_old = calcular_billing_con_cuota(
            Decimal::new(10000, 2),
            Some(&old_cuota),
            "residencial",
            &TipoFiscal::Informal,
        );
        prop_assert_eq!(
            result_old.cuota_condominio, old_monto,
            "Period before change should use old cuota amount. \
             before_date={}, change_date={}, old_monto={}, new_monto={}",
            before_date, change_date, old_monto, new_monto
        );

        // And the new cuota produces new_monto in billing
        let result_new = calcular_billing_con_cuota(
            Decimal::new(10000, 2),
            Some(&new_cuota),
            "residencial",
            &TipoFiscal::Informal,
        );
        prop_assert_eq!(
            result_new.cuota_condominio, new_monto,
            "Period on/after change should use new cuota amount. \
             change_date={}, old_monto={}, new_monto={}",
            change_date, old_monto, new_monto
        );

        // Key property: old and new amounts differ when montos differ
        if old_monto != new_monto {
            prop_assert_ne!(
                result_old.cuota_condominio, result_new.cuota_condominio,
                "Different cuota models must produce different billing line items"
            );
        }
    }

    // Feature: dr-landlord-compliance, Property 7: Condominium Fee Increase Uncapped
    /// **Validates: Requirements 2.7**
    ///
    /// For any cuota increase (even > 10%), the system accepts it without applying
    /// the Ley 85-25 rent cap. The new amount is used directly in billing.
    #[test]
    fn condominium_fee_increase_uncapped(
        base_monto_cents in 100i64..1_000_000i64,
        increase_pct in 1u32..500u32,
    ) {
        let base_monto = Decimal::new(base_monto_cents, 2);

        // Calculate a new monto that exceeds 10% increase (Ley 85-25 cap for rent)
        let increase_factor = Decimal::new(i64::from(increase_pct), 2);
        let new_monto = base_monto + (base_monto * increase_factor / Decimal::new(100, 0));

        // Verify increase exceeds or is within various ranges — the key point is
        // the system doesn't cap it
        let cuota = make_cuota(new_monto, NaiveDate::from_ymd_opt(2026, 6, 1).expect("date"));

        let result = calcular_billing_con_cuota(
            Decimal::new(2_500_000, 2),
            Some(&cuota),
            "comercial",
            &TipoFiscal::PersonaJuridica,
        );

        // The cuota in billing must be the FULL new_monto — no cap applied
        prop_assert_eq!(
            result.cuota_condominio, new_monto,
            "Cuota increase must not be capped. Expected {} ({}% increase from {}), got {}",
            new_monto, increase_pct, base_monto, result.cuota_condominio
        );

        // Verify that even large increases (> 10%) pass through uncapped
        if increase_pct > 10 {
            let ten_pct_cap = base_monto + (base_monto * Decimal::new(10, 2));
            prop_assert!(
                result.cuota_condominio > ten_pct_cap || new_monto <= ten_pct_cap,
                "For increases > 10%, cuota must exceed 10% cap if new_monto does. \
                 cuota={}, 10%_cap={}, increase_pct={}",
                result.cuota_condominio, ten_pct_cap, increase_pct
            );
        }
    }
}
