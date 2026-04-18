use proptest::prelude::*;
use std::collections::HashMap;

use crate::models::ocr::{OcrLine, OcrResult};

use super::ocr_mapping::{map_cedula, map_contrato, normalize_cedula};

fn expected_field_confidence(lines: &[OcrLine], value: &str) -> f64 {
    lines
        .iter()
        .filter(|l| l.text.contains(value) || value.contains(&l.text))
        .map(|l| l.confidence)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0)
}

fn is_cedula_format(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    parts[0].len() == 3
        && parts[1].len() == 7
        && parts[2].len() == 1
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

fn eleven_digits() -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..10, 11).prop_map(|digits| {
        digits.iter().map(|d| char::from(b'0' + d)).collect::<String>()
    })
}

fn eleven_digits_with_optional_dashes() -> impl Strategy<Value = String> {
    (eleven_digits(), any::<bool>()).prop_map(|(digits, insert_dashes)| {
        if insert_dashes {
            format!("{}-{}-{}", &digits[0..3], &digits[3..10], &digits[10..11])
        } else {
            digits
        }
    })
}

fn non_empty_alpha_string() -> impl Strategy<Value = String> {
    "[A-Za-z ]{1,30}".prop_map(|s| s.trim().to_string()).prop_filter("must be non-empty", |s| !s.is_empty())
}

// Feature: ocr-form-prefill, Property 6: map_cedula produces exactly the required fields
fn cedula_ocr_result() -> impl Strategy<Value = OcrResult> {
    (
        eleven_digits_with_optional_dashes(),
        non_empty_alpha_string(),
        non_empty_alpha_string(),
        0.5f64..1.0f64,
    )
        .prop_map(|(cedula, nombre, apellido, confidence)| {
            let mut structured_fields = HashMap::new();
            structured_fields.insert("cedula".to_string(), cedula.clone());
            structured_fields.insert("nombre".to_string(), nombre.clone());
            structured_fields.insert("apellido".to_string(), apellido.clone());

            let lines = vec![
                OcrLine {
                    text: cedula,
                    confidence,
                    bbox: vec![0.0, 0.0, 100.0, 20.0],
                },
                OcrLine {
                    text: nombre,
                    confidence,
                    bbox: vec![0.0, 20.0, 100.0, 40.0],
                },
                OcrLine {
                    text: apellido,
                    confidence,
                    bbox: vec![0.0, 40.0, 100.0, 60.0],
                },
            ];

            OcrResult {
                document_type: "cedula".to_string(),
                lines,
                structured_fields,
            }
        })
}

fn currency_prefix() -> impl Strategy<Value = String> {
    prop_oneof![Just("RD$".to_string()), Just("US$".to_string())]
}

fn monetary_amount() -> impl Strategy<Value = String> {
    (1u32..999_999, 0u32..100).prop_map(|(whole, cents)| {
        format!("{whole}.{cents:02}")
    })
}

fn dr_date() -> impl Strategy<Value = String> {
    (1u32..=28, 1u32..=12, 2020u32..=2035).prop_map(|(day, month, year)| {
        format!("{day:02}/{month:02}/{year}")
    })
}

// Feature: ocr-form-prefill, Property 7: map_contrato produces the required fields with graceful degradation
fn contrato_ocr_result_full() -> impl Strategy<Value = OcrResult> {
    (
        monetary_amount(),
        currency_prefix(),
        dr_date(),
        dr_date(),
        monetary_amount(),
        0.5f64..1.0f64,
    )
        .prop_map(|(monto, moneda, fecha_inicio, fecha_fin, deposito, confidence)| {
            let monto_with_prefix = format!("{moneda}{monto}");

            let mut structured_fields = HashMap::new();
            structured_fields.insert("monto_mensual".to_string(), monto_with_prefix.clone());
            structured_fields.insert("moneda".to_string(), moneda.clone());
            structured_fields.insert("fecha_inicio".to_string(), fecha_inicio.clone());
            structured_fields.insert("fecha_fin".to_string(), fecha_fin.clone());
            structured_fields.insert("deposito".to_string(), deposito.clone());

            let lines = vec![
                OcrLine {
                    text: monto_with_prefix,
                    confidence,
                    bbox: vec![0.0, 0.0, 100.0, 20.0],
                },
                OcrLine {
                    text: moneda,
                    confidence,
                    bbox: vec![0.0, 20.0, 100.0, 40.0],
                },
                OcrLine {
                    text: fecha_inicio,
                    confidence,
                    bbox: vec![0.0, 40.0, 100.0, 60.0],
                },
                OcrLine {
                    text: fecha_fin,
                    confidence,
                    bbox: vec![0.0, 60.0, 100.0, 80.0],
                },
                OcrLine {
                    text: deposito,
                    confidence,
                    bbox: vec![0.0, 80.0, 100.0, 100.0],
                },
            ];

            OcrResult {
                document_type: "contrato".to_string(),
                lines,
                structured_fields,
            }
        })
}

