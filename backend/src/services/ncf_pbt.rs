#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

use crate::models::ncf::TipoNCF;
use crate::services::ncf::validar_formato_ncf;

const ALERTA_UMBRAL: f64 = 0.80;

fn tipo_ncf_code(tipo: &TipoNCF) -> &'static str {
    match tipo {
        TipoNCF::B01 => "01",
        TipoNCF::B02 => "02",
        TipoNCF::B14 => "14",
        TipoNCF::B15 => "15",
    }
}

fn build_ncf(prefijo: char, tipo: &TipoNCF, numero: i32) -> String {
    let tipo_code = tipo_ncf_code(tipo);
    format!("{}{}{:08}", prefijo, tipo_code, numero)
}

fn should_alert(siguiente_numero: i32, rango_desde: i32, rango_hasta: i32) -> bool {
    let rango_total = rango_hasta - rango_desde;
    if rango_total <= 0 {
        return false;
    }
    let consumido = siguiente_numero - rango_desde;
    let porcentaje = f64::from(consumido) / f64::from(rango_total);
    porcentaje >= ALERTA_UMBRAL
}

fn arb_tipo_ncf() -> impl Strategy<Value = TipoNCF> {
    prop_oneof![
        Just(TipoNCF::B01),
        Just(TipoNCF::B02),
        Just(TipoNCF::B14),
        Just(TipoNCF::B15),
    ]
}

fn arb_prefijo() -> impl Strategy<Value = char> {
    prop_oneof![Just('B'), Just('E')]
}

fn arb_sequence_len() -> impl Strategy<Value = usize> {
    1usize..20
}

fn arb_start_number() -> impl Strategy<Value = i32> {
    1i32..99_999_990
}

fn arb_range() -> impl Strategy<Value = (i32, i32)> {
    (1i32..99_999_000).prop_flat_map(|desde| {
        let max_hasta = (desde + 100_000).min(99_999_999);
        (Just(desde), (desde + 1)..=max_hasta)
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn ncf_sequential_gapless_generation(
        prefijo in arb_prefijo(),
        tipo in arb_tipo_ncf(),
        start in arb_start_number(),
        len in arb_sequence_len(),
    ) {
        let mut generated: Vec<i32> = Vec::with_capacity(len);
        let mut siguiente_numero = start;

        for _ in 0..len {
            generated.push(siguiente_numero);
            siguiente_numero += 1;
        }

        for i in 0..generated.len() - 1 {
            let current = generated[i];
            let next = generated[i + 1];
            prop_assert_eq!(
                next,
                current + 1,
                "Gap detected: NCF at position {} is {}, but position {} is {} (expected {})",
                i, current, i + 1, next, current + 1
            );
        }

        for &num in &generated {
            let ncf = build_ncf(prefijo, &tipo, num);
            let result = validar_formato_ncf(&ncf);
            prop_assert!(
                result.is_ok(),
                "Generated NCF '{}' (prefijo={}, tipo={}, num={}) should be format-valid, got: {:?}",
                ncf, prefijo, tipo, num, result.err()
            );
        }
    }

    #[test]
    fn ncf_format_compliance(
        prefijo in arb_prefijo(),
        tipo in arb_tipo_ncf(),
        numero in 1i32..99_999_999,
    ) {
        let ncf = build_ncf(prefijo, &tipo, numero);

        let result = validar_formato_ncf(&ncf);
        prop_assert!(
            result.is_ok(),
            "NCF '{}' should match ^[A-Z]\\d{{10}}$, got error: {:?}",
            ncf, result.err()
        );

        let chars: Vec<char> = ncf.chars().collect();

        prop_assert_eq!(
            chars.len(),
            11,
            "NCF '{}' should be exactly 11 chars, got {}",
            ncf, chars.len()
        );

        prop_assert!(
            chars[0].is_ascii_uppercase(),
            "NCF '{}' first char '{}' should be uppercase letter",
            ncf, chars[0]
        );
        prop_assert_eq!(
            chars[0],
            prefijo,
            "NCF '{}' prefix should be '{}', got '{}'",
            ncf, prefijo, chars[0]
        );

        for (i, &c) in chars[1..].iter().enumerate() {
            prop_assert!(
                c.is_ascii_digit(),
                "NCF '{}' char at position {} ('{}') should be a digit",
                ncf, i + 1, c
            );
        }

        match prefijo {
            'E' => prop_assert_eq!(chars[0], 'E', "e-CF NCF should start with 'E'"),
            'B' => prop_assert_eq!(chars[0], 'B', "Physical NCF should start with 'B'"),
            _ => prop_assert!(false, "Unexpected prefix: {}", prefijo),
        }
    }

    #[test]
    fn ncf_range_boundary_enforcement(
        (rango_desde, rango_hasta) in arb_range(),
        offset_ratio in 0.0f64..1.5,
    ) {
        let rango_total = rango_hasta - rango_desde;

        let offset = (f64::from(rango_total) * offset_ratio) as i32;
        let siguiente_numero = rango_desde + offset;

        let is_exhausted = siguiente_numero > rango_hasta;

        if is_exhausted {
            prop_assert!(
                siguiente_numero > rango_hasta,
                "Expected exhaustion: siguiente_numero={} should be > rango_hasta={}",
                siguiente_numero, rango_hasta
            );
        } else {
            prop_assert!(
                siguiente_numero <= rango_hasta,
                "Expected valid: siguiente_numero={} should be <= rango_hasta={}",
                siguiente_numero, rango_hasta
            );
        }

        let expected_alert = should_alert(siguiente_numero, rango_desde, rango_hasta);

        if rango_total > 0 {
            let consumido = siguiente_numero - rango_desde;
            let porcentaje = f64::from(consumido) / f64::from(rango_total);

            if porcentaje >= ALERTA_UMBRAL {
                prop_assert!(
                    expected_alert,
                    "At {:.1}% consumption (siguiente={}, desde={}, hasta={}), alert should trigger",
                    porcentaje * 100.0, siguiente_numero, rango_desde, rango_hasta
                );
            } else {
                prop_assert!(
                    !expected_alert,
                    "At {:.1}% consumption (siguiente={}, desde={}, hasta={}), alert should NOT trigger",
                    porcentaje * 100.0, siguiente_numero, rango_desde, rango_hasta
                );
            }
        }

        if !is_exhausted && siguiente_numero >= 1 {
            let ncf = build_ncf('B', &TipoNCF::B01, siguiente_numero);
            let result = validar_formato_ncf(&ncf);
            prop_assert!(
                result.is_ok(),
                "NCF '{}' at position {} within range [{}, {}] should be format-valid, got: {:?}",
                ncf, siguiente_numero, rango_desde, rango_hasta, result.err()
            );
        }
    }
}
