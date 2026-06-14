#![allow(clippy::unwrap_used, clippy::expect_used)]

use proptest::prelude::*;

use crate::entities::organizacion;
use crate::models::fiscal::TipoFiscal;
use crate::services::fiscal::verificar_acceso_fiscal;
use crate::services::validacion_fiscal::{validar_cedula, validar_rnc};
use chrono::Utc;
use uuid::Uuid;

fn make_org(tipo_fiscal: &str) -> organizacion::Model {
    organizacion::Model {
        id: Uuid::new_v4(),
        tipo: "propietario".to_string(),
        nombre: "Test Org".to_string(),
        estado: "activo".to_string(),
        cedula: None,
        telefono: None,
        email_organizacion: None,
        rnc: None,
        razon_social: None,
        nombre_comercial: None,
        direccion_fiscal: None,
        representante_legal: None,
        dgii_data: None,
        tipo_fiscal: tipo_fiscal.to_string(),
        regimen_pagos: None,
        fecha_inicio_operaciones: None,
        is_ecf_certificado: false,
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }
}

fn rnc_prefix_8_digits() -> impl Strategy<Value = [u32; 8]> {
    prop::array::uniform8(0u32..10)
}

fn cedula_prefix_10_digits() -> impl Strategy<Value = [u32; 10]> {
    prop::array::uniform10(0u32..10)
}

fn rnc_mutation_index() -> impl Strategy<Value = usize> {
    0usize..9
}

fn cedula_mutation_index() -> impl Strategy<Value = usize> {
    0usize..11
}

fn digit_offset() -> impl Strategy<Value = u32> {
    1u32..10
}

fn tipo_fiscal_string() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("persona_juridica".to_string()),
        Just("persona_fisica".to_string()),
        Just("informal".to_string()),
    ]
}

fn invalid_rnc() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{1,8}",
        "[0-9]{10,15}",
        "[a-z][a-z0-9]{8}",
        Just(String::new()),
    ]
}

fn invalid_cedula() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{1,10}",
        "[0-9]{12,15}",
        "[a-z][a-z0-9]{10}",
        Just(String::new()),
    ]
}

fn compute_rnc_check_digit(prefix: &[u32; 8]) -> u32 {
    let weights: [u32; 8] = [7, 9, 8, 6, 5, 4, 3, 2];
    let sum: u32 = weights.iter().zip(prefix.iter()).map(|(w, d)| w * d).sum();
    let check = sum % 11;
    (10 - check) % 9 + 1
}

fn build_valid_rnc(prefix: &[u32; 8]) -> String {
    let check = compute_rnc_check_digit(prefix);
    let mut s = String::with_capacity(9);
    for d in prefix {
        s.push(char::from_digit(*d, 10).unwrap());
    }
    s.push(char::from_digit(check, 10).unwrap());
    s
}

fn compute_cedula_check_digit(prefix: &[u32; 10]) -> u32 {
    let weights: [u32; 10] = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
    let sum: u32 = weights
        .iter()
        .zip(prefix.iter())
        .map(|(w, d)| {
            let product = w * d;
            if product > 9 {
                product / 10 + product % 10
            } else {
                product
            }
        })
        .sum();
    (10 - (sum % 10)) % 10
}

fn build_valid_cedula(prefix: &[u32; 10]) -> String {
    let check = compute_cedula_check_digit(prefix);
    let mut s = String::with_capacity(11);
    for d in prefix {
        s.push(char::from_digit(*d, 10).unwrap());
    }
    s.push(char::from_digit(check, 10).unwrap());
    s
}