fn contrato_ocr_result_without_monto() -> impl Strategy<Value = OcrResult> {
    (
        currency_prefix(),
        dr_date(),
        dr_date(),
        monetary_amount(),
        0.5f64..1.0f64,
    )
        .prop_map(|(moneda, fecha_inicio, fecha_fin, deposito, confidence)| {
            let mut structured_fields = HashMap::new();
            structured_fields.insert("moneda".to_string(), moneda.clone());
            structured_fields.insert("fecha_inicio".to_string(), fecha_inicio.clone());
            structured_fields.insert("fecha_fin".to_string(), fecha_fin.clone());
            structured_fields.insert("deposito".to_string(), deposito.clone());

            let lines = vec![
                OcrLine {
                    text: moneda,
                    confidence,
                    bbox: vec![0.0, 0.0, 100.0, 20.0],
                },
                OcrLine {
                    text: fecha_inicio,
                    confidence,
                    bbox: vec![0.0, 20.0, 100.0, 40.0],
                },
                OcrLine {
                    text: fecha_fin,
                    confidence,
                    bbox: vec![0.0, 40.0, 100.0, 60.0],
                },
                OcrLine {
                    text: deposito,
                    confidence,
                    bbox: vec![0.0, 60.0, 100.0, 80.0],
                },
            ];

            OcrResult {
                document_type: "contrato".to_string(),
                lines,
                structured_fields,
            }
        })
}

