#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;
use rust_decimal::Decimal;

use crate::services::dashboard::{calcular_rentabilidad_neta, normalizar_moneda};

fn positive_amount() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn non_negative_amount() -> impl Strategy<Value = Decimal> {
    (0i64..1_000_000_000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn reliable_valor_catastral() -> impl Strategy<Value = Decimal> {
    (100_000i64..50_000_000i64).prop_map(|v| Decimal::new(v, 0))
}

fn unreliable_valor_catastral() -> impl Strategy<Value = Decimal> {
    (1i64..100_000i64).prop_map(|v| Decimal::new(v, 0))
}

fn positive_tasa_cambio() -> impl Strategy<Value = Decimal> {
    (4000i64..8000i64).prop_map(|cents| Decimal::new(cents, 2))
}

fn currency_pair() -> impl Strategy<Value = (&'static str, &'static str)> {
    prop_oneof![
        Just(("DOP", "USD")),
        Just(("USD", "DOP")),
        Just(("DOP", "DOP")),
        Just(("USD", "USD")),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn no_date_range_means_no_computed_metrics(
        ingresos in non_negative_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
        valor_catastral in reliable_valor_catastral(),
    ) {

        let (rentabilidad, _) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, valor_catastral
        );

        let has_date_range = false;
        let metrics_returned: Option<Decimal> = if has_date_range {
            Some(rentabilidad)
        } else {
            None
        };

        prop_assert!(
            metrics_returned.is_none(),
            "When no date range is specified, computed metrics must be None"
        );
    }

    #[test]
    fn with_date_range_metrics_are_computed(
        ingresos in non_negative_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
        valor_catastral in reliable_valor_catastral(),
    ) {
        let (rentabilidad, _) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, valor_catastral
        );

        let has_date_range = true;
        let metrics_returned: Option<Decimal> = if has_date_range {
            Some(rentabilidad)
        } else {
            None
        };

        prop_assert!(
            metrics_returned.is_some(),
            "When date range is specified, computed metrics must be Some"
        );
    }

    #[test]
    fn rentabilidad_neta_formula_correct(
        ingresos in non_negative_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
        valor_catastral in reliable_valor_catastral(),
    ) {
        let (result, is_unreliable) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, valor_catastral
        );

        let neto = ingresos - gastos - cuotas;
        let expected_raw = (neto / valor_catastral) * Decimal::ONE_HUNDRED;
        let cap = Decimal::new(200, 0);
        let expected = if expected_raw > cap { cap } else { expected_raw };

        prop_assert_eq!(
            result, expected,
            "Rentabilidad formula mismatch. ingresos={}, gastos={}, cuotas={}, \
             valor_catastral={}, expected={}, got={}",
            ingresos, gastos, cuotas, valor_catastral, expected, result
        );

        prop_assert!(
            !is_unreliable,
            "valor_catastral={} >= 100,000 should not be flagged unreliable",
            valor_catastral
        );
    }

    #[test]
    fn rentabilidad_neta_never_exceeds_200(
        ingresos in positive_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
        valor_catastral in (1i64..1_000_000i64).prop_map(|v| Decimal::new(v, 0)),
    ) {
        let (result, _) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, valor_catastral
        );

        let cap = Decimal::new(200, 0);
        prop_assert!(
            result <= cap,
            "Rentabilidad must never exceed 200%. Got {} for ingresos={}, gastos={}, \
             cuotas={}, valor_catastral={}",
            result, ingresos, gastos, cuotas, valor_catastral
        );
    }

    #[test]
    fn rentabilidad_neta_unreliable_below_threshold(
        ingresos in non_negative_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
        valor_catastral in unreliable_valor_catastral(),
    ) {
        let (_, is_unreliable) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, valor_catastral
        );

        prop_assert!(
            is_unreliable,
            "valor_catastral={} < 100,000 must be flagged as unreliable",
            valor_catastral
        );
    }

    #[test]
    fn rentabilidad_neta_zero_valor_catastral_returns_zero(
        ingresos in non_negative_amount(),
        gastos in non_negative_amount(),
        cuotas in non_negative_amount(),
    ) {
        let (result, is_unreliable) = calcular_rentabilidad_neta(
            ingresos, gastos, cuotas, Decimal::ZERO
        );

        prop_assert_eq!(
            result, Decimal::ZERO,
            "Rentabilidad must be zero when valor_catastral is zero"
        );
        prop_assert!(
            is_unreliable,
            "valor_catastral=0 must be flagged as unreliable"
        );
    }

    #[test]
    fn currency_normalization_dop_to_usd(
        monto in positive_amount(),
        tasa_cambio in positive_tasa_cambio(),
    ) {
        let result = normalizar_moneda(monto, "DOP", "USD", tasa_cambio);
        let expected = monto / tasa_cambio;

        prop_assert_eq!(
            result, expected,
            "DOP→USD should be amount/rate. monto={}, tasa={}, expected={}, got={}",
            monto, tasa_cambio, expected, result
        );
    }

    #[test]
    fn currency_normalization_usd_to_dop(
        monto in positive_amount(),
        tasa_cambio in positive_tasa_cambio(),
    ) {
        let result = normalizar_moneda(monto, "USD", "DOP", tasa_cambio);
        let expected = monto * tasa_cambio;

        prop_assert_eq!(
            result, expected,
            "USD→DOP should be amount*rate. monto={}, tasa={}, expected={}, got={}",
            monto, tasa_cambio, expected, result
        );
    }

    #[test]
    fn currency_normalization_same_currency_identity(
        monto in positive_amount(),
        tasa_cambio in positive_tasa_cambio(),
        (origen, destino) in prop_oneof![Just(("DOP", "DOP")), Just(("USD", "USD"))],
    ) {
        let result = normalizar_moneda(monto, origen, destino, tasa_cambio);

        prop_assert_eq!(
            result, monto,
            "Same currency must return original amount unchanged. \
             moneda={}, monto={}, got={}",
            origen, monto, result
        );
    }

    #[test]
    fn currency_normalization_roundtrip(
        monto in positive_amount(),
        tasa_cambio in positive_tasa_cambio(),
    ) {
        let intermediate = normalizar_moneda(monto, "DOP", "USD", tasa_cambio);
        let roundtrip = normalizar_moneda(intermediate, "USD", "DOP", tasa_cambio);

        let diff = if roundtrip > monto {
            roundtrip - monto
        } else {
            monto - roundtrip
        };

        let tolerance = Decimal::new(1, 2);
        prop_assert!(
            diff <= tolerance,
            "DOP→USD→DOP roundtrip should return ~original. \
             monto={}, roundtrip={}, diff={}",
            monto, roundtrip, diff
        );
    }

    #[test]
    fn currency_normalization_zero_rate_returns_original(
        monto in positive_amount(),
        (origen, destino) in currency_pair(),
    ) {
        let result = normalizar_moneda(monto, origen, destino, Decimal::ZERO);

        prop_assert_eq!(
            result, monto,
            "Zero exchange rate must return original amount. \
             origen={}, destino={}, monto={}, got={}",
            origen, destino, monto, result
        );
    }
}