proptest! {
    #![proptest_config(ProptestConfig { cases: crate::test_support::pbt_cases(), ..Default::default() })]

    #[test]
    fn rnc_check_digit_round_trip(
        prefix in rnc_prefix_8_digits(),
        mutation_idx in rnc_mutation_index(),
        offset in digit_offset(),
    ) {
        let valid_rnc = build_valid_rnc(&prefix);

        let result = validar_rnc(&valid_rnc);
        prop_assert!(
            result.is_ok(),
            "RNC built from prefix {:?} should be valid, got error: {:?}",
            prefix,
            result.err()
        );

        let mut digits: Vec<u32> = valid_rnc
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        let original = digits[mutation_idx];
        digits[mutation_idx] = (original + offset) % 10;

        if digits[mutation_idx] != original {
            let mutated: String = digits
                .iter()
                .map(|d| char::from_digit(*d, 10).unwrap())
                .collect();
            let mutated_result = validar_rnc(&mutated);

            if mutated_result.is_ok() {
                let mutated_prefix: [u32; 8] = [
                    digits[0], digits[1], digits[2], digits[3],
                    digits[4], digits[5], digits[6], digits[7],
                ];
                let recomputed_check = compute_rnc_check_digit(&mutated_prefix);
                prop_assert_eq!(
                    digits[8],
                    recomputed_check,
                    "Mutated RNC '{}' passes validation but check digit doesn't match recomputed value",
                    mutated
                );
            }
        }
    }

    #[test]
    fn cedula_luhn_round_trip(
        prefix in cedula_prefix_10_digits(),
        mutation_idx in cedula_mutation_index(),
        offset in digit_offset(),
    ) {
        let valid_cedula = build_valid_cedula(&prefix);

        let result = validar_cedula(&valid_cedula);
        prop_assert!(
            result.is_ok(),
            "Cédula built from prefix {:?} should be valid, got error: {:?}",
            prefix,
            result.err()
        );

        let mut digits: Vec<u32> = valid_cedula
            .chars()
            .map(|c| c.to_digit(10).unwrap())
            .collect();
        let original = digits[mutation_idx];
        digits[mutation_idx] = (original + offset) % 10;

        if digits[mutation_idx] != original {
            let mutated: String = digits
                .iter()
                .map(|d| char::from_digit(*d, 10).unwrap())
                .collect();
            let mutated_result = validar_cedula(&mutated);

            if mutated_result.is_ok() {
                let mutated_prefix: [u32; 10] = [
                    digits[0], digits[1], digits[2], digits[3], digits[4],
                    digits[5], digits[6], digits[7], digits[8], digits[9],
                ];
                let recomputed_check = compute_cedula_check_digit(&mutated_prefix);
                prop_assert_eq!(
                    digits[10],
                    recomputed_check,
                    "Mutated cédula '{}' passes validation but check digit doesn't match recomputed value",
                    mutated
                );
            }
        }
    }

    #[test]
    fn fiscal_feature_access_gate(tipo in tipo_fiscal_string()) {
        let org = make_org(&tipo);
        let result = verificar_acceso_fiscal(&org);

        match tipo.as_str() {
            "persona_juridica" | "persona_fisica" => {
                prop_assert!(
                    result.is_ok(),
                    "tipo_fiscal='{}' should grant fiscal access, got: {:?}",
                    tipo,
                    result.err()
                );
            }
            "informal" => {
                prop_assert!(
                    result.is_err(),
                    "tipo_fiscal='informal' should deny fiscal access"
                );
                if let Err(ref e) = result {
                    let status = actix_web::error::ResponseError::status_code(e);
                    prop_assert_eq!(
                        status,
                        actix_web::http::StatusCode::FORBIDDEN,
                        "Informal access denial should be 403, got {}",
                        status
                    );
                }
            }
            _ => {
                prop_assert!(false, "Unexpected tipo_fiscal: {}", tipo);
            }
        }
    }

    #[test]
    fn tipo_fiscal_transition_requires_valid_identifier_persona_juridica(
        prefix in rnc_prefix_8_digits(),
    ) {
        let valid_rnc = build_valid_rnc(&prefix);

        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = Some(&valid_rnc);

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_ok(),
            "Valid RNC '{}' should pass transition validation, got: {:?}",
            valid_rnc,
            validation_result.err()
        );
    }

    #[test]
    fn tipo_fiscal_transition_requires_valid_identifier_persona_fisica(
        prefix in cedula_prefix_10_digits(),
    ) {
        let valid_cedula = build_valid_cedula(&prefix);

        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = Some(&valid_cedula);

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_ok(),
            "Valid cédula '{}' should pass transition validation, got: {:?}",
            valid_cedula,
            validation_result.err()
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_invalid_rnc(invalid in invalid_rnc()) {
        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = if invalid.is_empty() {
            None
        } else {
            Some(&invalid)
        };

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Invalid RNC '{}' should be rejected for transition to persona_juridica",
            invalid
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_invalid_cedula(invalid in invalid_cedula()) {
        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = if invalid.is_empty() {
            None
        } else {
            Some(&invalid)
        };

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Invalid cédula '{}' should be rejected for transition to persona_fisica",
            invalid
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_missing_identifier_persona_juridica(_seed in 0u32..10) {
        let nuevo_tipo = TipoFiscal::PersonaJuridica;
        let identificador: Option<&str> = None;

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaJuridica => {
                match identificador {
                    Some(rnc) => validar_rnc(rnc),
                    None => Err(crate::errors::AppError::Validation(
                        "RNC requerido para persona jurídica".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Missing identifier should be rejected for transition to persona_juridica"
        );
    }

    #[test]
    fn tipo_fiscal_transition_rejects_missing_identifier_persona_fisica(_seed in 0u32..10) {
        let nuevo_tipo = TipoFiscal::PersonaFisica;
        let identificador: Option<&str> = None;

        let validation_result = match &nuevo_tipo {
            TipoFiscal::PersonaFisica => {
                match identificador {
                    Some(cedula) => validar_cedula(cedula),
                    None => Err(crate::errors::AppError::Validation(
                        "Cédula requerida para persona física".to_string(),
                    )),
                }
            }
            _ => Ok(()),
        };

        prop_assert!(
            validation_result.is_err(),
            "Missing identifier should be rejected for transition to persona_fisica"
        );
    }
}