// Feature: ocr-form-prefill, Property 8: Field confidence matches highest matching OCR line
fn confidence_cedula_ocr_result() -> impl Strategy<Value = (OcrResult, HashMap<String, f64>)> {
    (
        eleven_digits_with_optional_dashes(),
        non_empty_alpha_string(),
        non_empty_alpha_string(),
        prop::collection::vec(0.0f64..=1.0f64, 1..=3),
        prop::collection::vec(0.0f64..=1.0f64, 1..=3),
        prop::collection::vec(0.0f64..=1.0f64, 1..=3),
        prop::collection::vec(0.0f64..=1.0f64, 0..=3),
    )
        .prop_map(
            |(cedula, nombre, apellido, cedula_confs, nombre_confs, apellido_confs, noise_confs)| {
                let mut structured_fields = HashMap::new();
                structured_fields.insert("cedula".to_string(), cedula.clone());
                structured_fields.insert("nombre".to_string(), nombre.clone());
                structured_fields.insert("apellido".to_string(), apellido.clone());

                let mut lines = Vec::new();

                for (i, &conf) in cedula_confs.iter().enumerate() {
                    lines.push(OcrLine {
                        text: format!("prefix{i} {cedula} suffix{i}"),
                        confidence: conf,
                        bbox: vec![0.0, 0.0, 100.0, 20.0],
                    });
                }

                for (i, &conf) in nombre_confs.iter().enumerate() {
                    lines.push(OcrLine {
                        text: format!("line{i} {nombre} end{i}"),
                        confidence: conf,
                        bbox: vec![0.0, 20.0, 100.0, 40.0],
                    });
                }

                for (i, &conf) in apellido_confs.iter().enumerate() {
                    lines.push(OcrLine {
                        text: format!("row{i} {apellido} tail{i}"),
                        confidence: conf,
                        bbox: vec![0.0, 40.0, 100.0, 60.0],
                    });
                }

                for (i, &conf) in noise_confs.iter().enumerate() {
                    lines.push(OcrLine {
                        text: format!("XNOISE{i}UNRELATED{i}DATA"),
                        confidence: conf,
                        bbox: vec![0.0, 60.0, 100.0, 80.0],
                    });
                }

                let mut expected_confidences = HashMap::new();
                expected_confidences.insert(
                    "cedula".to_string(),
                    expected_field_confidence(&lines, &cedula),
                );
                expected_confidences.insert(
                    "nombre".to_string(),
                    expected_field_confidence(&lines, &nombre),
                );
                expected_confidences.insert(
                    "apellido".to_string(),
                    expected_field_confidence(&lines, &apellido),
                );

                let result = OcrResult {
                    document_type: "cedula".to_string(),
                    lines,
                    structured_fields,
                };

                (result, expected_confidences)
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    // Feature: ocr-form-prefill, Property 1: Cédula normalization is idempotent and format-preserving
    /// **Validates: Requirements 2.3, 4.4**
    #[test]
    fn cedula_normalization_idempotent_and_format_preserving(
        input in eleven_digits_with_optional_dashes()
    ) {
        let first = normalize_cedula(&input);

        prop_assert!(
            is_cedula_format(&first),
            "Expected NNN-NNNNNNN-N format, got: {first}"
        );

        let second = normalize_cedula(&first);
        prop_assert_eq!(
            &first,
            &second,
            "Idempotence violated: normalize({}) = {}, normalize({}) = {}",
            input, first, first, second
        );
    }

    // Feature: ocr-form-prefill, Property 6: map_cedula produces exactly the required fields
    /// **Validates: Requirements 4.2, 4.4**
    #[test]
    fn map_cedula_produces_required_fields(
        result in cedula_ocr_result()
    ) {
        let fields = map_cedula(&result).expect("map_cedula should not fail with valid structured_fields");

        prop_assert_eq!(
            fields.len(),
            3,
            "Expected exactly 3 fields, got {}",
            fields.len()
        );

        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        prop_assert_eq!(names, vec!["cedula", "nombre", "apellido"]);

        let labels: Vec<&str> = fields.iter().map(|f| f.label.as_str()).collect();
        prop_assert_eq!(labels, vec!["Cédula", "Nombre", "Apellido"]);

        let cedula_field = &fields[0];
        prop_assert!(
            is_cedula_format(&cedula_field.value),
            "Cédula value should be NNN-NNNNNNN-N format, got: {}",
            cedula_field.value
        );
    }

    // Feature: ocr-form-prefill, Property 7a: map_contrato with all fields present
    /// **Validates: Requirements 5.2, 5.4**
    #[test]
    fn map_contrato_produces_required_fields(
        result in contrato_ocr_result_full()
    ) {
        let fields = map_contrato(&result).expect("map_contrato should not fail with valid structured_fields");

        prop_assert_eq!(
            fields.len(),
            5,
            "Expected exactly 5 fields, got {}",
            fields.len()
        );

        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        prop_assert_eq!(
            names,
            vec!["monto_mensual", "moneda", "fecha_inicio", "fecha_fin", "deposito"]
        );

        let labels: Vec<&str> = fields.iter().map(|f| f.label.as_str()).collect();
        prop_assert_eq!(
            labels,
            vec!["Monto Mensual", "Moneda", "Fecha de Inicio", "Fecha de Fin", "Depósito"]
        );

        let monto_field = &fields[0];
        prop_assert!(
            !monto_field.value.is_empty(),
            "monto_mensual value should not be empty when field is present"
        );
        prop_assert!(
            monto_field.confidence > 0.0,
            "monto_mensual confidence should be > 0.0 when field is present, got {}",
            monto_field.confidence
        );
    }

    // Feature: ocr-form-prefill, Property 7b: map_contrato graceful degradation without monto_mensual
    /// **Validates: Requirements 5.2, 5.4**
    #[test]
    fn map_contrato_graceful_degradation_without_monto(
        result in contrato_ocr_result_without_monto()
    ) {
        let fields = map_contrato(&result).expect("map_contrato should not fail even without monto_mensual");

        prop_assert_eq!(
            fields.len(),
            5,
            "Expected exactly 5 fields even without monto_mensual, got {}",
            fields.len()
        );

        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        prop_assert_eq!(
            names,
            vec!["monto_mensual", "moneda", "fecha_inicio", "fecha_fin", "deposito"]
        );

        let monto_field = &fields[0];
        prop_assert_eq!(
            &monto_field.value,
            "",
            "monto_mensual value should be empty when absent, got: {}",
            monto_field.value
        );
        prop_assert!(
            monto_field.confidence == 0.0,
            "monto_mensual confidence should be 0.0 when absent, got {}",
            monto_field.confidence
        );
    }

    // Feature: ocr-form-prefill, Property 8: Field confidence matches highest matching OCR line
    /// **Validates: Requirements 4.3**
    #[test]
    fn field_confidence_matches_highest_matching_ocr_line(
        (result, expected_confidences) in confidence_cedula_ocr_result()
    ) {
        let fields = map_cedula(&result).expect("map_cedula should not fail");

        for field in &fields {
            let expected = expected_confidences
                .get(&field.name)
                .expect("expected confidence should exist for field");

            prop_assert!(
                (field.confidence - expected).abs() < f64::EPSILON,
                "Field '{}': expected confidence {}, got {}",
                field.name,
                expected,
                field.confidence
            );
        }
    }
}
